//! Map-lifetime volume uniforms, selectors and fog pipelines.
use super::*;
use crate::fog_volumes::{FOG_SLOTS, Mode, Table};
use sjk_shader::{FogPass, ShaderCull, ShaderDefinition};
use std::cell::{Cell, OnceCell};

/// The fog key's depth-bias bit for a `polygonOffset` shader. rd-vanilla runs
/// `RB_FogPass` before `RB_StageIteratorGeneric` disables `GL_POLYGON_OFFSET_FILL`, so
/// a decal's fog is offset like its stages; unbiased, its `Equal` test fights with the
/// surface under it pixel by pixel.
pub(super) fn polygon_offset(definition: Option<&ShaderDefinition>) -> u8 {
    if definition.is_some_and(|d| d.polygon_offset) {
        crate::world_stage::POLYGON_OFFSET
    } else {
        0
    }
}

/// GPU resources allocated once, with selectors aligned for dynamic uniform offsets.
pub(super) struct FogGpu {
    pub(super) table: Table,
    /// The table's GPU copy: rewritten only when Low FPS Night's level changes.
    buffer: wgpu::Buffer,
    /// Low FPS Night's level (as seen) the GPU copy's colours are scaled by; `None`: the
    /// map's own colours.
    night: Cell<Option<f32>>,
    group: wgpu::BindGroup,
    stride: u32,
    device: wgpu::Device,
    layout: wgpu::PipelineLayout,
    shader: wgpu::ShaderModule,
    /// The colour target's format, from the first key.
    format: Option<wgpu::TextureFormat>,
    keys: Vec<PipelineKey>,
    /// Per key, the world/mover and the entity variant: compiled with the key at map
    /// load, on a worker for a model loaded later ([`super::pipeline_jobs`]), or on
    /// first draw.
    mover: Vec<OnceCell<wgpu::RenderPipeline>>,
    entity: Vec<OnceCell<wgpu::RenderPipeline>>,
}

