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

#![forbid(unsafe_code)]

use iguanodon_platform::{
    run_standard_benchmark_suite, save_benchmark_snapshot_ppm, try_run_interactive_window_with_mode,
    verify_android_desktop_kmp_parity,
};
use iguanodon_theme::ThemeId;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let force_headless = args.iter().any(|a| a == "--bench" || a == "--headless");
    let android_mode = args.iter().any(|a| a == "--android");

    println!("==================================================================");
    if android_mode {
        println!(" 🦖 Iguanodon Vulkan UI Toolkit — Android KMP Test App (Shared UX)");
    } else {
        println!(" 🦖 Iguanodon Vulkan UI Toolkit — Standard Benchmark Sample App");
    }
    println!("==================================================================");

    if !force_headless && try_run_interactive_window_with_mode(android_mode) {
        return;
    }

    let kmp_parity = verify_android_desktop_kmp_parity();
    let report = run_standard_benchmark_suite(30);
    println!("Vulkan GPU Device       : {}", report.gpu_device_name);
    println!("Hardware Vulkan Active  : {}", report.hardware_vulkan_active);
    println!("Android/Desktop Parity  : {}", if kmp_parity { "100% Identical Framebuffer & State" } else { "MISMATCH" });
    println!("Total Stress Frames     : {}", report.total_frames_rendered);
    println!("Average Frame Time      : {:.3} ms", report.avg_frame_ms);
    println!("Average FPS             : {:.1} FPS", report.avg_fps);
    println!("1% Low FPS              : {:.1} FPS", report.one_percent_low_fps);
    println!(
        "4K UHD (3840x2160) Pass : {:.2} ms ({:.1} FPS)",
        report.uhd_4k_frame_ms, report.uhd_4k_fps
    );
    println!("Hardware GPU Pass Time  : {:.1} us", report.latest_gpu_us);
    println!(
        "LazyColumn Efficiency   : {} materialized nodes / {} logical items",
        report.materialized_scene_nodes, report.lazy_list_total_items
    );
    println!("Instanced Draw Calls    : {} per frame", report.draw_calls_per_frame);
    println!("Vulkan VRAM Allocated   : {} KB", report.vram_allocated_kb);

    if let Ok(()) = save_benchmark_snapshot_ppm("benchmark_snapshot.ppm", ThemeId::Material3Expressive) {
        println!("Saved UI Snapshot       : benchmark_snapshot.ppm (960x600)");
    }
    println!("==================================================================");
}
