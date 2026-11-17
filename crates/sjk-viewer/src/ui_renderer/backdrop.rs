//! The acrylic backdrop's blur ([`sjk_ui::DrawCommand::Backdrop`]): the frame's image under
//! the 2D layer, blurred at reduced resolution, which the shapes' shader then samples under
//! each backdrop's tint (`ui_shapes.wgsl`, [`sjk_ui::acrylic`]).
//!
//! - **What is blurred:** the image the 2D layer draws into, as it stands before the 2D
//!   layer: the world with its post-processing and flares. The 2D drawn under a backdrop is
//!   not in it: the backdrop covers it, as frosted glass covers what stands behind it.
//! - **How:** a dual Kawase chain (Bjørge, SIGGRAPH 2015, "Bandwidth-efficient rendering"):
//!   the frame to a quarter of its size in one pass (a 4x4 mean), halved [`levels`] - 1
//!   times more, then drawn back up to the quarter, every pass a fixed few taps
//!   (`ui_backdrop.wgsl`). The levels follow the window's height, so the blur reaches as
//!   far on any screen: the smallest is about [`SMALLEST_LINES`] lines tall (three levels at
//!   1080 lines, four at 2160).
//! - **Where:** only round the backdrops the frame draws: every pass is cut to their
//!   rectangles grown by the chain's [`margin`], so the two status bars blur two strips of
//!   the frame, not all of it ([`Regions`]).
//! - **When:** only on frames whose 2D layer draws a backdrop, and only when the frame's
//!   image can be sampled: a post-processing intermediate, or a swapchain image the surface
//!   lets shaders read ([`surface_usage`]: Vulkan and Metal do, DirectX 12's swapchains do
//!   not). Without it a backdrop draws its plain tint ([`sjk_ui::acrylic::fallback`]), as
//!   it does in the CPU menu snapshots.
//!
//! The textures are made when the window's size changes, never per frame; a frame with a
//! backdrop makes one bind group, for its swapchain image's view.

use sjk_ui::Rect;

/// The most separate regions a frame blurs; past that, their union.
const MAX_REGIONS: usize = 16;

/// Lines the smallest level holds, about: the blur's reach as a share of the screen.
pub(crate) const SMALLEST_LINES: f32 = 68.0;

/// The most levels, for a window over 4K.
const MAX_LEVELS: usize = 5;

/// The levels' format: display values, as everything the 2D layer samples.
const FORMAT: wgpu::TextureFormat = crate::ui_target::TEXTURE_FORMAT;

/// How many levels the chain has for a window `height` pixels tall: the quarter, then
/// halves until about [`SMALLEST_LINES`] lines.
pub(crate) fn levels(height: u32) -> usize {
    let quarters = (height.max(1) as f32 / SMALLEST_LINES).log2().round() as i32 - 1;
    quarters.clamp(1, MAX_LEVELS as i32) as usize
}

/// Level `level`'s size for a window of `size`: a quarter of it, halved `level` times.
pub(crate) fn level_size(size: [u32; 2], level: usize) -> [u32; 2] {
    let shift = 2 + level as u32;
    size.map(|side| side.div_ceil(1 << shift).max(1))
}

/// How far, in window pixels, a backdrop's region reaches past it so that what the
/// backdrop samples is all blurred from the frame: each pass after the first reads up to two
/// of its source's texels round its own, and a texel that reads past what the pass before
/// wrote holds the last frame's leftovers, which then spread inward a pass at a time.
pub(crate) fn margin(levels: usize) -> f32 {
    let steps = (1_u32 << levels.saturating_sub(1)) - 1;
    // Each step down reads two texels of the finer level, each step up two of the coarser.
    24.0 * steps as f32 + 4.0
}

/// Whether the swapchain should be made readable by shaders (when the surface allows it),
/// so a backdrop can blur the frame without a copy.
pub(crate) fn surface_usage(allowed: wgpu::TextureUsages) -> wgpu::TextureUsages {
    allowed & wgpu::TextureUsages::TEXTURE_BINDING
}

/// The rectangles a frame's backdrops cover, in window pixels.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Regions {
    rects: [Rect; MAX_REGIONS],
    count: usize,
}

impl Default for Regions {
    fn default() -> Self {
        Self {
            rects: [Rect::new(0.0, 0.0, 0.0, 0.0); MAX_REGIONS],
            count: 0,
        }
    }
}

impl Regions {
    fn clear(&mut self) {
        self.count = 0;
    }

