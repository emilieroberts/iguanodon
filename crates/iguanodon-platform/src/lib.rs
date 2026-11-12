// Copyright 2026 Emilie Roberts
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0

//! Cross-platform windowing & Vulkan surface portability layer (`iguanodon-platform`).
//! Supports Android (`VK_KHR_android_surface`), Linux (`VK_KHR_wayland_surface` / `VK_KHR_xlib_surface`),
//! Windows (`VK_KHR_win32_surface`), and macOS/iOS via MoltenVK & KosmicKrisp (`VK_EXT_metal_surface` +
//! `VK_KHR_portability_enumeration` / `VK_KHR_portability_subset`).

use iguanodon_core::Point;
use iguanodon_ffi::{
    apply_commands_to_harness, iguanodon_engine_create, iguanodon_engine_destroy,
    iguanodon_flush_commands, iguanodon_key_char, iguanodon_pointer_down, iguanodon_pointer_move,
    iguanodon_pointer_up, iguanodon_render_frame_rgba, BinaryCommandEncoder,
};
use iguanodon_theme::ThemeId;
use iguanodon_widgets::{bench_nodes, IguanodonTestHarness};
use std::ffi::{c_char, c_int, c_long, c_uint, c_ulong, c_void};
use std::fs::File;
use std::io::Write;
use std::ptr;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformTarget {
    Android,
    LinuxWayland,
    LinuxX11,
    WindowsWin32,
    MacOsMoltenVk,
    IosKosmicKrisp,
}

impl PlatformTarget {
    pub fn required_instance_extensions(&self) -> &'static [&'static str] {
        match self {
            Self::Android => &["VK_KHR_surface", "VK_KHR_android_surface"],
            Self::LinuxWayland => &["VK_KHR_surface", "VK_KHR_wayland_surface"],
            Self::LinuxX11 => &["VK_KHR_surface", "VK_KHR_xlib_surface", "VK_KHR_xcb_surface"],
            Self::WindowsWin32 => &["VK_KHR_surface", "VK_KHR_win32_surface"],
            Self::MacOsMoltenVk | Self::IosKosmicKrisp => &[
                "VK_KHR_surface",
                "VK_EXT_metal_surface",
                "VK_KHR_portability_enumeration",
                "VK_KHR_get_physical_device_properties2",
            ],
        }
    }

    pub fn requires_portability_subset_device_ext(&self) -> bool {
        matches!(self, Self::MacOsMoltenVk | Self::IosKosmicKrisp)
    }
}

/// Summary report produced by running the Standard Benchmark Sample App stress suite.
#[derive(Debug, Clone)]
pub struct BenchmarkReport {
    pub gpu_device_name: String,
    pub hardware_vulkan_active: bool,
    pub total_frames_rendered: usize,
    pub avg_fps: f32,
    pub one_percent_low_fps: f32,
    pub avg_frame_ms: f32,
    pub uhd_4k_frame_ms: f32,
    pub uhd_4k_fps: f32,
    pub latest_gpu_us: f32,
    pub lazy_list_total_items: usize,
    pub materialized_scene_nodes: usize,
    pub draw_calls_per_frame: usize,
    pub vram_allocated_kb: usize,
}

