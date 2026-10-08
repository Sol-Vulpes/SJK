//! Lightmap coordinates for an HD world pack, taken from the stock surfaces it
//! replaces.
//!
//! The static world's lamp light, and the baked terms live lighting still reads,
//! are keyed to each surface's lightmap parametrization. A pack has none, so each
//! of its triangles borrows the chart of the nearest stock triangle of the same
//! shader: that triangle's material (its lightmap page) and an affine extension
//! of its coordinates to the pack's vertices, kept inside the stock surface's chart.

use crate::scene_flatten::FlattenedScene;
use glam::{Vec2, Vec3};
use std::collections::{HashMap, HashSet};

/// Edge of a grid cell, in map units.
const CELL: f32 = 128.0;

struct Source {
    corners: [Vec3; 3],
    uvs: [Vec2; 3],
    material: usize,
    chart: (Vec2, Vec2),
}

/// Stock static-world triangles of some shaders, in a uniform grid.
pub(super) struct Charts {
    sources: Vec<Source>,
    grid: HashMap<(i32, i32, i32), Vec<u32>>,
}

/// Where a pack triangle lands in the stock map.
pub(super) struct Placement {
    /// The stock material (shader and lightmap page) to draw it with.
    pub(super) material: usize,
    /// Lightmap coordinates of its three corners.
    pub(super) uvs: [[f32; 2]; 3],
}

fn cell(p: Vec3) -> (i32, i32, i32) {
    (
        (p.x / CELL).floor() as i32,
        (p.y / CELL).floor() as i32,
        (p.z / CELL).floor() as i32,
    )
}

impl Charts {
    /// The triangles of every static-world draw (hidden or not) whose shader is
    /// in `shaders` (lower case).
    pub(super) fn of(flat: &FlattenedScene, shaders: &HashSet<String>) -> Self {
        let mut sources = Vec::new();
        let mut grid = HashMap::<(i32, i32, i32), Vec<u32>>::new();
        for draw in &flat.draws {
            if !draw.world_surface || draw.surface_index.is_none() {
                continue;
            }
            let shader = flat.materials[draw.material].shader.to_ascii_lowercase();
            if !shaders.contains(&shader) {
                continue;
            }
            let range = draw.indices.start as usize..draw.indices.end as usize;
            let mut low = Vec2::splat(f32::INFINITY);
            let mut high = Vec2::splat(f32::NEG_INFINITY);
            for &i in &flat.indices[range.clone()] {
                let uv = Vec2::from_array(flat.vertices[i as usize].lightmap_coordinates);
                low = low.min(uv);
                high = high.max(uv);
            }
            for triangle in flat.indices[range].chunks_exact(3) {
                let vertex = |k: usize| &flat.vertices[triangle[k] as usize];
                let corners = [0, 1, 2].map(|k| Vec3::from_array(vertex(k).position));
                let id = sources.len() as u32;
                let (lo, hi) = (
                    corners[0].min(corners[1]).min(corners[2]),
                    corners[0].max(corners[1]).max(corners[2]),
                );
                let (a, b) = (cell(lo), cell(hi));
                for x in a.0..=b.0 {
                    for y in a.1..=b.1 {
                        for z in a.2..=b.2 {
                            grid.entry((x, y, z)).or_default().push(id);
                        }
                    }
                }
                sources.push(Source {
                    corners,
                    uvs: [0, 1, 2].map(|k| Vec2::from_array(vertex(k).lightmap_coordinates)),
                    material: draw.material,
                    chart: (low, high),
                });
            }
        }
        Self { sources, grid }
    }

