// Copyright 2026 Emilie Roberts
// SPDX-License-Identifier: Apache-2.0
#version 450

struct SdfInstance {
    vec4 rect;             // [x, y, width, height]
    vec4 corner_radii;     // [tl, tr, br, bl] or [u0, v0, u1, v1] for glyph quads (mat == 14)
    vec4 clip_rect;        // [clip_x, clip_y, clip_w, clip_h]
    vec4 fill_color_start; // RGBA
    vec4 fill_color_end;   // RGBA
    vec4 border_color;     // RGBA
    vec4 shadow_color;     // RGBA
    vec4 params;           // [border_w, shadow_blur, exponent_p, packed_mat_border]
};

layout(std430, set = 0, binding = 0) readonly buffer InstanceBuffer {
    SdfInstance instances[];
} u_instances;

layout(set = 0, binding = 1) uniform sampler2D u_showcase_tex;
layout(set = 0, binding = 2) uniform sampler2D u_glyph_atlas;
layout(set = 0, binding = 3) uniform sampler2D u_noise_lut;

layout(push_constant) uniform PushConstants {
    vec2 viewport;
    vec2 pointer;
    float time_sec;
    uint flags;
    vec2 _pad;
} pc;

layout(location = 0) in vec2 v_px;
layout(location = 1) flat in uint v_inst_idx;

layout(location = 0) out vec4 out_color;

float eval_sdf_superellipse(vec2 px, vec4 rect, vec4 radii, float exponent_p) {
    vec2 half_sz = rect.zw * 0.5;
    vec2 center = rect.xy + half_sz;
    vec2 rel = px - center;
    float r = (rel.x < 0.0)
        ? ((rel.y < 0.0) ? radii.x : radii.w)
        : ((rel.y < 0.0) ? radii.y : radii.z);
    r = min(r, min(half_sz.x, half_sz.y));
    vec2 q = abs(rel) - (half_sz - vec2(r));
    vec2 q_pos = max(q, vec2(0.0));
    float p = clamp(exponent_p, 1.5, 8.0);
    float outside;
    if (abs(p - 2.0) < 0.05) {
        outside = length(q_pos);
    } else if (q_pos.x == 0.0) {
        outside = q_pos.y;
    } else if (q_pos.y == 0.0) {
        outside = q_pos.x;
    } else {
        float m = max(q_pos.x, q_pos.y);
        float ratio = min(q_pos.x, q_pos.y) / m;
        outside = m * pow(1.0 + pow(ratio, p), 1.0 / p);
    }
    float inside = min(max(q.x, q.y), 0.0);
    return outside + inside - r;
}

float sample_fbm(vec2 uv) {
    return texture(u_noise_lut, uv).r;
}

float sample_warped(vec2 uv) {
    return texture(u_noise_lut, uv).g;
}

vec4 blackbody_fire(float heat) {
    float t = clamp(heat, 0.0, 1.0);
    vec3 col;
    if (t < 0.25) {
        float k = t / 0.25;
        col = vec3(k * 0.55, k * 0.04, 0.0);
    } else if (t < 0.55) {
        float k = (t - 0.25) / 0.30;
        col = vec3(0.55 + k * 0.45, 0.04 + k * 0.38, k * 0.04);
    } else if (t < 0.82) {
        float k = (t - 0.55) / 0.27;
        col = vec3(1.0, 0.42 + k * 0.48, 0.04 + k * 0.22);
    } else {
        float k = (t - 0.82) / 0.18;
        col = vec3(1.0, 0.90 + k * 0.10, 0.26 + k * 0.74);
    }
    float alpha = clamp(pow(t, 0.75) * 1.15, 0.0, 1.0);
    return vec4(col, alpha);
}

vec4 smoke_ramp(float density) {
    float d = clamp(density, 0.0, 1.0);
    vec3 col;
    if (d < 0.4) {
        float k = d / 0.4;
        col = vec3(0.18 + k * 0.24, 0.22 + k * 0.26, 0.30 + k * 0.28);
    } else {
        float k = (d - 0.4) / 0.6;
        col = vec3(0.42 + k * 0.40, 0.48 + k * 0.38, 0.58 + k * 0.34);
    }
    return vec4(col, clamp(pow(d, 0.92) * 0.88, 0.0, 0.90));
}

