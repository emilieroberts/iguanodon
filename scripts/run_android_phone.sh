#!/usr/bin/env bash
# Copyright 2026 Emilie Roberts
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
#
# SPDX-License-Identifier: Apache-2.0

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=================================================================="
echo " 🦖 Iguanodon Vulkan UI Toolkit — Android Phone ADB Deployer"
echo "=================================================================="

# 1. Discover Android SDK & ADB
for candidate in \
    "${ANDROID_HOME:-}" \
    "${ANDROID_SDK_ROOT:-}" \
    "${HOME}/Android/Sdk" \
    "/opt/android-sdk" \
    "/usr/lib/android-sdk"; do
    if [[ -n "${candidate}" && -d "${candidate}" ]]; then
        export ANDROID_HOME="${candidate}"
        export ANDROID_SDK_ROOT="${candidate}"
        export PATH="${PATH}:${candidate}/platform-tools:${candidate}/cmdline-tools/latest/bin"
        break
    fi
done

if ! command -v adb >/dev/null 2>&1; then
    echo "[!] 'adb' was not found in PATH or ~/Android/Sdk/platform-tools."
    exit 1
fi

echo "[*] Checking connected Android devices via adb..."
adb start-server >/dev/null 2>&1 || true
ADB_DEVICES="$(adb devices | awk 'NR>1 && $2=="device" {print $1}')"

if [[ -z "${ADB_DEVICES}" ]]; then
    echo "[!] No authorized Android device detected over adb."
    adb devices -l
    exit 2
fi

SERIAL="$(echo "${ADB_DEVICES}" | head -n1)"
MODEL="$(adb -s "${SERIAL}" shell getprop ro.product.model | tr -d '\r')"
ABI="$(adb -s "${SERIAL}" shell getprop ro.product.cpu.abi | tr -d '\r')"
ANDROID_VER="$(adb -s "${SERIAL}" shell getprop ro.build.version.release | tr -d '\r')"

echo "[+] Connected to Android Phone: ${MODEL} (Android ${ANDROID_VER}, ABI: ${ABI}, Serial: ${SERIAL})"

# 2. Locate Android NDK for Rust cross-compilation (aarch64-linux-android)
NDK_DIR=""
if [[ -n "${ANDROID_NDK_HOME:-}" && -d "${ANDROID_NDK_HOME}" ]]; then
    NDK_DIR="${ANDROID_NDK_HOME}"
elif [[ -n "${ANDROID_HOME:-}" && -d "${ANDROID_HOME}/ndk" ]]; then
    NDK_DIR="$(find "${ANDROID_HOME}/ndk" -maxdepth 1 -mindepth 1 -type d | sort -V | tail -n1)"
elif [[ -d "/usr/lib/android-ndk" ]]; then
    NDK_DIR="/usr/lib/android-ndk"
fi

if [[ -z "${NDK_DIR}" ]]; then
    echo "[!] Android NDK not found."
    exit 3
fi

TOOLCHAIN_BIN="${NDK_DIR}/toolchains/llvm/prebuilt/linux-x86_64/bin"
echo "[*] Using Android NDK toolchain: ${TOOLCHAIN_BIN}"

rustup target add aarch64-linux-android >/dev/null 2>&1 || true

export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="${TOOLCHAIN_BIN}/aarch64-linux-android26-clang"
export CC_aarch64_linux_android="${TOOLCHAIN_BIN}/aarch64-linux-android26-clang"
export AR_aarch64_linux_android="${TOOLCHAIN_BIN}/llvm-ar"
# Ensure 16KB ELF page-size alignment for Android 15/16/17 devices (e.g. Pixel 9/10/11 Pro)
export RUSTFLAGS="-C link-arg=-Wl,-z,max-page-size=16384"

echo "[*] Cross-compiling libiguanodon_ffi.so (Rust Vulkan + 1080p Water Drop + 21px/26px Roboto engine) for arm64-v8a..."
cd "${ROOT_DIR}"
cargo build --release --target aarch64-linux-android -p iguanodon-ffi

SO_PATH="${ROOT_DIR}/target/aarch64-linux-android/release/libiguanodon_ffi.so"
JNI_LIB_DIR="${ROOT_DIR}/kotlin/android-benchmark-app/src/main/jniLibs/arm64-v8a"
mkdir -p "${JNI_LIB_DIR}"
cp "${SO_PATH}" "${JNI_LIB_DIR}/libiguanodon_ffi.so"
echo "[+] Copied arm64-v8a/libiguanodon_ffi.so into Android module."

