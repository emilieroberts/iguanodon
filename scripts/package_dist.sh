#!/usr/bin/env bash
set -euo pipefail

# ==============================================================================
#  🦖 Iguanodon Distributable SDK & Library Packager (`package_dist.sh`)
# ==============================================================================
# Produces ready-to-distribute single-file artifacts in `dist/`:
#   1. dist/iguanodon-compose-android-0.1.0.aar            (Drop-in Pure-Runtime Android AAR with arm64-v8a JNI .so)
#   2. dist/iguanodon-compose-desktop-linux-x64-0.1.0.jar  (Drop-in Pure-Runtime Linux Desktop Fat JAR with embedded x86_64 .so)
#   3. dist/include/iguanodon.h                            (C/C++/Qt/GTK FFI Header)
#   4. dist/native/linux-x86_64/libiguanodon_ffi.so        (Linux x86_64 shared library)
#   5. dist/native/linux-x86_64/libiguanodon_ffi.a         (Linux x86_64 static library)
#   6. dist/native/android-arm64-v8a/libiguanodon_ffi.so   (Android arm64-v8a shared library)
#   7. dist/pkgconfig/iguanodon.pc                         (Meson / Autotools / pkg-config descriptor)
#   8. dist/cmake/IguanodonConfig.cmake                    (CMake find_package(Iguanodon) descriptor)
# ==============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
DIST_DIR="${ROOT_DIR}/dist"

echo "=================================================================="
echo " 🦖 Packaging Iguanodon Distributable Libraries -> ${DIST_DIR}"
echo "=================================================================="

mkdir -p "${DIST_DIR}/include" \
         "${DIST_DIR}/native/linux-x86_64" \
         "${DIST_DIR}/native/android-arm64-v8a" \
         "${DIST_DIR}/pkgconfig" \
         "${DIST_DIR}/cmake"

# 1. Build Linux x86_64 release libraries (.so and .a)
echo "[*] Building Linux x86_64 release libraries (libiguanodon_ffi.so & libiguanodon_ffi.a)..."
cd "${ROOT_DIR}"
cargo build --release -p iguanodon-ffi
cp "${ROOT_DIR}/target/release/libiguanodon_ffi.so" "${DIST_DIR}/native/linux-x86_64/libiguanodon_ffi.so"
cp "${ROOT_DIR}/target/release/libiguanodon_ffi.a" "${DIST_DIR}/native/linux-x86_64/libiguanodon_ffi.a"

# 2. Copy Android arm64-v8a release library if built
ANDROID_SO="${ROOT_DIR}/target/aarch64-linux-android/release/libiguanodon_ffi.so"
if [[ -f "${ANDROID_SO}" ]]; then
    cp "${ANDROID_SO}" "${DIST_DIR}/native/android-arm64-v8a/libiguanodon_ffi.so"
    cp "${ANDROID_SO}" "${ROOT_DIR}/kotlin/android-benchmark-app/src/main/jniLibs/arm64-v8a/libiguanodon_ffi.so" 2>/dev/null || true
    echo "[+] Included Android arm64-v8a libiguanodon_ffi.so"
fi

# 3. Generate C/C++/Qt/GTK Header (`dist/include/iguanodon.h`)
cat > "${DIST_DIR}/include/iguanodon.h" << 'EOF'
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
EOF

# 4. Generate pkg-config (`dist/pkgconfig/iguanodon.pc`) & CMake (`dist/cmake/IguanodonConfig.cmake`)
cat > "${DIST_DIR}/pkgconfig/iguanodon.pc" << 'EOF'
prefix=${pcfiledir}/..
exec_prefix=${prefix}
libdir=${exec_prefix}/native/linux-x86_64
includedir=${prefix}/include

Name: iguanodon
Description: Iguanodon High-Performance Vulkan UI Toolkit (Rust / Pure Compose Runtime / Qt / GTK)
Version: 0.0.1-alpha
Libs: -L${libdir} -liguanodon_ffi -ldl -lpthread -lm
Cflags: -I${includedir}
EOF

cat > "${DIST_DIR}/cmake/IguanodonConfig.cmake" << 'EOF'
get_filename_component(IGUANODON_CMAKE_DIR "${CMAKE_CURRENT_LIST_FILE}" PATH)
get_filename_component(IGUANODON_DIST_ROOT "${IGUANODON_CMAKE_DIR}/.." ABSOLUTE)

add_library(Iguanodon::iguanodon_ffi SHARED IMPORTED)
if(ANDROID)
    set_target_properties(Iguanodon::iguanodon_ffi PROPERTIES
        IMPORTED_LOCATION "${IGUANODON_DIST_ROOT}/native/android-arm64-v8a/libiguanodon_ffi.so"
        INTERFACE_INCLUDE_DIRECTORIES "${IGUANODON_DIST_ROOT}/include"
    )
