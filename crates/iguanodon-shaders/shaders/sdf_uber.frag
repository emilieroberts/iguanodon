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
    float r = clamp(radii.x, 0.0, min(half_sz.x, half_sz.y));
    vec2 q = abs(rel) - (half_sz - vec2(r));
    if (q.x <= 0.0 || q.y <= 0.0) {
        return max(q.x, q.y) - r;
    }
    float p = clamp(exponent_p, 1.0, 4.75);
    float m = max(q.x, q.y);
    float ratio = min(q.x, q.y) / m;
    return m * pow(1.0 + pow(ratio, p), 1.0 / p) - r;
}

// FireSimulationLut in Rust stores 16 periodic noise cells across 256 texels (`sx = nx * 16.0`),
// so 1 unit of `(nx, ny)` equals `16.0 / 256.0 = 1.0 / 16.0` in normalized [0, 1] texture UV space.
float sample_turbulence(float nx, float ny) {
    return texture(u_noise_lut, vec2(nx, ny) * (1.0 / 16.0)).r;
}

float sample_warped_fire(float nx, float ny) {
    return texture(u_noise_lut, vec2(nx, ny) * (1.0 / 16.0)).g;
}

float hash21(int ix, int iy) {
    uint n = uint(ix) * 374761393u ^ uint(iy) * 668265263u;
    n = (n ^ (n >> 13u)) * 1274126177u;
    return float((n ^ (n >> 16u)) & 0x00FFFFFFu) / 16777215.0;
}

vec4 blackbody_fire_color(float temp) {
    float t = clamp(temp, 0.0, 1.0);
    float r = pow(clamp(t * 2.1, 0.0, 1.0), 0.75);
    float g = pow(clamp((t - 0.22) * 1.45, 0.0, 1.0), 1.35);
    float b = pow(clamp((t - 0.62) * 2.4, 0.0, 1.0), 1.8);
    return vec4(r, g, b, 1.0);
}

vec4 color_cycling_spectrum(float hue) {
    float tau = 6.28318530718;
    float r = clamp(0.58 + 0.42 * cos(tau * (hue + 0.00)), 0.0, 1.0);
    float g = clamp(0.58 + 0.42 * cos(tau * (hue + 0.33)), 0.0, 1.0);
    float b = clamp(0.64 + 0.36 * cos(tau * (hue + 0.67)), 0.0, 1.0);
    return vec4(r, g, b, 1.0);
}

vec4 flowing_yellow_white_gradient(float uv_x, float uv_y, float t_sec, vec4 base_tint) {
    float tau = 6.28318530718;
    float phase = fract(uv_x - t_sec * 0.085 + uv_y * 0.12);
    float wave = 0.5 + 0.5 * cos(phase * tau);
    float shimmer = 0.5 + 0.5 * sin((phase * 2.0 - 0.15) * tau);

    vec4 deep_yellow = vec4(
        clamp(base_tint.r * 0.35 + 0.64, 0.82, 0.99),
        clamp(base_tint.g * 0.30 + 0.52, 0.66, 0.88),
        clamp(base_tint.b * 0.15 + 0.08, 0.08, 0.28),
        1.0
    );
    vec4 warm_butter = vec4(1.0, 0.92, 0.48, 1.0);
    vec4 luminous_white = vec4(1.0, 0.99, 0.93, 1.0);

    vec4 mid = mix(deep_yellow, warm_butter, wave);
    return mix(mid, luminous_white, clamp(wave * wave * 0.72 + shimmer * 0.22, 0.0, 0.92));
}

vec3 sample_4stop_linear(float t, vec3 c0, vec3 c1, vec3 c2, vec3 c3) {
    float tc = clamp(t, 0.0, 1.0) * 3.0;
    if (tc < 1.0) {
        return mix(c0, c1, tc);
    } else if (tc < 2.0) {
        return mix(c1, c2, tc - 1.0);
    } else {
        return mix(c2, c3, tc - 2.0);
    }
}

