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

fn saturate(color: vec3<f32>, amount: f32) -> vec3<f32> {
    let y = luma(color);
    return clamp(vec3<f32>(y) + (color - vec3<f32>(y)) * amount, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn rotate(local: vec2<f32>, angle: f32) -> vec2<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec2<f32>(
        local.x * c - local.y * s,
        local.x * s + local.y * c,
    );
}

fn dot_mask(local_01: vec2<f32>, radius: f32) -> f32 {
    let p = local_01 - vec2<f32>(0.5, 0.5);
    let dist = length(p);
    let aa = max(fwidth(dist), 0.0015);
    return 1.0 - smoothstep(radius - aa, radius + aa, dist);
}

fn edge_energy(uv: vec2<f32>, px: vec2<f32>) -> f32 {
    let c = sample_frame(uv);
    let r = sample_frame(uv + vec2<f32>(px.x, 0.0));
    let l = sample_frame(uv - vec2<f32>(px.x, 0.0));
    let u = sample_frame(uv - vec2<f32>(0.0, px.y));
    let d = sample_frame(uv + vec2<f32>(0.0, px.y));

    let gx = abs(r - l);
    let gy = abs(d - u);
    let g = dot(gx + gy, vec3<f32>(0.3333, 0.3333, 0.3333));
    let center_luma = luma(c);
    return clamp(g * 1.35 + abs(center_luma - 0.5) * 0.2, 0.0, 1.0);
}

fn style_classic(base: vec3<f32>, intensity: f32) -> vec3<f32> {
    let warm = mix(base, base * vec3<f32>(1.02, 0.98, 0.9), 0.5);
    return saturate(clamp(warm + vec3<f32>(0.02, 0.015, 0.0) * (0.3 + intensity * 0.35), vec3<f32>(0.0), vec3<f32>(1.0)), 1.0 + intensity * 0.35);
}

fn style_manga(base: vec3<f32>, intensity: f32) -> vec3<f32> {
    let y = luma(base);
    let threshold = smoothstep(0.24, 0.76, y);
    let inked = mix(vec3<f32>(0.05), vec3<f32>(0.95), threshold);
    let merged = mix(vec3<f32>(y), inked, 0.58 + 0.2 * intensity);
    return clamp((merged - vec3<f32>(0.5)) * (1.22 + intensity * 0.65) + vec3<f32>(0.5), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn style_pop(base: vec3<f32>, uv: vec2<f32>, px: vec2<f32>, intensity: f32) -> vec3<f32> {
    let shift = px * (0.35 + intensity * 0.6);
    let r = sample_frame(uv + vec2<f32>(shift.x, 0.0)).r;
    let b = sample_frame(uv - vec2<f32>(shift.x, 0.0)).b;
    let split = vec3<f32>(r, base.g, b);
    let cmy_shift = vec3<f32>(split.g, split.b, split.r);
    let pop = mix(split, cmy_shift, 0.28 + 0.22 * intensity);
    return saturate(clamp(pop, vec3<f32>(0.0), vec3<f32>(1.0)), 1.08 + intensity * 0.45);
}

fn apply_style(base: vec3<f32>, uv: vec2<f32>, px: vec2<f32>, intensity: f32) -> vec3<f32> {
    if params.color_mode < 0.5 {
        return style_classic(base, intensity);
    }
    if params.color_mode < 1.5 {
        return style_manga(base, intensity);
    }
    return style_pop(base, uv, px, intensity);
}

fn outline_glow_color() -> vec3<f32> {
    if params.color_mode < 0.5 {
        return vec3<f32>(0.95, 0.72, 0.32);
    }
    if params.color_mode < 1.5 {
        return vec3<f32>(0.85, 0.85, 0.9);
    }
    return vec3<f32>(0.98, 0.28, 0.46);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(frame_texture));
    let px = vec2<f32>(1.0 / dims.x, 1.0 / dims.y);
    let uv = clamp(in.uv, vec2<f32>(0.0), vec2<f32>(1.0));
    let grid = uv * dims;

    let base = sample_frame(uv);
    let intensity = clamp(0.35 + params.color_intensity * 0.75, 0.2, 1.6);
    var color = apply_style(base, uv, px, intensity);

    let lum = luma(color);
    let density = clamp(params.scanline_strength, 0.0, 1.0);

    let shadow_scale = 0.85 + density * 1.15;
    let mid_scale = 0.65 + density * 0.95;
    let high_scale = 0.52 + density * 0.75;

    let shadow_centered = rotate(fract(grid * shadow_scale + vec2<f32>(0.11, 0.07)) - vec2<f32>(0.5, 0.5), 0.33);
    let mid_centered = rotate(fract(grid * mid_scale + vec2<f32>(0.37, 0.21)) - vec2<f32>(0.5, 0.5), -0.28);
    let high_centered = rotate(fract(grid * high_scale + vec2<f32>(0.63, 0.47)) - vec2<f32>(0.5, 0.5), 0.52);

    let shadow_dots = dot_mask(shadow_centered + vec2<f32>(0.5, 0.5), clamp(0.43 - density * 0.1, 0.27, 0.45));
    let mid_dots = dot_mask(mid_centered + vec2<f32>(0.5, 0.5), clamp(0.34 - density * 0.1, 0.2, 0.38));
    let high_dots = dot_mask(high_centered + vec2<f32>(0.5, 0.5), clamp(0.24 - density * 0.08, 0.12, 0.28));

    let shadow_band = 1.0 - smoothstep(0.22, 0.54, lum);
    let mid_band = smoothstep(0.2, 0.47, lum) * (1.0 - smoothstep(0.56, 0.82, lum));
    let high_band = smoothstep(0.6, 0.93, lum);

    let shadow_amount = shadow_band * shadow_dots * (0.2 + intensity * 0.26);
    color = mix(color, color * 0.2, clamp(shadow_amount, 0.0, 0.92));

    let mid_amount = mid_band * mid_dots * (0.11 + intensity * 0.16);
    color = mix(color, color * vec3<f32>(1.05, 0.98, 0.9), clamp(mid_amount, 0.0, 0.75));

    let high_amount = high_band * high_dots * (0.09 + intensity * 0.16);
    color += vec3<f32>(1.0, 0.94, 0.86) * high_amount * 0.26;

    let edge = edge_energy(uv, px);
    let center = 0.34 + 0.02 * sin(params.time_seconds * 2.7 + grid.x * 0.07 + grid.y * 0.05);
    let width = 0.07 + density * 0.05;
    let outline = 1.0 - smoothstep(width, width + 0.045, abs(edge - center));
    let pulse = 0.9 + 0.1 * sin(params.time_seconds * 4.1 + grid.x * 0.09 + grid.y * 0.06);
    let ink = outline * pulse * (0.28 + density * 0.42);

    color = mix(color, vec3<f32>(0.03, 0.02, 0.03), clamp(ink, 0.0, 0.88));
    color += outline_glow_color() * ink * (0.015 + params.color_intensity * 0.07);

    let centered = uv * 2.0 - vec2<f32>(1.0, 1.0);
    let corner_falloff = smoothstep(0.28, 1.2, dot(centered, centered));
    let panel_edge = smoothstep(0.74, 0.98, max(abs(centered.x), abs(centered.y)));
    let vignette = 1.0
        - corner_falloff * (0.06 + params.curvature * 0.24)
        - panel_edge * (0.03 + params.curvature * 0.17);
    color = clamp(color * vignette, vec3<f32>(0.0), vec3<f32>(1.0));

    return vec4<f32>(color, 1.0);
}
