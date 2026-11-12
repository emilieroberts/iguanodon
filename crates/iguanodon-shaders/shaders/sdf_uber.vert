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

layout(push_constant) uniform PushConstants {
    vec2 viewport;
    vec2 pointer;
    float time_sec;
    uint flags;
    vec2 _pad;
} pc;

layout(location = 0) out vec2 v_px;
layout(location = 1) flat out uint v_inst_idx;

void main() {
    uint idx = uint(gl_InstanceIndex);
    SdfInstance inst = u_instances.instances[idx];

    uint raw_mat = uint(inst.params.w + 0.5);
    uint mat = raw_mat % 100u;
    uint border_fx = raw_mat / 100u;

    vec2 q_min;
    vec2 q_max;

    if (mat == 14u) {
        // GPU Glyph Atlas Quad: tight glyph box clipped to clip_rect
        vec2 c_min = inst.clip_rect.xy;
        vec2 c_max = inst.clip_rect.xy + inst.clip_rect.zw;
        q_min = max(inst.rect.xy, c_min);
        q_max = min(inst.rect.xy + inst.rect.zw, c_max);
    } else {
        float halo = inst.params.y * 2.5 + inst.params.x + 3.0;
        if (border_fx != 0u || mat == 2u || mat == 7u) {
            halo = max(halo, 32.0);
        }
        q_min = inst.rect.xy - vec2(halo);
        q_max = inst.rect.xy + inst.rect.zw + vec2(halo);

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
        q_min = max(q_min, c_min);
        q_max = min(q_max, c_max);
    }

    vec2 uv;
    uint v = uint(gl_VertexIndex);
    if (v == 0u)      uv = vec2(0.0, 0.0);
    else if (v == 1u) uv = vec2(1.0, 0.0);
    else if (v == 2u) uv = vec2(1.0, 1.0);
    else if (v == 3u) uv = vec2(0.0, 0.0);
    else if (v == 4u) uv = vec2(1.0, 1.0);
    else              uv = vec2(0.0, 1.0);

    vec2 px = mix(q_min, q_max, uv);
    vec2 ndc = (px / max(pc.viewport, vec2(1.0))) * 2.0 - 1.0;
    gl_Position = vec4(ndc, 0.0, 1.0);
    v_px = px;
    v_inst_idx = idx;
}
