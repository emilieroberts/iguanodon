---
name: qt-to-iguanodon
description: >-
  Version 1 workflow and toolkit for porting Qt 5 and Qt 6 applications (QWidgets .ui XML,
  C++ signals/slots, and Qt Quick .qml) to Iguanodon targeting Linux Desktop and Android.
  Use this skill whenever the user wants to port, migrate, or convert a Qt or QML app to Iguanodon.
---

# Porting Qt (QWidgets & QML) Apps to Iguanodon (Linux & Android) — v1.0

This skill guides you through converting a Qt 5 / Qt 6 application (`QWidgets` `.ui` files, C++ `QObject::connect` signals/slots, or Qt Quick `.qml` files) into a hardware-accelerated **Iguanodon** application running natively on **Linux Desktop** and **Android** (with Edge-to-Edge `WindowInsets` and responsive Portrait/Landscape layouts).

## Workflow Steps

### 1. Scan the Source Qt Project & Generate an Iguanodon Scaffold
Run the bundled Qt scanner script against the target Qt project directory to inventory all `.ui`, `.qml`, `.h`, and `.cpp` files, map every Qt widget to its `UnifiedWidgetKind` / `@Composable` equivalent, and emit a starter Iguanodon Kotlin/Rust scaffold:

```bash
python3 skills/qt-to-iguanodon/scripts/scan_qt_project.py <path-to-qt-project> --out <output-dir>
```
- Inspector script: [scan_qt_project.py](./scripts/scan_qt_project.py)
- Complete Qt $\rightarrow$ Iguanodon widget & signal mapping table: [qt-widget-mapping.md](./references/qt-widget-mapping.md)

### 2. Choose the Integration Target (C++/CMake FFI vs. Kotlin Multiplatform)
- **Option A — Keep C++ Business Logic & Link `libiguanodon_ffi.so` via CMake**:
  1. In `CMakeLists.txt`, add:
     ```cmake
     list(APPEND CMAKE_PREFIX_PATH "<path-to-iguanodon>/dist/cmake")
     find_package(Iguanodon REQUIRED)
     target_link_libraries(your_app PRIVATE Iguanodon::iguanodon_ffi)
     ```
  2. Include `#include <iguanodon.h>` from [`dist/include/iguanodon.h`](../../../dist/include/iguanodon.h).
- **Option B — Port UI to Shared Kotlin Multiplatform (`@Composable` on Linux & Android)**:
  1. Drop [`dist/iguanodon-compose-desktop-linux-x64-0.0.1-alpha.jar`](../../../dist/iguanodon-compose-desktop-linux-x64-0.0.1-alpha.jar) (Linux) and [`dist/iguanodon-compose-android-0.0.1-alpha.aar`](../../../dist/iguanodon-compose-android-0.0.1-alpha.aar) (Android) into `libs/`, or link `includeBuild("<path-to-iguanodon>/kotlin")`.
  2. Use the direct Qt compatibility wrappers in [`ComposePortingCompat.kt`](../../../kotlin/iguanodon-compose/src/commonMain/kotlin/iguanodon/compose/compat/ComposePortingCompat.kt) (`QMainWindow`, `QSplitter`, `QDockWidget`, `QTabWidget`, `QGroupBox`, `QPushButton`, `QCheckBox`, `QLineEdit`, `QSpinBox`, `QComboBox`, `QProgressBar`, `TreeView`, `DataTable`).

### 3. Convert Qt Layouts, Dock Widgets, and Signals/Slots
1. **Layouts & Splitters**:
   - Replace `QHBoxLayout` / `RowLayout` with `Row`, `QVBoxLayout` / `ColumnLayout` with `Column`, and `QSplitter` with `QSplitter(horizontal = ..., splitRatio = ...)` (`HorizontalSplitPane` / `VerticalSplitPane`).
   - Replace `QDockWidget` with `QDockWidget(title = ...)` / `DraggablePanel(title = ...)`.
2. **Signals & Slots (`QObject::connect`)**:
   - Convert `QProperty<T>` / `NOTIFY` signals into `var state by remember { mutableStateOf(...) }` or `StateFlow<T>`.
   - Convert `connect(btn, &QPushButton::clicked, ...)` into `QPushButton(text = "...") { ... }`.
3. **Android Edge-to-Edge & Responsive Portrait Adaptation**:
   - Ensure `QMainWindow` sidebars/docks collapse into a single column or `NavigationSuiteScaffold` when `session.isPortrait == true`, and keep all touch targets $\ge 48.0\text{dp}$ (`MIN_TOUCH_TARGET_DP`).

### 4. Validate & Deploy (Linux & Android)
1. Verify all unit and widget catalog tests pass:
   ```bash
   cargo test --workspace
   ```
2. Deploy and test on a connected Android phone via `adb` (Portrait + Landscape + Light/Dark themes):
   ```bash
   ./scripts/run_android_phone.sh
   ```