else()
    set_target_properties(Iguanodon::iguanodon_ffi PROPERTIES
        IMPORTED_LOCATION "${IGUANODON_DIST_ROOT}/native/linux-x86_64/libiguanodon_ffi.so"
        INTERFACE_INCLUDE_DIRECTORIES "${IGUANODON_DIST_ROOT}/include"
    )
endif()
EOF

# 5. Build Pure-Runtime `:iguanodon-compose` Desktop JAR & Android AAR
echo "[*] Building pure-runtime :iguanodon-compose via Gradle..."
cd "${ROOT_DIR}/kotlin"
./gradlew :iguanodon-compose:desktopJar :iguanodon-compose:assembleRelease --offline

WORK_JAR="${ROOT_DIR}/target/dist-jar-work"
rm -rf "${WORK_JAR}"
mkdir -p "${WORK_JAR}/classes/META-INF/native/linux-x86_64"

python3 -c "
import glob, zipfile
jars = sorted(glob.glob('${ROOT_DIR}/kotlin/iguanodon-compose/build/libs/iguanodon-compose-desktop-*.jar'))
assert jars, 'No iguanodon-compose-desktop jar found'
with zipfile.ZipFile(jars[-1], 'r') as z:
    z.extractall('${WORK_JAR}/classes')
"

cp "${DIST_DIR}/native/linux-x86_64/libiguanodon_ffi.so" \
   "${WORK_JAR}/classes/META-INF/native/linux-x86_64/libiguanodon_ffi.so"

jar --create \
    --file "${DIST_DIR}/iguanodon-compose-desktop-linux-x64-0.0.1-alpha.jar" \
    -C "${WORK_JAR}/classes" .
cp "${DIST_DIR}/iguanodon-compose-desktop-linux-x64-0.0.1-alpha.jar" \
   "${DIST_DIR}/iguanodon-compose-desktop-linux-x64-0.1.0.jar"

echo "[+] Built Desktop Linux Fat JAR: ${DIST_DIR}/iguanodon-compose-desktop-linux-x64-0.0.1-alpha.jar"

# 6. Build Drop-In Android Archive (`dist/iguanodon-compose-android-0.1.0.aar`)
WORK_AAR="${ROOT_DIR}/target/dist-aar-work"
rm -rf "${WORK_AAR}"
mkdir -p "${WORK_AAR}/aar_root/jni/arm64-v8a"

if [[ -f "${ROOT_DIR}/kotlin/iguanodon-compose/build/outputs/aar/iguanodon-compose-release.aar" ]]; then
    python3 -c "
import zipfile
with zipfile.ZipFile('${ROOT_DIR}/kotlin/iguanodon-compose/build/outputs/aar/iguanodon-compose-release.aar', 'r') as z:
    z.extractall('${WORK_AAR}/aar_root')
"
else
    mkdir -p "${WORK_AAR}/aar_root/res"
    jar --create \
        --file "${WORK_AAR}/aar_root/classes.jar" \
        -C "${WORK_JAR}/classes" iguanodon
    cat > "${WORK_AAR}/aar_root/AndroidManifest.xml" << 'EOF'
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="iguanodon.compose">
    <uses-sdk android:minSdkVersion="26" />
</manifest>
EOF
    touch "${WORK_AAR}/aar_root/R.txt"
fi

if [[ -f "${DIST_DIR}/native/android-arm64-v8a/libiguanodon_ffi.so" ]]; then
    cp "${DIST_DIR}/native/android-arm64-v8a/libiguanodon_ffi.so" \
       "${WORK_AAR}/aar_root/jni/arm64-v8a/libiguanodon_ffi.so"
fi

python3 -c "
import os, zipfile
root = '${WORK_AAR}/aar_root'
out_aar = '${DIST_DIR}/iguanodon-compose-android-0.0.1-alpha.aar'
with zipfile.ZipFile(out_aar, 'w', compression=zipfile.ZIP_DEFLATED) as z:
    for dirpath, _, filenames in os.walk(root):
        for f in filenames:
            full = os.path.join(dirpath, f)
            rel = os.path.relpath(full, root)
            z.write(full, rel)
"
cp "${DIST_DIR}/iguanodon-compose-android-0.0.1-alpha.aar" \
   "${DIST_DIR}/iguanodon-compose-android-0.1.0.aar"

echo "[+] Built Android Drop-In AAR:   ${DIST_DIR}/iguanodon-compose-android-0.0.1-alpha.aar"
echo "=================================================================="
ls -lh "${DIST_DIR}" "${DIST_DIR}/native/linux-x86_64" "${DIST_DIR}/include"