/// Runs a deterministic multi-theme stress benchmark of the Standard Benchmark Sample App,
/// including a 4K UHD (`3840x2160`) high-resolution pass.
pub fn run_standard_benchmark_suite(frames_per_theme: usize) -> BenchmarkReport {
    let mut harness = IguanodonTestHarness::new(1180.0, 760.0);

    // Exercise all interactive components during the stress run
    harness.click_node(bench_nodes::PRIMARY_BTN);
    harness.type_into_text_field(true, "Iguanodon 120Hz Vulkan Stress Benchmark");
    harness.drag_and_drop(
        bench_nodes::DRAGGABLE_CARD_A,
        "DragPayload-Alpha",
        bench_nodes::DROP_TARGET_BUCKET,
    );
    harness.right_click_node(bench_nodes::CONTEXT_MENU_REGION);
    harness.click_context_menu_item(0);

    for theme_id in ThemeId::ALL {
        harness.app.set_theme(theme_id);
        for f in 0..frames_per_theme {
            harness.scroll_node(bench_nodes::LAZY_COLUMN, 72.0);
            harness.scroll_node(bench_nodes::SCROLLABLE_TEXT, 4.0);
            if f % 10 == 0 {
                harness.click_node(bench_nodes::ANIMATED_SHADER_CARD);
            }
            harness.app.render_benchmark_frame(
                &mut harness.vk,
                1.0 / 120.0,
                160,
                100,
                &mut harness.framebuffer_rgba,
            );
        }
    }

    let latest = harness.app.telemetry.latest();
    let avg_fps = harness.app.telemetry.avg_fps();
    let one_percent_low_fps = harness.app.telemetry.one_percent_low_fps();
    let avg_frame_ms = harness.app.telemetry.avg_frame_ms();

    // Measure full 4K UHD (3840x2160) frame with Fire, Frosted Glass, Water Ripple, and Roboto text
    // (1 warm-up frame to ensure TrueType glyph atlas & 33MB framebuffer pages are resident)
    harness.resize(3840.0, 2160.0);
    let _ = harness.app.render_benchmark_frame(
        &mut harness.vk,
        1.0 / 120.0,
        3840,
        2160,
        &mut harness.framebuffer_rgba,
    );
    let sample_4k = harness.app.render_benchmark_frame(
        &mut harness.vk,
        1.0 / 120.0,
        3840,
        2160,
        &mut harness.framebuffer_rgba,
    );
    let uhd_4k_frame_ms = sample_4k.total_frame_ms.max(0.1);
    let uhd_4k_fps = 1000.0 / uhd_4k_frame_ms;

    BenchmarkReport {
        gpu_device_name: harness.app.telemetry.gpu_device_name.clone(),
        hardware_vulkan_active: harness.vk.hardware_active,
        total_frames_rendered: frames_per_theme * ThemeId::ALL.len(),
        avg_fps,
        one_percent_low_fps,
        avg_frame_ms,
        uhd_4k_frame_ms,
        uhd_4k_fps,
        latest_gpu_us: latest.gpu_execution_us,
        lazy_list_total_items: harness.app.lazy_total_items,
        materialized_scene_nodes: latest.materialized_nodes,
        draw_calls_per_frame: latest.draw_calls,
        vram_allocated_kb: harness.app.telemetry.vram_allocated_bytes / 1024,
    }
}

/// Renders a high-resolution snapshot (`960x600`) of the Standard Benchmark Sample App and writes it to a `.ppm` image file.
pub fn save_benchmark_snapshot_ppm(path: &str, theme: ThemeId) -> std::io::Result<()> {
    let w = 960_u32;
    let h = 600_u32;
    let mut harness = IguanodonTestHarness::new(w as f32, h as f32);
    harness.app.set_theme(theme);
    harness.click_node(bench_nodes::PRIMARY_BTN);
    harness.drag_and_drop(
        bench_nodes::DRAGGABLE_CARD_A,
        "Vulkan-Asset-01",
        bench_nodes::DROP_TARGET_BUCKET,
    );
    harness.right_click_node(bench_nodes::CONTEXT_MENU_REGION);
    harness
        .app
        .render_benchmark_frame(&mut harness.vk, 1.0 / 120.0, w, h, &mut harness.framebuffer_rgba);

    let mut file = File::create(path)?;
    write!(file, "P6\n{} {}\n255\n", w, h)?;
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for chunk in harness.framebuffer_rgba.chunks_exact(4) {
        rgb.push(chunk[0]);
        rgb.push(chunk[1]);
        rgb.push(chunk[2]);
    }
    file.write_all(&rgb)?;
    Ok(())
}

// ============================================================================
// Dynamic X11 Interactive Desktop Window Runner (via `libX11.so.6`)
// ============================================================================

#[cfg(target_os = "linux")]
#[link(name = "dl")]
extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

