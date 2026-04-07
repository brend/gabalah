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
    return textureSample(frame_texture, frame_sampler, clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0))).rgb;
}

fn quantize(v: vec3<f32>, steps: f32) -> vec3<f32> {
    return floor(v * steps + 0.5) / steps;
}

fn funky_palette(uv: vec2<f32>, luma: f32, t: f32) -> vec3<f32> {
    let phase = luma * 11.0 + uv.x * 16.0 - uv.y * 13.0 + t * 1.4;
    return vec3<f32>(
        0.5 + 0.5 * sin(phase),
        0.5 + 0.5 * sin(phase + 2.0943951),
        0.5 + 0.5 * sin(phase + 4.1887902),
    );
}

fn edge_energy(uv: vec2<f32>) -> f32 {
    let px = vec2<f32>(1.0 / 160.0, 1.0 / 144.0);
    let c = sample_frame(uv);
    let r = sample_frame(uv + vec2<f32>(px.x, 0.0));
    let l = sample_frame(uv - vec2<f32>(px.x, 0.0));
    let u = sample_frame(uv - vec2<f32>(0.0, px.y));
    let d = sample_frame(uv + vec2<f32>(0.0, px.y));
    let gx = abs(r - l);
    let gy = abs(d - u);
    let g = dot(gx + gy, vec3<f32>(0.3333, 0.3333, 0.3333));
    let center_luma = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
    return clamp(g * 1.6 + center_luma * 0.2, 0.0, 1.0);
}

fn style_a(base: vec3<f32>, uv: vec2<f32>, intensity: f32, t: f32) -> vec3<f32> {
    let luma = dot(base, vec3<f32>(0.2126, 0.7152, 0.0722));
    let pal = funky_palette(uv, luma, t);
    let poster = quantize(base, 4.0 + intensity * 3.0);
    let neon = mix(poster, pal * (0.6 + 0.7 * luma), 0.45 + 0.4 * intensity);
    return clamp(neon, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn style_b(base: vec3<f32>, uv: vec2<f32>, intensity: f32, t: f32) -> vec3<f32> {
    let luma = dot(base, vec3<f32>(0.2126, 0.7152, 0.0722));
    let wave = 0.5 + 0.5 * sin((uv.x + uv.y) * 24.0 + t * 2.2);
    let remap = vec3<f32>(base.b, base.r, base.g);
    let pal = funky_palette(uv.yx, luma + wave * 0.5, t + 0.7);
    return clamp(mix(remap, pal, 0.4 + 0.45 * intensity), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn style_c(base: vec3<f32>, uv: vec2<f32>, intensity: f32, t: f32) -> vec3<f32> {
    let luma = dot(base, vec3<f32>(0.2126, 0.7152, 0.0722));
    let pal = funky_palette(uv * 1.7, luma, t * 0.85);
    let band = floor(luma * (5.0 + intensity * 4.0)) / (5.0 + intensity * 4.0);
    let ink = mix(vec3<f32>(band), pal, 0.55 + 0.3 * intensity);
    return clamp(mix(base, ink, 0.5 + 0.25 * intensity), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn apply_style(base: vec3<f32>, uv: vec2<f32>, intensity: f32, t: f32) -> vec3<f32> {
    if params.color_mode < 0.5 {
        return style_a(base, uv, intensity, t);
    }
    if params.color_mode < 1.5 {
        return style_b(base, uv, intensity, t);
    }
    return style_c(base, uv, intensity, t);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let intensity = clamp(0.4 + params.color_intensity * 0.7, 0.0, 1.5);
    let base = sample_frame(uv);
    var color = apply_style(base, uv, intensity, params.time_seconds);

    let edge = edge_energy(uv);
    let glow = funky_palette(uv, edge, params.time_seconds + 1.3);
    color = mix(color, color + glow * edge * (0.25 + 0.35 * intensity), 0.55);

    return vec4<f32>(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
