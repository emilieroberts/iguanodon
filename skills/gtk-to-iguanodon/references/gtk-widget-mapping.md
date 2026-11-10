# GTK 3 / GTK 4 & Libadwaita to Iguanodon Widget and Signal Mapping Reference

## 1. Windows, HeaderBars, Split Views & Navigation

| GTK 4 / Libadwaita Class | Iguanodon `@Composable` (`iguanodon.compose.*`) | Iguanodon Rust (`iguanodon_widgets`) | Notes |
| :--- | :--- | :--- | :--- |
| `AdwApplicationWindow` / `GtkApplicationWindow` | `AdwApplicationWindow(title, headerBar) { ... }` | `UnifiedWidgetKind::AppWindow` | Supports client-side header bars and Android Edge-to-Edge safe insets. |
| `AdwHeaderBar` / `GtkHeaderBar` | `AdwHeaderBar(title, startWidgets, endWidgets)` | `UnifiedWidgetKind::TopAppBar` | Translucent frosted-glass top bar. |
| `AdwNavigationSplitView` / `AdwOverlaySplitView` | `AdwNavigationSplitView(sidebar, content)` | `DraggableSplitPane` | Auto-collapses in Portrait mode on Android phones. |
| `AdwNavigationView` | `NavDisplay(backStack, entryProvider)` | `Navigation3Controller` | Maps `push` / `pop` to `backStack.add` / `backStack.removeLastOrNull`. |
| `AdwViewSwitcherBar` / `AdwViewSwitcher` | `NavigationSuiteScaffold` / `NavigationBar` | `Navigation3Controller` | Renders bottom `NavigationBar` in Portrait, `NavigationRail` in Landscape. |
| `GtkPaned` | `GtkPaned(horizontal, splitRatio, startChild, endChild)` | `DraggableSplitPane` | 48dp interactive draggable sash handle. |
| `AdwFlap` | `AdwFlap(flap, content, revealFlap)` / `DraggablePanel` | `DraggableFloatingPanel` | Draggable & dockable overlay/side panel. |
| `GtkNotebook` / `AdwTabView` + `AdwTabBar` | `TabRow` + `Tab` | `UnifiedWidgetKind::TabRow` | Tabbed workspace switching. |

## 2. Preferences Rows, Controls & Virtualized Data Views

| GTK 4 / Libadwaita Class | Iguanodon `@Composable` (`iguanodon.compose.*`) | Iguanodon Rust (`iguanodon_widgets`) |
| :--- | :--- | :--- |
| `AdwPreferencesPage` | `AdwPreferencesPage { ... }` | `UnifiedWidgetKind::Column` |
| `AdwPreferencesGroup` | `AdwPreferencesGroup(title, description) { ... }` | `UnifiedWidgetKind::GroupBox` |
| `AdwActionRow` | `AdwActionRow(title, subtitle, suffix)` | `UnifiedWidgetKind::Button` |
| `AdwSwitchRow` / `GtkSwitch` | `AdwSwitchRow(title, subtitle, active, onActiveChanged)` | `UnifiedWidgetKind::Switch` |
| `AdwEntryRow` / `GtkEntry` | `AdwEntryRow(title, text, onTextChanged)` | `TextEditState` |
| `AdwComboRow` / `GtkDropDown` | `AdwComboRow(title, model, selected, onSelectedChanged)` | `UnifiedWidgetKind::DropdownCombo` |
| `AdwExpanderRow` / `GtkExpander` | `ExpandableCard(title, expanded, onExpandedChange)` | `UnifiedWidgetKind::ExpanderCard` |
| `GtkButton` | `Button(label, onClick)` | `UnifiedWidgetKind::Button` |
| `GtkCheckButton` | `Checkbox(checked, onCheckedChange)` | `UnifiedWidgetKind::Checkbox` |
| `GtkScale` | `Slider(value, onValueChange, valueRange)` | `UnifiedWidgetKind::Slider` |
| `GtkSpinButton` / `AdwSpinRow` | `QSpinBox(value, range, onValueChanged)` | `UnifiedWidgetKind::SpinBox` |
| `GtkProgressBar` / `GtkLevelBar` | `ProgressBar(progress)` | `UnifiedWidgetKind::ProgressBar` |
| `GtkListView` / `GtkListBox` | `LazyColumn(items, showScrollbar = true)` | `LazyListLayout` (10,000+ items virtualized) |
| `GtkGridView` / `GtkFlowBox` | `LazyVerticalGrid(columns, items)` | `UnifiedWidgetKind::LazyGrid` |
| `GtkColumnView` / `GtkTreeView` | `DataTable(headers, rows)` / `TreeView(nodes)` | `UnifiedWidgetKind::TableView` |
| `GtkPopoverMenu` | `ContextMenuRegion(items, onSelectItem)` | `ContextMenuManager` (Z-ordered frosted glass) |

## 3. Style Manager (`AdwStyleManager`) Mapping

- `AdwStyleManager:dark` / `ADW_COLOR_SCHEME_PREFER_DARK` & `ADW_COLOR_SCHEME_FORCE_LIGHT` $\rightarrow$ `MaterialTheme(themeId = IguanodonThemeId.LinuxEliteObsidian, darkTheme = true / false)` (`ThemeMode::Dark` / `ThemeMode::Light`).