    /// The stock triangle of `shader` nearest to `corners`' centroid, and the
    /// coordinates its surface's chart gives each corner.
    pub(super) fn place(
        &self,
        flat: &FlattenedScene,
        shader: &str,
        corners: [Vec3; 3],
    ) -> Option<Placement> {
        let centroid = (corners[0] + corners[1] + corners[2]) / 3.0;
        let centre = cell(centroid);
        let mut best: Option<(f32, &Source)> = None;
        // Widen the search until a match is found, up to a few cells.
        for radius in 1..=4 {
            for x in -radius..=radius {
                for y in -radius..=radius {
                    for z in -radius..=radius {
                        let Some(ids) = self.grid.get(&(centre.0 + x, centre.1 + y, centre.2 + z))
                        else {
                            continue;
                        };
                        for &id in ids {
                            let source = &self.sources[id as usize];
                            if !flat.materials[source.material]
                                .shader
                                .eq_ignore_ascii_case(shader)
                            {
                                continue;
                            }
                            let distance =
                                closest(centroid, &source.corners).distance_squared(centroid);
                            if best.is_none_or(|(d, _)| distance < d) {
                                best = Some((distance, source));
                            }
                        }
                    }
                }
            }
            if best.is_some() {
                break;
            }
        }
        let (_, source) = best?;
        let uvs = corners.map(|corner| {
            let weights = barycentric(corner, &source.corners);
            let uv =
                source.uvs[0] * weights.x + source.uvs[1] * weights.y + source.uvs[2] * weights.z;
            uv.clamp(source.chart.0, source.chart.1).to_array()
        });
        Some(Placement {
            material: source.material,
            uvs,
        })
    }
}

/// The point of triangle `t` nearest to `p` (Ericson, Real-Time Collision Detection).
fn closest(p: Vec3, t: &[Vec3; 3]) -> Vec3 {
    let (a, b, c) = (t[0], t[1], t[2]);
    let (ab, ac, ap) = (b - a, c - a, p - a);
    let (d1, d2) = (ab.dot(ap), ac.dot(ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = p - b;
    let (d3, d4) = (ab.dot(bp), ac.dot(bp));
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return a + ab * (d1 / (d1 - d3));
    }
    let cp = p - c;
    let (d5, d6) = (ab.dot(cp), ac.dot(cp));
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return a + ac * (d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && d4 - d3 >= 0.0 && d5 - d6 >= 0.0 {
        return b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let denominator = 1.0 / (va + vb + vc);
    a + ab * (vb * denominator) + ac * (vc * denominator)
}

/// Barycentric weights of `p` projected onto the plane of `t`, unclamped (zero
/// weights for a degenerate triangle give its first corner).
fn barycentric(p: Vec3, t: &[Vec3; 3]) -> Vec3 {
    let (v0, v1, v2) = (t[1] - t[0], t[2] - t[0], p - t[0]);
    let (d00, d01, d11) = (v0.dot(v0), v0.dot(v1), v1.dot(v1));
    let (d20, d21) = (v2.dot(v0), v2.dot(v1));
    let denominator = d00 * d11 - d01 * d01;
    if denominator.abs() < 1e-9 {
        return Vec3::X;
    }
    let v = (d11 * d20 - d01 * d21) / denominator;
    let w = (d00 * d21 - d01 * d20) / denominator;
    Vec3::new(1.0 - v - w, v, w)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn barycentric_weights_extend_affinely() {
        let t = [Vec3::ZERO, Vec3::X, Vec3::Y];
        assert!(
            barycentric(Vec3::new(0.25, 0.25, 5.0), &t)
                .abs_diff_eq(Vec3::new(0.5, 0.25, 0.25), 1e-6)
        );
        assert!(
            barycentric(Vec3::new(2.0, 0.0, 0.0), &t).abs_diff_eq(Vec3::new(-1.0, 2.0, 0.0), 1e-6)
        );
    }

    #[test]
    fn the_closest_point_stays_on_the_triangle() {
        let t = [Vec3::ZERO, Vec3::X, Vec3::Y];
        assert!(closest(Vec3::new(0.2, 0.2, 3.0), &t).abs_diff_eq(Vec3::new(0.2, 0.2, 0.0), 1e-6));
        assert!(closest(Vec3::new(-1.0, -1.0, 0.0), &t).abs_diff_eq(Vec3::ZERO, 1e-6));
        assert!(closest(Vec3::new(2.0, 2.0, 0.0), &t).abs_diff_eq(Vec3::new(0.5, 0.5, 0.0), 1e-6));
    }
}