# 3. Package & Sign APK using Gradle if available, or directly via Android SDK build-tools (aapt2 + javac + d8 + apksigner)
if [[ -x "${ROOT_DIR}/kotlin/gradlew" ]]; then
    cd "${ROOT_DIR}/kotlin"
    ./gradlew :android-benchmark-app:installDebug
else
    BUILD_TOOLS_DIR="$(find "${ANDROID_HOME}/build-tools" -maxdepth 1 -mindepth 1 -type d | sort -V | tail -n1)"
    PLATFORM_DIR="$(find "${ANDROID_HOME}/platforms" -maxdepth 1 -mindepth 1 -type d | sort -V | tail -n1)"
    ANDROID_JAR="${PLATFORM_DIR}/android.jar"

    echo "[*] Packaging APK directly with Android SDK Build-Tools: $(basename "${BUILD_TOOLS_DIR}") + $(basename "${PLATFORM_DIR}")"
    WORK_APK_DIR="${ROOT_DIR}/target/android-apk-build"
    rm -rf "${WORK_APK_DIR}"
    mkdir -p "${WORK_APK_DIR}/src/iguanodon/compose/bridge" \
             "${WORK_APK_DIR}/src/iguanodon/compose/android" \
             "${WORK_APK_DIR}/src/iguanodon/android/benchmark" \
             "${WORK_APK_DIR}/classes" \
             "${WORK_APK_DIR}/apk_root/lib/arm64-v8a"

    cat > "${WORK_APK_DIR}/src/iguanodon/compose/bridge/IguanodonNativeBridge.java" << 'EOF'
package iguanodon.compose.bridge;

public final class IguanodonNativeBridge {
    static {
        System.loadLibrary("iguanodon_ffi");
    }

    public static native long nativeCreate(float width, float height);
    public static native void nativeDestroy(long handle);
    public static native void nativeResize(long handle, float width, float height);
    public static native void nativeSetSafeInsets(long handle, float left, float top, float right, float bottom);
    public static native int nativeFlushCommands(long handle, byte[] batch);
    public static native void nativePointerMove(long handle, float x, float y);
    public static native void nativePointerDown(long handle, float x, float y, byte isRightOrLongPress);
    public static native void nativePointerUp(long handle, float x, float y);
    public static native void nativeScroll(long handle, float x, float y, float deltaY);
    public static native void nativeKeyChar(long handle, int codepoint);
    public static native void nativeBackspace(long handle);
    public static native float nativeRenderFrameByteArray(
        long handle,
        float dtSec,
        int width,
        int height,
        byte[] outRgba
    );
}
EOF

    cat > "${WORK_APK_DIR}/src/iguanodon/compose/android/IguanodonVulkanSurfaceView.java" << 'EOF'
package iguanodon.compose.android;

import android.content.Context;
import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.graphics.Insets;
import android.graphics.Rect;
import android.os.Build;
import android.view.Choreographer;
import android.view.GestureDetector;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.SurfaceHolder;
import android.view.SurfaceView;
import android.view.WindowInsets;
import iguanodon.compose.bridge.IguanodonNativeBridge;
import java.nio.ByteBuffer;

