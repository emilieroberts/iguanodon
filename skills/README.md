# 🦖 Iguanodon AI Porting Skills (`skills/` & `.agents/skills/`)

This directory contains ready-to-use AI agent skills (compatible with Google Antigravity, Gemini CLI, Claude Code, Cursor, and Windsurf) for migrating existing **Jetpack Compose / Compose Multiplatform**, **Qt 5 / Qt 6**, and **GTK 3 / GTK 4 / Libadwaita** applications to **Iguanodon** targeting **Linux Desktop** and **Android**.

> **Note:** Both `skills/` (for easy human discovery on GitHub) and `.agents/skills/` (for automatic AI IDE discovery) contain the same skill bundles.

## Available Skills

| Skill | Instructions | Scanner & Automation Scripts | Mapping Reference |
| :--- | :--- | :--- | :--- |
| **[`compose-to-iguanodon`](./compose-to-iguanodon/SKILL.md)** | Port Jetpack Compose & Compose Multiplatform apps (including Navigation 3, `NavigationSuiteScaffold`, `NavigationBar`, `NavigationRail`, and `HorizontalSplitPane` / `VerticalSplitPane`) with optional build-time renderer selection (`-Piguanodon.renderer=iguanodon\|compose-ui`). | [`scan_compose_project.py`](./compose-to-iguanodon/scripts/scan_compose_project.py)<br>[`migrate_compose_imports.sh`](../scripts/migrate_compose_imports.sh) | [`compose-nav3-mapping.md`](./compose-to-iguanodon/references/compose-nav3-mapping.md) |
| **[`qt-to-iguanodon`](./qt-to-iguanodon/SKILL.md)** | Port Qt 5 / Qt 6 applications (`QWidgets` `.ui` XML, C++ signals/slots, and Qt Quick `.qml`) to Iguanodon via C++/CMake FFI or Kotlin Multiplatform (`QMainWindow`, `QSplitter`, `QDockWidget`, `QTabWidget`). | [`scan_qt_project.py`](./qt-to-iguanodon/scripts/scan_qt_project.py) | [`qt-widget-mapping.md`](./qt-to-iguanodon/references/qt-widget-mapping.md) |
| **[`gtk-to-iguanodon`](./gtk-to-iguanodon/SKILL.md)** | Port GTK 3, GTK 4, and GNOME Libadwaita applications (`GtkBuilder` `.ui` XML, Blueprint `.blp`, C/Rust/Python) to Iguanodon (`AdwApplicationWindow`, `AdwHeaderBar`, `AdwNavigationSplitView`, `AdwPreferencesPage`). | [`scan_gtk_project.py`](./gtk-to-iguanodon/scripts/scan_gtk_project.py) | [`gtk-widget-mapping.md`](./gtk-to-iguanodon/references/gtk-widget-mapping.md) |
