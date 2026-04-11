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

fn sample_frame(uv: vec2<f32>) -> vec3<f32> {
    return textureSample(
        frame_texture,
        frame_sampler,
        clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0)),
    ).rgb;
}

fn luma(color: vec3<f32>) -> f32 {
    return dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn darkness(color: vec3<f32>) -> f32 {
    return 1.0 - luma(color);
}

fn dark_cluster_density(uv: vec2<f32>, px: vec2<f32>) -> f32 {
    let d00 = smoothstep(0.32, 0.9, darkness(sample_frame(uv + vec2<f32>(-px.x, -px.y))));
    let d10 = smoothstep(0.32, 0.9, darkness(sample_frame(uv + vec2<f32>(0.0, -px.y))));
    let d20 = smoothstep(0.32, 0.9, darkness(sample_frame(uv + vec2<f32>(px.x, -px.y))));
    let d01 = smoothstep(0.32, 0.9, darkness(sample_frame(uv + vec2<f32>(-px.x, 0.0))));
    let d11 = smoothstep(0.32, 0.9, darkness(sample_frame(uv)));
    let d21 = smoothstep(0.32, 0.9, darkness(sample_frame(uv + vec2<f32>(px.x, 0.0))));
    let d02 = smoothstep(0.32, 0.9, darkness(sample_frame(uv + vec2<f32>(-px.x, px.y))));
    let d12 = smoothstep(0.32, 0.9, darkness(sample_frame(uv + vec2<f32>(0.0, px.y))));
    let d22 = smoothstep(0.32, 0.9, darkness(sample_frame(uv + vec2<f32>(px.x, px.y))));

    let weighted =
        d00 * 1.0 + d10 * 1.8 + d20 * 1.0 +
        d01 * 1.8 + d11 * 3.2 + d21 * 1.8 +
        d02 * 1.0 + d12 * 1.8 + d22 * 1.0;
    return weighted / 14.4;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let uv = clamp(in.uv, vec2<f32>(0.0), vec2<f32>(1.0));
    let dims = vec2<f32>(textureDimensions(frame_texture));
    let px = vec2<f32>(1.0 / dims.x, 1.0 / dims.y);

    let base = sample_frame(uv);
    let base_dark = smoothstep(0.24, 0.94, darkness(base));
    let neighborhood_dark = dark_cluster_density(uv, px);
    let cluster = clamp(neighborhood_dark * (0.58 + 0.42 * base_dark), 0.0, 1.0);

    let response_curve = 1.0 + 0.6 * (1.0 - params.scanline_strength);
    let cluster_focus = pow(cluster, response_curve);
    let amount = clamp((0.2 + params.color_intensity * 0.92) * cluster_focus, 0.0, 0.95);

    let red_shift = vec3<f32>(
        min(1.0, base.r + cluster_focus * 0.55),
        base.g * (1.0 - cluster_focus * 0.62),
        base.b * (1.0 - cluster_focus * 0.78),
    );
    var color = mix(base, red_shift, amount);

    let bleed = vec3<f32>(1.0, 0.14, 0.08) * cluster_focus * (0.05 + params.curvature * 0.14);
    color += bleed;

    let shimmer = 0.96 + 0.04 * sin(params.time_seconds * 8.0 + uv.y * 30.0);
    color *= shimmer;

    return vec4<f32>(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
