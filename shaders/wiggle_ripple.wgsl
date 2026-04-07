struct ShaderParams {
    time_seconds: f32,
    scanline_strength: f32,
    curvature: f32,
    color_intensity: f32,
    color_mode: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0)
var frame_texture: texture_2d<f32>;

@group(0) @binding(1)
var frame_sampler: sampler;

@group(0) @binding(2)
var<uniform> params: ShaderParams;

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );

    let pos = positions[idx];
    var out: VsOut;
    out.clip_pos = vec4<f32>(pos, 0.0, 1.0);
    let uv = pos * 0.5 + vec2<f32>(0.5, 0.5);
    out.uv = vec2<f32>(uv.x, 1.0 - uv.y);
    return out;
}

fn sample_cell(cell: vec2<f32>, dims: vec2<f32>) -> vec3<f32> {
    let uv = (cell + vec2<f32>(0.5, 0.5)) / dims;
    return textureSample(
        frame_texture,
        frame_sampler,
        clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0)),
    ).rgb;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(frame_texture));
    let grid = in.uv * dims;
    let t = params.time_seconds;

    let intensity = clamp(0.18 + params.color_intensity * 0.42, 0.12, 0.62);
    let speed = 1.1 + params.scanline_strength * 2.5;
    let freq = 0.17 + params.curvature * 0.35;

    let radial_centered = (grid - dims * 0.5) / max(dims.x, dims.y);
    let radial = length(radial_centered);
    let spiral = sin(radial * 24.0 - t * speed * 1.4);
    let wave_x = sin(grid.y * freq + t * speed + spiral * 0.8);
    let wave_y = cos(grid.x * (freq * 0.92) - t * speed * 0.9 + spiral * 0.7);
    let displacement = vec2<f32>(wave_x, wave_y) * intensity;

    let src_cell = floor(grid + displacement);
    let color = sample_cell(src_cell, dims);

    let local = fract(grid);
    let edge = min(min(local.x, 1.0 - local.x), min(local.y, 1.0 - local.y));
    let vignette = 0.9 + 0.1 * smoothstep(0.0, 0.22, edge);
    let final_color = clamp(color * vignette, vec3<f32>(0.0), vec3<f32>(1.0));

    return vec4<f32>(final_color, 1.0);
}