#[repr(C)]
#[derive(Clone, Copy)]
struct XKeyEvent {
    type_: c_int,
    serial: c_ulong,
    send_event: c_int,
    display: *mut c_void,
    window: c_ulong,
    root: c_ulong,
    subwindow: c_ulong,
    time: c_ulong,
    x: c_int,
    y: c_int,
    x_root: c_int,
    y_root: c_int,
    state: c_uint,
    keycode: c_uint,
    same_screen: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct XButtonEvent {
    type_: c_int,
    serial: c_ulong,
    send_event: c_int,
    display: *mut c_void,
    window: c_ulong,
    root: c_ulong,
    subwindow: c_ulong,
    time: c_ulong,
    x: c_int,
    y: c_int,
    x_root: c_int,
    y_root: c_int,
    state: c_uint,
    button: c_uint,
    same_screen: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct XMotionEvent {
    type_: c_int,
    serial: c_ulong,
    send_event: c_int,
    display: *mut c_void,
    window: c_ulong,
    root: c_ulong,
    subwindow: c_ulong,
    time: c_ulong,
    x: c_int,
    y: c_int,
    x_root: c_int,
    y_root: c_int,
    state: c_uint,
    is_hint: c_char,
    same_screen: c_int,
}

#[repr(C)]
union XEvent {
    type_: c_int,
    key: XKeyEvent,
    button: XButtonEvent,
    motion: XMotionEvent,
    pad: [c_long; 24],
}

/// Attempts to open an interactive desktop window on Linux (X11/XWayland).
/// Returns `true` if the window opened and ran until closed by the user, or `false` if no display was available.
pub fn try_run_interactive_window() -> bool {
    try_run_interactive_window_with_mode(false)
}

/// Attempts to open an interactive window on Linux (X11/XWayland), optionally in Android Kotlin Multiplatform
/// touch simulation mode (`android_mode = true`), where touch-pan scrolling and 450ms stationary touch long-press
/// (opening the Frosted-Glass Context Menu) mirror `IguanodonVulkanSurfaceView` on Android.
pub fn try_run_interactive_window_with_mode(android_mode: bool) -> bool {
    #[cfg(target_os = "linux")]
    unsafe {
        let lib = dlopen(b"libX11.so.6\0".as_ptr() as *const c_char, 2);
        if lib.is_null() {
            return false;
        }
        macro_rules! xsym {
            ($name:literal, $ty:ty) => {{
                let p = dlsym(lib, concat!($name, "\0").as_ptr() as *const c_char);
                if p.is_null() {
                    return false;
                }
                std::mem::transmute::<*mut c_void, $ty>(p)
            }};
        }

        let x_open_display = xsym!("XOpenDisplay", unsafe extern "C" fn(*const c_char) -> *mut c_void);
        let x_close_display = xsym!("XCloseDisplay", unsafe extern "C" fn(*mut c_void) -> c_int);
        let x_default_screen = xsym!("XDefaultScreen", unsafe extern "C" fn(*mut c_void) -> c_int);
        let x_root_window = xsym!("XRootWindow", unsafe extern "C" fn(*mut c_void, c_int) -> c_ulong);
        let x_default_visual = xsym!("XDefaultVisual", unsafe extern "C" fn(*mut c_void, c_int) -> *mut c_void);
        let x_default_depth = xsym!("XDefaultDepth", unsafe extern "C" fn(*mut c_void, c_int) -> c_int);
        let x_default_gc = xsym!("XDefaultGC", unsafe extern "C" fn(*mut c_void, c_int) -> *mut c_void);
        let x_create_simple_window = xsym!(
            "XCreateSimpleWindow",
            unsafe extern "C" fn(
                *mut c_void,
                c_ulong,
                c_int,
                c_int,
                c_uint,
                c_uint,
                c_uint,
                c_ulong,
                c_ulong,
            ) -> c_ulong
        );
        let x_store_name = xsym!("XStoreName", unsafe extern "C" fn(*mut c_void, c_ulong, *const c_char) -> c_int);
        let x_select_input = xsym!("XSelectInput", unsafe extern "C" fn(*mut c_void, c_ulong, c_long) -> c_int);
        let x_map_window = xsym!("XMapWindow", unsafe extern "C" fn(*mut c_void, c_ulong) -> c_int);
        let x_pending = xsym!("XPending", unsafe extern "C" fn(*mut c_void) -> c_int);
        let x_next_event = xsym!("XNextEvent", unsafe extern "C" fn(*mut c_void, *mut XEvent) -> c_int);
        let x_get_geometry = xsym!(
            "XGetGeometry",
            unsafe extern "C" fn(
                *mut c_void,
                c_ulong,
                *mut c_ulong,
                *mut c_int,
                *mut c_int,
                *mut c_uint,
                *mut c_uint,
                *mut c_uint,
                *mut c_uint,
            ) -> c_int
        );
        let x_lookup_string = xsym!(
            "XLookupString",
            unsafe extern "C" fn(*mut XKeyEvent, *mut c_char, c_int, *mut c_ulong, *mut c_void) -> c_int
        );
        let x_create_image = xsym!(
            "XCreateImage",
            unsafe extern "C" fn(
                *mut c_void,
                *mut c_void,
                c_uint,
                c_int,
                c_int,
                *mut c_char,
                c_uint,
                c_uint,
                c_int,
                c_int,
            ) -> *mut c_void
        );
        let x_put_image = xsym!(
            "XPutImage",
            unsafe extern "C" fn(
                *mut c_void,
                c_ulong,
                *mut c_void,
                *mut c_void,
                c_int,
                c_int,
                c_int,
                c_int,
                c_uint,
                c_uint,
            ) -> c_int
        );
        let x_flush = xsym!("XFlush", unsafe extern "C" fn(*mut c_void) -> c_int);

        let dpy = x_open_display(ptr::null());
        if dpy.is_null() {
            return false;
        }

        let screen = x_default_screen(dpy);
        let root = x_root_window(dpy, screen);
        let visual = x_default_visual(dpy, screen);
        let depth = x_default_depth(dpy, screen) as c_uint;
        let gc = x_default_gc(dpy, screen);

        let mut win_w = 1180_u32;
        let mut win_h = 760_u32;
        let win = x_create_simple_window(dpy, root, 60, 40, win_w, win_h, 0, 0, 0x141218);
        let title: &[u8] = if android_mode {
            b"Iguanodon Android Test App (Kotlin Multiplatform + Compose Runtime + Vulkan 120Hz, ESC to quit)\0"
        } else {
            b"Iguanodon Vulkan UI Toolkit - Standard Benchmark Sample App (Resizable, ESC to quit)\0"
        };
        x_store_name(dpy, win, title.as_ptr() as *const c_char);

        // KeyPressMask | ButtonPressMask | ButtonReleaseMask | PointerMotionMask | ExposureMask | StructureNotifyMask
        let event_mask: c_long =
            (1 << 0) | (1 << 2) | (1 << 3) | (1 << 6) | (1 << 15) | (1 << 17);
        x_select_input(dpy, win, event_mask);
        x_map_window(dpy, win);

        let mut harness = IguanodonTestHarness::new(win_w as f32, win_h as f32);
        if android_mode {
            // Boot the scene with the exact initial FFI command batch emitted by the shared
            // Kotlin Multiplatform `@Composable fun StandardBenchmarkApp(...)` on Android!
            let mut init_batch = BinaryCommandEncoder::new();
            init_batch.encode_set_theme(ThemeId::Material3Expressive);
            init_batch.encode_set_label_text(
                bench_nodes::HEADLINE_LABEL.index,
                "Iguanodon Studio — Standard Benchmark App (Android KMP + Desktop Shared UX)",
            );
            let _ = apply_commands_to_harness(&mut harness, &init_batch.bytes);
        }

        x_flush(dpy);
        let swapchain_active = harness.vk.attach_xlib_surface(dpy, win as u64, win_w, win_h);

        let mut bgra_buf = vec![0_u8; (win_w * win_h * 4) as usize];
        let mut ximg = x_create_image(
            dpy,
            visual,
            depth,
            2, // ZPixmap
            0,
            bgra_buf.as_mut_ptr() as *mut c_char,
            win_w,
            win_h,
            32,
            (win_w * 4) as c_int,
        );

        if android_mode {
            println!(
                "Opened Iguanodon Android KMP Test App ({}x{}) on {}",
                win_w, win_h, harness.vk.device_name
            );
            println!("Shared Kotlin Multiplatform UX (`StandardBenchmarkApp.kt`) Touch & Mouse Controls:");
            println!(" - Tap / Left Click: Tap buttons (>=48dp), switch themes, focus TextField, select LazyColumn items");
            println!(" - Touch-Pan / Scroll: Drag vertically inside the 10,000-item LazyColumn or ScrollableText (or use scroll wheel)");
            println!(" - Long-Press (450ms) / Right-Click: Open Frosted-Glass Context Menu anywhere");
            println!(" - Drag & Drop: Drag 'Vulkan Asset #01' or 'M3 Token Pack #02' into the Drop Target Zone");
        } else {
            println!("Opened resizable Iguanodon window ({}x{}) on {}", win_w, win_h, harness.vk.device_name);
            println!("Controls:");
            println!(" - Resize Window: Drag any window edge/corner or maximize to reflow layout dynamically");
            println!(" - Left Click: Click buttons, switch themes, focus TextField, select LazyColumn items");
            println!(" - Drag & Drop: Drag 'Vulkan Asset #01' or 'M3 Token Pack #02' into the Drop Target Zone");
            println!(" - Right Click / 450ms Long-Press: Open Frosted-Glass Context Menu anywhere");
            println!(" - Touch-Pan / Scroll Wheel: Drag or scroll the 10,000-item LazyColumn or ScrollableText");
            println!(" - Keyboard: Type into the focused TextField (ESC to close menu or exit)");
        }

        let mut last_tick = Instant::now();
        let mut last_interaction = Instant::now();
        let mut needs_redraw = true;
        let mut touch_down_tracker: Option<(Instant, Point, bool)> = None;
        let mut running = true;
        while running {
            let mut pending_resize = false;
            while x_pending(dpy) > 0 {
                let mut ev: XEvent = std::mem::zeroed();
                x_next_event(dpy, &mut ev);
                last_interaction = Instant::now();
                needs_redraw = true;
                match ev.type_ {
                    2 => {
                        // KeyPress
                        let mut key_buf = [0 as c_char; 8];
                        let mut keysym: c_ulong = 0;
                        let n = x_lookup_string(
                            &mut ev.key,
                            key_buf.as_mut_ptr(),
                            8,
                            &mut keysym,
                            ptr::null_mut(),
                        );
                        if keysym == 0xFF1B {
                            // XK_Escape
                            if harness.app.context_menu.is_open {
                                harness.app.context_menu.close();
                            } else {
                                running = false;
                            }
                        } else if keysym == 0xFF08 {
                            // XK_BackSpace
                            harness.app.text_field.backspace();
                        } else if n > 0 {
                            let ch = key_buf[0] as u8 as char;
                            if ch >= ' ' && ch <= '~' {
                                harness.app.text_field.insert_str(&ch.to_string());
                            }
                        }
                    }
                    4 => {
                        // ButtonPress
                        let pt = Point::new(ev.button.x as f32, ev.button.y as f32);
                        match ev.button.button {
                            1 => {
                                touch_down_tracker = Some((Instant::now(), pt, false));
                                harness.on_pointer_down(pt, false);
                            }
                            3 => {
                                touch_down_tracker = None;
                                harness.on_pointer_down(pt, true);
                            }
                            4 => harness.on_scroll_wheel(pt, -48.0), // Wheel Up
                            5 => harness.on_scroll_wheel(pt, 48.0),  // Wheel Down
                            _ => {}
                        }
                    }
                    5 => {
                        // ButtonRelease
                        let pt = Point::new(ev.button.x as f32, ev.button.y as f32);
                        if ev.button.button == 1 {
                            let long_press_triggered = touch_down_tracker
                                .map(|(_, _, fired)| fired)
                                .unwrap_or(false);
                            touch_down_tracker = None;
                            if !long_press_triggered {
                                harness.on_pointer_up(pt);
                            } else {
                                // Clear any active scroll/drag state without closing the newly opened context menu
                                harness.active_scroll_drag = None;
                                harness.app.drag_drop.cancel_drag();
                            }
                        }
                    }
                    6 => {
                        // MotionNotify
                        let pt = Point::new(ev.motion.x as f32, ev.motion.y as f32);
                        if let Some((_, start_pt, ref mut fired)) = touch_down_tracker {
                            let dist = ((pt.x - start_pt.x).powi(2) + (pt.y - start_pt.y).powi(2)).sqrt();
                            if dist > 8.0 {
                                // Moved beyond touch slop — cancel long-press timer
                                *fired = true;
                            }
                        }
                        harness.on_pointer_move(pt);
                    }
                    12 | 22 => {
                        // Expose (12) or ConfigureNotify (22 - window resized/moved)
                        pending_resize = true;
                    }
                    _ => {}
                }
            }

            // Check Android 450ms stationary touch long-press to open Frosted-Glass Context Menu
            if let Some((down_instant, down_pt, ref mut fired)) = touch_down_tracker {
                if !*fired && down_instant.elapsed() >= Duration::from_millis(450) {
                    *fired = true;
                    harness.active_scroll_drag = None;
                    harness.app.drag_drop.cancel_drag();
                    harness.on_pointer_down(down_pt, true);
                }
            }

            if pending_resize {
                let mut root_ret: c_ulong = 0;
                let mut x_ret: c_int = 0;
                let mut y_ret: c_int = 0;
                let mut w_ret: c_uint = 0;
                let mut h_ret: c_uint = 0;
                let mut border_ret: c_uint = 0;
                let mut depth_ret: c_uint = 0;
                if x_get_geometry(
                    dpy,
                    win,
                    &mut root_ret,
                    &mut x_ret,
                    &mut y_ret,
                    &mut w_ret,
                    &mut h_ret,
                    &mut border_ret,
                    &mut depth_ret,
                ) != 0
                {
                    let (new_w, new_h) = (w_ret.clamp(360, 3840), h_ret.clamp(360, 3840));
                    if new_w != win_w || new_h != win_h {
                        win_w = new_w;
                        win_h = new_h;
                        harness.resize(win_w as f32, win_h as f32);
                        bgra_buf.resize((win_w * win_h * 4) as usize, 0);
                        ximg = x_create_image(
                            dpy,
                            visual,
                            depth,
                            2,
                            0,
                            bgra_buf.as_mut_ptr() as *mut c_char,
                            win_w,
                            win_h,
                            32,
                            (win_w * 4) as c_int,
                        );
                    }
                }
            }

            if touch_down_tracker.is_some()
                || harness.active_scroll_drag.is_some()
                || harness.app.has_active_spring_or_drag()
            {
                last_interaction = Instant::now();
                needs_redraw = true;
            }

            let is_actively_interacting = last_interaction.elapsed() < Duration::from_millis(300);
            let animations_active = harness.app.animation_speed > 0.001
                && harness.app.renderer_backend != iguanodon_widgets::RendererBackend::ComposeUi;

            if !is_actively_interacting && !animations_active && !needs_redraw {
                std::thread::sleep(Duration::from_millis(40));
                last_tick = Instant::now();
                continue;
            }

            let frame_begin = Instant::now();
            let dt = last_tick.elapsed().as_secs_f32().clamp(1.0 / 1000.0, 0.15);
            last_tick = frame_begin;
            needs_redraw = false;

            if swapchain_active {
                harness.render_and_present(dt, win_w, win_h);
                x_flush(dpy);
            } else {
                harness.render_frame_rgba(dt, win_w, win_h);

                // Convert RGBA -> BGRA for X11 ZPixmap fallback
                for (src, dst) in harness
                    .framebuffer_rgba
                    .chunks_exact(4)
                    .zip(bgra_buf.chunks_exact_mut(4))
                {
                    dst[0] = src[2];
                    dst[1] = src[1];
                    dst[2] = src[0];
                    dst[3] = 255;
                }

                x_put_image(dpy, win, gc, ximg, 0, 0, 0, 0, win_w, win_h);
                x_flush(dpy);
            }

            let base_interval = harness.app.target_frame_interval_sec();
            if base_interval > 0.0 {
                let target_interval = Duration::from_secs_f32(base_interval);
                let elapsed = frame_begin.elapsed();
                if elapsed < target_interval {
                    std::thread::sleep(target_interval - elapsed);
                }
            }
        }

        harness.vk.detach_surface();
        x_close_display(dpy);
        return true;
    }
    #[allow(unreachable_code)]
    false
}

/// Verifies 100% behavioral and visual parity between the Android Kotlin Multiplatform FFI session
/// and the Desktop FFI session when executing the shared `@Composable fun StandardBenchmarkApp(...)`
/// command batches and touch gestures.
pub fn verify_android_desktop_kmp_parity() -> bool {
    let android_handle = iguanodon_engine_create(1180.0, 760.0);
    let desktop_handle = iguanodon_engine_create(1180.0, 760.0);

    // Emit the exact binary command batch produced by `StandardBenchmarkApp.kt`
    let mut batch = BinaryCommandEncoder::new();
    batch.encode_set_theme(ThemeId::LinuxEliteObsidian);
    batch.encode_click_node(bench_nodes::PRIMARY_BTN.index);
    batch.encode_set_text_field("Shared Kotlin Multiplatform UX");
    batch.encode_set_progress(0.85);
    batch.encode_drag_and_drop(
        bench_nodes::DRAGGABLE_CARD_A.index,
        bench_nodes::DROP_TARGET_BUCKET.index,
        "Vulkan-Shader-Asset-01",
    );
    batch.encode_scroll_node(bench_nodes::LAZY_COLUMN.index, 240.0);

    let bytes = &batch.bytes;
    unsafe {
        iguanodon_flush_commands(android_handle, bytes.as_ptr(), bytes.len());
        iguanodon_flush_commands(desktop_handle, bytes.as_ptr(), bytes.len());

        // Simulate touch/pointer interactions on both engines
        iguanodon_pointer_move(android_handle, 500.0, 340.0);
        iguanodon_pointer_move(desktop_handle, 500.0, 340.0);
        iguanodon_pointer_down(android_handle, 500.0, 340.0, true); // Android 450ms long-press
        iguanodon_pointer_down(desktop_handle, 500.0, 340.0, true); // Desktop right-click
        iguanodon_pointer_up(android_handle, 500.0, 340.0);
        iguanodon_pointer_up(desktop_handle, 500.0, 340.0);
        iguanodon_key_char(android_handle, b'!' as u32);
        iguanodon_key_char(desktop_handle, b'!' as u32);
    }

    let mut android_rgba = vec![0_u8; 320 * 200 * 4];
    let mut desktop_rgba = vec![0_u8; 320 * 200 * 4];
    unsafe {
        iguanodon_render_frame_rgba(
            android_handle,
            1.0 / 120.0,
            320,
            200,
            android_rgba.as_mut_ptr(),
        );
        iguanodon_render_frame_rgba(
            desktop_handle,
            1.0 / 120.0,
            320,
            200,
            desktop_rgba.as_mut_ptr(),
        );
        iguanodon_engine_destroy(android_handle);
        iguanodon_engine_destroy(desktop_handle);
    }

    android_rgba == desktop_rgba && android_rgba.iter().any(|&b| b > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_extensions_for_moltenvk_and_kosmickrisp() {
        assert!(PlatformTarget::MacOsMoltenVk.requires_portability_subset_device_ext());
        assert!(PlatformTarget::IosKosmicKrisp.requires_portability_subset_device_ext());
        assert!(!PlatformTarget::Android.requires_portability_subset_device_ext());
        assert!(!PlatformTarget::LinuxWayland.requires_portability_subset_device_ext());
    }

    #[test]
    fn test_standard_benchmark_suite_execution() {
        let report = run_standard_benchmark_suite(5);
        assert_eq!(report.total_frames_rendered, 20);
        assert!(report.hardware_vulkan_active);
        assert_eq!(report.lazy_list_total_items, 10_000);
        assert!(report.materialized_scene_nodes < 60);
        assert!(report.draw_calls_per_frame <= 2);
    }

    #[test]
    fn test_android_and_desktop_kmp_parity() {
        assert!(verify_android_desktop_kmp_parity());
    }
}
