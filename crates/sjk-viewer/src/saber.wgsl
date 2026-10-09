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
// animated here from the instance's time and seed and the skin's parameters, which come
// from its blade-skin file (blade_skin_file.rs) in the uniform array `skins`: a hot core
// with a fringe, a corona graded from its rim to the inside, granulation drifting along
// the blade, flame tongues licking outward, shimmer, and flares running from the hilt to
// the tip, swelling the glow. Nothing here knows any one skin. The dynamic glow pass
// shades the glow the same way, so its bloom follows the flares.
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

// One loaded skin's parameters (saber_skins.rs `SkinUniform`, blade_skin_file.rs for the
// meaning of each). `grain` is the granulation in [0, 1] and `flare` the flares' sum.
struct Skin {
    // Core: rgb the hot centre's colour, w how much a flare brightens it.
    core_white: vec4<f32>,
    // Fringe colours, cool and hot; w: fringe_heat base and grain.
    core_fringe_cool: vec4<f32>,
    core_fringe_hot: vec4<f32>,
    // Fringe brightness: base, grain, flare.
    core_fringe: vec4<f32>,
    // The core's breathing across: amount, rate, along, seed.
    core_breathe: vec4<f32>,
    // Corona colours, rim cool and hot and the inside; w: inner_width, rim heat by
    // grain and by flare.
    rim_cool: vec4<f32>,
    rim_hot: vec4<f32>,
    inner: vec4<f32>,
    // The inside colour's share: base, grain, flare.
    inner_mix: vec4<f32>,
    // The glow's brightness: base, grain, flare.
    brightness: vec4<f32>,
    // Reach (widest, in stock capsules), swell by grain and by flare.
    swell: vec4<f32>,
    // Granulation octaves: scale, speed, evolve, offset.
    grain_coarse: vec4<f32>,
    grain_fine: vec4<f32>,
    // Octave weights, then the contrast's low and high.
    grain_mix: vec4<f32>,
    // Flares: rate, rate per track, rate per seed, track count.
    flare_rate: vec4<f32>,
    // Phase per seed and per track, the lit threshold, overshoot.
    flare_shape: vec4<f32>,
    // Size and its jitter.
    flare_size: vec4<f32>,
    // Shimmer waves: amount, rate, along, seed.
    shimmer_a: vec4<f32>,
    shimmer_b: vec4<f32>,
    // Tongues: along, offset, out, speed; then edge low and high, low, range.
    tongue_a: vec4<f32>,
    tongue_b: vec4<f32>,
}

@group(2) @binding(0)
var<uniform> skins: array<Skin, MAX_SKINS>;

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
    // KIND_RETAIL, KIND_NEUTRAL, or KIND_SKIN plus the skin's number.
    @location(6) @interpolate(flat) kind: u32,
    // Presentation seconds (wrapped) and the blade's seed in [0, 1).
    @location(7) @interpolate(flat) animation: vec2<f32>,
}

// TaystJK's CG_DoSaber submits the core RT_LINE twice (taystjk cgame cg_players.c:6429 and 6465;
// the second adds `saber` where `sbak` was meant). The owner's reference look is TaystJK's. Two
// GL_ONE GL_ONE adds with a clamp after each are one add of twice the colour.
const CORE_DRAWS: f32 = 2.0;
// Slot of the engine-generated neutral glow/core pair (saber_rgb.rs), and of the first
// loaded blade skin's (saber_rgb.rs `SKIN_MATERIAL`); skins loaded at once.
const NEUTRAL_MATERIAL: u32 = 6u;
const SKIN_MATERIAL: u32 = 7u;
const MAX_SKINS: u32 = 8u;
const KIND_RETAIL: u32 = 0u;
const KIND_NEUTRAL: u32 = 1u;
const KIND_SKIN: u32 = 2u;
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
    var reach = 1.0;
    if blade_material >= SKIN_MATERIAL && blade_material < SKIN_MATERIAL + MAX_SKINS {
        output.kind = KIND_SKIN + (blade_material - SKIN_MATERIAL);
        // A skin's glow reaches further, for its flares and shimmer.
        reach = skins[blade_material - SKIN_MATERIAL].swell.x;
    }
    output.animation = blade_animation;
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

