//! HD world pack (proof of concept): a binary glTF whose meshes are drawn in
//! place of a map's static-world surfaces.
//!
//! The BSP stays the source of collision, entities, visibility, light and
//! shadows. Only the colour pass changes: every static-world draw whose shader
//! the pack also carries is hidden (`DrawBatch::hidden`) and the pack's meshes
//! are appended as extra world draws. A pack therefore replaces a map shader by
//! shader and may cover only part of the map.
//!
//! The pack's frame is the one `hd_world_export` writes (1:1 scale, glTF Y-up
//! from the map's Z-up, counter-clockwise fronts), which is also what Blender
//! writes back. Material names are BSP shader names. Until packs are mounted
//! like game data, the file is named by `SJK_HD_WORLD`.

use crate::gpu_vertex::GpuVertex;
use crate::hd_world_lightmap::Charts;
use crate::scene_flatten::{DrawBatch, FlattenedScene, index_bounds};
use glam::{Mat4, Quat, Vec3};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::error::Error;
use std::path::Path;

/// Environment variable naming the pack file.
pub(crate) const ENV: &str = "SJK_HD_WORLD";

/// One drawable piece of a pack, in map space with map winding.
pub(crate) struct Mesh {
    pub(crate) shader: String,
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

/// What [`apply`] changed.
#[derive(Debug, Default)]
pub(crate) struct Summary {
    pub(crate) meshes: usize,
    pub(crate) triangles: usize,
    pub(crate) draws: usize,
    /// Triangles with no stock surface of their shader to borrow a lightmap chart from.
    pub(crate) unplaced_triangles: usize,
    pub(crate) hidden_draws: usize,
    pub(crate) shaders_without_stock: Vec<String>,
}

type Error_ = Box<dyn Error>;

/// Read and decode a pack.
pub(crate) fn load(path: &Path) -> Result<Vec<Mesh>, Error_> {
    let bytes = std::fs::read(path)?;
    if bytes.len() < 20 || &bytes[..4] != b"glTF" {
        return Err("not a binary glTF".into());
    }
    let mut json: Option<Value> = None;
    let mut bin: &[u8] = &[];
    let mut at = 12;
    while at + 8 <= bytes.len() {
        let length = u32::from_le_bytes(bytes[at..at + 4].try_into()?) as usize;
        let kind = &bytes[at + 4..at + 8];
        let body = bytes
            .get(at + 8..at + 8 + length)
            .ok_or("truncated chunk")?;
        match kind {
            b"JSON" => json = Some(serde_json::from_slice(body)?),
            b"BIN\0" => bin = body,
            _ => {}
        }
        at += 8 + length.div_ceil(4) * 4;
    }
    let json = json.ok_or("no JSON chunk")?;
    let mut meshes = Vec::new();
    let scene = json["scene"].as_u64().unwrap_or(0) as usize;
    let roots = json["scenes"][scene]["nodes"]
        .as_array()
        .ok_or("scene has no nodes")?;
    for root in roots {
        walk(
            &json,
            bin,
            root.as_u64().ok_or("bad node index")? as usize,
            Mat4::IDENTITY,
            &mut meshes,
        )?;
    }
    Ok(meshes)
}

fn node_matrix(node: &Value) -> Mat4 {
    if let Some(m) = node["matrix"].as_array() {
        let values: Vec<f32> = m.iter().map(|v| v.as_f64().unwrap_or(0.0) as f32).collect();
        if values.len() == 16 {
            return Mat4::from_cols_slice(&values);
        }
    }
    let vec3 = |key: &str, default: f32| -> Vec3 {
        node[key].as_array().map_or(Vec3::splat(default), |a| {
            Vec3::new(
                a[0].as_f64().unwrap_or(0.0) as f32,
                a[1].as_f64().unwrap_or(0.0) as f32,
                a[2].as_f64().unwrap_or(0.0) as f32,
            )
        })
    };
    let rotation = node["rotation"].as_array().map_or(Quat::IDENTITY, |a| {
        Quat::from_xyzw(
            a[0].as_f64().unwrap_or(0.0) as f32,
            a[1].as_f64().unwrap_or(0.0) as f32,
            a[2].as_f64().unwrap_or(0.0) as f32,
            a[3].as_f64().unwrap_or(1.0) as f32,
        )
    });
    Mat4::from_scale_rotation_translation(vec3("scale", 1.0), rotation, vec3("translation", 0.0))
}

fn walk(
    json: &Value,
    bin: &[u8],
    index: usize,
    parent: Mat4,
    out: &mut Vec<Mesh>,
) -> Result<(), Error_> {
    let node = &json["nodes"][index];
    let world = parent * node_matrix(node);
    if let Some(mesh) = node["mesh"].as_u64() {
        for primitive in json["meshes"][mesh as usize]["primitives"]
            .as_array()
            .ok_or("mesh without primitives")?
        {
            if primitive["mode"].as_u64().is_some_and(|mode| mode != 4) {
                continue;
            }
            out.push(primitive_mesh(json, bin, primitive, world)?);
        }
    }
    for child in node["children"].as_array().into_iter().flatten() {
        walk(
            json,
            bin,
            child.as_u64().ok_or("bad child index")? as usize,
            world,
            out,
        )?;
    }
    Ok(())
}

/// The accessor's elements as `components` floats each (normalised integers
/// are scaled to 0..1).
fn floats(
    json: &Value,
    bin: &[u8],
    accessor: usize,
    components: usize,
) -> Result<Vec<f32>, Error_> {
    let a = &json["accessors"][accessor];
    let view =
        &json["bufferViews"][a["bufferView"].as_u64().ok_or("accessor without view")? as usize];
    let count = a["count"].as_u64().ok_or("accessor without count")? as usize;
    let kind = a["componentType"].as_u64().ok_or("accessor without type")?;
    let width = match kind {
        5126 | 5125 => 4,
        5123 => 2,
        5121 | 5120 => 1,
        _ => return Err(format!("unsupported component type {kind}").into()),
    };
    let have = match a["type"].as_str().ok_or("accessor without element type")? {
        "SCALAR" => 1,
        "VEC2" => 2,
        "VEC3" => 3,
        "VEC4" => 4,
        other => return Err(format!("unsupported element type {other}").into()),
    };
    let stride = view["byteStride"]
        .as_u64()
        .map_or(have * width, |s| s as usize);
    let start = view["byteOffset"].as_u64().unwrap_or(0) as usize
        + a["byteOffset"].as_u64().unwrap_or(0) as usize;
    let normalized = a["normalized"].as_bool().unwrap_or(false);
    let mut out = Vec::with_capacity(count * components);
    for element in 0..count {
        for c in 0..components {
            // A missing component (RGB colours asked for four) reads as 1.
            if c >= have {
                out.push(1.0);
                continue;
            }
            let at = start + element * stride + c * width;
            let raw = bin
                .get(at..at + width)
                .ok_or("accessor outside the buffer")?;
            out.push(match (kind, normalized) {
                (5126, _) => f32::from_le_bytes(raw.try_into()?),
                (5125, _) => u32::from_le_bytes(raw.try_into()?) as f32,
                (5123, true) => f32::from(u16::from_le_bytes(raw.try_into()?)) / 65535.0,
                (5123, false) => f32::from(u16::from_le_bytes(raw.try_into()?)),
                (5121, true) => f32::from(raw[0]) / 255.0,
                (5121, false) => f32::from(raw[0]),
                _ => f32::from(raw[0] as i8),
            });
        }
    }
    Ok(out)
}

fn primitive_mesh(
    json: &Value,
    bin: &[u8],
    primitive: &Value,
    world: Mat4,
) -> Result<Mesh, Error_> {
    let attributes = &primitive["attributes"];
    let position = attributes["POSITION"]
        .as_u64()
        .ok_or("primitive without POSITION")? as usize;
    let raw = floats(json, bin, position, 3)?;
    // glTF (Y up) back to the map's Z-up frame: (a, b, c) -> (a, -c, b).
    let to_map = |v: Vec3| [v.x, -v.z, v.y];
    let positions: Vec<[f32; 3]> = raw
        .chunks_exact(3)
        .map(|p| to_map(world.transform_point3(Vec3::new(p[0], p[1], p[2]))))
        .collect();
    let normal_matrix = glam::Mat3::from_mat4(world).inverse().transpose();
    let normals: Vec<[f32; 3]> = match attributes["NORMAL"].as_u64() {
        Some(a) => floats(json, bin, a as usize, 3)?
            .chunks_exact(3)
            .map(|n| to_map((normal_matrix * Vec3::new(n[0], n[1], n[2])).normalize_or_zero()))
            .collect(),
        None => vec![[0.0, 0.0, 1.0]; positions.len()],
    };
    let uvs: Vec<[f32; 2]> = match attributes["TEXCOORD_0"].as_u64() {
        Some(a) => floats(json, bin, a as usize, 2)?
            .chunks_exact(2)
            .map(|t| [t[0], t[1]])
            .collect(),
        None => vec![[0.0; 2]; positions.len()],
    };
    let colors: Vec<[f32; 4]> = match attributes["COLOR_0"].as_u64() {
        Some(a) => floats(json, bin, a as usize, 4)?
            .chunks_exact(4)
            .map(|c| [c[0], c[1], c[2], c[3]])
            .collect(),
        None => vec![[1.0; 4]; positions.len()],
    };
    let mut indices: Vec<u32> = match primitive["indices"].as_u64() {
        Some(a) => floats(json, bin, a as usize, 1)?
            .into_iter()
            .map(|i| i as u32)
            .collect(),
        None => (0..positions.len() as u32).collect(),
    };
    // The pack's fronts are counter-clockwise; the map's triangles run the
    // other way (see `hd_world_export`).
    for triangle in indices.chunks_exact_mut(3) {
        triangle.swap(1, 2);
    }
    if [normals.len(), uvs.len(), colors.len()]
        .iter()
        .any(|&n| n != positions.len())
    {
        return Err("attribute counts differ".into());
    }
    let material = primitive["material"]
        .as_u64()
        .ok_or("primitive without material")? as usize;
    let mut shader = json["materials"][material]["name"]
        .as_str()
        .ok_or("unnamed material")?
        .to_owned();
    // Blender suffixes duplicate names: `name.001`.
    if let Some((stem, suffix)) = shader.rsplit_once('.')
        && suffix.len() == 3
        && suffix.bytes().all(|b| b.is_ascii_digit())
    {
        shader = stem.to_owned();
    }
    Ok(Mesh {
        shader,
        positions,
        normals,
        uvs,
        colors,
        indices,
    })
}

/// Hide the stock draws of the pack's shaders and append the pack's meshes.
pub(crate) fn apply(flat: &mut FlattenedScene, meshes: &[Mesh]) -> Result<Summary, Error_> {
    let packed: HashSet<String> = meshes
        .iter()
        .map(|m| m.shader.to_ascii_lowercase())
        .collect();
    let mut summary = Summary::default();
    for draw in &mut flat.draws {
        if draw.world_surface
            && draw.surface_index.is_some()
            && packed.contains(&flat.materials[draw.material].shader.to_ascii_lowercase())
        {
            draw.hidden = true;
            summary.hidden_draws += 1;
        }
    }
    // The stock material of each shader (preferring one on a lightmap page), for
    // triangles that find no stock surface to borrow a chart from.
    let mut stock_materials = HashMap::<String, usize>::new();
    for (index, material) in flat.materials.iter().enumerate() {
        let entry = stock_materials
            .entry(material.shader.to_ascii_lowercase())
            .or_insert(index);
        if flat.materials[*entry].lightmap < 0 && material.lightmap >= 0 {
            *entry = index;
        }
    }
    let charts = Charts::of(flat, &packed);
    for mesh in meshes {
        let Some(&fallback) = stock_materials.get(&mesh.shader.to_ascii_lowercase()) else {
            summary.shaders_without_stock.push(mesh.shader.clone());
            continue;
        };
        // Each triangle takes the material (lightmap page) and lightmap
        // coordinates of the nearest stock surface of its shader; its vertices
        // are not shared, as neighbours may land on different charts.
        let mut groups = BTreeMap::<usize, (Vec<GpuVertex>, Vec<u32>)>::new();
        for triangle in mesh.indices.chunks_exact(3) {
            let corner = |k: usize| Vec3::from_array(mesh.positions[triangle[k] as usize]);
            let placement = charts.place(flat, &mesh.shader, [corner(0), corner(1), corner(2)]);
            if placement.is_none() {
                summary.unplaced_triangles += 1;
            }
            let (material, uvs) =
                placement.map_or((fallback, [[0.0; 2]; 3]), |p| (p.material, p.uvs));
            let (vertices, indices) = groups.entry(material).or_default();
            for (k, &i) in triangle.iter().enumerate() {
                let i = i as usize;
                indices.push(vertices.len() as u32);
                vertices.push(GpuVertex {
                    position: mesh.positions[i],
                    normal: mesh.normals[i],
                    color: mesh.colors[i],
                    texture_coordinates: mesh.uvs[i],
                    lightmap_coordinates: uvs[k],
                });
            }
        }
        for (material, (vertices, indices)) in groups {
            let base: u32 = flat.vertices.len().try_into()?;
            let start: u32 = flat.indices.len().try_into()?;
            let bounds = index_bounds(vertices.iter().map(|v| v.position));
            flat.vertices.extend(vertices);
            flat.indices.extend(indices.iter().map(|&i| base + i));
            flat.draws.push(DrawBatch {
                indices: start..u32::try_from(flat.indices.len())?,
                material,
                clusters: Vec::new(),
                surface_index: None,
                world_surface: false,
                hidden: false,
                overlay: true,
                bounds,
            });
            summary.draws += 1;
        }
        summary.meshes += 1;
        summary.triangles += mesh.indices.len() / 3;
    }
    Ok(summary)
}

/// Apply the pack named by [`ENV`], if any; a bad pack is reported and skipped.
pub(crate) fn apply_from_env(flat: &mut FlattenedScene) {
    let Some(path) = std::env::var_os(ENV) else {
        return;
    };
    // `SJK_HD_SKIP=a,b` keeps the shaders containing `a` or `b` stock, to find
    // which part of a pack changes the picture.
    let skip: Vec<String> = std::env::var("SJK_HD_SKIP")
        .map(|v| {
            v.split(',')
                .map(|s| s.trim().to_ascii_lowercase())
                .collect()
        })
        .unwrap_or_default();
    let loaded = load(Path::new(&path)).map(|mut meshes| {
        meshes.retain(|m| {
            let shader = m.shader.to_ascii_lowercase();
            !skip.iter().any(|s| !s.is_empty() && shader.contains(s))
        });
        meshes
    });
    match loaded.and_then(|meshes| apply(flat, &meshes)) {
        Ok(summary) => crate::log::progress(format_args!("HD world: {summary:?}")),
        Err(error) => {
            crate::log::progress(format_args!("HD world: {error}; the stock map is drawn"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One triangle under a translated node: positions come back in the map's
    /// frame, two indices swapped, the Blender name suffix dropped.
    #[test]
    fn a_pack_loads_in_the_map_frame() {
        let mut bin = Vec::new();
        for v in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            bin.extend_from_slice(&v.to_le_bytes());
        }
        for i in [0u32, 1, 2] {
            bin.extend_from_slice(&i.to_le_bytes());
        }
        let json = serde_json::json!({
            "asset": {"version": "2.0"},
            "scene": 0,
            "scenes": [{"nodes": [0]}],
            "nodes": [{"mesh": 0, "translation": [10.0, 0.0, 0.0]}],
            "meshes": [{"primitives": [{
                "attributes": {"POSITION": 0}, "indices": 1, "material": 0,
            }]}],
            "materials": [{"name": "textures/a/b.001"}],
            "accessors": [
                {"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3"},
                {"bufferView": 1, "componentType": 5125, "count": 3, "type": "SCALAR"},
            ],
            "bufferViews": [
                {"buffer": 0, "byteOffset": 0, "byteLength": 36},
                {"buffer": 0, "byteOffset": 36, "byteLength": 12},
            ],
            "buffers": [{"byteLength": 48}],
        });
        let mut json_bytes = serde_json::to_vec(&json).expect("json");
        while !json_bytes.len().is_multiple_of(4) {
            json_bytes.push(b' ');
        }
        let total = 12 + 8 + json_bytes.len() + 8 + bin.len();
        let mut glb = Vec::new();
        glb.extend_from_slice(b"glTF");
        glb.extend_from_slice(&2u32.to_le_bytes());
        glb.extend_from_slice(&(total as u32).to_le_bytes());
        glb.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
        glb.extend_from_slice(b"JSON");
        glb.extend_from_slice(&json_bytes);
        glb.extend_from_slice(&(bin.len() as u32).to_le_bytes());
        glb.extend_from_slice(b"BIN\0");
        glb.extend_from_slice(&bin);
        let directory = tempfile::tempdir().expect("a directory");
        let path = directory.path().join("pack.glb");
        std::fs::write(&path, glb).expect("write the pack");

        let meshes = load(&path).expect("the pack");
        assert_eq!(meshes.len(), 1);
        let mesh = &meshes[0];
        assert_eq!(mesh.shader, "textures/a/b");
        // glTF (10, 0, 0), (11, 0, 0), (10, 1, 0) is map (10, 0, 0), (11, 0, 0), (10, 0, 1).
        assert_eq!(
            mesh.positions,
            [[10.0, 0.0, 0.0], [11.0, 0.0, 0.0], [10.0, 0.0, 1.0]]
        );
        assert_eq!(mesh.indices, [0, 2, 1]);
        assert_eq!(mesh.colors, [[1.0; 4]; 3]);
    }
}
