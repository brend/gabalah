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

fn sample_cell(cell_uv: vec2<f32>) -> vec3<f32> {
    return textureSample(
        frame_texture,
        frame_sampler,
        clamp(cell_uv, vec2<f32>(0.0), vec2<f32>(1.0)),
    ).rgb;
}

fn saturate(color: vec3<f32>, amount: f32) -> vec3<f32> {
    let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    return clamp(vec3<f32>(luma) + (color - vec3<f32>(luma)) * amount, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn heart_mask(local: vec2<f32>) -> f32 {
    var p = local * 2.0 - vec2<f32>(1.0, 1.0);
    p = p * vec2<f32>(1.25, 1.25);
    p.y = -p.y + 0.15;

    let x2 = p.x * p.x;
    let y2 = p.y * p.y;
    let q = x2 + y2 - 1.0;
    let heart = q * q * q - x2 * p.y * y2;

    let aa = max(fwidth(heart), 0.0025);
    return 1.0 - smoothstep(-aa, aa, heart);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(frame_texture));
    let cell = floor(in.uv * dims);
    let cell_uv = (cell + vec2<f32>(0.5, 0.5)) / dims;
    let local = fract(in.uv * dims);

    var color = sample_cell(cell_uv);
    color = saturate(color, 1.0 + params.color_intensity * 0.75);

    let heart = heart_mask(local);
    let cell_id = cell.x + cell.y * dims.x;
    let pulse = 0.94 + 0.06 * sin(params.time_seconds * 2.4 + cell_id * 0.013);
    let heart_color = color * pulse;

    let bg = vec3<f32>(0.02, 0.01, 0.015) * (0.8 + params.scanline_strength);
    let final_color = mix(bg, heart_color, heart);

    return vec4<f32>(clamp(final_color, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