public final class IguanodonVulkanSurfaceView extends SurfaceView
        implements SurfaceHolder.Callback, Choreographer.FrameCallback {

    private long nativeHandle;
    private int viewportWidth = 1280;
    private int viewportHeight = 760;
    private byte[] rgbaBytes = new byte[1280 * 760 * 4];
    private Bitmap frameBitmap = Bitmap.createBitmap(1280, 760, Bitmap.Config.ARGB_8888);
    private boolean running = false;
    private long lastFrameNanos = 0L;
    private boolean longPressTriggeredOnCurrentTouch = false;
    private int rawInsetLeft = 0;
    private int rawInsetTop = 0;
    private int rawInsetRight = 0;
    private int rawInsetBottom = 0;
    private final GestureDetector gestureDetector;

    public IguanodonVulkanSurfaceView(Context context) {
        super(context);
        this.nativeHandle = IguanodonNativeBridge.nativeCreate(viewportWidth, viewportHeight);
        getHolder().addCallback(this);
        setFocusable(true);
        setFocusableInTouchMode(true);

        setOnApplyWindowInsetsListener((v, insets) -> {
            if (Build.VERSION.SDK_INT >= 30) {
                int mask = WindowInsets.Type.systemBars() | WindowInsets.Type.displayCutout();
                Insets sys = insets.getInsets(mask);
                rawInsetLeft = sys.left;
                rawInsetTop = sys.top;
                rawInsetRight = sys.right;
                rawInsetBottom = sys.bottom;
            } else {
                rawInsetLeft = insets.getSystemWindowInsetLeft();
                rawInsetTop = insets.getSystemWindowInsetTop();
                rawInsetRight = insets.getSystemWindowInsetRight();
                rawInsetBottom = insets.getSystemWindowInsetBottom();
            }
            applyScaledSafeInsets();
            return v.onApplyWindowInsets(insets);
        });

        this.gestureDetector = new GestureDetector(context, new GestureDetector.SimpleOnGestureListener() {
            @Override
            public void onLongPress(MotionEvent e) {
                longPressTriggeredOnCurrentTouch = true;
                if (nativeHandle != 0L) {
                    IguanodonNativeBridge.nativePointerDown(
                        nativeHandle,
                        mapTouchX(e.getX()),
                        mapTouchY(e.getY()),
                        (byte) 1
                    );
                }
            }

            @Override
            public boolean onScroll(MotionEvent e1, MotionEvent e2, float distanceX, float distanceY) {
                if (nativeHandle != 0L) {
                    float scaleY = (float) viewportHeight / (float) Math.max(1, getHeight());
                    IguanodonNativeBridge.nativeScroll(
                        nativeHandle,
                        mapTouchX(e2.getX()),
                        mapTouchY(e2.getY()),
                        distanceY * scaleY * 0.45f
                    );
                }
                return true;
            }
        });
    }

    private float mapTouchX(float rawX) {
        return rawX * (float) viewportWidth / (float) Math.max(1, getWidth());
    }

    private float mapTouchY(float rawY) {
        return rawY * (float) viewportHeight / (float) Math.max(1, getHeight());
    }

    private void applyScaledSafeInsets() {
        int w = getWidth();
        int h = getHeight();
        if (w <= 0 || h <= 0 || nativeHandle == 0L) return;
        float scaleX = (float) viewportWidth / (float) w;
        float scaleY = (float) viewportHeight / (float) h;
        IguanodonNativeBridge.nativeSetSafeInsets(
            nativeHandle,
            rawInsetLeft * scaleX,
            rawInsetTop * scaleY,
            rawInsetRight * scaleX,
            rawInsetBottom * scaleY
        );
    }

    @Override
    public void surfaceCreated(SurfaceHolder holder) {
        running = true;
        lastFrameNanos = System.nanoTime();
        requestApplyInsets();
        Choreographer.getInstance().postFrameCallback(this);
    }

    @Override
    public void surfaceChanged(SurfaceHolder holder, int format, int width, int height) {
        if (width <= 0 || height <= 0 || nativeHandle == 0L) return;
        float aspect = (float) width / (float) height;
        int logicalW;
        int logicalH;
        if (width < height) {
            // Portrait orientation: single-column stack
            logicalW = 600;
            logicalH = Math.max(1140, Math.min(1520, (int) (logicalW / aspect)));
        } else {
            // Landscape orientation: 3-column dashboard
            logicalH = 760;
            logicalW = Math.max(960, Math.min(1680, (int) (logicalH * aspect)));
        }
        if (logicalW != viewportWidth || logicalH != viewportHeight) {
            viewportWidth = logicalW;
            viewportHeight = logicalH;
            IguanodonNativeBridge.nativeResize(nativeHandle, viewportWidth, viewportHeight);
            rgbaBytes = new byte[viewportWidth * viewportHeight * 4];
            frameBitmap = Bitmap.createBitmap(viewportWidth, viewportHeight, Bitmap.Config.ARGB_8888);
        }
        applyScaledSafeInsets();
    }

    @Override
    public void surfaceDestroyed(SurfaceHolder holder) {
        running = false;
        Choreographer.getInstance().removeFrameCallback(this);
    }

    @Override
    public void doFrame(long frameTimeNanos) {
        if (!running || nativeHandle == 0L) return;
        float dt = (frameTimeNanos - lastFrameNanos) / 1_000_000_000.0f;
        if (dt < (1.0f / 240.0f)) dt = 1.0f / 240.0f;
        if (dt > 0.05f) dt = 0.05f;
        lastFrameNanos = frameTimeNanos;

        IguanodonNativeBridge.nativeRenderFrameByteArray(
            nativeHandle,
            dt,
            viewportWidth,
            viewportHeight,
            rgbaBytes
        );

        Canvas canvas = getHolder().lockCanvas();
        if (canvas != null) {
            try {
                frameBitmap.copyPixelsFromBuffer(ByteBuffer.wrap(rgbaBytes));
                Rect dst = new Rect(0, 0, getWidth(), getHeight());
                canvas.drawBitmap(frameBitmap, null, dst, null);
            } finally {
                getHolder().unlockCanvasAndPost(canvas);
            }
        }
        Choreographer.getInstance().postFrameCallback(this);
    }

    @Override
    public boolean onTouchEvent(MotionEvent event) {
        gestureDetector.onTouchEvent(event);
        if (nativeHandle == 0L) return false;
        float x = mapTouchX(event.getX());
        float y = mapTouchY(event.getY());
        int action = event.getActionMasked();
        if (action == MotionEvent.ACTION_DOWN) {
            requestFocus();
            longPressTriggeredOnCurrentTouch = false;
            IguanodonNativeBridge.nativePointerMove(nativeHandle, x, y);
            IguanodonNativeBridge.nativePointerDown(nativeHandle, x, y, (byte) 0);
            return true;
        } else if (action == MotionEvent.ACTION_MOVE) {
            IguanodonNativeBridge.nativePointerMove(nativeHandle, x, y);
            return true;
        } else if (action == MotionEvent.ACTION_UP || action == MotionEvent.ACTION_CANCEL) {
            if (!longPressTriggeredOnCurrentTouch) {
                IguanodonNativeBridge.nativePointerUp(nativeHandle, x, y);
            }
            longPressTriggeredOnCurrentTouch = false;
            return true;
        }
        return super.onTouchEvent(event);
    }

    @Override
    public boolean onKeyDown(int keyCode, KeyEvent event) {
        if (nativeHandle == 0L) return false;
        if (keyCode == KeyEvent.KEYCODE_DEL) {
            IguanodonNativeBridge.nativeBackspace(nativeHandle);
            return true;
        }
        int unicode = event.getUnicodeChar();
        if (unicode >= 32 && unicode <= 126) {
            IguanodonNativeBridge.nativeKeyChar(nativeHandle, unicode);
            return true;
        }
        return super.onKeyDown(keyCode, event);
    }

    public void releaseEngine() {
        running = false;
        Choreographer.getInstance().removeFrameCallback(this);
        if (nativeHandle != 0L) {
            IguanodonNativeBridge.nativeDestroy(nativeHandle);
            nativeHandle = 0L;
        }
    }
}
EOF

    cat > "${WORK_APK_DIR}/src/iguanodon/android/benchmark/IguanodonBenchmarkActivity.java" << 'EOF'
