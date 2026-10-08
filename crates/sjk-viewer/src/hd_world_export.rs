//! Export of a map's drawn surfaces to a binary glTF, for rebuilding the world
//! in Blender (HD world proof of concept).
//!
//! One object per (BSP model, shader) under an empty named `model_<n>`, so the
//! static world (`model_0`) and every inline model (doors, lifts) stay apart.
//! The scale is 1:1 (one Blender unit is one map unit) and the axes are the
//! glTF ones that Blender's importer turns back into the map's Z-up frame, so
//! a vertex lands at its map coordinates. Material names are the BSP shader
//! names; each material that resolves to an image gets it as a PNG in
//! `textures/`.

use serde_json::{Value, json};
use sjk_bsp::Bsp;
use sjk_scene::StaticWorld;
use sjk_shader::ShaderCatalog;
use sjk_vfs::VirtualFileSystem;
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::path::{Path, PathBuf};

const SURFACE_SKY: u32 = 0x0000_2000;

/// What an export wrote.
#[derive(Debug)]
pub(crate) struct Summary {
    pub(crate) glb: PathBuf,
    pub(crate) models: usize,
    pub(crate) objects: usize,
    pub(crate) triangles: usize,
    pub(crate) textures: usize,
    pub(crate) missing_textures: Vec<String>,
    /// Triangles whose map winding agrees with their vertex normals, and
    /// those that disagree: the map's front-face convention, measured.
    pub(crate) winding: (usize, usize),
}

#[derive(Default)]
struct Group {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colors: Vec<[u8; 4]>,
    indices: Vec<u32>,
    surfaces: usize,
}