vec3 electric_jellyfish_field(vec2 uv, float time_sec, bool dark_mode) {
    float t = time_sec * 0.55;
    vec3 base_col = dark_mode
        ? mix(vec3(0.015, 0.032, 0.095), vec3(0.008, 0.018, 0.058), uv.y)
        : mix(vec3(0.76, 0.84, 0.95), vec3(0.66, 0.76, 0.90), uv.y);

    vec2 c0 = vec2(0.26 + sin(t * 0.62) * 0.16, 0.34 + cos(t * 0.48) * 0.14);
    vec2 c1 = vec2(0.74 + cos(t * 0.51 + 1.2) * 0.15, 0.64 + sin(t * 0.67 + 0.8) * 0.15);
    vec2 c2 = vec2(0.52 + sin(t * 0.43 + 2.5) * 0.20, 0.26 + sin(t * 0.59 + 1.9) * 0.12);
    vec2 c3 = vec2(0.35 + cos(t * 0.38 + 4.1) * 0.18, 0.74 + cos(t * 0.54 + 3.1) * 0.12);

    vec2 centers[4] = vec2[4](c0, c1, c2, c3);
    vec2 radii[4] = vec2[4](vec2(0.19, 0.14), vec2(0.22, 0.16), vec2(0.16, 0.12), vec2(0.20, 0.15));
    float phases[4] = float[4](0.0, 1.7, 3.3, 4.9);
    vec3 colors[4] = vec3[4](
        vec3(0.08, 0.68, 0.98),
        vec3(0.28, 0.38, 0.98),
        vec3(0.05, 0.86, 0.82),
        vec3(0.52, 0.26, 0.96)
    );

    vec3 accum = base_col;
    for (int i = 0; i < 4; ++i) {
        float pulse = 0.84 + 0.16 * sin(t * 2.1 + phases[i]);
        float rx = radii[i].x * pulse;
        float ry = radii[i].y * (1.84 - pulse);
        float dx = (uv.x - centers[i].x) / max(rx, 0.01);
        float dy_raw = (uv.y - centers[i].y) / max(ry, 0.01);
        float dome_dy = (dy_raw > 0.0) ? (dy_raw * 1.38) : (dy_raw * 0.92);
        float d2 = dx * dx + dome_dy * dome_dy;
        float core = 1.0 / (1.0 + d2 * 3.2);
        float halo = 1.0 / (1.0 + d2 * 1.15);
        float rim = max(1.0 - abs(d2 - 0.62) * 2.2, 0.0) * core;
        float rib = (0.5 + 0.5 * cos(dx * 6.28318 + sin(t + phases[i]) * 0.6)) * core * 0.28;
        float intensity = (core * 0.52 + halo * 0.34 + rim * 0.42 + rib) * (dark_mode ? 0.82 : 0.45);
        accum += colors[i] * intensity;
    }

    float n1 = sample_fbm(vec2(uv.x * 2.2 + t * 0.03, uv.y * 2.2 - t * 0.02));
    float n2 = sample_fbm(vec2(uv.x * 4.5 - t * 0.02, uv.y * 4.5 + t * 0.03));
    float frost = (n1 - 0.5) * 0.035 + (n2 - 0.5) * 0.018;
    float sheen = clamp(1.0 - abs(uv.x * 0.65 + uv.y * 0.35 - 0.48) * 1.8, 0.0, 1.0) * 0.045;
    return clamp(accum + vec3(frost * 0.6 + sheen * 0.5, frost * 0.8 + sheen * 0.7, frost + sheen), vec3(0.0), vec3(1.0));
}

