// Saber blade: a smooth glow capsule and a flat hot core, blended GL_ONE GL_ONE into the
// legacy effect layer (display values, per-channel clamp; `effect_layer.rs`).
//
// Stock draws the glow as RB_SurfaceSaberGlow's chain of camera-facing sprites, one every
// 0.65·radius from the tip down, each 0.017 wider, plus a hilt sprite of radius 5.5-5.75
// (rd-vanilla tr_surface.cpp:470-490), and the core as RB_SurfaceLine's flat quad from the
// tip to one unit behind the hilt (tr_surface.cpp:511-566, CG_DoSaber cg_players.c:5359).
// SJK keeps one smooth capsule (owner decision, step 526i) with the same brightness: the
// chain's sum is integrated along the blade instead of summed per sprite, which is its
// continuous limit. For sprites at density 1/(0.65 r) along a blade whose projection is
// `shaft` long, a point at (x, y) receives
//     (L / (0.65 r)) · mean over v in [v_lo, v_hi] of glow(u, v),
//     u = 0.5 + x / 2r, v_hi = 0.5 + y / 2r, v_lo = 0.5 + (y - shaft) / 2r,
// and the mean comes from the glow image's column integral, `glow_integral`.
//
// Blade skins (saber_skins.rs) have their own grey glow/core pair and are coloured and
// animated here from the instance's time and seed: the Sun blade's white-hot core runs into
// gold, its orange corona carries slowly drifting granulation along the blade and flame
// tongues outward, and now and then a brighter flare (a prominence) runs from the hilt to
// the tip, swelling the glow. The dynamic glow pass shades the glow the same way, so its
// bloom follows the flares.
struct Camera {
    view_projection: mat4x4<f32>,
    position: vec3<f32>,
    _padding: f32,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(1) @binding(0)
var glow_texture: texture_2d<f32>;
@group(1) @binding(1)
var core_texture: texture_2d<f32>;
@group(1) @binding(2)
var saber_sampler: sampler;
// Row j holds the glow image's mean of rows 0..j-1 per column; row 0 is zero.
@group(1) @binding(3)
var glow_integral: texture_2d<f32>;
// The core line is sampled as stock samples it: rd-vulkan's box mip chain, trilinear with its
// default 2x anisotropy (saber.rs `Samplers`). At normal distances the line is a few pixels
// wide for 64 texels, and the mip average widens the white core's fringe.
@group(1) @binding(4)
var core_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    // Across and along the projected blade, world units; along is 0 at the hilt.
    @location(1) blade: vec2<f32>,
    @location(2) @interpolate(flat) shaft: f32,
    @location(3) @interpolate(flat) radius: f32,
    @location(4) @interpolate(flat) length: f32,
    // Hilt sprite radius for the glow; zero selects the core line.
    @location(5) @interpolate(flat) hilt: f32,
    // KIND_RETAIL, KIND_NEUTRAL or KIND_SUN.
    @location(6) @interpolate(flat) kind: u32,
    // Presentation seconds (wrapped) and the blade's seed in [0, 1).
    @location(7) @interpolate(flat) animation: vec2<f32>,
}

// TaystJK's CG_DoSaber submits the core RT_LINE twice (taystjk cgame cg_players.c:6429 and 6465;
// the second adds `saber` where `sbak` was meant). The owner's reference look is TaystJK's. Two
// GL_ONE GL_ONE adds with a clamp after each are one add of twice the colour.
const CORE_DRAWS: f32 = 2.0;
// Slot of the engine-generated neutral glow/core pair (saber_rgb.rs), and of the Sun blade's
// (saber_rgb.rs `SKIN_MATERIAL`).
const NEUTRAL_MATERIAL: u32 = 6u;
const SUN_MATERIAL: u32 = 7u;
const KIND_RETAIL: u32 = 0u;
const KIND_NEUTRAL: u32 = 1u;
const KIND_SUN: u32 = 2u;
// How far the Sun's glow reaches past the stock capsule, for its flares and shimmer
// (saber_skins.rs `GLOW_REACH`).
const SUN_REACH: f32 = 1.6;
// RB_SurfaceSaberGlow: sprite spacing and per-sprite growth.
const SPACING: f32 = 0.65;
const GROWTH: f32 = 0.017;