    /// Take in `rect`; past [`MAX_REGIONS`], it joins the last one.
    pub(crate) fn add(&mut self, rect: Rect) {
        if rect.width <= 0.0 || rect.height <= 0.0 {
            return;
        }
        if self.count < MAX_REGIONS {
            self.rects[self.count] = rect;
            self.count += 1;
        } else {
            let last = &mut self.rects[MAX_REGIONS - 1];
            *last = last.union(rect);
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// The regions grown by `margin` and cut to `size`, those that overlap merged into
    /// their union, so no pass draws a pixel twice.
    pub(crate) fn grown(&self, margin: f32, size: [u32; 2]) -> ([Rect; MAX_REGIONS], usize) {
        let screen = Rect::new(0.0, 0.0, size[0] as f32, size[1] as f32);
        let mut out = [Rect::new(0.0, 0.0, 0.0, 0.0); MAX_REGIONS];
        let mut count = 0;
        for rect in &self.rects[..self.count] {
            let grown = Rect::new(
                rect.x - margin,
                rect.y - margin,
                rect.width + margin * 2.0,
                rect.height + margin * 2.0,
            );
            if let Some(cut) = intersection(grown, screen) {
                out[count] = cut;
                count += 1;
            }
        }
        // Merge until no two overlap (at most sixteen: a few comparisons).
        let mut i = 0;
        while i < count {
            let mut j = i + 1;
            let mut grew = false;
            while j < count {
                if intersection(out[i], out[j]).is_some() {
                    out[i] = out[i].union(out[j]);
                    out[j] = out[count - 1];
                    count -= 1;
                    grew = true;
                } else {
                    j += 1;
                }
            }
            // A region that grew may now reach one already passed: look again.
            if grew {
                i = 0;
            } else {
                i += 1;
            }
        }
        (out, count)
    }
}

/// The overlap of `a` and `b`, when they overlap.
fn intersection(a: Rect, b: Rect) -> Option<Rect> {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    let right = a.right().min(b.right());
    let bottom = a.bottom().min(b.bottom());
    (right > x && bottom > y).then(|| Rect::new(x, y, right - x, bottom - y))
}

/// One level of the chain.
struct Level {
    view: wgpu::TextureView,
    /// The level read: by the next pass, and for the first level by the shapes' shader.
    group: wgpu::BindGroup,
    size: [u32; 2],
}

/// The blur's pipelines, its levels and this frame's regions.
pub(crate) struct BackdropBlur {
    quarter: wgpu::RenderPipeline,
    down: wgpu::RenderPipeline,
    up: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
    levels: Vec<Level>,
    /// The window size the levels were made for.
    size: [u32; 2],
    /// Whether this frame's image can be sampled ([`Self::prepare`]).
    available: bool,
    regions: Regions,
}

impl BackdropBlur {
    /// The pipelines, sampling through `layout` (a texture and a filtering sampler, the
    /// shapes' own layout); no level yet.
    pub(crate) fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("SJK acrylic backdrop blur"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../ui_backdrop.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SJK acrylic backdrop blur layout"),
            bind_group_layouts: &[Some(layout)],
            immediate_size: 0,
        });
        let pipeline = |entry: &str| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("SJK acrylic backdrop blur pass"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vertex_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        Self {
            quarter: pipeline("quarter"),
            down: pipeline("down"),
            up: pipeline("up"),
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("SJK acrylic backdrop sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),
            levels: Vec::new(),
            size: [0, 0],
            available: false,
            regions: Regions::default(),
        }
    }

    /// Before the frame's shapes are laid out: whether its image can be sampled
    /// (`available`), and the window's `size`, for which the levels are made again when it
    /// changed.
    pub(crate) fn prepare(
        &mut self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        size: [u32; 2],
        available: bool,
    ) {
        self.available = available && size[0] > 0 && size[1] > 0;
        if !self.available || (size == self.size && !self.levels.is_empty()) {
            return;
        }
        self.size = size;
        self.levels = (0..levels(size[1]))
            .map(|level| {
                let level_size = level_size(size, level);
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("SJK acrylic backdrop level"),
                    size: wgpu::Extent3d {
                        width: level_size[0],
                        height: level_size[1],
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: FORMAT,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                });
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                let group = self.bind(device, layout, &view);
                Level {
                    view,
                    group,
                    size: level_size,
                }
            })
            .collect();
    }

    fn bind(
        &self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SJK acrylic backdrop source"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    /// A new frame's shapes: no region yet.
    pub(crate) fn begin_frame(&mut self) {
        self.regions.clear();
    }

    /// Whether backdrops can be blurred this frame.
    pub(crate) fn ready(&self) -> bool {
        self.available && !self.levels.is_empty()
    }

    /// A backdrop over `rect` (window pixels) this frame.
    pub(crate) fn add(&mut self, rect: Rect) {
        self.regions.add(rect);
    }

    /// Whether the frame draws a backdrop.
    pub(crate) fn used(&self) -> bool {
        self.ready() && !self.regions.is_empty()
    }

    /// The blurred frame, for the shapes' shader.
    pub(crate) fn result(&self) -> Option<&wgpu::BindGroup> {
        self.levels.first().map(|level| &level.group)
    }

    /// Blur `source` (the frame's image as the 2D layer is about to draw over it, the size
    /// [`Self::prepare`] was given) round this frame's backdrops; nothing without one.
    pub(crate) fn encode(
        &self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        encoder: &mut wgpu::CommandEncoder,
        source: &wgpu::TextureView,
    ) {
        if !self.used() {
            return;
        }
        let (regions, count) = self.regions.grown(margin(self.levels.len()), self.size);
        let regions = &regions[..count];
        let source = self.bind(device, layout, source);
        self.pass(encoder, &self.quarter, &source, 0, regions);
        for level in 1..self.levels.len() {
            self.pass(
                encoder,
                &self.down,
                &self.levels[level - 1].group,
                level,
                regions,
            );
        }
        for level in (0..self.levels.len().saturating_sub(1)).rev() {
            self.pass(
                encoder,
                &self.up,
                &self.levels[level + 1].group,
                level,
                regions,
            );
        }
    }

    /// One pass into level `level`, reading `source`, cut to `regions`.
    fn pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        pipeline: &wgpu::RenderPipeline,
        source: &wgpu::BindGroup,
        level: usize,
        regions: &[Rect],
    ) {
        let target = &self.levels[level];
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("SJK acrylic backdrop blur"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target.view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, source, &[]);
        let scale = (4_u32 << level) as f32;
        for region in regions {
            let x = (region.x / scale).floor().max(0.0) as u32;
            let y = (region.y / scale).floor().max(0.0) as u32;
            let right = ((region.right() / scale).ceil() as u32).min(target.size[0]);
            let bottom = ((region.bottom() / scale).ceil() as u32).min(target.size[1]);
            if right > x && bottom > y {
                pass.set_scissor_rect(x, y, right - x, bottom - y);
                pass.draw(0..3, 0..1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_blur_reaches_as_far_on_any_screen() {
        assert_eq!(levels(1080), 3);
        assert_eq!(levels(1440), 3);
        assert_eq!(levels(2160), 4);
        assert_eq!(levels(720), 2);
        assert_eq!(levels(1), 1);
        // The smallest level stays about the same height.
        for height in [720_u32, 1080, 1440, 2160, 4320] {
            let smallest = level_size([height * 16 / 9, height], levels(height) - 1)[1];
            assert!((34..=136).contains(&smallest), "{height}: {smallest}");
        }
        assert_eq!(level_size([3840, 2160], 0), [960, 540]);
        assert_eq!(level_size([1366, 768], 0), [342, 192]);
        assert_eq!(level_size([5, 3], 3), [1, 1]);
    }

    #[test]
    fn the_margin_covers_the_chain() {
        assert_eq!(margin(1), 4.0);
        assert_eq!(margin(3), 76.0);
        assert_eq!(margin(4), 172.0);
    }

    #[test]
    fn regions_grow_merge_and_stay_on_screen() {
        let mut regions = Regions::default();
        // The two status bars, far apart: two strips.
        regions.add(Rect::new(0.0, 0.0, 1920.0, 26.0));
        regions.add(Rect::new(0.0, 1054.0, 1920.0, 26.0));
        let (rects, count) = regions.grown(76.0, [1920, 1080]);
        assert_eq!(count, 2);
        assert_eq!(rects[0], Rect::new(0.0, 0.0, 1920.0, 102.0));
        assert_eq!(rects[1], Rect::new(0.0, 978.0, 1920.0, 102.0));
        // Two cards side by side: one region.
        let mut regions = Regions::default();
        regions.add(Rect::new(100.0, 100.0, 200.0, 200.0));
        regions.add(Rect::new(320.0, 100.0, 200.0, 200.0));
        regions.add(Rect::new(0.0, 0.0, 0.0, 10.0));
        let (rects, count) = regions.grown(20.0, [1920, 1080]);
        assert_eq!(count, 1);
        assert_eq!(rects[0], Rect::new(80.0, 80.0, 460.0, 240.0));
        // Past the limit, the rest join the last.
        let mut regions = Regions::default();
        for index in 0..MAX_REGIONS + 4 {
            regions.add(Rect::new(index as f32 * 100.0, 0.0, 10.0, 10.0));
        }
        let (_, count) = regions.grown(0.0, [4000, 1080]);
        assert_eq!(count, MAX_REGIONS);
    }
}