void blend_4stop_radial(
    inout vec3 dst,
    float t,
    vec3 c0,
    float a0,
    vec3 c1,
    float a1,
    vec3 c2,
    float a2
) {
    float tc = clamp(t, 0.0, 1.0) * 3.0;
    vec3 s_col;
    float s_a;
    if (tc < 1.0) {
        float u = tc;
        s_col = mix(c0, c1, u);
        s_a = mix(a0, a1, u);
    } else if (tc < 2.0) {
        float u = tc - 1.0;
        s_col = mix(c1, c2, u);
        s_a = mix(a1, a2, u);
    } else {
        float u = 1.0 - (tc - 2.0);
        s_col = c2;
        s_a = a2 * u;
    }
    dst = dst * (1.0 - s_a) + s_col * s_a;
}

vec4 evaluate_electric_jellyfish_backdrop(
    float uv_x,
    float uv_y,
    float width,
    float height,
    float t_sec,
    bool dark_theme
) {
    float w = max(width, 1.0);
    float h = max(height, 1.0);
    float min_dim = max(min(w, h), 1.0);
    float px = uv_x * w;
    float py = uv_y * h;

    float diag_t = clamp((uv_x + uv_y) * 0.5, 0.0, 1.0);
    vec3 rgb = dark_theme
        ? sample_4stop_linear(
            diag_t,
            vec3(4.0 / 255.0, 10.0 / 255.0, 24.0 / 255.0),
            vec3(7.0 / 255.0, 20.0 / 255.0, 43.0 / 255.0),
            vec3(10.0 / 255.0, 29.0 / 255.0, 59.0 / 255.0),
            vec3(5.0 / 255.0, 14.0 / 255.0, 32.0 / 255.0)
        )
        : sample_4stop_linear(
            diag_t,
            vec3(86.0 / 255.0, 160.0 / 255.0, 235.0 / 255.0),
            vec3(66.0 / 255.0, 140.0 / 255.0, 224.0 / 255.0),
            vec3(78.0 / 255.0, 152.0 / 255.0, 232.0 / 255.0),
            vec3(104.0 / 255.0, 177.0 / 255.0, 246.0 / 255.0)
        );

    vec2 base_pos[5] = vec2[5](
        vec2(0.22, 0.28),
        vec2(0.76, 0.24),
        vec2(0.52, 0.62),
        vec2(0.18, 0.78),
        vec2(0.84, 0.74)
    );
    vec2 orbit[5] = vec2[5](
        vec2(0.09, 0.08),
        vec2(0.08, 0.10),
        vec2(0.10, 0.09),
        vec2(0.08, 0.07),
        vec2(0.09, 0.08)
    );
    float speed_mul[5] = float[5](1.0, -0.8, 0.9, 1.1, -1.0);
    float phase_off[5] = float[5](0.0, 1.4, 2.7, 4.1, 5.2);
    float rad_fac[5] = float[5](0.48, 0.44, 0.54, 0.40, 0.46);

    float drift_phase = t_sec * (6.28318530718 / 22.0);
    float pulse_phase = t_sec * (6.28318530718 / 6.0);

    float outer_alpha = dark_theme ? 0.62 : 0.60;
    float mid_alpha = dark_theme ? 0.44 : 0.42;
    float rim_alpha = dark_theme ? 0.20 : 0.24;
    float core_white_alpha = dark_theme ? 0.48 : 0.54;
    float core_tint_alpha = dark_theme ? 0.64 : 0.58;

    for (int i = 0; i < 5; ++i) {
        vec3 b_core;
        vec3 b_mid;
        vec3 b_rim;
        if (dark_theme) {
            if (i == 0) {
                b_core = vec3(0.00, 0.96, 1.00);
                b_mid = vec3(0.00, 0.57, 0.92);
                b_rim = vec3(0.19, 0.31, 1.00);
            } else if (i == 1) {
                b_core = vec3(0.49, 0.30, 1.00);
                b_mid = vec3(0.00, 0.69, 1.00);
                b_rim = vec3(0.00, 0.90, 1.00);
            } else if (i == 2) {
                b_core = vec3(0.09, 1.00, 1.00);
                b_mid = vec3(0.16, 0.38, 1.00);
                b_rim = vec3(0.67, 0.00, 1.00);
            } else if (i == 3) {
                b_core = vec3(0.39, 1.00, 0.85);
                b_mid = vec3(0.00, 0.72, 0.83);
                b_rim = vec3(0.24, 0.35, 1.00);
            } else {
                b_core = vec3(0.88, 0.25, 0.98);
                b_mid = vec3(0.33, 0.43, 1.00);
                b_rim = vec3(0.00, 0.90, 1.00);
            }
        } else {
            if (i == 0) {
                b_core = vec3(1.00, 1.00, 1.00);
                b_mid = vec3(0.50, 0.97, 1.00);
                b_rim = vec3(0.09, 1.00, 1.00);
            } else if (i == 1) {
                b_core = vec3(0.91, 0.87, 1.00);
                b_mid = vec3(0.51, 0.69, 1.00);
                b_rim = vec3(0.00, 0.90, 1.00);
            } else if (i == 2) {
                b_core = vec3(1.00, 1.00, 1.00);
                b_mid = vec3(0.39, 1.00, 0.85);
                b_rim = vec3(0.33, 0.43, 1.00);
            } else if (i == 3) {
                b_core = vec3(0.88, 1.00, 0.98);
                b_mid = vec3(0.39, 1.00, 0.85);
                b_rim = vec3(0.00, 0.69, 1.00);
            } else {
                b_core = vec3(0.97, 0.73, 0.82);
                b_mid = vec3(0.50, 0.85, 1.00);
                b_rim = vec3(0.09, 1.00, 1.00);
            }
        }

        float angle = drift_phase * speed_mul[i] + phase_off[i];
        float pulse = 0.80 + 0.20 * sin(pulse_phase + phase_off[i]);
        float cx = w * (base_pos[i].x + orbit[i].x * cos(angle));
        float cy = h * (base_pos[i].y + orbit[i].y * sin(angle * 1.3));
        float radius = max(min_dim * rad_fac[i] * pulse, 80.0);
        float dx = px - cx;
        float dy = py - cy;
        float d_sq = dx * dx + dy * dy;
        if (d_sq < radius * radius) {
            float d = sqrt(d_sq);
            float t_outer = d / radius;
            blend_4stop_radial(
                rgb,
                t_outer,
                b_core,
                outer_alpha * pulse,
                b_mid,
                mid_alpha * pulse,
                b_rim,
                rim_alpha
            );
            float inner_radius = radius * 0.40;
            if (d < inner_radius) {
                float t_inner = d / inner_radius;
                blend_4stop_radial(
                    rgb,
                    t_inner,
                    vec3(1.0, 1.0, 1.0),
                    core_white_alpha * pulse,
                    b_core,
                    core_tint_alpha * pulse,
                    b_rim,
                    rim_alpha
                );
            }
        }
    }

    vec3 veil_col;
    float veil_a;
    if (dark_theme) {
        if (uv_y < 0.5) {
            float u = uv_y * 2.0;
            veil_col = vec3(
                mix(7.0, 9.0, u) / 255.0,
                mix(19.0, 25.0, u) / 255.0,
                mix(38.0, 51.0, u) / 255.0
            );
            veil_a = mix(0.28, 0.22, u);
        } else {
            float u = (uv_y - 0.5) * 2.0;
            veil_col = vec3(
                mix(9.0, 6.0, u) / 255.0,
                mix(25.0, 16.0, u) / 255.0,
                mix(51.0, 34.0, u) / 255.0
            );
            veil_a = mix(0.22, 0.30, u);
        }
    } else if (uv_y < 0.5) {
        float u = uv_y * 2.0;
        veil_col = vec3(
            mix(214.0, 196.0, u) / 255.0,
            mix(236.0, 228.0, u) / 255.0,
            1.0
        );
        veil_a = mix(0.18, 0.14, u);
    } else {
        float u = (uv_y - 0.5) * 2.0;
        veil_col = vec3(
            mix(196.0, 214.0, u) / 255.0,
            mix(228.0, 236.0, u) / 255.0,
            1.0
        );
        veil_a = mix(0.14, 0.20, u);
    }

    rgb = clamp(rgb * (1.0 - veil_a) + veil_col * veil_a, vec3(0.0), vec3(1.0));
    return vec4(rgb, 1.0);
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

    // 2. Scissor clip check (expanding for fire/smoke/glowbar border halos when clip_rect == rect)
    vec2 c_min = inst.clip_rect.xy;
    vec2 c_max = inst.clip_rect.xy + inst.clip_rect.zw;
    bool clip_is_self =
        abs(inst.clip_rect.x - inst.rect.x) < 1.0 &&
        abs(inst.clip_rect.y - inst.rect.y) < 1.0 &&
        abs(inst.clip_rect.z - inst.rect.z) < 1.0 &&
        abs(inst.clip_rect.w - inst.rect.w) < 1.0;
    if (clip_is_self && (border_fx != 0u || mat == 2u || mat == 5u || mat == 7u || mat == 10u)) {
        c_min -= vec2(32.0);
        c_max += vec2(32.0);
    }
    if (v_px.x < c_min.x || v_px.y < c_min.y || v_px.x > c_max.x || v_px.y > c_max.y) {
        discard;
    }

    float px = v_px.x;
    float py = v_px.y;
    float cx = inst.rect.x + inst.rect.z * 0.5;
    float cy = inst.rect.y + inst.rect.w * 0.5;
    float border_w = inst.params.x;
    float shadow_blur = inst.params.y;
    float dist = eval_sdf_superellipse(v_px, inst.rect, inst.corner_radii, inst.params.z);
    vec2 uv = clamp((v_px - inst.rect.xy) / max(inst.rect.zw, vec2(1.0)), 0.0, 1.0);
    float uv_x = uv.x;
    float uv_y = uv.y;
    float t_sec = pc.time_sec;

    bool has_smoke_border = (border_fx == 2u);
    bool has_fire_border = (border_fx == 1u)
        || (mat == 2u)
        || (!has_smoke_border && border_w > 0.0 && (mat == 5u || (inst.border_color.r > 0.9 && inst.border_color.g < 0.35)));

    // 3. Outside the SDF boundary: analytic drop shadow + optional external fire/smoke/glowbar corona
    if (dist > 0.5) {
        vec4 sh_col = vec4(inst.shadow_color.rgb, 1.0);
        float sh_alpha = 0.0;
        if (shadow_blur > 0.5 && dist < shadow_blur * 2.0) {
            float t = 1.0 - (dist / (shadow_blur * 2.0));
            sh_alpha = clamp(t * t * inst.shadow_color.a, 0.0, 1.0);
        }

        if ((has_smoke_border || mat == 7u) && dist < 30.0) {
            float top_bias = clamp(1.0 - uv_y, 0.0, 1.0);
            float reach = 22.0 + 8.0 * top_bias;
            if (dist < reach) {
                float drift = sin(t_sec * 0.22 + uv_y * 1.8) * 0.35;
                float nx = (px - inst.rect.x) * 0.024 + drift;
                float ny = (py - inst.rect.y) * 0.028 + t_sec * 0.34;
                float falloff = clamp(1.0 - dist / reach, 0.0, 1.0);
                float threshold = 0.16 + 0.10 * (1.0 - falloff);
                float wisp = clamp((sample_warped_fire(nx, ny) - threshold) * 1.35, 0.0, 1.0);
                float mist_a = clamp(wisp * falloff * (0.48 + 0.42 * falloff) * 0.76, 0.0, 1.0);
                if (mist_a > 0.01) {
                    vec4 mist_c = vec4(0.84, 0.91, 0.99, 1.0);
                    float combined = sh_alpha * (1.0 - mist_a) + mist_a;
                    sh_col = mix(sh_col, mist_c, clamp(mist_a / max(combined, 1e-4), 0.0, 1.0));
                    sh_alpha = clamp(combined, 0.0, 1.0);
                }
            }
        } else if (has_fire_border && dist < 18.0) {
            float top_boost = clamp(1.0 - uv_y, 0.0, 1.0);
            float upward_reach = (mat == 2u) ? (8.5 + 6.5 * top_boost) : (7.0 + 5.5 * top_boost);
            if (dist < upward_reach) {
                float wind = clamp((px - pc.pointer.x) * 0.0045, -0.35, 0.35);
                float nx = (px - inst.rect.x) * 0.045 + wind * (1.0 - uv_y);
                float ny = (py - inst.rect.y) * 0.055 + t_sec * 1.65;
                float flame_noise = sample_warped_fire(nx, ny);
                float corona_falloff = clamp(1.0 - (dist / upward_reach), 0.0, 1.0);
                float temp = clamp(corona_falloff * (0.42 + flame_noise * 0.85), 0.0, 1.0);
                if (temp > 0.08) {
                    vec4 flame_c = blackbody_fire_color(temp * 0.92);
                    float corona_alpha = clamp(temp * corona_falloff * 0.85, 0.0, 1.0);
                    float combined_alpha = sh_alpha * (1.0 - corona_alpha) + corona_alpha;
                    if (combined_alpha > 1e-4) {
                        sh_col = mix(sh_col, flame_c, clamp(corona_alpha / combined_alpha, 0.0, 1.0));
                        sh_alpha = clamp(combined_alpha, 0.0, 1.0);
                    }
                }
            }
        } else if (mat == 10u && dist < 12.0) {
            float hue = fract(t_sec * 0.065 + uv_x * 0.38);
            vec4 glow_c = color_cycling_spectrum(hue);
            float falloff = clamp(1.0 - dist / 12.0, 0.0, 1.0);
            float bloom_a = clamp(falloff * falloff * 0.62, 0.0, 1.0);
            float combined = sh_alpha * (1.0 - bloom_a) + bloom_a;
            if (combined > 1e-4) {
                sh_col = mix(sh_col, glow_c, clamp(bloom_a / combined, 0.0, 1.0));
                sh_alpha = clamp(combined, 0.0, 1.0);
            }
        }

        if (sh_alpha <= 0.002) {
            discard;
        }
        out_color = vec4(clamp(sh_col.rgb, 0.0, 1.0), sh_alpha);
        return;
    }

    // 4. Inside shape or on anti-aliased edge
    vec4 start_c = inst.fill_color_start;
    vec4 end_c = inst.fill_color_end;
    vec4 fill = mix(start_c, end_c, uv_y);

    if (mat == 1u) {
        // FrostedGlass
        float grain = (sample_turbulence(px * 0.12 + t_sec * 0.35, py * 0.12) - 0.5) * 0.035;
        float o = 1.0 - uv_y;
        float sheen = o * o * o * 0.06;
        fill = vec4(
            clamp(fill.r + grain + sheen, 0.0, 1.0),
            clamp(fill.g + grain + sheen, 0.0, 1.0),
            clamp(fill.b + grain + sheen * 1.2, 0.0, 1.0),
            clamp(fill.a, 0.28, 0.78)
        );
    } else if (mat == 2u) {
        // ProceduralFire
        float wind = clamp((px - pc.pointer.x) * 0.004, -0.3, 0.3);
        float nx = (px - inst.rect.x) * 0.038 + wind * (1.0 - uv_y);
        float ny = (py - inst.rect.y) * 0.048 + t_sec * 1.55;
        float fbm = sample_warped_fire(nx, ny);

        float rim_t = clamp(1.0 - clamp(-dist * (1.0 / 16.0), 0.0, 1.0), 0.0, 1.0);
        float rim_factor = rim_t * rim_t;
        float hearth_factor = 0.46 + 0.54 * uv_y;
        float temp = clamp(hearth_factor * (0.38 + fbm * 0.92) + rim_factor * 0.38, 0.0, 1.0);
        vec4 fire_col = blackbody_fire_color(temp);

        int spark_cell_x = int(floor((px + wind * 18.0) * 0.18));
        float spark_y_coord = py * 0.18 + t_sec * 4.5;
        int spark_cell_y = int(floor(spark_y_coord));
        float spark_seed = hash21(spark_cell_x, spark_cell_y);
        float spark_intensity = 0.0;
        if (spark_seed > 0.965) {
            float fy = spark_y_coord - float(spark_cell_y);
            spark_intensity = clamp((1.0 - abs(fy - 0.5) * 2.0) * (1.0 - uv_y * 0.3), 0.0, 1.0);
        }

        vec4 charcoal_base = vec4(0.11, 0.04, 0.05, clamp(fill.a, 0.58, 0.86));
        float ember_blend = clamp(temp * 0.90, 0.18, 0.94);
        vec4 fire_fill = mix(
            mix(charcoal_base, fire_col, ember_blend),
            vec4(1.0, 0.95, 0.78, 1.0),
            spark_intensity * 0.85
        );
        fire_fill.a = clamp(fill.a, 0.62, 0.85);
        fill = fire_fill;
    } else if (mat == 3u) {
        // WaterRipple
        float dx = px - pc.pointer.x;
        float dy = py - pc.pointer.y;
        float d_sq = dx * dx + dy * dy;
        if (d_sq < 90000.0) {
            float d_ptr = sqrt(d_sq);
            float atten = 1.0 / (1.0 + d_ptr * 0.025);
            float ripple = sin(d_ptr * 0.25 - t_sec * 4.2) * atten;
            float highlight = clamp(ripple * 0.25, -0.15, 0.25);
            fill = vec4(
                clamp(fill.r + highlight * 0.5, 0.0, 1.0),
                clamp(fill.g + highlight * 0.8, 0.0, 1.0),
                clamp(fill.b + highlight, 0.0, 1.0),
                fill.a
            );
        }
    } else if (mat == 4u) {
        // FluentRevealHighlight
        float dx = px - pc.pointer.x;
        float dy = py - pc.pointer.y;
        float d_sq = dx * dx + dy * dy;
        if (d_sq < 14400.0) {
            float radial = 1.0 - (d_sq * (1.0 / 14400.0));
            fill = mix(fill, vec4(1.0), radial * radial * 0.28);
        }
    } else if (mat == 5u) {
        // ObsidianNeonPulse
        if (border_w > 0.0 && dist > -border_w - 6.0 && !has_smoke_border) {
            float wind = clamp((px - pc.pointer.x) * 0.004, -0.3, 0.3);
            float nx = (px - inst.rect.x) * 0.045 + wind * (1.0 - uv_y);
            float ny = (py - inst.rect.y) * 0.06 + t_sec * 1.65;
            float fbm = sample_warped_fire(nx, ny);
            float rim_t = clamp(1.0 - clamp((-dist) / (border_w + 6.0), 0.0, 1.0), 0.0, 1.0);
            float temp = clamp(rim_t * (0.48 + fbm * 0.85), 0.0, 1.0);
            vec4 fire_col = blackbody_fire_color(temp);
            float prev_a = fill.a;
            fill = mix(fill, fire_col, clamp(rim_t * 0.92, 0.0, 1.0));
            fill.a = max(prev_a, rim_t * 0.95);
        }
    } else if (mat == 6u) {
        // TexturedImage + WaterRipple
        float d = distance(v_px, pc.pointer);
        float wave = sin(d * 0.11 - t_sec * 5.5) * exp(-d * 0.005);
        vec2 disp_uv = clamp(uv + vec2(wave * 0.018), 0.0, 1.0);
        vec3 tex_rgb = texture(u_showcase_tex, disp_uv).rgb;
        float spec = max(wave, 0.0) * 0.28;
        fill = vec4(clamp(tex_rgb + vec3(spec * 0.65, spec * 0.85, spec), 0.0, 1.0), fill.a);
    } else if (mat == 7u) {
        // GentleSmoke
        float sway = sin(t_sec * 0.22 + uv_y * 2.4) * 0.28;
        float nx = (px - inst.rect.x) * 0.020 + sway * (1.0 - uv_y * 0.6);
        float ny = (py - inst.rect.y) * 0.024 + t_sec * 0.32;
        float fbm = sample_warped_fire(nx, ny);
        float wisp = clamp((fbm - 0.18) * 1.25, 0.0, 1.0);
        float density = clamp(wisp * wisp * (0.48 + 0.52 * (1.0 - uv_y * 0.40)), 0.0, 0.88);
        float avg_luma = (start_c.r + start_c.g + start_c.b + end_c.r + end_c.g + end_c.b) * 0.5;
        vec4 plume_color = (avg_luma > 1.5)
            ? vec4(0.68, 0.76, 0.88, 1.0)
            : vec4(0.84, 0.90, 0.98, 1.0);
        float prev_a = fill.a;
        fill = mix(fill, plume_color, density * 0.68);
        fill.a = clamp(prev_a * 0.88 + density * 0.12, 0.38, 0.84);
    } else if (mat == 8u) {
        // GentlePulsingLights
        vec2 o_pos[3] = vec2[3](
            vec2(0.24 + 0.06 * sin(t_sec * 0.20), 0.38 + 0.08 * cos(t_sec * 0.15)),
            vec2(0.54 + 0.07 * cos(t_sec * 0.18 + 1.4), 0.62 + 0.07 * sin(t_sec * 0.22)),
            vec2(0.78 + 0.05 * sin(t_sec * 0.16 + 2.8), 0.34 + 0.08 * cos(t_sec * 0.19))
        );
        float o_pulse[3] = float[3](
            0.55 + 0.45 * sin(t_sec * 0.48),
            0.55 + 0.45 * sin(t_sec * 0.42 + 2.1),
            0.55 + 0.45 * sin(t_sec * 0.52 + 4.2)
        );
        vec3 o_col[3] = vec3[3](
            vec3(1.00, 0.84, 0.42),
            vec3(0.38, 0.92, 1.00),
            vec3(0.92, 0.58, 1.00)
        );
        vec3 add_rgb = vec3(0.0);
        for (int i = 0; i < 3; ++i) {
            float dx = uv_x - o_pos[i].x;
            float dy = (uv_y - o_pos[i].y) * 0.65;
            float d_sq = dx * dx + dy * dy;
            if (d_sq < 0.22) {
                float t = 1.0 - d_sq * (1.0 / 0.22);
                float w = t * t * o_pulse[i] * 0.58;
                add_rgb += o_col[i] * w;
            }
        }
        fill = vec4(clamp(fill.rgb + add_rgb, 0.0, 1.0), clamp(fill.a, 0.42, 0.82));
    } else if (mat == 9u) {
        // ReflectiveGlass
        float ptr_parallax = clamp((pc.pointer.x - cx) * 0.0006 + (pc.pointer.y - cy) * 0.0004, -0.22, 0.22);
        float sweep = uv_x * 0.72 + uv_y * 0.44 - ptr_parallax + sin(t_sec * 0.14) * 0.09;
        float band_primary = clamp(1.0 - abs(sweep - 0.48) * 6.5, 0.0, 1.0);
        float band_secondary = clamp(1.0 - abs(sweep - 0.74) * 11.0, 0.0, 1.0);
        float spec_band = band_primary * band_primary * 0.38 + band_secondary * band_secondary * 0.22;

        float edge_t = clamp(1.0 - clamp(-dist * (1.0 / 14.0), 0.0, 1.0), 0.0, 1.0);
        float fresnel_rim = edge_t * edge_t * edge_t * 0.42;
        float o = 1.0 - uv_y;
        float top_glint = o * o * o * 0.14;
        float refl = clamp(spec_band + fresnel_rim + top_glint, 0.0, 0.78);

        vec4 sky_spec = vec4(0.92, 0.97, 1.0, 1.0);
        vec4 glass = mix(fill, sky_spec, refl);
        glass.a = clamp(fill.a * 0.78 + refl * 0.20, 0.38, 0.80);
        fill = glass;
    } else if (mat == 10u) {
        // ColorCyclingGlowbar
        float hue = fract(t_sec * 0.065 + uv_x * 0.38);
        vec4 spec = color_cycling_spectrum(hue);
        float core_y = clamp(1.0 - abs(uv_y - 0.5) * 1.75, 0.0, 1.0);
        float core_boost = core_y * core_y * 0.48;
        vec4 glow = mix(mix(fill, spec, 0.82), vec4(1.0), core_boost);
        glow.a = clamp(fill.a, 0.55, 0.86);
        fill = glow;
    } else if (mat == 11u) {
        // FlowingGentleGradient
        vec4 grad = flowing_yellow_white_gradient(uv_x, uv_y, t_sec, start_c);
        vec4 out_c = mix(fill, grad, 0.86);
        out_c.a = clamp(fill.a, 0.50, 0.82);
        fill = out_c;
    } else if (mat == 12u) {
        // ElectricJellyfish
        float avg_luma = (start_c.r + start_c.g + start_c.b + end_c.r + end_c.g + end_c.b) * 0.5;
        bool dark_theme = avg_luma < 1.5;
        vec4 jelly = evaluate_electric_jellyfish_backdrop(
            uv_x,
            uv_y,
            max(inst.rect.z, 1.0),
            max(inst.rect.w, 1.0),
            t_sec,
            dark_theme
        );
        float grain = (sample_turbulence(px * 0.14 + t_sec * 0.15, py * 0.14) - 0.5) * 0.025;
        jelly.rgb = clamp(jelly.rgb + vec3(grain), 0.0, 1.0);
        jelly.a = clamp(fill.a, 0.58, 0.90);
        fill = jelly;
    } else if (mat == 13u) {
        // SubtleGlassReflection
        float ptr_parallax = clamp((pc.pointer.x - cx) * 0.00045 + (pc.pointer.y - cy) * 0.00035, -0.16, 0.16);
        float sweep = uv_x * 0.68 + uv_y * 0.42 - ptr_parallax + sin(t_sec * 0.10) * 0.06;
        float b1 = clamp(1.0 - abs(sweep - 0.46) * 5.2, 0.0, 1.0);
        float b2 = clamp(1.0 - abs(sweep - 0.72) * 9.0, 0.0, 1.0);
        float spec_band = b1 * b1 * 0.075 + b2 * b2 * 0.040;

        float edge_t = clamp(1.0 - clamp(-dist * (1.0 / 12.0), 0.0, 1.0), 0.0, 1.0);
        float fresnel_rim = edge_t * edge_t * edge_t * 0.075;
        float o = 1.0 - uv_y;
        float top_glint = o * o * o * o * 0.055;
        float refl = clamp(spec_band + fresnel_rim + top_glint, 0.0, 0.14);

        vec4 sky_spec = vec4(0.90, 0.96, 1.0, 1.0);
        vec4 glass = mix(fill, sky_spec, refl);
        glass.a = clamp(fill.a + refl * 0.08, 0.30, 0.78);
        fill = glass;
    }

    // 5. Interior rim & border stroke for GentleSmokeBorder or FireBorder
    if (border_w > 0.0 && dist > -border_w - 5.5) {
        if (has_smoke_border) {
            float sway = sin(t_sec * 0.24 + uv_y * 2.0) * 0.30;
            float nx = (px - inst.rect.x) * 0.032 + sway;
            float ny = (py - inst.rect.y) * 0.036 + t_sec * 0.38;
            float fbm = sample_warped_fire(nx, ny);
            float rim_t = clamp(1.0 - clamp((-dist) / (border_w + 5.5), 0.0, 1.0), 0.0, 1.0);
            float wisp_noise = clamp((fbm - 0.14) * 1.32, 0.0, 1.0);
            float wisp = clamp(rim_t * wisp_noise, 0.0, 1.0);
            vec4 smoke_c = vec4(0.86, 0.93, 1.0, 0.95);
            float prev_a = fill.a;
            fill = mix(fill, smoke_c, clamp(wisp * 0.85, 0.0, 1.0));
            fill.a = max(prev_a, rim_t * 0.92);
        } else if (has_fire_border && mat != 2u) {
            float wind = clamp((px - pc.pointer.x) * 0.004, -0.3, 0.3);
            float nx = (px - inst.rect.x) * 0.045 + wind * (1.0 - uv_y);
            float ny = (py - inst.rect.y) * 0.06 + t_sec * 1.65;
            float fbm = sample_warped_fire(nx, ny);
            float rim_t = clamp(1.0 - clamp((-dist) / (border_w + 5.5), 0.0, 1.0), 0.0, 1.0);
            float temp = clamp(rim_t * (0.48 + fbm * 0.85), 0.0, 1.0);
            vec4 fire_col = blackbody_fire_color(temp);
            float prev_a = fill.a;
            fill = mix(fill, fire_col, clamp(rim_t * 0.92, 0.0, 1.0));
            fill.a = max(prev_a, rim_t * 0.95);
        }
    }

    // 6. Border stroke near `dist in [-border_w, 0.0]`
    if (border_w > 0.0 && dist >= -border_w) {
        vec4 bc = inst.border_color;
        if (has_smoke_border) {
            float rim_noise = sample_warped_fire(px * 0.035, py * 0.035 + t_sec * 0.42);
            float wisp_t = clamp(0.45 + rim_noise * 0.55, 0.0, 1.0);
            bc = mix(vec4(0.76, 0.86, 0.98, 0.92), vec4(0.96, 0.99, 1.0, 0.98), wisp_t);
        } else if (has_fire_border) {
            float rim_noise = sample_warped_fire(px * 0.05, py * 0.05 + t_sec * 1.8);
            bc = blackbody_fire_color(clamp(0.55 + rim_noise * 0.45, 0.0, 1.0));
        }
        float next_a = max(fill.a, bc.a);
        fill = mix(fill, bc, bc.a);
        fill.a = next_a;
    }

    float edge_aa = clamp(0.5 - dist, 0.0, 1.0);
    float total_alpha = clamp(edge_aa * fill.a, 0.0, 1.0);
    if (total_alpha <= 0.002) {
        discard;
    }
    out_color = vec4(clamp(fill.rgb, 0.0, 1.0), total_alpha);
}