const QUAD: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2(-1.0, 0.0), vec2(1.0, 0.0), vec2(1.0, 1.0),
    vec2(-1.0, 0.0), vec2(1.0, 1.0), vec2(-1.0, 1.0),
);

// Sprite radius at fraction `along` of the blade (0 hilt, 1 tip): the chain starts at the
// tip with the drawn radius and every sprite toward the hilt is GROWTH wider.
fn chain_radius(radius: f32, length: f32, along: f32) -> f32 {
    return radius + GROWTH * (1.0 - along) * length / (SPACING * radius);
}

@vertex
fn vertex_main(
    @builtin(vertex_index) vertex_index: u32,
    @location(0) blade_base: vec3<f32>,
    @location(1) blade_length: f32,
    @location(2) blade_direction: vec3<f32>,
    @location(3) blade_radius: f32,
    @location(4) blade_color: vec4<f32>,
    @location(5) blade_material: u32,
    @location(6) blade_animation: vec2<f32>,
) -> VertexOutput {
    let direction = normalize(blade_direction);
    let view_direction = normalize(camera.position - (blade_base + direction * blade_length * 0.5));
    var side = cross(direction, view_direction);
    let projected = length(side);
    if projected < 0.001 {
        // Pick an axis that cannot be parallel, including a blade aimed along Z.
        let axis = select(vec3(0.0,0.0,1.0),vec3(0.0,1.0,0.0),abs(view_direction.z) > 0.9);
        side = cross(axis,view_direction);
    }
    side = normalize(side);
    let up = normalize(cross(view_direction,side));
    let local = QUAD[vertex_index];
    let hilt = blade_color.a;
    var output: VertexOutput;
    output.kind = KIND_RETAIL;
    if blade_material == NEUTRAL_MATERIAL { output.kind = KIND_NEUTRAL; }
    if blade_material == SUN_MATERIAL { output.kind = KIND_SUN; }
    output.animation = blade_animation;
    let reach = select(1.0, SUN_REACH, output.kind == KIND_SUN);
    var world: vec3<f32>;
    if hilt > 0.0 {
        // Wide enough for the grown hilt-end sprites and the hilt sprite itself.
        let extent = max(chain_radius(blade_radius, blade_length, 0.0) * reach, hilt);
        let shaft = blade_length * projected;
        world = blade_base + direction * (local.y * blade_length)
            + (side * local.x + up * (local.y * 2.0 - 1.0)) * extent;
        output.blade = vec2(local.x * extent, local.y * (shaft + 2.0 * extent) - extent);
        output.shaft = shaft;
    } else {
        // RB_SurfaceLine: flat, from one unit behind the hilt to the tip.
        world = blade_base + direction * mix(-1.0, blade_length, local.y)
            + side * (local.x * blade_radius);
        output.blade = vec2(local.x * blade_radius, local.y);
        output.shaft = 0.0;
    }
    output.clip_position = camera.view_projection * vec4(world, 1.0);
    output.color = blade_color.rgb;
    output.radius = blade_radius;
    output.length = blade_length;
    output.hilt = hilt;
    return output;
}

fn glow(u: f32, v: f32) -> vec3<f32> {
    if u < 0.0 || u > 1.0 || v < 0.0 || v > 1.0 { return vec3(0.0); }
    return textureSampleLevel(glow_texture, saber_sampler, vec2(u, v), 0.0).rgb;
}

fn integral(u: f32, v: f32) -> vec3<f32> {
    let rows = f32(textureDimensions(glow_integral).y) - 1.0;
    let y = (clamp(v, 0.0, 1.0) * rows + 0.5) / (rows + 1.0);
    return textureSampleLevel(glow_integral, saber_sampler, vec2(u, y), 0.0).rgb;
}