/// Map space (X forward, Y left, Z up) to glTF space (Y up, right-handed); a
/// pure rotation, so the winding is kept. Blender's importer undoes it.
fn to_gltf(v: [f32; 3]) -> [f32; 3] {
    [v[0], v[2], -v[1]]
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn pad4(bytes: &mut Vec<u8>, with: u8) {
    while !bytes.len().is_multiple_of(4) {
        bytes.push(with);
    }
}

/// Write `<directory>/<stem>.glb` and `<directory>/textures/*.png`.
pub(crate) fn write_map(
    bsp: &Bsp,
    scene: &StaticWorld,
    vfs: &VirtualFileSystem,
    shaders: &ShaderCatalog,
    directory: &Path,
    stem: &str,
) -> Result<Summary, Box<dyn Error>> {
    let render = bsp.render();
    let mut model_of = vec![0usize; render.surfaces().len()];
    for (model, info) in render.models().iter().enumerate() {
        for surface in info.surfaces.clone() {
            if let Some(slot) = model_of.get_mut(surface) {
                *slot = model;
            }
        }
    }

    // Gather triangles per (model, shader), merging lightmap pages: the HD
    // world has no baked light.
    let mut groups = BTreeMap::<(usize, usize), Group>::new();
    let mut agree = 0usize;
    let mut disagree = 0usize;
    for batch in scene.batches() {
        let shader = batch.material.shader;
        if bsp.shaders()[shader].surface_flags & SURFACE_SKY != 0 {
            continue;
        }
        for draw in &batch.draws {
            let group = groups
                .entry((model_of[draw.surface_index], shader))
                .or_default();
            group.surfaces += 1;
            let mut remap = HashMap::<u32, u32>::new();
            for &index in &batch.indices[draw.indices.start as usize..draw.indices.end as usize] {
                let mapped = *remap.entry(index).or_insert_with(|| {
                    let vertex = &batch.vertices[index as usize];
                    group.positions.push(to_gltf(vertex.position));
                    group.normals.push(to_gltf(vertex.normal));
                    group.uvs.push(vertex.texture_coordinates);
                    group.colors.push(vertex.color);
                    (group.positions.len() - 1) as u32
                });
                group.indices.push(mapped);
            }
            // Winding against the vertex normals, in the glTF frame.
            for triangle in group.indices[group.indices.len() - (draw.indices.len())..].chunks(3) {
                let [a, b, c] =
                    [triangle[0], triangle[1], triangle[2]].map(|i| group.positions[i as usize]);
                let edge = |p: [f32; 3], q: [f32; 3]| [q[0] - p[0], q[1] - p[1], q[2] - p[2]];
                let (u, v) = (edge(a, b), edge(a, c));
                let n = [
                    u[1] * v[2] - u[2] * v[1],
                    u[2] * v[0] - u[0] * v[2],
                    u[0] * v[1] - u[1] * v[0],
                ];
                let vn = group.normals[triangle[0] as usize];
                let dot = n[0] * vn[0] + n[1] * vn[1] + n[2] * vn[2];
                if dot > 0.0 {
                    agree += 1;
                } else if dot < 0.0 {
                    disagree += 1;
                }
            }
        }
    }
    // glTF fronts are counter-clockwise. If most triangles run the other way
    // against their normals, flip them all, so Blender's face orientation
    // matches the normals.
    let flip = disagree > agree;
    if flip {
        for group in groups.values_mut() {
            for triangle in group.indices.chunks_mut(3) {
                triangle.swap(1, 2);
            }
        }
    }

    // Materials and their images.
    std::fs::create_dir_all(directory.join("textures"))?;
    let mut material_of = HashMap::<usize, usize>::new();
    let mut materials = Vec::<Value>::new();
    let mut images = Vec::<Value>::new();
    let mut missing = Vec::new();
    for &(_, shader) in groups.keys() {
        if material_of.contains_key(&shader) {
            continue;
        }
        let name = bsp.shaders()[shader].name_lossy().into_owned();
        let mut pbr = json!({"metallicFactor": 0.0, "roughnessFactor": 1.0});
        let mut texture_slot = None;
        match shaders.resolve_image(vfs, &name) {
            Ok(Some(path)) => {
                let asset = vfs.read(path.as_str())?.ok_or("resolved image vanished")?;
                match crate::gpu_texture::decode_image(&asset.bytes, path.as_str()) {
                    Ok(image) => {
                        let file = format!("{}.png", sanitize(&name));
                        image
                            .to_rgba8()
                            .save(directory.join("textures").join(&file))?;
                        images.push(json!({"uri": format!("textures/{file}"), "name": name}));
                        texture_slot = Some(images.len() - 1);
                    }
                    Err(error) => missing.push(format!("{name}: {error}")),
                }
            }
            _ => missing.push(format!("{name}: no image")),
        }
        if let Some(image) = texture_slot {
            pbr["baseColorTexture"] = json!({"index": image});
        } else {
            let hash = name.bytes().fold(2166136261u32, |h, b| {
                (h ^ u32::from(b)).wrapping_mul(16777619)
            });
            let channel = |shift: u32| 0.25 + ((hash >> shift) & 0xff) as f32 / 255.0 * 0.6;
            pbr["baseColorFactor"] = json!([channel(0), channel(8), channel(16), 1.0]);
        }
        materials.push(json!({
            "name": name,
            "pbrMetallicRoughness": pbr,
            "doubleSided": true,
        }));
        material_of.insert(shader, materials.len() - 1);
    }

    // Binary buffer, accessors, meshes, nodes.
    let mut bin = Vec::<u8>::new();
    let mut views = Vec::<Value>::new();
    let mut accessors = Vec::<Value>::new();
    let mut meshes = Vec::<Value>::new();
    let mut nodes = Vec::<Value>::new();
    let mut model_nodes = BTreeMap::<usize, usize>::new();
    /// Append `bytes` to the buffer as a view and describe it by `accessor`;
    /// returns the accessor's index.
    fn add(
        bin: &mut Vec<u8>,
        views: &mut Vec<Value>,
        accessors: &mut Vec<Value>,
        bytes: &[u8],
        target: u32,
        mut accessor: Value,
    ) -> usize {
        pad4(bin, 0);
        views.push(json!({
            "buffer": 0, "byteOffset": bin.len(), "byteLength": bytes.len(), "target": target,
        }));
        bin.extend_from_slice(bytes);
        accessor["bufferView"] = json!(views.len() - 1);
        accessors.push(accessor);
        accessors.len() - 1
    }
    let mut triangles = 0;
    let mut object_nodes = Vec::<(usize, usize)>::new();
    for (&(model, shader), group) in &groups {
        triangles += group.indices.len() / 3;
        let mut low = [f32::INFINITY; 3];
        let mut high = [f32::NEG_INFINITY; 3];
        for p in &group.positions {
            for axis in 0..3 {
                low[axis] = low[axis].min(p[axis]);
                high[axis] = high[axis].max(p[axis]);
            }
        }
        let count = group.positions.len();
        let position = add(
            &mut bin,
            &mut views,
            &mut accessors,
            bytemuck::cast_slice(&group.positions),
            34962,
            json!({"componentType": 5126, "count": count, "type": "VEC3", "min": low, "max": high}),
        );
        let normal = add(
            &mut bin,
            &mut views,
            &mut accessors,
            bytemuck::cast_slice(&group.normals),
            34962,
            json!({"componentType": 5126, "count": count, "type": "VEC3"}),
        );
        let uv = add(
            &mut bin,
            &mut views,
            &mut accessors,
            bytemuck::cast_slice(&group.uvs),
            34962,
            json!({"componentType": 5126, "count": count, "type": "VEC2"}),
        );
        let color = add(
            &mut bin,
            &mut views,
            &mut accessors,
            bytemuck::cast_slice(&group.colors),
            34962,
            json!({"componentType": 5121, "normalized": true, "count": count, "type": "VEC4"}),
        );
        let indices = add(
            &mut bin,
            &mut views,
            &mut accessors,
            bytemuck::cast_slice(&group.indices),
            34963,
            json!({"componentType": 5125, "count": group.indices.len(), "type": "SCALAR"}),
        );
        let shader_name = bsp.shaders()[shader].name_lossy().into_owned();
        let short = sanitize(
            shader_name
                .strip_prefix("textures/")
                .unwrap_or(&shader_name),
        );
        let name = format!("m{model}__{short}");
        meshes.push(json!({
            "name": name,
            "primitives": [{
                "attributes": {
                    "POSITION": position, "NORMAL": normal,
                    "TEXCOORD_0": uv, "COLOR_0": color,
                },
                "indices": indices,
                "material": material_of[&shader],
            }],
        }));
        nodes.push(json!({
            "name": name,
            "mesh": meshes.len() - 1,
            "extras": {"model": model, "shader": shader_name, "surfaces": group.surfaces},
        }));
        object_nodes.push((model, nodes.len() - 1));
    }
    let object_count = object_nodes.len();
    for (model, node) in object_nodes {
        model_nodes.entry(model).or_insert_with(|| {
            nodes.push(json!({"name": format!("model_{model}"), "children": []}));
            nodes.len() - 1
        });
        let parent = model_nodes[&model];
        nodes[parent]["children"]
            .as_array_mut()
            .expect("children")
            .push(json!(node));
    }
    let roots: Vec<usize> = model_nodes.values().copied().collect();

    let document = json!({
        "asset": {"version": "2.0", "generator": "SJK hd_world_export"},
        "scene": 0,
        "scenes": [{"name": stem, "nodes": roots}],
        "nodes": nodes,
        "meshes": meshes,
        "materials": materials,
        "images": images,
        "textures": (0..images.len()).map(|i| json!({"source": i})).collect::<Vec<_>>(),
        "accessors": accessors,
        "bufferViews": views,
        "buffers": [{"byteLength": bin.len()}],
    });
    let mut json_bytes = serde_json::to_vec(&document)?;
    pad4(&mut json_bytes, b' ');
    pad4(&mut bin, 0);
    let total = 12 + 8 + json_bytes.len() + 8 + bin.len();
    let mut glb = Vec::with_capacity(total);
    glb.extend_from_slice(b"glTF");
    glb.extend_from_slice(&2u32.to_le_bytes());
    glb.extend_from_slice(&(total as u32).to_le_bytes());
    glb.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    glb.extend_from_slice(b"JSON");
    glb.extend_from_slice(&json_bytes);
    glb.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    glb.extend_from_slice(b"BIN\0");
    glb.extend_from_slice(&bin);
    let path = directory.join(format!("{stem}.glb"));
    std::fs::write(&path, glb)?;
    Ok(Summary {
        glb: path,
        models: model_nodes.len(),
        objects: object_count,
        triangles,
        textures: images.len(),
        missing_textures: missing,
        winding: (agree, disagree),
    })
}
