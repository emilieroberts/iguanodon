/*
 * Copyright 2026 Emilie Roberts
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#ifndef IGUANODON_H
#define IGUANODON_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct IguanodonTestHarness IguanodonEngine;

/* Theme Family IDs */
#define IGUANODON_THEME_MATERIAL3_EXPRESSIVE 0
#define IGUANODON_THEME_WINDOWS_FLUENT2      1
#define IGUANODON_THEME_IOS_LIQUID_GLASS     2
#define IGUANODON_THEME_LINUX_ELITE_OBSIDIAN 3

/* Theme Polarity Modes */
#define IGUANODON_MODE_DARK  0
#define IGUANODON_MODE_LIGHT 1

/* Legacy & Benchmark Binary Command Opcodes (0x01..0x0B) */
#define IGUANODON_OP_SET_THEME            0x01
#define IGUANODON_OP_SET_LABEL_TEXT       0x02
#define IGUANODON_OP_CLICK_NODE           0x03
#define IGUANODON_OP_SET_TEXT_FIELD_VALUE 0x04
#define IGUANODON_OP_SCROLL_NODE          0x05
#define IGUANODON_OP_SET_PROGRESS         0x06
#define IGUANODON_OP_DRAG_AND_DROP        0x07
#define IGUANODON_OP_RIGHT_CLICK_NODE     0x08
#define IGUANODON_OP_INVOKE_CONTEXT_MENU  0x09
#define IGUANODON_OP_TOGGLE_FIRE_SHADER   0x0A
#define IGUANODON_OP_SET_THEME_MODE       0x0B

/* Retained Applier Scene-Tree Opcodes (0x10..0x1A) */
#define IGUANODON_OP_INSERT_CHILD         0x10
#define IGUANODON_OP_MOVE_CHILDREN        0x11
#define IGUANODON_OP_SET_LAYOUT_STYLE     0x12
#define IGUANODON_OP_SET_VISUAL_STYLE     0x13
#define IGUANODON_OP_SET_SHADER_EFFECT    0x14
#define IGUANODON_OP_SET_TEXT             0x15
#define IGUANODON_OP_SET_IMAGE_BYTES      0x16
#define IGUANODON_OP_SET_SCROLL_STATE     0x17
#define IGUANODON_OP_SET_EVENT_MASK       0x18
#define IGUANODON_OP_REMOVE_CHILDREN      0x19
#define IGUANODON_OP_CLEAR_TREE           0x1A

/* Reverse UI Event Types (32-byte records drained by iguanodon_engine_poll_events) */
#define IGUANODON_EVENT_CLICK                   0x01
#define IGUANODON_EVENT_LONG_OR_SECONDARY_CLICK 0x02
#define IGUANODON_EVENT_HOVER_ENTER             0x03
#define IGUANODON_EVENT_HOVER_EXIT              0x04
#define IGUANODON_EVENT_DRAG_DELTA              0x05
#define IGUANODON_EVENT_SCROLL_DELTA            0x06
#define IGUANODON_EVENT_KEY_CHAR                0x07
#define IGUANODON_EVENT_KEY_BACKSPACE           0x08
#define IGUANODON_EVENT_WINDOW_RESIZE           0x09
#define IGUANODON_EVENT_SAFE_INSETS             0x0A
#define IGUANODON_EVENT_THEME_MODE_CHANGED      0x0B

IguanodonEngine* iguanodon_engine_create(float width, float height);
void iguanodon_engine_destroy(IguanodonEngine* engine_ptr);
void iguanodon_engine_resize(IguanodonEngine* engine_ptr, float width, float height);
void iguanodon_set_safe_insets(IguanodonEngine* engine_ptr, float left, float top, float right, float bottom);
int32_t iguanodon_flush_commands(IguanodonEngine* engine_ptr, const uint8_t* buffer_ptr, size_t byte_len);

/* Drains pending 32-byte ReverseUiEvent records into out_events_ptr; returns event count. */
int32_t iguanodon_engine_poll_events(IguanodonEngine* engine_ptr, uint8_t* out_events_ptr, size_t max_bytes);

void iguanodon_pointer_move(IguanodonEngine* engine_ptr, float x, float y);
void iguanodon_pointer_down(IguanodonEngine* engine_ptr, float x, float y, bool is_right_or_long_press);
void iguanodon_pointer_up(IguanodonEngine* engine_ptr, float x, float y);
void iguanodon_scroll_wheel(IguanodonEngine* engine_ptr, float x, float y, float delta_y);
void iguanodon_key_char(IguanodonEngine* engine_ptr, uint32_t unicode_codepoint);
void iguanodon_key_backspace(IguanodonEngine* engine_ptr);

float iguanodon_render_frame(IguanodonEngine* engine_ptr, float dt_sec);
bool  iguanodon_attach_android_surface(
    IguanodonEngine* engine_ptr,
    void* anative_window,
    uint32_t width,
    uint32_t height
);
bool  iguanodon_attach_xlib_surface(
    IguanodonEngine* engine_ptr,
    void* dpy,
    uint64_t window,
    uint32_t width,
    uint32_t height
);
float iguanodon_render_and_present(
    IguanodonEngine* engine_ptr,
    float dt_sec,
    uint32_t width,
    uint32_t height
);
float iguanodon_render_frame_rgba(
    IguanodonEngine* engine_ptr,
    float dt_sec,
    uint32_t fb_width,
    uint32_t fb_height,
    uint8_t* out_rgba_ptr
);

#ifdef __cplusplus
}
#endif

#endif /* IGUANODON_H */