fn glow_capsule(input: VertexOutput) -> vec3<f32> {
    let x = input.blade.x;
    let y = input.blade.y;
    let r = chain_radius(input.radius, input.length,
        clamp(y / max(input.shaft, 0.0001), 0.0, 1.0));
    var sum = vec3(0.0);
    let u = 0.5 + x / (2.0 * r);
    if u >= 0.0 && u <= 1.0 {
        let high = 0.5 + y / (2.0 * r);
        let low = high - input.shaft / (2.0 * r);
        let count = input.length / (SPACING * r);
        if high - low < 1.0 / 64.0 {
            // Seen almost end-on the sprites stack: `count` copies of one texel.
            sum = count * glow(u, 0.5 * (high + low));
        } else {
            sum = count * (integral(u, high) - integral(u, low)) / (high - low);
        }
    }
    // The hilt sprite, centred on the blade's base.
    let hilt = 0.5 + vec2(x, y) / (2.0 * input.hilt);
    return sum + glow(hilt.x, hilt.y);
}

// --- The Sun blade -------------------------------------------------------------------------

const SUN_RED: vec3<f32> = vec3(1.0, 0.2, 0.02);
const SUN_ORANGE: vec3<f32> = vec3(1.0, 0.42, 0.05);
const SUN_GOLD: vec3<f32> = vec3(1.0, 0.72, 0.24);
const SUN_WHITE: vec3<f32> = vec3(1.0, 0.97, 0.86);

// PCG hash of a lattice point, in [0, 1].
fn sun_hash(cell: vec2<i32>) -> f32 {
    var v = bitcast<u32>(cell.x) * 747796405u + 2891336453u;
    v = v ^ (bitcast<u32>(cell.y) * 2654435761u);
    v = v * 747796405u + 2891336453u;
    let word = ((v >> ((v >> 28u) + 4u)) ^ v) * 277803737u;
    return f32((word >> 22u) ^ word) / 4294967295.0;
}

