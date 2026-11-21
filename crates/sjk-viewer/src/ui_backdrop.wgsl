// The acrylic backdrop's blur (`ui_renderer/backdrop.rs`): a dual Kawase chain over the
// frame's image as it stands before the 2D layer, at a quarter of its size and below. Every
// pass draws one triangle over its target, cut to the backdrops' regions by scissors.

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;

@vertex
fn vertex_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    // One triangle covering the target: uv runs 0 to 1 across it, top to bottom.
    let corner = vec2(f32((index << 1u) & 2u), f32(index & 2u));
    var output: VertexOutput;
    output.position = vec4(corner * 2.0 - 1.0, 0.0, 1.0);
    output.uv = vec2(corner.x, 1.0 - corner.y);
    return output;
}

fn tap(uv: vec2<f32>) -> vec3<f32> {
    return textureSampleLevel(source, source_sampler, uv, 0.0).rgb;
}

// The frame to a quarter of its size in one step: four bilinear taps, each the mean of a
// 2x2 block of the frame, so the mean of the 4x4 block under the target's texel.
@fragment
fn quarter(input: VertexOutput) -> @location(0) vec4<f32> {
    let o = 1.0 / vec2<f32>(textureDimensions(source));
    let rgb = tap(input.uv + vec2(-o.x, -o.y)) + tap(input.uv + vec2(o.x, -o.y))
        + tap(input.uv + vec2(-o.x, o.y)) + tap(input.uv + vec2(o.x, o.y));
    return vec4(rgb * 0.25, 1.0);
}

// Dual Kawase's step down to half the size: the middle, weighted four, and the four
// diagonal taps a source texel out.
@fragment
fn down(input: VertexOutput) -> @location(0) vec4<f32> {
    let o = 1.0 / vec2<f32>(textureDimensions(source));
    let rgb = tap(input.uv) * 4.0 + tap(input.uv - o) + tap(input.uv + o)
        + tap(input.uv + vec2(o.x, -o.y)) + tap(input.uv + vec2(-o.x, o.y));
    return vec4(rgb * 0.125, 1.0);
}

// Dual Kawase's step up to twice the size: four taps a source texel out along the axes and
// four half a texel out on the diagonals, the diagonal ones weighted two.
@fragment
fn up(input: VertexOutput) -> @location(0) vec4<f32> {
    let o = 1.0 / vec2<f32>(textureDimensions(source));
    let h = o * 0.5;
    let axes = tap(input.uv + vec2(-o.x, 0.0)) + tap(input.uv + vec2(o.x, 0.0))
        + tap(input.uv + vec2(0.0, -o.y)) + tap(input.uv + vec2(0.0, o.y));
    let diagonals = tap(input.uv + vec2(-h.x, -h.y)) + tap(input.uv + vec2(h.x, -h.y))
        + tap(input.uv + vec2(-h.x, h.y)) + tap(input.uv + vec2(h.x, h.y));
    return vec4((axes + diagonals * 2.0) / 12.0, 1.0);
}