impl FogGpu {
    /// Upload the immutable fog table and aligned selector block once.
    pub(super) fn new(device: &wgpu::Device, camera: &wgpu::BindGroupLayout, table: Table) -> Self {
        let entries = [uniform_entry(0, false), uniform_entry(1, true)];
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SJK fog volume layout"),
            entries: &entries,
        });
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SJK map fog table"),
            contents: bytemuck::cast_slice(&table.entries),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let stride = device.limits().min_uniform_buffer_offset_alignment.max(16);
        let mut selectors = vec![0_u8; stride as usize * FOG_SLOTS * 2];
        for index in 0..FOG_SLOTS * 2 {
            let offset = index * stride as usize;
            let fog = (index % FOG_SLOTS) as u32;
            let global_exp2 = (index / FOG_SLOTS) as u32;
            selectors[offset..offset + 4].copy_from_slice(&fog.to_le_bytes());
            selectors[offset + 4..offset + 8].copy_from_slice(&global_exp2.to_le_bytes());
        }
        let selector = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SJK immutable fog selectors"),
            contents: &selectors,
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SJK fog volumes"),
            layout: &bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &selector,
                        offset: 0,
                        size: NonZeroU64::new(16),
                    }),
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SJK fog pipeline layout"),
            bind_group_layouts: &[
                Some(camera),
                Some(&bind_layout),
                Some(&crate::shared_geometry::quads::layout(device)),
                Some(&crate::world_stage::geometry_uniform::layout(device)),
            ],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("SJK RB_FogPass"),
            source: wgpu::ShaderSource::Wgsl(crate::fog_volumes::SHADER.into()),
        });
        Self {
            table,
            buffer,
            night: Cell::new(None),
            group,
            stride,
            device: device.clone(),
            layout,
            shader,
            format: None,
            keys: Vec::new(),
            mover: Vec::new(),
            entity: Vec::new(),
        }
    }

    /// Low FPS Night (`r_lowFpsNight`): the fog colours darkened to `level` (as seen, the
    /// bake's level), so fog does not glow in the dark map; `None` gives the map's own
    /// back. The table is rewritten only when the level changes, never per frame.
    pub(super) fn set_night(&self, queue: &crate::frame_queue::FrameQueue, level: Option<f32>) {
        if self.night.get() == level {
            return;
        }
        self.night.set(level);
        let entries = night_fogs(&self.table, level);
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&entries));
    }

    /// The colour the world pass clears to: the map's global fog
    /// ([`Table::global_color`]) under Low FPS Night's level.
    pub(super) fn clear_color(&self) -> Option<[f32; 3]> {
        let scale = self.night.get().unwrap_or(1.);
        self.table
            .global_color()
            .map(|color| color.map(|value| value * scale))
    }

    pub(super) fn pipeline_with_geometry(
        &mut self,
        _device: &wgpu::Device,
        format: wgpu::TextureFormat,
        fog_pass: FogPass,
        cull: ShaderCull,
        geometry: u8,
    ) -> Option<usize> {
        let index = self.slot(format, fog_pass, cull, geometry)?;
        self.mover(index);
        self.entity(index);
        Some(index)
    }

    /// The key index of a fog pass, registered without compiling when new.
    pub(super) fn slot(
        &mut self,
        format: wgpu::TextureFormat,
        fog_pass: FogPass,
        cull: ShaderCull,
        geometry: u8,
    ) -> Option<usize> {
        if fog_pass == FogPass::None {
            return None;
        }
        self.format = Some(format);
        // RB_FogPass, tr_shade.cpp:1120-1125: alpha blend, no depth write.
        let key = PipelineKey {
            geometry,
            source: wgpu::BlendFactor::SrcAlpha,
            destination: wgpu::BlendFactor::OneMinusSrcAlpha,
            depth_write: false,
            depth: if fog_pass == FogPass::Equal {
                wgpu::CompareFunction::Equal
            } else {
                wgpu::CompareFunction::LessEqual
            },
            cull: crate::world_stage::cull_face(cull),
        };
        if let Some(index) = self.keys.iter().position(|known| *known == key) {
            return Some(index);
        }
        let index = self.keys.len();
        self.keys.push(key);
        self.mover.push(OnceCell::new());
        self.entity.push(OnceCell::new());
        Some(index)
    }

    /// The world/mover variant of key `index`. Static world fog draws bind the identity
    /// instance and share the mover program: one driver compile per key instead of two.
    fn mover(&self, index: usize) -> &wgpu::RenderPipeline {
        self.mover[index].get_or_init(|| self.make(index, false)())
    }

    fn entity(&self, index: usize) -> &wgpu::RenderPipeline {
        self.entity[index].get_or_init(|| self.make(index, true)())
    }

    /// What compiles the `entity` or world/mover variant of key `index`, on any thread.
    fn make(
        &self,
        index: usize,
        entity: bool,
    ) -> impl FnOnce() -> wgpu::RenderPipeline + Send + 'static {
        let device = self.device.clone();
        let layout = self.layout.clone();
        let shader = self.shader.clone();
        let format = self.format.expect("a fog key carries its format");
        let key = self.keys[index];
        move || {
            if entity {
                create_entity_pipeline(&device, &layout, &shader, format, key, true)
            } else {
                create_pipeline(&device, &layout, &shader, format, key, true)
            }
        }
    }

    /// Jobs for the variants of key `index` not compiled yet.
    pub(super) fn jobs(&self, index: usize, jobs: &mut super::pipeline_jobs::Jobs) {
        use super::pipeline_jobs::Slot;
        for entity in [false, true] {
            let cell = if entity {
                &self.entity[index]
            } else {
                &self.mover[index]
            };
            if cell.get().is_none() {
                jobs.push(Slot::Fog { index, entity }, self.make(index, entity));
            }
        }
    }

    /// Put a pipeline a worker compiled in its slot; a slot filled meanwhile keeps its own.
    pub(super) fn install(&self, index: usize, entity: bool, pipeline: wgpu::RenderPipeline) {
        let cell = if entity {
            &self.entity[index]
        } else {
            &self.mover[index]
        };
        let _ = cell.set(pipeline);
    }

    /// Select a table index without modifying or allocating a GPU buffer.
    pub(super) fn bind<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        fog: u32,
        mode: Mode,
    ) {
        let bank = u32::from(mode == Mode::GlobalExp2) * FOG_SLOTS as u32;
        pass.set_bind_group(1, &self.group, &[(fog + bank) * self.stride]);
    }

    /// Bind the world, mover or entity vertex variant for the selected material.
    pub(super) fn set_pipeline<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        index: usize,
        kind: usize,
    ) {
        pass.set_pipeline(match kind {
            0 | 1 => self.mover(index),
            _ => self.entity(index),
        });
    }
}

fn uniform_entry(binding: u32, dynamic: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: dynamic,
            min_binding_size: None,
        },
        count: None,
    }
}

/// The fog table's entries with every fog's colour scaled by Low FPS Night's `level`
/// (none: as the map has them).
fn night_fogs(table: &Table, level: Option<f32>) -> [crate::fog_volumes::GpuFog; FOG_SLOTS] {
    let mut entries = table.entries;
    let scale = level.unwrap_or(1.).clamp(0., 1.);
    for fog in &mut entries[1..=table.count] {
        for value in &mut fog.color[..3] {
            *value *= scale;
        }
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::Zeroable;

    /// Low FPS Night darkens every fog's colour, global or brush, and leaves its density,
    /// bounds and marker alone; no level gives the map's own back.
    #[test]
    fn low_fps_night_darkens_the_fog_colours_only() {
        let mut table = Table {
            entries: [crate::fog_volumes::GpuFog::zeroed(); FOG_SLOTS],
            count: 2,
        };
        table.entries[1].color = [0.8, 0.6, 0.4, 1.0];
        table.entries[1].bounds_min = [1.0, 2.0, 3.0, 0.25];
        table.entries[2].color = [0.2, 0.4, 1.0, 0.0];
        let night = night_fogs(&table, Some(0.5));
        assert_eq!(night[1].color, [0.4, 0.3, 0.2, 1.0]);
        assert_eq!(night[1].bounds_min, table.entries[1].bounds_min);
        assert_eq!(night[2].color, [0.1, 0.2, 0.5, 0.0]);
        assert_eq!(night_fogs(&table, None)[1].color, table.entries[1].color);
    }
}