// Smooth value noise in [0, 1].
fn sun_noise(p: vec2<f32>) -> f32 {
    let cell = vec2<i32>(floor(p));
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = sun_hash(cell);
    let b = sun_hash(cell + vec2(1, 0));
    let c = sun_hash(cell + vec2(0, 1));
    let d = sun_hash(cell + vec2(1, 1));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Granulation at `along` world units from the hilt: two octaves drifting toward the tip and
// slowly changing as they go.
fn sun_granulation(along: f32, time: f32, seed: f32) -> f32 {
    let coarse = sun_noise(vec2(along * 0.32 - time * 2.1, time * 0.45 + seed * 61.0));
    let fine = sun_noise(vec2(along * 0.85 - time * 3.6, time * 0.9 + seed * 113.0));
    return smoothstep(0.15, 0.85, coarse * 0.62 + fine * 0.38);
}

// Flares at `along`: three tracks, each sending a bright knot from the hilt to the tip once a
// cycle and lit on about half its cycles, so they come now and then and never in step.
fn sun_flares(along: f32, length: f32, time: f32, seed: f32) -> f32 {
    var total = 0.0;
    for (var track = 0; track < 3; track++) {
        let rate = 0.38 + 0.11 * f32(track) + 0.08 * seed;
        let phase = time * rate + seed * 7.3 + f32(track) * 0.37;
        let cycle = floor(phase);
        let travel = fract(phase);
        let lit = step(0.55, sun_hash(vec2(i32(cycle), track * 7919 + i32(seed * 4096.0))));
        let position = travel * (length + 10.0) - 5.0;
        let width = 2.6 + 1.4 * sun_hash(vec2(i32(cycle), track + 31));
        let offset = (along - position) / width;
        total += lit * sin(3.14159265 * travel) * exp(-offset * offset);
    }
    return total;
}

// Along the blade from the hilt, world units: the glow's projected `y`, or the core line's
// texture `v` (one unit behind the hilt to the tip).
fn sun_along(input: VertexOutput) -> f32 {
    if input.hilt > 0.0 {
        return clamp(input.blade.y / max(input.shaft, 0.0001), 0.0, 1.0) * input.length;
    }
    return mix(-1.0, input.length, input.blade.y);
}

fn sun_glow(input: VertexOutput) -> vec3<f32> {
    let time = input.animation.x;
    let seed = input.animation.y;
    let along = sun_along(input);
    let grain = sun_granulation(along, time, seed);
    let flare = sun_flares(along, input.length, time, seed);
    // Shimmer and swell: the corona breathes a little, a flare widens it.
    let shimmer = 0.045 * sin(time * 9.0 + along * 0.7 + seed * 6.283)
        + 0.03 * sin(time * 15.7 - along * 1.3);
    let widen = min(1.0 + shimmer + 0.14 * (grain - 0.5) + 0.4 * flare, SUN_REACH);
    var shaded = input;
    shaded.blade.x = input.blade.x / widen;
    let capsule = glow_capsule(shaded) / widen;
    // Flame tongues licking outward in the corona's edge.
    let r = chain_radius(input.radius, input.length, clamp(along / max(input.length, 0.0001), 0.0, 1.0));
    let out = abs(input.blade.x) / (r * widen);
    let tongue = sun_noise(vec2(along * 0.45 + seed * 17.0, out * 2.2 - time * 3.4));
    let edge = smoothstep(0.25, 0.9, out);
    let flicker = mix(1.0, 0.3 + 1.4 * tongue, edge);
    // Gold by the core, orange in the corona, red at its rim; granulation and flares heat it.
    let inner = 1.0 - smoothstep(0.0, 0.8, out);
    let rim = mix(SUN_RED, SUN_ORANGE, clamp(grain * 1.2 + flare * 0.6, 0.0, 1.0));
    let color = mix(rim, SUN_GOLD, clamp(inner * inner * (0.55 + 0.45 * grain) + flare * 0.25, 0.0, 1.0));
    let brightness = (0.5 + 0.7 * grain + 1.3 * flare) * flicker;
    return capsule * color * brightness;
}

fn sun_core(input: VertexOutput, texel: vec4<f32>) -> vec3<f32> {
    let time = input.animation.x;
    let seed = input.animation.y;
    let along = sun_along(input);
    let grain = sun_granulation(along, time, seed);
    let flare = sun_flares(along, input.length, time, seed);
    let fringe = mix(SUN_ORANGE, SUN_GOLD, 0.35 + 0.65 * grain) * (0.7 + 0.6 * grain + 1.0 * flare);
    return CORE_DRAWS * (SUN_WHITE * texel.r * (1.0 + 0.4 * flare) + fringe * texel.g);
}

// The core line's texture coordinates; the Sun's core breathes a little across.
fn core_coordinates(input: VertexOutput) -> vec2<f32> {
    var across = input.blade.x / (2.0 * input.radius);
    if input.kind == KIND_SUN {
        let along = mix(-1.0, input.length, input.blade.y);
        let time = input.animation.x;
        across *= 1.0 + 0.06 * sin(time * 11.0 + along * 0.9 + input.animation.y * 6.283);
    }
    return vec2(0.5 + across, 1.0 - input.blade.y);
}

// Dynamic glow: the glow capsule only; the core line's shader has no `glow` stage.
@fragment
fn fragment_glow(input: VertexOutput) -> @location(0) vec4<f32> {
    if input.hilt <= 0.0 { discard; }
    if input.kind == KIND_SUN { return vec4(sun_glow(input), 1.0); }
    return vec4(glow_capsule(input) * input.color, 1.0);
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // v runs from the tip (0) to behind the hilt (1), as DoLine's texture coordinates do.
    // Derivatives are taken before any branch (uniform control flow); the glow ignores them.
    let core_uv = core_coordinates(input);
    let core_dx = dpdx(core_uv);
    let core_dy = dpdy(core_uv);
    if input.hilt > 0.0 {
        if input.kind == KIND_SUN { return vec4(sun_glow(input), 1.0); }
        return vec4(glow_capsule(input) * input.color, 1.0);
    }
    var texel = textureSampleGrad(core_texture, core_sampler, core_uv, core_dx, core_dy);
    if input.kind == KIND_SUN {
        return vec4(sun_core(input, texel), 1.0);
    }
    if input.kind == KIND_NEUTRAL {
        // Neutral core: red = white-hot core, green = tinted fringe.
        return vec4(CORE_DRAWS * (vec3(texel.r) + input.color * texel.g), 1.0);
    }
    return vec4(CORE_DRAWS * texel.rgb * input.color, 1.0);
}