package iguanodon.android.benchmark;

import android.app.Activity;
import android.graphics.Color;
import android.os.Build;
import android.os.Bundle;
import android.view.View;
import android.view.Window;
import android.view.WindowManager;
import iguanodon.compose.android.IguanodonVulkanSurfaceView;

public final class IguanodonBenchmarkActivity extends Activity {
    private IguanodonVulkanSurfaceView surfaceView;

    @SuppressWarnings("deprecation")
    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        requestWindowFeature(Window.FEATURE_NO_TITLE);

        Window window = getWindow();
        if (Build.VERSION.SDK_INT >= 30) {
            window.setDecorFitsSystemWindows(false);
        } else {
            window.getDecorView().setSystemUiVisibility(
                View.SYSTEM_UI_FLAG_LAYOUT_STABLE
                    | View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
                    | View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
            );
        }

        if (Build.VERSION.SDK_INT >= 28) {
            WindowManager.LayoutParams lp = window.getAttributes();
            lp.layoutInDisplayCutoutMode = Build.VERSION.SDK_INT >= 30
                ? WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_ALWAYS
                : WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES;
            window.setAttributes(lp);
        }

        window.setStatusBarColor(Color.TRANSPARENT);
        window.setNavigationBarColor(Color.TRANSPARENT);

