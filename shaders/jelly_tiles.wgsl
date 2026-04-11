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

fn rounded_box_sdf(p: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(p) - half_size + vec2<f32>(radius, radius);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

fn gel_highlight(local: vec2<f32>, t: f32) -> f32 {
    let highlight_pos = vec2<f32>(-0.18 + 0.018 * sin(t * 0.8), 0.17 + 0.02 * cos(t * 0.9));
    let h = (local - highlight_pos) * vec2<f32>(2.5, 3.3);
    return exp(-dot(h, h) * 5.5);
}

fn style_candy(base: vec3<f32>, intensity: f32) -> vec3<f32> {
    let softened = mix(base, sqrt(clamp(base, vec3<f32>(0.0), vec3<f32>(1.0))), 0.4 + 0.2 * intensity);
    let tint = vec3<f32>(0.04, 0.03, 0.05) * (0.5 + intensity * 0.6);
    return saturate(clamp(softened + tint, vec3<f32>(0.0), vec3<f32>(1.0)), 1.05 + 0.35 * intensity);
}

fn style_citrus(base: vec3<f32>, intensity: f32) -> vec3<f32> {
    let remap = vec3<f32>(
        base.r * 0.55 + base.g * 0.45,
        base.g * 0.75 + base.r * 0.25,
        base.b * 0.35 + base.g * 0.65,
    );
    let contrast = clamp((remap - vec3<f32>(0.5)) * (1.2 + intensity * 0.55) + vec3<f32>(0.5), vec3<f32>(0.0), vec3<f32>(1.0));
    let tint = vec3<f32>(0.07, 0.05, 0.01) * (0.5 + intensity * 0.7);
    return saturate(clamp(contrast + tint, vec3<f32>(0.0), vec3<f32>(1.0)), 1.12 + 0.4 * intensity);
}

fn style_glass(base: vec3<f32>, intensity: f32) -> vec3<f32> {
    let y = luma(base);
    let glass = mix(vec3<f32>(y), base, 0.55 + 0.12 * intensity);
    let cool = glass * vec3<f32>(0.82, 0.95, 1.08) + vec3<f32>(0.01, 0.03, 0.06) * (0.55 + intensity * 0.5);
    return saturate(clamp(cool, vec3<f32>(0.0), vec3<f32>(1.0)), 0.98 + 0.24 * intensity);
}

fn apply_style(base: vec3<f32>, intensity: f32) -> vec3<f32> {
    if params.color_mode < 0.5 {
        return style_candy(base, intensity);
    }
    if params.color_mode < 1.5 {
        return style_citrus(base, intensity);
    }
    return style_glass(base, intensity);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(frame_texture));
    let centered = in.uv - vec2<f32>(0.5, 0.5);
    let radial = clamp(length(centered * 2.0), 0.0, 1.0);

    let center_zoom = 1.0 - params.curvature * 0.06 * (1.0 - radial);
    let warped_uv = vec2<f32>(0.5, 0.5) + centered * center_zoom;

    let grid = warped_uv * dims;
    let wobble_strength = clamp(params.scanline_strength, 0.0, 1.0);
    let speed = 1.8 + wobble_strength * 3.8;

    // Macro wobble shifts whole pixel tiles so motion is clearly visible.
    let pre_cell = floor(grid);
    let pre_id = pre_cell.x + pre_cell.y * dims.x;
    let pre_phase = params.time_seconds * speed + pre_id * 0.021;
    let macro_amp = 0.045 + wobble_strength * 0.24;
    let macro_offset = vec2<f32>(
        sin(pre_phase + grid.y * 0.09) + 0.45 * sin(pre_phase * 1.7 - grid.x * 0.06),
        cos(pre_phase * 0.9 - grid.x * 0.082) + 0.4 * cos(pre_phase * 1.55 + grid.y * 0.05),
    ) * macro_amp;

    let animated_grid = grid + macro_offset;
    let cell = floor(animated_grid);
    let cell_uv = (cell + vec2<f32>(0.5, 0.5)) / dims;
    let base = sample_frame(cell_uv);

    let intensity = clamp(0.25 + params.color_intensity * 0.85, 0.0, 1.6);
    let styled = apply_style(base, intensity);

    let cell_id = cell.x + cell.y * dims.x;
    let phase = params.time_seconds * speed * 1.35 + cell_id * 0.037;

    let local = fract(animated_grid) - vec2<f32>(0.5, 0.5);
    let micro_amp = 0.04 + wobble_strength * 0.16;
    let jelly_pulse = 1.0 + 0.2 * sin(phase * 0.72 + cell_id * 0.02);
    let local_wobble = local * jelly_pulse
        + vec2<f32>(
            sin(phase + local.y * 13.0) + 0.35 * sin(phase * 1.9 - local.x * 7.0),
            cos(phase * 0.83 - local.x * 12.0) + 0.3 * cos(phase * 1.6 + local.y * 8.0),
        ) * micro_amp;

    let radius = clamp(0.11 + params.scanline_strength * 0.13, 0.1, 0.25);
    let sdf = rounded_box_sdf(local_wobble, vec2<f32>(0.46, 0.46), radius);
    let aa = max(fwidth(sdf), 0.0025);
    let mask = 1.0 - smoothstep(-aa, aa, sdf);

    let rim = 1.0 - smoothstep(0.0, 0.09, abs(sdf));
    let rim_darkening = rim * (0.08 + params.scanline_strength * 0.28);

    let highlight = gel_highlight(local_wobble, params.time_seconds)
        * (0.07 + params.color_intensity * 0.27);

    let tile = clamp(styled * (1.0 - rim_darkening) + vec3<f32>(highlight), vec3<f32>(0.0), vec3<f32>(1.0));
    let bg = styled * (0.34 + 0.14 * params.scanline_strength);
    var color = mix(bg, tile, mask);

    let vignette = 1.0 - params.curvature * 0.55 * smoothstep(0.35, 1.0, radial);
    color = clamp(color * vignette, vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(color, 1.0);
}
