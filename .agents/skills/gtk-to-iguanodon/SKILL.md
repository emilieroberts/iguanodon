---
name: gtk-to-iguanodon
description: >-
  Version 1 workflow and toolkit for porting GTK 3, GTK 4, and Libadwaita applications
  (GtkBuilder .ui XML, Blueprint .blp, C/Vala/Python/gtk4-rs) to Iguanodon targeting
  Linux Desktop and Android. Use this skill whenever the user wants to port or migrate a
  GTK or Libadwaita app to Iguanodon.
---

# Porting GTK 3 / GTK 4 & Libadwaita Apps to Iguanodon (Linux & Android) — v1.0

This skill guides you through migrating a GTK 3, GTK 4, or GNOME Libadwaita application (`GtkBuilder` `.ui` XML, Blueprint `.blp`, C, `gtk4-rs`, or `PyGObject`) to **Iguanodon** so it runs natively with Vulkan GPU acceleration on both **Linux Desktop** and **Android** (with Edge-to-Edge `WindowInsets`, Portrait/Landscape responsive adaptation, and Light/Dark themes).

## Workflow Steps

### 1. Scan the Source GTK / Libadwaita Project
Run the GTK scanner script against the target repository to inventory all `.ui`, `.blp`, `.c`, `.rs`, and `.py` files, map every `Gtk*` / `Adw*` widget to its Iguanodon equivalent, and generate a starter Iguanodon scaffold:

```bash
python3 skills/gtk-to-iguanodon/scripts/scan_gtk_project.py <path-to-gtk-project> --out <output-dir>
```
- Scanner script: [scan_gtk_project.py](./scripts/scan_gtk_project.py)
- Complete GTK 4 / Libadwaita $\rightarrow$ Iguanodon mapping table: [gtk-widget-mapping.md](./references/gtk-widget-mapping.md)

### 2. Choose the Integration Target (Rust / C pkg-config vs. Kotlin Multiplatform)
- **Option A — Rust (`gtk4-rs` / `relm4`) or C/Meson Native Port**:
  1. For Rust projects, replace `gtk4` / `libadwaita` crate dependencies with `iguanodon-widgets`, `iguanodon-theme`, and `iguanodon-vk` (or link `libiguanodon_ffi.so` via `pkg-config` using [`dist/pkgconfig/iguanodon.pc`](../../../dist/pkgconfig/iguanodon.pc)).
- **Option B — Shared Kotlin Multiplatform (`@Composable` for Linux & Android)**:
  1. Include [`dist/iguanodon-compose-desktop-linux-x64-0.0.1-alpha.jar`](../../../dist/iguanodon-compose-desktop-linux-x64-0.0.1-alpha.jar) (Linux) and [`dist/iguanodon-compose-android-0.0.1-alpha.aar`](../../../dist/iguanodon-compose-android-0.0.1-alpha.aar) (Android).
  2. Use the direct GTK / Libadwaita compatibility composables in [`ComposePortingCompat.kt`](../../../kotlin/iguanodon-compose/src/commonMain/kotlin/iguanodon/compose/compat/ComposePortingCompat.kt) (`AdwApplicationWindow`, `AdwHeaderBar`, `AdwNavigationSplitView`, `AdwFlap`, `GtkPaned`, `AdwPreferencesPage`, `AdwPreferencesGroup`, `AdwActionRow`, `AdwSwitchRow`, `AdwEntryRow`, `AdwComboRow`).

### 3. Map GTK / Libadwaita Patterns to Iguanodon
1. **Adaptive Split Views & Breakpoints (`AdwNavigationSplitView` / `AdwBreakpoint`)**:
   - `AdwNavigationSplitView` and `NavigationSuiteScaffold` automatically collapse sidebars into a single-column Portrait stack or bottom `NavigationBar` on Android phones (`session.isPortrait == true`) and expand into a side `NavigationRail` / `HorizontalSplitPane` on Linux Desktop and Android Landscape.
2. **Navigation Stacks (`AdwNavigationView` / `GtkStack`)**:
   - Map `adw_navigation_view_push` and `adw_navigation_view_pop` directly onto Iguanodon's Navigation 3 [`rememberNavBackStack`](../../../kotlin/iguanodon-compose/src/commonMain/kotlin/iguanodon/compose/navigation3/Navigation3.kt) (`backStack.add(key)` / `backStack.removeLastOrNull()`) and [`NavDisplay`](../../../kotlin/iguanodon-compose/src/commonMain/kotlin/iguanodon/compose/navigation3/Navigation3.kt).
3. **GObject Signals & Properties (`g_signal_connect` / `GAction`)**:
   - Convert `notify::active` / `clicked` signals and `GSettings` properties into reactive `@Composable` state (`remember { mutableStateOf(...) }`) or Rust state fields.

### 4. Validate & Deploy (Linux & Android)
1. Run workspace unit tests:
   ```bash
   cargo test --workspace
   ```
2. Deploy to a connected Android phone via `adb`:
   ```bash
   ./scripts/run_android_phone.sh
   ```
