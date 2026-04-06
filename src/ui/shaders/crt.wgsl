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

fn hash21(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453123);
}

fn apply_prism_mode(base: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    let chroma_amount = (0.5 + 1.75 * params.color_intensity) / 160.0;
    let wobble = sin(params.time_seconds * 2.1 + uv.y * 22.0) * chroma_amount;
    let split = vec3<f32>(
        sample_frame(uv + vec2<f32>(chroma_amount + wobble, 0.0)).r,
        base.g,
        sample_frame(uv - vec2<f32>(chroma_amount - wobble, 0.0)).b,
    );
    let luma = dot(split, vec3<f32>(0.2126, 0.7152, 0.0722));
    let phase = luma * 8.0 + (uv.x - uv.y) * 4.0 + params.time_seconds * 0.55;
    let rainbow = vec3<f32>(
        0.5 + 0.5 * sin(phase),
        0.5 + 0.5 * sin(phase + 2.0943951),
        0.5 + 0.5 * sin(phase + 4.1887902),
    );
    let mix_amount = clamp(0.28 + params.color_intensity * 0.55, 0.0, 1.0);
    return mix(split, split * (0.7 + 0.3 * rainbow) + rainbow * 0.25, mix_amount);
}

fn apply_aurora_mode(base: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    let pulse = 0.5 + 0.5 * sin(params.time_seconds * 1.7 + (uv.x + uv.y) * 26.0);
    let shuffled = vec3<f32>(
        mix(base.g, base.b, 0.35 + 0.25 * pulse),
        mix(base.b, base.r, 0.25 + 0.35 * pulse),
        mix(base.r, base.g, 0.45 - 0.2 * pulse),
    );
    let aurora = vec3<f32>(
        0.5 + 0.5 * sin(uv.y * 10.0 + params.time_seconds * 0.8),
        0.5 + 0.5 * sin((uv.x * 12.0 - uv.y * 6.0) - params.time_seconds * 0.65 + 2.0943951),
        0.5 + 0.5 * sin((uv.x + uv.y) * 14.0 + params.time_seconds * 0.75 + 4.1887902),
    );
    let mix_amount = clamp((0.2 + 0.7 * pulse) * params.color_intensity, 0.0, 1.0);
    return mix(shuffled, shuffled * 0.4 + aurora * 0.9, mix_amount);
}

fn apply_palette_mutation_mode(base: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    let luma = dot(base, vec3<f32>(0.2126, 0.7152, 0.0722));
    let band_index = floor(clamp(luma * 4.0, 0.0, 3.999));
    let band_mix = band_index / 3.0;

    let cell = floor(uv * vec2<f32>(28.0, 24.0));
    let seed = hash21(cell + vec2<f32>(band_index * 17.0, band_index * 11.0));
    let hue_seed = hash21(cell.yx + vec2<f32>(13.0 + band_index, 5.0 + band_index * 2.0));
    let hue_offset = hue_seed * 6.2831853;

    let phase_a =
        dot(uv, vec2<f32>(24.0 + 10.0 * seed, 9.0 - 4.0 * seed))
        + params.time_seconds * (0.22 + seed * 0.85)
        + band_index * 1.5707963
        + hue_offset;
    let phase_b = phase_a * 1.73 + seed * 6.2831853 + params.time_seconds * 0.18;
    let phase_c = phase_a * 0.61 - phase_b * 0.27 + hue_offset * 1.7 + params.time_seconds * 0.12;

    let palette_a = vec3<f32>(
        0.5 + 0.5 * sin(phase_a),
        0.5 + 0.5 * sin(phase_a + 2.0943951),
        0.5 + 0.5 * sin(phase_a + 4.1887902),
    );
    let palette_b = vec3<f32>(
        0.5 + 0.5 * cos(phase_b + 2.2),
        0.5 + 0.5 * cos(phase_b + 4.6),
        0.5 + 0.5 * cos(phase_b + 1.1),
    );
    let palette_c = vec3<f32>(
        0.5 + 0.5 * sin(phase_c + 1.3),
        0.5 + 0.5 * sin(phase_c + 3.7),
        0.5 + 0.5 * sin(phase_c + 5.2),
    );
    let blend_ab = 0.2 + 0.6 * hash21(cell + vec2<f32>(3.1, 7.7));
    let blend_c = 0.12 + 0.33 * (0.5 + 0.5 * sin(params.time_seconds * 0.4 + seed * 7.0));
    let palette = mix(mix(palette_a, palette_b, blend_ab), palette_c, blend_c);
    let palette_soft = mix(vec3<f32>(luma), palette, 0.58 + 0.24 * params.color_intensity);

    let tone = mix(0.45 + band_mix * 0.45, 0.82 - band_mix * 0.15, 0.3 + 0.25 * seed);
    let spark = 0.92 + 0.08 * sin(params.time_seconds * 1.4 + seed * 20.0 + luma * 12.0);
    let mutated = palette_soft * tone * spark;
    let mutation_with_detail = mix(base, mutated, 0.45);

    let mix_amount = clamp(0.1 + params.color_intensity * 0.35, 0.0, 0.6);
    return clamp(mix(base, mutation_with_detail, mix_amount), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn apply_color_mode(base: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    if params.color_mode < 0.5 {
        return base;
    }
    if params.color_mode < 1.5 {
        return apply_prism_mode(base, uv);
    }
    if params.color_mode < 2.5 {
        return apply_aurora_mode(base, uv);
    }
    return apply_palette_mutation_mode(base, uv);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let uv = warp_uv(in.uv, params.curvature);
    if any(uv < vec2<f32>(0.0, 0.0)) || any(uv > vec2<f32>(1.0, 1.0)) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    var color = vec4<f32>(apply_color_mode(sample_frame(uv), uv), 1.0);

    let scanline_phase = (uv.y + params.time_seconds * 0.15) * 720.0;
    let scanline = 0.5 + 0.5 * sin(scanline_phase);
    let scanline_mask = 1.0 - scanline * params.scanline_strength;
    color = vec4<f32>(color.rgb * scanline_mask, color.a);

    return vec4<f32>(color.rgb, 1.0);
}
