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

fn warp_uv(uv: vec2<f32>, curvature: f32) -> vec2<f32> {
    let centered = uv * 2.0 - vec2<f32>(1.0, 1.0);
    let radius2 = dot(centered, centered);
    let warped = centered * (1.0 + curvature * radius2);
    return warped * 0.5 + vec2<f32>(0.5, 0.5);
}

fn sample_frame(uv: vec2<f32>) -> vec3<f32> {
    return textureSample(frame_texture, frame_sampler, clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0))).rgb;
}

fn phosphor_mask(uv: vec2<f32>) -> vec3<f32> {
    let triad = fract(uv.x * 480.0);
    if triad < 0.3333 {
        return vec3<f32>(1.0, 0.92, 0.92);
    }
    if triad < 0.6666 {
        return vec3<f32>(0.92, 1.0, 0.92);
    }
    return vec3<f32>(0.92, 0.92, 1.0);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let uv = warp_uv(in.uv, params.curvature);
    if any(uv < vec2<f32>(0.0, 0.0)) || any(uv > vec2<f32>(1.0, 1.0)) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    var color = sample_frame(uv);

    let scanline_phase = (uv.y + params.time_seconds * 0.12) * 720.0;
    let scanline = 0.5 + 0.5 * sin(scanline_phase);
    let scanline_mask = 1.0 - scanline * params.scanline_strength;
    color *= scanline_mask;

    let mask_strength = clamp(0.08 + params.color_intensity * 0.12, 0.0, 0.3);
    color *= mix(vec3<f32>(1.0), phosphor_mask(uv), mask_strength);

    let flicker = 0.985 + 0.015 * sin(params.time_seconds * 120.0);
    color *= flicker;

    return vec4<f32>(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
