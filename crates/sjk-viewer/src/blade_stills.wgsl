// The Collection's blade loops (`blade_stills.rs`): each baked picture, drawn by the blade
// renderer at twice the picture's size in one row of the baking target, brought down to its
// atlas cell with a soft glow round its light (a cheap bloom, done once when it is baked),
// then faded out toward the cell's edges as the live pictures are (`blade_pictures.wgsl`).
// Display values, alpha the brightest channel, as the live pictures', so the UI adds them
// the same way.
const STILL: vec2<f32> = vec2<f32>(384.0, 96.0);
// One row of the baking target, in pixels.
const ROW: vec2<i32> = vec2<i32>(768, 192);
// In shares of the picture (52 by 13 units): the blade starts 3 units in.
const LEFT: vec2<f32> = vec2<f32>(0.5 / 52.0, 4.5 / 52.0);
const RIGHT: f32 = 4.0 / 52.0;
const EDGE: f32 = 2.6 / 13.0;
// The glow: taps on a disc this wide (baking pixels), how much of it is added, and how
// much the light's own colour leads it over the white core.
const TAPS: u32 = 48u;
const RADIUS: f32 = 34.0;
const GLOW: f32 = 0.6;
@group(0) @binding(0) var bake: texture_2d<f32>;

struct Out {
    @builtin(position) at: vec4<f32>,
    @location(0) @interpolate(flat) row: u32,
}

@vertex fn vs_main(
    @builtin(vertex_index) index: u32,
    @builtin(instance_index) row: u32,
) -> Out {
    let p = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var out: Out;
    out.at = vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
    out.row = row;
    return out;
}

// A baking pixel as the live picture shows it: clamped to the display's range first (the
// live picture's 8-bit target does), so a core far over 1 does not spread white into its
// neighbours when four are averaged.
fn texel(p: vec2<i32>, row: i32) -> vec3<f32> {
    let q = clamp(p, vec2<i32>(0), ROW - vec2<i32>(1));
    return clamp(textureLoad(bake, q + vec2<i32>(0, row * ROW.y), 0).rgb, vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment fn fs_main(in: Out) -> @location(0) vec4<f32> {
    // The pixel's place in its cell (the cells tile the atlas).
    let local = in.at.xy - floor(in.at.xy / STILL) * STILL;
    let row = i32(in.row);
    let p = vec2<i32>(floor(local * 2.0));
    let base = (texel(p, row) + texel(p + vec2<i32>(1, 0), row) + texel(p + vec2<i32>(0, 1), row)
        + texel(p + vec2<i32>(1, 1), row)) * 0.25;
    // A disc of taps (a golden-angle spiral), Gaussian weighted: the blade's light spread.
    var glow = vec3<f32>(0.0);
    var weight = 0.0;
    for (var k = 0u; k < TAPS; k++) {
        let r = sqrt((f32(k) + 0.5) / f32(TAPS)) * RADIUS;
        let a = f32(k) * 2.3999632;
        let offset = vec2<f32>(cos(a), sin(a)) * r;
        let w = exp(-r * r / (RADIUS * RADIUS * 0.22));
        let c = texel(vec2<i32>(floor(local * 2.0 + offset)), row);
        // The core's white spreads as the light's colour: the glow of a saber, not of a lamp.
        let hue = c - vec3<f32>(min(c.r, min(c.g, c.b)) * 0.6);
        glow += (hue * 1.4 + c * 0.25) * w;
        weight += w;
    }
    glow = glow / weight;
    // The glow only where the blade is dark (a halo round it): the blade keeps its own
    // colours and shape, as the live picture shows them (15/11/2026: added over the blade
    // too, it washed many kybers out to white).
    let dark = 1.0 - max(base.r, max(base.g, base.b));
    let lit = base + min(glow * GLOW, vec3<f32>(1.0)) * dark * dark;
    let cell = local / STILL;
    let fade = smoothstep(LEFT.x, LEFT.y, cell.x)
        * (1.0 - smoothstep(1.0 - RIGHT, 1.0, cell.x))
        * smoothstep(0.0, EDGE, cell.y)
        * (1.0 - smoothstep(1.0 - EDGE, 1.0, cell.y));
    let c = clamp(lit * fade, vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(c, max(c.r, max(c.g, c.b)));
}