void main() {
    SdfInstance inst = u_instances.instances[v_inst_idx];
    uint raw_mat = uint(inst.params.w + 0.5);
    uint mat = raw_mat % 100u;
    uint border_fx = raw_mat / 100u;

    // 1. GPU Glyph Atlas Quad (mat == 14)
    if (mat == 14u) {
        if (v_px.x < inst.clip_rect.x || v_px.y < inst.clip_rect.y ||
            v_px.x > inst.clip_rect.x + inst.clip_rect.z ||
            v_px.y > inst.clip_rect.y + inst.clip_rect.w) {
            discard;
        }
        vec2 uv = clamp((v_px - inst.rect.xy) / max(inst.rect.zw, vec2(1.0)), 0.0, 1.0);
        vec2 atlas_uv = mix(inst.corner_radii.xy, inst.corner_radii.zw, uv);
        float cov = texture(u_glyph_atlas, atlas_uv).r;
        if (cov <= 0.01) {
            discard;
        }
        out_color = vec4(inst.fill_color_start.rgb, inst.fill_color_start.a * cov);
        return;
    }

    // 2. Scissor clip check (expanding for fire/smoke border halos when clip_rect == rect)
    vec2 c_min = inst.clip_rect.xy;
    vec2 c_max = inst.clip_rect.xy + inst.clip_rect.zw;
    bool clip_is_self =
        abs(inst.clip_rect.x - inst.rect.x) < 1.0 &&
        abs(inst.clip_rect.y - inst.rect.y) < 1.0 &&
        abs(inst.clip_rect.z - inst.rect.z) < 1.0 &&
        abs(inst.clip_rect.w - inst.rect.w) < 1.0;
    if (clip_is_self && (border_fx != 0u || mat == 2u || mat == 7u)) {
        c_min -= vec2(32.0);
        c_max += vec2(32.0);
    }
    if (v_px.x < c_min.x || v_px.y < c_min.y || v_px.x > c_max.x || v_px.y > c_max.y) {
        discard;
    }

    float dist = eval_sdf_superellipse(v_px, inst.rect, inst.corner_radii, inst.params.z);
    float shape_alpha = clamp(0.5 - dist, 0.0, 1.0);
    float border_w = inst.params.x;
    float shadow_blur = inst.params.y;
    vec2 uv = clamp((v_px - inst.rect.xy) / max(inst.rect.zw, vec2(1.0)), 0.0, 1.0);
    vec4 grad = mix(inst.fill_color_start, inst.fill_color_end, uv.y);

    vec4 fill = grad;
    if (mat == 1u) {
        // FrostedGlass
        float spec = pow(1.0 - uv.y, 3.0) * 0.22 + (1.0 - uv.x) * 0.07;
        float lum = dot(grad.rgb, vec3(0.299, 0.587, 0.114));
        vec3 glass_rgb = (lum > 0.55)
            ? mix(grad.rgb, vec3(0.95, 0.97, 1.0), 0.35) + vec3(spec * 0.45)
            : mix(grad.rgb, vec3(0.18, 0.23, 0.34), 0.35) + vec3(spec);
        fill = vec4(clamp(glass_rgb, 0.0, 1.0), clamp(grad.a, 0.28, 0.90));
    } else if (mat == 2u) {
        // ProceduralFire
        float n = sample_warped(vec2(uv.x * 1.65, uv.y * 1.85 - pc.time_sec * 0.72));
        float detail = sample_fbm(vec2(uv.x * 3.4 + 0.3, uv.y * 3.2 - pc.time_sec * 1.15));
        float vertical_cooling = pow(uv.y, 0.55) * 0.82 + 0.18;
        float heat = clamp(vertical_cooling * 0.62 + n * 0.52 + detail * 0.22, 0.0, 1.0);
        vec4 fire = blackbody_fire(heat);
        fill = vec4(mix(grad.rgb, fire.rgb, 0.85), max(grad.a, fire.a));
    } else if (mat == 3u) {
        // WaterRipple
        float d = distance(v_px, pc.pointer);
        float wave = sin(d * 0.12 - pc.time_sec * 6.0) * exp(-d * 0.006);
        float caustic = max(wave, 0.0) * 0.35;
        fill = vec4(
            clamp(grad.rgb + vec3(caustic * 0.6, caustic * 0.85, caustic * 1.2), 0.0, 1.0),
            grad.a
        );
    } else if (mat == 4u) {
        // FluentRevealHighlight
        float d = distance(v_px, pc.pointer);
        float glow = exp(-d * 0.015) * 0.45;
        fill = vec4(clamp(grad.rgb + vec3(glow), 0.0, 1.0), grad.a);
    } else if (mat == 5u) {
        // ObsidianNeonPulse
        float pulse = (sin(pc.time_sec * 3.5) * 0.5 + 0.5) * 0.35;
        float edge_glow = exp(-abs(dist) * 0.25) * (0.4 + pulse);
        fill = vec4(
            clamp(grad.rgb + inst.border_color.rgb * edge_glow, 0.0, 1.0),
            grad.a
        );
    } else if (mat == 6u) {
        // TexturedImage + WaterRipple
        float d = distance(v_px, pc.pointer);
        float wave = sin(d * 0.11 - pc.time_sec * 5.5) * exp(-d * 0.005);
        vec2 disp_uv = clamp(uv + vec2(wave * 0.018), 0.0, 1.0);
        vec3 tex_rgb = texture(u_showcase_tex, disp_uv).rgb;
        float spec = max(wave, 0.0) * 0.28;
        fill = vec4(clamp(tex_rgb + vec3(spec * 0.65, spec * 0.85, spec), 0.0, 1.0), grad.a);
    } else if (mat == 7u) {
        // GentleSmoke
        float drift_x = sin(pc.time_sec * 0.35 + uv.y * 2.4) * 0.12;
        float w1 = sample_warped(vec2(uv.x * 1.35 + drift_x, uv.y * 1.45 - pc.time_sec * 0.22));
        float w2 = sample_fbm(vec2(uv.x * 2.6 - drift_x * 0.7, uv.y * 2.4 - pc.time_sec * 0.34));
        float envelope = sin(uv.y * 3.14159265) * 0.75 + 0.25;
        float density = clamp((w1 * 0.62 + w2 * 0.38) * envelope, 0.0, 1.0);
        vec4 smk = smoke_ramp(density);
        fill = vec4(mix(grad.rgb, smk.rgb, 0.72), max(grad.a, smk.a * 0.85));
    } else if (mat == 8u) {
        // GentlePulsingLights
        float t = pc.time_sec * 0.85;
        vec2 o1 = vec2(0.28 + sin(t * 0.9) * 0.18, 0.45 + cos(t * 0.7) * 0.22);
        vec2 o2 = vec2(0.72 + cos(t * 0.8) * 0.16, 0.52 + sin(t * 1.1) * 0.20);
        vec2 o3 = vec2(0.50 + sin(t * 0.6 + 1.4) * 0.22, 0.38 + cos(t * 0.95) * 0.18);
        float g1 = exp(-dot(uv - o1, uv - o1) * 9.5) * (0.65 + 0.35 * sin(t * 1.8));
        float g2 = exp(-dot(uv - o2, uv - o2) * 8.5) * (0.65 + 0.35 * cos(t * 2.1 + 1.0));
        float g3 = exp(-dot(uv - o3, uv - o3) * 10.5) * (0.65 + 0.35 * sin(t * 1.5 + 2.2));
        vec3 add_rgb = vec3(
            g1 * 0.25 + g2 * 0.65 + g3 * 0.95,
            g1 * 0.82 + g2 * 0.40 + g3 * 0.55,
            g1 * 0.96 + g2 * 0.98 + g3 * 0.35
        );
        fill = vec4(clamp(grad.rgb * 0.78 + add_rgb * 0.68, 0.0, 1.0), grad.a);
    } else if (mat == 9u) {
        // ReflectiveGlass
        vec2 center = inst.rect.xy + inst.rect.zw * 0.5;
        float ptr_shift = clamp((pc.pointer.x - center.x) * 0.0012 + (pc.pointer.y - center.y) * 0.0008, -0.35, 0.35);
        float diag = uv.x * 0.72 + uv.y * 0.48 - ptr_shift + sin(pc.time_sec * 0.45) * 0.08;
        float b1 = pow(clamp(1.0 - abs(diag - 0.52) * 5.5, 0.0, 1.0), 2.0) * 0.52;
        float b2 = pow(clamp(1.0 - abs(diag - 0.78) * 9.0, 0.0, 1.0), 2.0) * 0.28;
        float fresnel = pow(1.0 - sin(uv.y * 3.14159265), 2.0) * 0.18;
        float sheen = b1 + b2 + fresnel;
        fill = vec4(
            clamp(grad.rgb * 0.85 + vec3(sheen * 0.88, sheen * 0.94, sheen * 1.05), 0.0, 1.0),
            clamp(grad.a + sheen * 0.22, 0.68, 0.96)
        );
    } else if (mat == 10u) {
        // ColorCyclingGlowbar
        float phase = fract(uv.x * 0.85 - pc.time_sec * 0.28 + sin(uv.y * 3.14159265) * 0.08);
        vec3 cyc = vec3(
            0.55 + 0.42 * cos(6.2831853 * (phase + 0.00)),
            0.55 + 0.42 * cos(6.2831853 * (phase + 0.33)),
            0.55 + 0.42 * cos(6.2831853 * (phase + 0.67))
        );
        float core = 1.0 - abs(uv.y - 0.5) * 1.35;
        float boost = clamp(core, 0.15, 1.0) * 0.28;
        fill = vec4(clamp(mix(grad.rgb, cyc, 0.78) + vec3(boost), 0.0, 1.0), grad.a);
    } else if (mat == 11u) {
        // FlowingGentleGradient
        float w1 = sin(uv.x * 4.2 + pc.time_sec * 0.75 + uv.y * 2.1) * 0.5 + 0.5;
        float w2 = cos(uv.y * 3.6 - pc.time_sec * 0.55 + uv.x * 1.8) * 0.5 + 0.5;
        float blend = clamp(w1 * 0.58 + w2 * 0.42, 0.0, 1.0);
        vec3 gold = vec3(0.96, 0.76, 0.22);
        vec3 ivory = vec3(1.00, 0.97, 0.88);
        vec3 silk = mix(gold, ivory, blend);
        fill = vec4(mix(grad.rgb, silk, 0.82), grad.a);
    } else if (mat == 12u) {
        // ElectricJellyfish
        float lum = dot(grad.rgb, vec3(0.299, 0.587, 0.114));
        vec3 jf = electric_jellyfish_field(uv, pc.time_sec, lum < 0.5);
        fill = vec4(jf, max(grad.a, 0.96));
    } else if (mat == 13u) {
        // SubtleGlassReflection
        vec2 center = inst.rect.xy + inst.rect.zw * 0.5;
        float ptr_parallax = clamp((pc.pointer.x - center.x) * 0.00045 + (pc.pointer.y - center.y) * 0.00035, -0.16, 0.16);
        float sweep = uv.x * 0.68 + uv.y * 0.42 - ptr_parallax + sin(pc.time_sec * 0.10) * 0.06;
        float b1 = clamp(1.0 - abs(sweep - 0.46) * 5.2, 0.0, 1.0);
        float b2 = clamp(1.0 - abs(sweep - 0.72) * 9.0, 0.0, 1.0);
        float top_glint = pow(1.0 - uv.y, 4.0) * 0.055;
        float refl = min(b1 * b1 * 0.075 + b2 * b2 * 0.040 + top_glint, 0.14);
        fill = vec4(
            clamp(mix(grad.rgb, vec3(0.90, 0.96, 1.00), refl), 0.0, 1.0),
            clamp(grad.a + refl * 0.08, 0.30, 0.78)
        );
    }

    // 3. Border evaluation (Standard, FireBorder == 1, GentleSmokeBorder == 2)
    vec4 active_border_color = inst.border_color;
    float border_mask = 0.0;
    if (border_w > 0.0) {
        border_mask = clamp(0.5 - abs(dist + border_w * 0.5) + border_w * 0.5, 0.0, 1.0);
    }

    if (border_fx == 1u && abs(dist) < 28.0) {
        float n = sample_warped(vec2(v_px.x * 0.016 + 0.7, v_px.y * 0.016 - pc.time_sec * 0.95));
        float reach = 4.0 + n * 18.0 + (1.0 - uv.y) * 6.0;
        float band = clamp(1.0 - abs(dist) / max(reach, 2.0), 0.0, 1.0);
        float heat = clamp(band * (0.62 + n * 0.58), 0.0, 1.0);
        vec4 fb = blackbody_fire(heat);
        active_border_color = fb;
        border_mask = max(border_mask, band * fb.a);
    } else if (border_fx == 2u && abs(dist) < 32.0) {
        float drift = sin(pc.time_sec * 0.42 + v_px.y * 0.025) * 0.15;
        float w1 = sample_warped(vec2(v_px.x * 0.014 + drift, v_px.y * 0.014 - pc.time_sec * 0.30));
        float w2 = sample_fbm(vec2(v_px.x * 0.031 - drift * 0.8, v_px.y * 0.029 - pc.time_sec * 0.46));
        float ridge = clamp(1.0 - abs(w1 - 0.48) * 3.8, 0.0, 1.0);
        float wisp = pow(ridge, 2.0) * (0.45 + 0.55 * w2);
        float reach = 3.0 + wisp * 26.0;
        float falloff = clamp(1.0 - abs(dist) / reach, 0.0, 1.0);
        float density = clamp(falloff * (0.28 + wisp * 0.85), 0.0, 1.0);
        vec4 sb = smoke_ramp(density);
        active_border_color = sb;
        border_mask = max(border_mask, falloff * sb.a * 0.92);
    }

    // 4. Outer drop shadow + fire/smoke corona
    float shadow_alpha = 0.0;
    vec4 outer_color = inst.shadow_color;
    if (dist > 0.0) {
        if (mat == 2u && dist < 24.0) {
            float n = sample_warped(vec2(v_px.x * 0.018, v_px.y * 0.018 - pc.time_sec * 0.85));
            float reach = 6.0 + n * 16.0 + (1.0 - uv.y) * 5.0;
            float corona = clamp(1.0 - dist / max(reach, 2.0), 0.0, 1.0) * n;
            vec4 fc = blackbody_fire(corona * 0.92);
            outer_color = fc;
            shadow_alpha = corona * 0.88;
        } else if (mat == 7u && dist < 30.0) {
            float w1 = sample_warped(vec2(v_px.x * 0.014, v_px.y * 0.014 - pc.time_sec * 0.28));
            float ridge = clamp(1.0 - abs(w1 - 0.48) * 3.6, 0.0, 1.0);
            float wisp = ridge * ridge;
            float reach = 4.0 + wisp * 25.0;
            float plume = clamp(1.0 - dist / reach, 0.0, 1.0) * (0.30 + wisp * 0.70);
            vec4 sc = smoke_ramp(plume * 0.88);
            outer_color = sc;
            shadow_alpha = plume * 0.78;
        } else if (shadow_blur > 0.0) {
            float t = clamp(1.0 - dist / (shadow_blur * 2.5), 0.0, 1.0);
            shadow_alpha = t * t * inst.shadow_color.a;
        }
    }

    vec3 surface_rgb = mix(fill.rgb, active_border_color.rgb, border_mask * active_border_color.a);
    float fg_alpha = max(shape_alpha * fill.a, border_mask * active_border_color.a);
    float total_alpha = clamp(fg_alpha + shadow_alpha * (1.0 - fg_alpha), 0.0, 1.0);
    if (total_alpha <= 0.002) {
        discard;
    }
    vec3 final_rgb = (surface_rgb * fg_alpha + outer_color.rgb * shadow_alpha * (1.0 - fg_alpha)) / max(total_alpha, 1e-4);
    out_color = vec4(clamp(final_rgb, 0.0, 1.0), total_alpha);
}
