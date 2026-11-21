//! The feats' medal pictures ([`crate::hud::feats`]): a 4 x 4 grid of [`CELL`] cells,
//! one per [`crate::feats::Feat`], kept apart from the icon atlas, which has no room
//! left. The grid is made on the first upload, so a client whose game data has no feat
//! pictures carries none of it (1 MB).

use sjk_ui::TextureId;

/// Edge of one picture, as the icon atlas's cells.
pub(crate) const CELL: u32 = super::ICON_SIZE;
const COLUMNS: u32 = 4;
/// Pictures the grid holds.
pub(crate) const CELLS: u32 = COLUMNS * COLUMNS;
const SIZE: u32 = COLUMNS * CELL;
const TEXTURE_BASE: u32 = 0x6300_0000;

const _: () = assert!(crate::feats::Feat::ALL.len() as u32 <= CELLS);

/// `TexturedQuad` texture naming picture `cell`.
pub(crate) fn cell_texture(cell: u32) -> TextureId {
    TextureId(TEXTURE_BASE + cell.min(CELLS - 1))
}

/// The picture `texture` names.
pub(crate) fn cell_of(texture: TextureId) -> Option<u32> {
    (TEXTURE_BASE..TEXTURE_BASE + CELLS)
        .contains(&texture.0)
        .then(|| texture.0 - TEXTURE_BASE)
}

/// The texture coordinates of picture `cell` (top-left and bottom-right).
pub(super) fn uv_range(cell: u32) -> ([f32; 2], [f32; 2]) {
    let cell = cell.min(CELLS - 1);
    let (x, y) = ((cell % COLUMNS) * CELL, (cell / COLUMNS) * CELL);
    let uv = |x: u32, y: u32| [x as f32 / SIZE as f32, y as f32 / SIZE as f32];
    (uv(x, y), uv(x + CELL, y + CELL))
}

/// The grid, made when first needed.
#[derive(Default)]
pub(super) struct FeatPictures {
    grid: Option<(wgpu::Texture, wgpu::BindGroup)>,
}

impl FeatPictures {
    pub(super) fn group(&self) -> Option<&wgpu::BindGroup> {
        self.grid.as_ref().map(|(_, group)| group)
    }

    /// Put `rgba` ([`CELL`] square) in picture `cell`, making the grid first.
    pub(super) fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &crate::frame_queue::FrameQueue,
        layout: &wgpu::BindGroupLayout,
        cell: u32,
        rgba: &[u8],
    ) {
        if rgba.len() != (CELL * CELL * 4) as usize || cell >= CELLS {
            return;
        }
        let (texture, _) = self.grid.get_or_insert_with(|| grid(device, layout));
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: (cell % COLUMNS) * CELL,
                    y: (cell / COLUMNS) * CELL,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(CELL * 4),
                rows_per_image: Some(CELL),
            },
            wgpu::Extent3d {
                width: CELL,
                height: CELL,
                depth_or_array_layers: 1,
            },
        );
    }
}

fn grid(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> (wgpu::Texture, wgpu::BindGroup) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("SJK feat pictures"),
        size: wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: crate::ui_target::TEXTURE_FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("SJK feat pictures sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("SJK feat pictures bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    (texture, group)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pictures_name_their_cells_and_stay_apart_from_other_textures() {
        assert_eq!(cell_of(cell_texture(5)), Some(5));
        assert_eq!(cell_of(cell_texture(CELLS + 3)), Some(CELLS - 1));
        assert_eq!(cell_of(TextureId(TEXTURE_BASE - 1)), None);
        assert!(super::super::map_edit_pictures::cell_of(cell_texture(0)).is_none());
        assert!(crate::cards::faces::index_of(cell_texture(0)).is_none());
        assert!(crate::chat_gifs::slot_of(cell_texture(0)).is_none());
        assert!(crate::medals::Medal::from_art(cell_texture(0)).is_none());
        let (a, b) = uv_range(5);
        assert_eq!(a, [0.25, 0.25]);
        assert_eq!(b, [0.5, 0.5]);
    }
}
