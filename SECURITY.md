# Security Policy & Audit Summary (`v0.0.1-alpha`)

> [!WARNING]
> **Experimental Alpha Disclaimer**: Iguanodon (`v0.0.1-alpha`) is an experimental research prototype built with Google Gemini 4 and is **not intended for production environments**.

## Pre-Release Security Audit (`2026-11-10`)

Prior to publishing [`github.com/emilieroberts/iguanodon`](https://github.com/emilieroberts/iguanodon), a full security and privacy audit was conducted across all Rust crates, Kotlin Multiplatform modules, build scripts, and AI skill definitions:

### 1. Rust Memory Safety & `#![forbid(unsafe_code)]`
- **Safe-by-Default Crates**: `iguanodon-core`, `iguanodon-theme`, `iguanodon-layout`, `iguanodon-text`, `iguanodon-shaders`, `iguanodon-widgets`, `benchmark_app`, and `token-sync` all enforce `#![forbid(unsafe_code)]` at the crate root.
- **Isolated FFI & Driver Boundaries**: `unsafe` code is restricted strictly to:
  - [`crates/iguanodon-ffi/src/lib.rs`](./crates/iguanodon-ffi/src/lib.rs): C-ABI and JNI entrypoints.
  - [`crates/iguanodon-vk/src/lib.rs`](./crates/iguanodon-vk/src/lib.rs): Dynamic `libvulkan.so` C API function table loading.
  - [`crates/iguanodon-platform/src/lib.rs`](./crates/iguanodon-platform/src/lib.rs): Dynamic `libX11.so.6` windowing C API loading.

### 2. Binary Command Decoder & JNI Boundary Hardening (`crates/iguanodon-ffi`)
- **Overflow-Safe Cursor Arithmetic**: All binary command readers (`read_u16`, `read_u32`, `read_u64`, `read_f32`, `read_str`, and `FfiOpcode::SetImageBytes`) use `cursor.checked_add(len)` before slicing `buf`, preventing 32-bit/64-bit `usize` wrap-around on malformed length prefixes.
- **Batch Size & Framebuffer Dimension Caps**:
  - `iguanodon_flush_commands`, `nativeFlushCommands`, and `nativePollEvents` enforce `MAX_FFI_BATCH_BYTES = 64 MiB` (`64 * 1024 * 1024`), preventing unbounded heap allocations from corrupted JNI array lengths.
  - `iguanodon_render_frame_rgba` and `nativeRenderFrameByteArray` clamp dimensions to `MAX_FRAMEBUFFER_DIM = 8192` and compute `width * height * 4` via `checked_mul` before copying pixels.
- **Null-Pointer Guards**: Every `extern "C"` and JNI function checks `engine_ptr`, `env`, and array/buffer pointers for `NULL` prior to dereferencing.

### 3. Custom Shader Validation (`crates/iguanodon-shaders`)
- [`validate_custom_wgsl`](./crates/iguanodon-shaders/src/lib.rs) enforces a strict 16 KiB source limit, rejects non-terminating loop constructs (`loop {`, `while(true)`, `while (true)`), and blocks `unsafe` / `asm!` / unchecked storage pointer directives.

### 4. Native Library Extraction & Credential / Path Hygiene
- **Temp File Permissions**: [`IguanodonNativeBridge.kt`](./kotlin/iguanodon-compose/src/commonMain/kotlin/iguanodon/compose/bridge/IguanodonNativeBridge.kt) sets owner-only read/write/execute (`0700`) permissions on temporary `.so` files extracted from JAR resources before writing bytes and invoking `System.load`.
- **Zero Secrets or Local Machine Paths**:
  - Audited all tracked files to ensure no API keys, tokens, private signing keys, or user-specific paths exist in the repository.
  - [`dist/pkgconfig/iguanodon.pc`](./dist/pkgconfig/iguanodon.pc) uses relocatable `${pcfiledir}/..` paths.
  - Local Android SDK paths (`kotlin/local.properties`) and temporary debug keystores (`*.keystore`) are excluded via [`.gitignore`](./.gitignore).

## Reporting a Vulnerability

If you discover a security issue in Iguanodon, please open an issue or security advisory on [`https://github.com/emilieroberts/iguanodon`](https://github.com/emilieroberts/iguanodon).
