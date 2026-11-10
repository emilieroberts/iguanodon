# Qt 5 / Qt 6 to Iguanodon Widget, Layout, and Signal/Slot Mapping Reference

## 1. Windows, Docks, Splitters & Navigation Containers

| Qt `QWidget` / `QML` Type | Iguanodon `@Composable` (`iguanodon.compose.*`) | Iguanodon Rust (`iguanodon_widgets`) | Notes |
| :--- | :--- | :--- | :--- |
| `QMainWindow` / `ApplicationWindow` | `QMainWindow(...)` / `Scaffold(...)` | `UnifiedWidgetKind::AppWindow` | Supports `menuBar`, `toolBar`, `dockWidgets`, `statusBar`, and `centralWidget`. |
| `QSplitter` / `SplitView` | `QSplitter(...)` / `HorizontalSplitPane` / `VerticalSplitPane` | `DraggableSplitPane` | Interactive 48dp sash handle with `[min_ratio, max_ratio]` clamping. |
| `QDockWidget` / `Drawer` | `QDockWidget(title)` / `DraggablePanel(title)` | `DraggableFloatingPanel` | Draggable 48dp header with left-edge dock snapping. |
| `QTabWidget` / `QTabBar` / `TabBar` | `QTabWidget(...)` / `TabRow` + `Tab` | `UnifiedWidgetKind::TabRow` | Maps `currentIndex` and `currentChanged(int)` to state. |
| `QStackedWidget` / `StackView` | `NavDisplay(...)` (`Navigation3.kt`) | `Navigation3Controller` | Maps `push()` / `pop()` onto Navigation 3 `rememberNavBackStack`. |
| `QToolBox` | `NavigationSuiteScaffold` | `Navigation3Controller` | Renders `NavigationRail` in Landscape and `NavigationBar` in Portrait. |
| `QGroupBox` / `GroupBox` | `QGroupBox(title) { ... }` / `Card` | `UnifiedWidgetKind::GroupBox` | Translucent SDF card with title header. |
| `QScrollArea` / `Flickable` | `ScrollableText` / `LazyColumn` | `ScrollbarMetrics` | Includes proportional interactive scrollbar thumb ($\ge 48\text{dp}$). |

## 2. Controls, Inputs & Item Views

| Qt `QWidget` / `QML` Type | Iguanodon `@Composable` (`iguanodon.compose.*`) | Iguanodon Rust (`iguanodon_widgets`) |
| :--- | :--- | :--- |
| `QLabel` / `Text` | `Label(text)` / `Text(text)` | `UnifiedWidgetKind::Label` |
| `QPushButton` / `Button` | `QPushButton(text, onClicked)` / `Button` | `UnifiedWidgetKind::Button` |
| `QToolButton` / `RoundButton` | `IconButton(onClick) { ... }` | `UnifiedWidgetKind::IconButton` |
| `QCheckBox` / `CheckBox` | `QCheckBox(text, checked, onToggled)` / `Checkbox` | `UnifiedWidgetKind::Checkbox` |
| `QRadioButton` / `RadioButton` | `RadioButton(selected, onClick)` | `UnifiedWidgetKind::RadioButton` |
| `QLineEdit` / `TextField` | `QLineEdit(text, placeholderText, onTextChanged)` | `TextEditState` |
| `QTextEdit` / `QPlainTextEdit` / `TextArea` | `ScrollableText` + `TextField` | `MultilineTextLayout` |
| `QSpinBox` / `QDoubleSpinBox` | `QSpinBox(value, range, onValueChanged)` | `UnifiedWidgetKind::SpinBox` |
| `QSlider` / `Slider` | `Slider(value, onValueChange, valueRange)` | `UnifiedWidgetKind::Slider` |
| `QComboBox` / `ComboBox` | `QComboBox(items, currentIndex, onCurrentIndexChanged)` | `UnifiedWidgetKind::DropdownCombo` |
| `QProgressBar` / `ProgressBar` | `QProgressBar(value, minimum, maximum)` / `ProgressBar` | `UnifiedWidgetKind::ProgressBar` |
| `QListView` / `QListWidget` / `ListView` | `LazyColumn(items, showScrollbar = true)` | `LazyListLayout` (O(1) virtualized) |
| `QTreeView` / `QTreeWidget` / `TreeView` | `TreeView(nodes) { idx, node -> ... }` | `UnifiedWidgetKind::TreeView` |
| `QTableView` / `QTableWidget` / `TableView` | `DataTable(headers, rows) { idx, row -> ... }` | `UnifiedWidgetKind::TableView` |
| `QMenu` / `contextMenuEvent` | `ContextMenuRegion(items, onSelectItem)` | `ContextMenuManager` (Z-ordered frosted glass) |

## 3. Signal / Slot & Theme Polarity Mapping

- `QGuiApplication::styleHints()->colorScheme()` (`Qt::ColorScheme::Dark` / `Qt::ColorScheme::Light`) $\rightarrow$ `MaterialTheme(themeId = ..., darkTheme = true / false)` (`ThemeMode::Dark` / `ThemeMode::Light`).
- `QObject::connect(sender, &Sender::signal, receiver, &Receiver::slot)` $\rightarrow$ unidirectional state + event lambda (`onClicked = { viewModel.onAction() }`).