// --- Blade skins ---------------------------------------------------------------------------

const PI: f32 = 3.14159265;

// PCG hash of a lattice point, in [0, 1].
fn skin_hash(cell: vec2<i32>) -> f32 {
    var v = bitcast<u32>(cell.x) * 747796405u + 2891336453u;
    v = v ^ (bitcast<u32>(cell.y) * 2654435761u);
    v = v * 747796405u + 2891336453u;
    let word = ((v >> ((v >> 28u) + 4u)) ^ v) * 277803737u;
    return f32((word >> 22u) ^ word) / 4294967295.0;
}

// Smooth value noise in [0, 1].
fn skin_noise(p: vec2<f32>) -> f32 {
    let cell = vec2<i32>(floor(p));
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = skin_hash(cell);
    let b = skin_hash(cell + vec2(1, 0));
    let c = skin_hash(cell + vec2(0, 1));
    let d = skin_hash(cell + vec2(1, 1));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// The skin's number, below MAX_SKINS.
fn skin_index(input: VertexOutput) -> u32 {
    return min(input.kind - KIND_SKIN, MAX_SKINS - 1u);
}

// Granulation at `along` world units from the hilt: two octaves drifting toward the tip and
// slowly changing as they go.
fn skin_granulation(skin: Skin, along: f32, time: f32, seed: f32) -> f32 {
    let c = skin.grain_coarse;
    let f = skin.grain_fine;
    let coarse = skin_noise(vec2(along * c.x - time * c.y, time * c.z + seed * c.w));
    let fine = skin_noise(vec2(along * f.x - time * f.y, time * f.z + seed * f.w));
    let mixed = skin.grain_mix;
    return smoothstep(mixed.z, mixed.w, coarse * mixed.x + fine * mixed.y);
}

// Flares at `along`: tracks each sending a bright knot from the hilt to the tip once a cycle,
// lit on the cycles whose draw reaches the threshold, so they come now and then and never in
// step.
fn skin_flares(skin: Skin, along: f32, length: f32, time: f32, seed: f32) -> f32 {
    let rates = skin.flare_rate;
    let shape = skin.flare_shape;
    let size = skin.flare_size;
    var total = 0.0;
    for (var track = 0; track < i32(rates.w); track++) {
        let rate = rates.x + rates.y * f32(track) + rates.z * seed;
        let phase = time * rate + seed * shape.x + f32(track) * shape.y;
        let cycle = floor(phase);
        let travel = fract(phase);
        let lit = step(shape.z, skin_hash(vec2(i32(cycle), track * 7919 + i32(seed * 4096.0))));
        let position = travel * (length + 2.0 * shape.w) - shape.w;
        let width = size.x + size.y * skin_hash(vec2(i32(cycle), track + 31));
        let offset = (along - position) / width;
        total += lit * sin(PI * travel) * exp(-offset * offset);
    }
    return total;
}

// One wave of shimmer or breathing: amount · sin(time · rate + along · along + seed · seed).
fn skin_wave(wave: vec4<f32>, time: f32, along: f32, seed: f32) -> f32 {
    return wave.x * sin(time * wave.y + along * wave.z + seed * wave.w);
}

// Along the blade from the hilt, world units: the glow's projected `y`, or the core line's
// texture `v` (one unit behind the hilt to the tip).
fn skin_along(input: VertexOutput) -> f32 {
    if input.hilt > 0.0 {
        return clamp(input.blade.y / max(input.shaft, 0.0001), 0.0, 1.0) * input.length;
    }
    return mix(-1.0, input.length, input.blade.y);
}

fn skin_glow(input: VertexOutput) -> vec3<f32> {
    let skin = skins[skin_index(input)];
    let time = input.animation.x;
    let seed = input.animation.y;
    let along = skin_along(input);
    let grain = skin_granulation(skin, along, time, seed);
    let flare = skin_flares(skin, along, input.length, time, seed);
    // Shimmer and swell: the corona breathes a little, a flare widens it.
    let shimmer = skin_wave(skin.shimmer_a, time, along, seed)
        + skin_wave(skin.shimmer_b, time, along, seed);
    let swell = skin.swell;
    let widen = min(1.0 + shimmer + swell.y * (grain - 0.5) + swell.z * flare, swell.x);
    var shaded = input;
    shaded.blade.x = input.blade.x / widen;
    let capsule = glow_capsule(shaded) / widen;
    // Flame tongues licking outward in the corona's edge.
    let r = chain_radius(input.radius, input.length, clamp(along / max(input.length, 0.0001), 0.0, 1.0));
    let out = abs(input.blade.x) / (r * widen);
    let ta = skin.tongue_a;
    let tb = skin.tongue_b;
    let tongue = skin_noise(vec2(along * ta.x + seed * ta.y, out * ta.z - time * ta.w));
    let edge = smoothstep(tb.x, tb.y, out);
    let flicker = mix(1.0, tb.z + tb.w * tongue, edge);
    // The inside colour by the core, the rim's at the edge; granulation and flares heat it.
    let inner = 1.0 - smoothstep(0.0, skin.rim_cool.w, out);
    let rim = mix(skin.rim_cool.rgb, skin.rim_hot.rgb,
        clamp(grain * skin.rim_hot.w + flare * skin.inner.w, 0.0, 1.0));
    let share = skin.inner_mix;
    let color = mix(rim, skin.inner.rgb,
        clamp(inner * inner * (share.x + share.y * grain) + flare * share.z, 0.0, 1.0));
    let light = skin.brightness;
    let brightness = (light.x + light.y * grain + light.z * flare) * flicker;
    return capsule * color * brightness;
}

fn skin_core(input: VertexOutput, texel: vec4<f32>) -> vec3<f32> {
    let skin = skins[skin_index(input)];
    let time = input.animation.x;
    let seed = input.animation.y;
    let along = skin_along(input);
    let grain = skin_granulation(skin, along, time, seed);
    let flare = skin_flares(skin, along, input.length, time, seed);
    let bright = skin.core_fringe;
    let fringe = mix(skin.core_fringe_cool.rgb, skin.core_fringe_hot.rgb,
        skin.core_fringe_cool.w + skin.core_fringe_hot.w * grain)
        * (bright.x + bright.y * grain + bright.z * flare);
    return CORE_DRAWS * (skin.core_white.rgb * texel.r * (1.0 + skin.core_white.w * flare)
        + fringe * texel.g);
}

// The core line's texture coordinates; a skin's core breathes a little across.
fn core_coordinates(input: VertexOutput) -> vec2<f32> {
    var across = input.blade.x / (2.0 * input.radius);
    if input.kind >= KIND_SKIN {
        let along = mix(-1.0, input.length, input.blade.y);
        let breathe = skins[skin_index(input)].core_breathe;
        across *= 1.0 + skin_wave(breathe, input.animation.x, along, input.animation.y);
    }
    return vec2(0.5 + across, 1.0 - input.blade.y);
}

// Dynamic glow: the glow capsule only; the core line's shader has no `glow` stage.
@fragment
fn fragment_glow(input: VertexOutput) -> @location(0) vec4<f32> {
    if input.hilt <= 0.0 { discard; }
    if input.kind >= KIND_SKIN { return vec4(skin_glow(input), 1.0); }
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
        if input.kind >= KIND_SKIN { return vec4(skin_glow(input), 1.0); }
        return vec4(glow_capsule(input) * input.color, 1.0);
    }
    var texel = textureSampleGrad(core_texture, core_sampler, core_uv, core_dx, core_dy);
    if input.kind >= KIND_SKIN {
        return vec4(skin_core(input, texel), 1.0);
    }
    if input.kind == KIND_NEUTRAL {
        // Neutral core: red = white-hot core, green = tinted fringe.
        return vec4(CORE_DRAWS * (vec3(texel.r) + input.color * texel.g), 1.0);
    }
    return vec4(CORE_DRAWS * texel.rgb * input.color, 1.0);
}