        surfaceView = new IguanodonVulkanSurfaceView(this);
        setContentView(surfaceView);
    }

    @Override
    protected void onDestroy() {
        if (surfaceView != null) {
            surfaceView.releaseEngine();
            surfaceView = null;
        }
        super.onDestroy();
    }
}
EOF

    javac --release 11 \
        -cp "${ANDROID_JAR}" \
        -d "${WORK_APK_DIR}/classes" \
        "${WORK_APK_DIR}/src/iguanodon/compose/bridge/IguanodonNativeBridge.java" \
        "${WORK_APK_DIR}/src/iguanodon/compose/android/IguanodonVulkanSurfaceView.java" \
        "${WORK_APK_DIR}/src/iguanodon/android/benchmark/IguanodonBenchmarkActivity.java"

    CLASS_FILES=()
    while IFS= read -r -d '' f; do
        CLASS_FILES+=("$f")
    done < <(find "${WORK_APK_DIR}/classes" -name "*.class" -print0)

    "${BUILD_TOOLS_DIR}/d8" \
        --lib "${ANDROID_JAR}" \
        --min-api 26 \
        --output "${WORK_APK_DIR}/apk_root" \
        "${CLASS_FILES[@]}"

    "${BUILD_TOOLS_DIR}/aapt2" link \
        -o "${WORK_APK_DIR}/unaligned.apk" \
        --manifest "${ROOT_DIR}/kotlin/android-benchmark-app/src/main/AndroidManifest.xml" \
        -I "${ANDROID_JAR}"

    cp "${SO_PATH}" "${WORK_APK_DIR}/apk_root/lib/arm64-v8a/libiguanodon_ffi.so"
    "${TOOLCHAIN_BIN}/llvm-strip" --strip-unneeded "${WORK_APK_DIR}/apk_root/lib/arm64-v8a/libiguanodon_ffi.so" || true

    python3 -c "
import zipfile
with zipfile.ZipFile('${WORK_APK_DIR}/unaligned.apk', 'a') as z:
    z.write('${WORK_APK_DIR}/apk_root/classes.dex', 'classes.dex', compress_type=zipfile.ZIP_DEFLATED)
    z.write('${WORK_APK_DIR}/apk_root/lib/arm64-v8a/libiguanodon_ffi.so', 'lib/arm64-v8a/libiguanodon_ffi.so', compress_type=zipfile.ZIP_STORED)
"

    "${BUILD_TOOLS_DIR}/zipalign" -f -P 16 4 \
        "${WORK_APK_DIR}/unaligned.apk" \
        "${WORK_APK_DIR}/aligned.apk"

    KEYSTORE="${ROOT_DIR}/target/android-debug.keystore"
    if [[ ! -f "${KEYSTORE}" ]]; then
        keytool -genkeypair \
            -keystore "${KEYSTORE}" \
            -storepass android \
            -keypass android \
            -alias androiddebugkey \
            -keyalg RSA \
            -keysize 2048 \
            -validity 10000 \
            -dname "CN=Android Debug,O=Android,C=US" >/dev/null 2>&1
    fi

    FINAL_APK="${ROOT_DIR}/target/iguanodon-android-benchmark.apk"
    "${BUILD_TOOLS_DIR}/apksigner" sign \
        --ks "${KEYSTORE}" \
        --ks-pass pass:android \
        --key-pass pass:android \
        --out "${FINAL_APK}" \
        "${WORK_APK_DIR}/aligned.apk"

    echo "[+] Built & signed APK: ${FINAL_APK}"
    echo "[*] Installing APK onto ${MODEL} (${SERIAL})..."
    if ! adb -s "${SERIAL}" install -r "${FINAL_APK}"; then
        echo "[*] Existing debug keystore changed; reinstalling package cleanly..."
        adb -s "${SERIAL}" uninstall dev.iguanodon.android.benchmark >/dev/null 2>&1 || true
        adb -s "${SERIAL}" install -r "${FINAL_APK}"
    fi
fi

echo "[*] Launching IguanodonBenchmarkActivity on ${MODEL}..."
adb -s "${SERIAL}" shell am start -n dev.iguanodon.android.benchmark/iguanodon.android.benchmark.IguanodonBenchmarkActivity
echo "[+] Successfully launched Iguanodon Vulkan Benchmark App on ${MODEL}!"
