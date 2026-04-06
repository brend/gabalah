struct ShaderParams {
    time_seconds: f32,
    scanline_strength: f32,
    curvature: f32,
    _pad: f32,
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

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let uv = warp_uv(in.uv, params.curvature);
    if any(uv < vec2<f32>(0.0, 0.0)) || any(uv > vec2<f32>(1.0, 1.0)) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    var color = textureSample(frame_texture, frame_sampler, uv);

    let scanline_phase = (uv.y + params.time_seconds * 0.15) * 720.0;
    let scanline = 0.5 + 0.5 * sin(scanline_phase);
    let scanline_mask = 1.0 - scanline * params.scanline_strength;
    color = vec4<f32>(color.rgb * scanline_mask, color.a);

    return vec4<f32>(color.rgb, 1.0);
}
