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

//! Unified Cross-Toolkit Widget Catalog (`Compose` + `Qt 6` + `GTK 4 / Libadwaita`),
//! Compose Multiplatform **Navigation 3** (`NavBackStack`, `NavDisplay`, `entryProvider`),
//! Adaptive **`NavigationBar` & `NavigationRail`**, and **`DraggablePanel` / `DraggableSplitPane`**.

use iguanodon_core::{NodeId, Point, Rect, SpringValue};
use iguanodon_layout::MIN_TOUCH_TARGET_DP;
use iguanodon_shaders::{SdfPrimitiveInstance, ShaderMaterialId};
use iguanodon_theme::{InteractionShaderKind, ThemeDescriptor};

/// Canonical cross-toolkit widget taxonomy mapping **Jetpack Compose / Compose Multiplatform**,
/// **Qt 6 (`QWidgets` / `QML`)**, and **GTK 4 / Libadwaita** controls onto Iguanodon's
/// GPU-accelerated SDF + Flexbox scene graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnifiedWidgetKind {
    // 1. Application Shells, Windows & Scaffolds
    AppWindow,
    Scaffold,
    TopAppBar,
    BottomAppBar,
    StatusBar,
    MenuBar,
    ToolBar,

    // 2. Navigation Bars, Rails, Drawers, Stacks & Tabs
    NavigationBar,
    NavigationRail,
    NavigationDrawer,
    NavDisplay,
    TabRow,
    StackedView,
    BreadcrumbBar,

    // 3. Draggable Panels, Splitters, Docks & Sheets
    DraggablePanel,
    SplitPane,
    DockWidget,
    BottomSheet,
    ExpanderCard,
    ScrollArea,

    // 4. Layout Containers & Cards
    Row,
    Column,
    BoxOverlay,
    Grid,
    FlowBox,
    SurfaceCard,
    GroupBox,
    Divider,
    Spacer,

    // 5. Buttons, Toggles & Selection Controls (all >= 48dp touch targets)
    Button,
    FilledTonalButton,
    OutlinedButton,
    TextButton,
    IconButton,
    FloatingActionButton,
    SegmentedButton,
    Switch,
    Checkbox,
    RadioButton,
    Chip,

    // 6. Text, Numeric, Combo & Picker Inputs
    Label,
    TextField,
    MultilineTextEditor,
    SearchBar,
    PasswordField,
    SpinBox,
    Slider,
    RangeSlider,
    DialKnob,
    DropdownCombo,
    DateTimePicker,
    ColorPickerButton,

    // 7. Virtualized Data Views (Lists, Trees, Tables, Grids)
    LazyColumn,
    LazyRow,
    LazyGrid,
    TreeView,
    TableView,

    // 8. Feedback, Overlays, Menus, Dialogs & Media
    ProgressBar,
    CircularSpinner,
    LevelBar,
    Badge,
    ToastBanner,
    AlertDialog,
    ContextMenuPopup,
    Popover,
    Tooltip,
    ImageCanvas,
    PerfTelemetryHud,
}

impl UnifiedWidgetKind {
    /// Maps a Qt 5 / Qt 6 `QWidget` or `QML` class name to its canonical [`UnifiedWidgetKind`].
    pub fn from_qt_class(class_name: &str) -> Option<Self> {
        match class_name.trim() {
            "QMainWindow" | "QWidget" | "Window" | "ApplicationWindow" => Some(Self::AppWindow),
            "QDialog" | "Dialog" => Some(Self::AlertDialog),
            "QToolBar" | "ToolBar" => Some(Self::ToolBar),
            "QMenuBar" | "MenuBar" => Some(Self::MenuBar),
            "QStatusBar" => Some(Self::StatusBar),
            "QTabWidget" | "QTabBar" | "TabBar" => Some(Self::TabRow),
            "QStackedWidget" | "StackView" | "SwipeView" => Some(Self::StackedView),
            "QToolBox" => Some(Self::NavigationRail),
            "QSplitter" | "SplitView" => Some(Self::SplitPane),
            "QDockWidget" => Some(Self::DockWidget),
            "QMdiSubWindow" | "Frame" => Some(Self::DraggablePanel),
            "QScrollArea" | "ScrollView" | "Flickable" => Some(Self::ScrollArea),
            "QGroupBox" | "GroupBox" => Some(Self::GroupBox),
            "QHBoxLayout" | "RowLayout" | "Row" => Some(Self::Row),
            "QVBoxLayout" | "ColumnLayout" | "Column" => Some(Self::Column),
            "QGridLayout" | "GridLayout" | "Grid" => Some(Self::Grid),
            "QFormLayout" => Some(Self::Column),
            "QFrame" => Some(Self::SurfaceCard),
            "QPushButton" | "Button" | "QCommandLinkButton" => Some(Self::Button),
            "QToolButton" | "RoundButton" => Some(Self::IconButton),
            "QCheckBox" | "CheckBox" => Some(Self::Checkbox),
            "QSwitch" | "Switch" => Some(Self::Switch),
            "QRadioButton" | "RadioButton" => Some(Self::RadioButton),
            "QDialogButtonBox" => Some(Self::SegmentedButton),
            "QLabel" | "Text" | "QLCDNumber" => Some(Self::Label),
            "QLineEdit" | "TextField" => Some(Self::TextField),
            "QTextEdit" | "QPlainTextEdit" | "TextArea" => Some(Self::MultilineTextEditor),
            "QSpinBox" | "QDoubleSpinBox" | "SpinBox" => Some(Self::SpinBox),
            "QSlider" | "Slider" => Some(Self::Slider),
            "QDial" | "Dial" => Some(Self::DialKnob),
            "QComboBox" | "QFontComboBox" | "ComboBox" => Some(Self::DropdownCombo),
            "QDateTimeEdit" | "QDateEdit" | "QTimeEdit" | "QCalendarWidget" => {
                Some(Self::DateTimePicker)
            }
            "QColorDialog" => Some(Self::ColorPickerButton),
            "QListView" | "QListWidget" | "ListView" => Some(Self::LazyColumn),
            "QTreeView" | "QTreeWidget" | "TreeView" => Some(Self::TreeView),
            "QTableView" | "QTableWidget" | "TableView" | "QColumnView" => Some(Self::TableView),
            "QProgressBar" | "ProgressBar" => Some(Self::ProgressBar),
            "QBusyIndicator" | "BusyIndicator" => Some(Self::CircularSpinner),
            "QMenu" | "Menu" => Some(Self::ContextMenuPopup),
            "QMessageBox" => Some(Self::AlertDialog),
            "QToolTip" | "ToolTip" => Some(Self::Tooltip),
            "QGraphicsView" | "Image" => Some(Self::ImageCanvas),
            _ => None,
        }
    }

    /// Maps a GTK 3 / GTK 4 / Libadwaita class name to its canonical [`UnifiedWidgetKind`].
    pub fn from_gtk_class(class_name: &str) -> Option<Self> {
        match class_name.trim() {
            "GtkWindow" | "GtkApplicationWindow" | "AdwWindow" | "AdwApplicationWindow" => {
                Some(Self::AppWindow)
            }
            "AdwToolbarView" => Some(Self::Scaffold),
            "GtkHeaderBar" | "AdwHeaderBar" => Some(Self::TopAppBar),
            "GtkActionBar" => Some(Self::BottomAppBar),
            "GtkStatusbar" => Some(Self::StatusBar),
            "GtkPopoverMenuBar" => Some(Self::MenuBar),
            "AdwViewSwitcherBar" => Some(Self::NavigationBar),
            "GtkStackSidebar" | "AdwNavigationSplitView" | "AdwOverlaySplitView" => {
                Some(Self::NavigationRail)
            }
            "AdwNavigationView" => Some(Self::NavDisplay),
            "GtkNotebook" | "AdwTabBar" | "AdwTabView" | "GtkStackSwitcher"
            | "AdwViewSwitcher" => Some(Self::TabRow),
            "GtkStack" | "AdwCarousel" => Some(Self::StackedView),
            "GtkPaned" => Some(Self::SplitPane),
            "AdwFlap" => Some(Self::DraggablePanel),
            "AdwBottomSheet" => Some(Self::BottomSheet),
            "GtkExpander" | "GtkRevealer" | "AdwExpanderRow" => Some(Self::ExpanderCard),
            "GtkScrolledWindow" | "GtkViewport" => Some(Self::ScrollArea),
            "GtkBox" => Some(Self::Column),
            "GtkOverlay" | "GtkFixed" => Some(Self::BoxOverlay),
            "GtkGrid" => Some(Self::Grid),
            "GtkFlowBox" => Some(Self::FlowBox),
            "GtkFrame" | "AdwBin" | "AdwClamp" => Some(Self::SurfaceCard),
            "AdwPreferencesGroup" | "AdwPreferencesPage" => Some(Self::GroupBox),
            "GtkSeparator" => Some(Self::Divider),
            "GtkButton" | "AdwActionRow" | "AdwButtonRow" => Some(Self::Button),
            "GtkToggleButton" => Some(Self::FilledTonalButton),
            "GtkLinkButton" => Some(Self::TextButton),
            "GtkMenuButton" => Some(Self::IconButton),
            "GtkSwitch" | "AdwSwitchRow" => Some(Self::Switch),
            "GtkCheckButton" => Some(Self::Checkbox),
            "GtkLabel" | "AdwStatusPage" => Some(Self::Label),
            "GtkEntry" | "AdwEntryRow" => Some(Self::TextField),
            "GtkSearchEntry" | "GtkSearchBar" => Some(Self::SearchBar),
            "GtkPasswordEntry" | "AdwPasswordEntryRow" => Some(Self::PasswordField),
            "GtkTextView" => Some(Self::MultilineTextEditor),
            "GtkSpinButton" | "AdwSpinRow" => Some(Self::SpinBox),
            "GtkScale" | "GtkScaleButton" => Some(Self::Slider),
            "GtkDropDown" | "GtkComboBox" | "GtkComboBoxText" | "AdwComboRow" => {
                Some(Self::DropdownCombo)
            }
            "GtkCalendar" => Some(Self::DateTimePicker),
            "GtkColorDialogButton" | "GtkColorButton" => Some(Self::ColorPickerButton),
            "GtkListView" | "GtkListBox" => Some(Self::LazyColumn),
            "GtkGridView" => Some(Self::LazyGrid),
            "GtkColumnView" | "GtkTreeView" => Some(Self::TableView),
            "GtkProgressBar" => Some(Self::ProgressBar),
            "GtkSpinner" | "AdwSpinner" => Some(Self::CircularSpinner),
            "GtkLevelBar" => Some(Self::LevelBar),
            "AdwToastOverlay" | "AdwBanner" | "GtkInfoBar" => Some(Self::ToastBanner),
            "GtkDialog" | "GtkAlertDialog" | "AdwAlertDialog" | "AdwMessageDialog" => {
                Some(Self::AlertDialog)
            }
            "GtkPopoverMenu" => Some(Self::ContextMenuPopup),
            "GtkPopover" => Some(Self::Popover),
            "GtkPicture" | "GtkImage" | "GtkDrawingArea" => Some(Self::ImageCanvas),
            _ => None,
        }
    }

    /// Maps a Jetpack Compose / Compose Multiplatform `@Composable` name to [`UnifiedWidgetKind`].
    pub fn from_compose_name(name: &str) -> Option<Self> {
        match name.trim() {
            "Scaffold" | "NavigationSuiteScaffold" => Some(Self::Scaffold),
            "TopAppBar" | "CenterAlignedTopAppBar" | "MediumTopAppBar" | "LargeTopAppBar" => {
                Some(Self::TopAppBar)
            }
            "BottomAppBar" => Some(Self::BottomAppBar),
            "NavigationBar" => Some(Self::NavigationBar),
            "NavigationRail" => Some(Self::NavigationRail),
            "ModalNavigationDrawer" | "PermanentNavigationDrawer" | "DismissibleNavigationDrawer" => {
                Some(Self::NavigationDrawer)
            }
            "NavDisplay" | "NavHost" => Some(Self::NavDisplay),
            "TabRow" | "ScrollableTabRow" | "PrimaryTabRow" | "SecondaryTabRow" => {
                Some(Self::TabRow)
            }
            "DraggablePanel" => Some(Self::DraggablePanel),
            "SplitPane" | "HorizontalSplitPane" | "VerticalSplitPane" => Some(Self::SplitPane),
            "ModalBottomSheet" | "BottomSheetScaffold" => Some(Self::BottomSheet),
            "Row" => Some(Self::Row),
            "Column" => Some(Self::Column),
            "Box" | "BoxWithConstraints" => Some(Self::BoxOverlay),
            "Surface" | "Card" | "ElevatedCard" | "OutlinedCard" => Some(Self::SurfaceCard),
            "HorizontalDivider" | "VerticalDivider" | "Divider" => Some(Self::Divider),
            "Spacer" => Some(Self::Spacer),
            "Button" | "ElevatedButton" => Some(Self::Button),
            "FilledTonalButton" => Some(Self::FilledTonalButton),
            "OutlinedButton" => Some(Self::OutlinedButton),
            "TextButton" => Some(Self::TextButton),
            "IconButton" | "FilledIconButton" => Some(Self::IconButton),
            "FloatingActionButton" | "ExtendedFloatingActionButton" | "SmallFloatingActionButton" => {
                Some(Self::FloatingActionButton)
            }
            "SingleChoiceSegmentedButtonRow" | "MultiChoiceSegmentedButtonRow" => {
                Some(Self::SegmentedButton)
            }
            "Switch" => Some(Self::Switch),
            "Checkbox" | "TriStateCheckbox" => Some(Self::Checkbox),
            "RadioButton" => Some(Self::RadioButton),
            "FilterChip" | "AssistChip" | "InputChip" | "SuggestionChip" => Some(Self::Chip),
            "Text" | "Label" => Some(Self::Label),
            "TextField" | "OutlinedTextField" | "BasicTextField" => Some(Self::TextField),
            "SearchBar" | "DockedSearchBar" => Some(Self::SearchBar),
            "ScrollableText" => Some(Self::MultilineTextEditor),
            "Slider" => Some(Self::Slider),
            "RangeSlider" => Some(Self::RangeSlider),
            "ExposedDropdownMenuBox" | "DropdownMenu" => Some(Self::DropdownCombo),
            "DatePicker" | "TimePicker" => Some(Self::DateTimePicker),
            "LazyColumn" => Some(Self::LazyColumn),
            "LazyRow" => Some(Self::LazyRow),
            "LazyVerticalGrid" | "LazyHorizontalGrid" => Some(Self::LazyGrid),
            "ProgressBar" | "LinearProgressIndicator" => Some(Self::ProgressBar),
            "CircularProgressIndicator" => Some(Self::CircularSpinner),
            "Badge" | "BadgedBox" => Some(Self::Badge),
            "SnackbarHost" | "Snackbar" => Some(Self::ToastBanner),
            "AlertDialog" => Some(Self::AlertDialog),
            "ContextMenuRegion" => Some(Self::ContextMenuPopup),
            "TooltipBox" | "PlainTooltip" | "RichTooltip" => Some(Self::Tooltip),
            "Image" | "Canvas" => Some(Self::ImageCanvas),
            "PerfOverlay" => Some(Self::PerfTelemetryHud),
            _ => None,
        }
    }
}

// ============================================================================
// Compose Multiplatform Navigation 3 (`NavBackStack`, `NavDisplay`, `NavEntry`)
// + Adaptive `NavigationBar` (Portrait) & `NavigationRail` (Landscape)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationItemSpec {
    pub route_key: String,
    pub label: String,
    pub badge_count: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavTransitionDirection {
    ForwardPush,
    BackwardPop,
    TopLevelReplace,
}

/// Rust-side controller backing **Compose Multiplatform Navigation 3**
/// (`rememberNavBackStack` + `NavDisplay` + `entryProvider`) together with
/// adaptive **`NavigationBar`** (Portrait) and **`NavigationRail`** (Landscape).
pub struct Navigation3Controller {
    pub back_stack: Vec<String>,
    pub nav_items: Vec<NavigationItemSpec>,
    pub last_direction: NavTransitionDirection,
    pub transition_spring: SpringValue,
    pub active_pill_spring: SpringValue,
}

impl Navigation3Controller {
    pub fn new(start_key: impl Into<String>, nav_items: Vec<NavigationItemSpec>) -> Self {
        let start = start_key.into();
        let initial_idx = nav_items
            .iter()
            .position(|item| item.route_key == start)
            .unwrap_or(0) as f32;
        let motion = ThemeDescriptor::default().motion;
        Self {
            back_stack: vec![start],
            nav_items,
            last_direction: NavTransitionDirection::TopLevelReplace,
            transition_spring: SpringValue::new(1.0, motion.default_spatial),
            active_pill_spring: SpringValue::new(initial_idx, motion.fast_spatial),
        }
    }

    /// Returns the active destination key at the top of the Navigation 3 back stack.
    pub fn current_key(&self) -> &str {
        self.back_stack
            .last()
            .map(|s| s.as_str())
            .unwrap_or("home")
    }

    pub fn can_go_back(&self) -> bool {
        self.back_stack.len() > 1
    }

    /// Equivalent to `backStack.add(key)` in Compose Navigation 3.
    pub fn push(&mut self, route_key: impl Into<String>) {
        let key = route_key.into();
        self.back_stack.push(key.clone());
        self.last_direction = NavTransitionDirection::ForwardPush;
        self.transition_spring.value = 0.0;
        self.transition_spring.set_target(1.0);
        self.sync_pill_target(&key);
    }

    /// Equivalent to `backStack.removeLastOrNull()` in Compose Navigation 3 (`onBack`).
    pub fn pop(&mut self) -> Option<String> {
        if self.back_stack.len() <= 1 {
            return None;
        }
        let popped = self.back_stack.pop();
        self.last_direction = NavTransitionDirection::BackwardPop;
        self.transition_spring.value = 0.0;
        self.transition_spring.set_target(1.0);
        let cur = self.current_key().to_string();
        self.sync_pill_target(&cur);
        popped
    }

    /// Selects a top-level destination from a `NavigationBar` or `NavigationRail`.
    pub fn select_top_level(&mut self, index: usize) {
        if let Some(spec) = self.nav_items.get(index) {
            let key = spec.route_key.clone();
            self.back_stack.clear();
            self.back_stack.push(key);
            self.last_direction = NavTransitionDirection::TopLevelReplace;
            self.transition_spring.value = 0.0;
            self.transition_spring.set_target(1.0);
            self.active_pill_spring.set_target(index as f32);
        }
    }

    fn sync_pill_target(&mut self, key: &str) {
        if let Some(idx) = self.nav_items.iter().position(|i| i.route_key == key) {
            self.active_pill_spring.set_target(idx as f32);
        }
    }

    pub fn step(&mut self, dt: f32) {
        self.transition_spring.step(dt);
        self.active_pill_spring.step(dt);
    }

    /// Computes the bounds of the `NavigationRail` (Landscape left side, width >= 84dp)
    /// or `NavigationBar` (Portrait bottom bar, height >= 64dp) and each item's >= 48dp hit target.
    pub fn compute_bar_or_rail_geometry(
        &self,
        safe_viewport: Rect,
    ) -> (Rect, Vec<Rect>, Rect) {
        let count = self.nav_items.len().max(1) as f32;
        let is_portrait = safe_viewport.width < safe_viewport.height;
        if is_portrait {
            // Bottom NavigationBar (Android Portrait)
            let bar_h = 64.0_f32.max(MIN_TOUCH_TARGET_DP);
            let bar_rect = Rect::new(
                safe_viewport.x,
                safe_viewport.y + safe_viewport.height - bar_h,
                safe_viewport.width,
                bar_h,
            );
            let item_w = (bar_rect.width / count).max(MIN_TOUCH_TARGET_DP);
            let mut items = Vec::with_capacity(self.nav_items.len());
            for i in 0..self.nav_items.len() {
                items.push(Rect::new(
                    bar_rect.x + (i as f32) * item_w,
                    bar_rect.y,
                    item_w,
                    bar_h,
                ));
            }
            let content_rect = Rect::new(
                safe_viewport.x,
                safe_viewport.y,
                safe_viewport.width,
                (safe_viewport.height - bar_h - 8.0).max(MIN_TOUCH_TARGET_DP),
            );
            (bar_rect, items, content_rect)
        } else {
            // Left Side NavigationRail (Linux Desktop / Android Landscape)
            let rail_w = 88.0_f32.max(MIN_TOUCH_TARGET_DP);
            let rail_rect = Rect::new(
                safe_viewport.x,
                safe_viewport.y,
                rail_w,
                safe_viewport.height,
            );
            let item_h = 56.0_f32.max(MIN_TOUCH_TARGET_DP);
            let mut items = Vec::with_capacity(self.nav_items.len());
            for i in 0..self.nav_items.len() {
                items.push(Rect::new(
                    rail_rect.x + 6.0,
                    rail_rect.y + 12.0 + (i as f32) * (item_h + 8.0),
                    rail_w - 12.0,
                    item_h,
                ));
            }
            let content_rect = Rect::new(
                safe_viewport.x + rail_w + 12.0,
                safe_viewport.y,
                (safe_viewport.width - rail_w - 12.0).max(MIN_TOUCH_TARGET_DP),
                safe_viewport.height,
            );
            (rail_rect, items, content_rect)
        }
    }

    /// Emits `SdfPrimitiveInstance`s for the active `NavigationBar` or `NavigationRail` and its
    /// spring-animated active pill indicator.
    pub fn build_sdf_instances(
        &self,
        safe_viewport: Rect,
        theme: &ThemeDescriptor,
    ) -> Vec<SdfPrimitiveInstance> {
        let (container_rect, item_rects, _) = self.compute_bar_or_rail_geometry(safe_viewport);
        let mut out = Vec::with_capacity(item_rects.len() + 2);
        let p = theme.shapes.shape_exponent_p;
        let mat = match theme.interaction_shader {
            InteractionShaderKind::ObsidianNeonPulse => ShaderMaterialId::ObsidianNeonPulse,
            _ => ShaderMaterialId::FrostedGlass,
        };

        out.push(SdfPrimitiveInstance {
            rect: container_rect.to_array(),
            corner_radii: [theme.shapes.radius_lg; 4],
            clip_rect: safe_viewport.to_array(),
            fill_color_start: theme.colors.surface_container.to_array(),
            fill_color_end: theme.colors.surface.to_array(),
            border_color: theme.colors.outline.to_array(),
            shadow_color: [0.0, 0.0, 0.0, 0.45],
            params: [theme.shapes.border_width, 10.0, p, mat as u32 as f32],
        });

        if !item_rects.is_empty() {
            let max_idx = (item_rects.len() - 1) as f32;
            let pos = self.active_pill_spring.value.clamp(0.0, max_idx);
            let i0 = pos.floor() as usize;
            let i1 = (i0 + 1).min(item_rects.len() - 1);
            let frac = pos - (i0 as f32);
            let r0 = item_rects[i0];
            let r1 = item_rects[i1];
            let pill_rect = [
                r0.x + (r1.x - r0.x) * frac + 4.0,
                r0.y + (r1.y - r0.y) * frac + 4.0,
                r0.width - 8.0,
                r0.height - 8.0,
            ];
            out.push(SdfPrimitiveInstance {
                rect: pill_rect,
                corner_radii: [theme.shapes.radius_md; 4],
                clip_rect: container_rect.to_array(),
                fill_color_start: theme.colors.primary_container.to_array(),
                fill_color_end: theme.colors.primary.with_alpha(0.65).to_array(),
                border_color: theme.colors.accent_glow.to_array(),
                shadow_color: [0.0; 4],
                params: [theme.shapes.border_width, 4.0, p, mat as u32 as f32],
            });
        }

        out
    }
}

// ============================================================================
// Draggable SplitPane (`QSplitter` / `GtkPaned` / Compose `SplitPane`) &
// Draggable Dock Panel (`QDockWidget` / `AdwFlap` / Compose `DraggablePanel`)
// ============================================================================

#[derive(Debug, Clone)]
pub struct DraggableSplitPane {
    pub id: NodeId,
    pub is_horizontal: bool,
    pub split_ratio: f32,
    pub min_ratio: f32,
    pub max_ratio: f32,
    pub dragging_sash: bool,
}

impl DraggableSplitPane {
    pub fn new(id: NodeId, is_horizontal: bool, initial_ratio: f32) -> Self {
        Self {
            id,
            is_horizontal,
            split_ratio: initial_ratio.clamp(0.15, 0.85),
            min_ratio: 0.18,
            max_ratio: 0.82,
            dragging_sash: false,
        }
    }

    /// Computes `(first_pane_rect, sash_hit_rect_48dp, second_pane_rect)`.
    pub fn compute_panes(&self, bounds: Rect) -> (Rect, Rect, Rect) {
        let handle_thickness = MIN_TOUCH_TARGET_DP; // 48dp interactive touch/drag target!
        let visual_gap = 12.0_f32;
        if self.is_horizontal {
            let split_x = bounds.x + bounds.width * self.split_ratio;
            let first = Rect::new(
                bounds.x,
                bounds.y,
                (split_x - bounds.x - visual_gap * 0.5).max(MIN_TOUCH_TARGET_DP),
                bounds.height,
            );
            let second_x = split_x + visual_gap * 0.5;
            let second = Rect::new(
                second_x,
                bounds.y,
                (bounds.x + bounds.width - second_x).max(MIN_TOUCH_TARGET_DP),
                bounds.height,
            );
            let sash_hit = Rect::new(
                split_x - handle_thickness * 0.5,
                bounds.y,
                handle_thickness,
                bounds.height.max(MIN_TOUCH_TARGET_DP),
            );
            (first, sash_hit, second)
        } else {
            let split_y = bounds.y + bounds.height * self.split_ratio;
            let first = Rect::new(
                bounds.x,
                bounds.y,
                bounds.width,
                (split_y - bounds.y - visual_gap * 0.5).max(MIN_TOUCH_TARGET_DP),
            );
            let second_y = split_y + visual_gap * 0.5;
            let second = Rect::new(
                bounds.x,
                second_y,
                bounds.width,
                (bounds.y + bounds.height - second_y).max(MIN_TOUCH_TARGET_DP),
            );
            let sash_hit = Rect::new(
                bounds.x,
                split_y - handle_thickness * 0.5,
                bounds.width.max(MIN_TOUCH_TARGET_DP),
                handle_thickness,
            );
            (first, sash_hit, second)
        }
    }

    pub fn on_pointer_down(&mut self, bounds: Rect, pt: Point) -> bool {
        let (_, sash_hit, _) = self.compute_panes(bounds);
        if sash_hit.contains(pt) {
            self.dragging_sash = true;
            return true;
        }
        false
    }

    pub fn on_pointer_move(&mut self, bounds: Rect, pt: Point) -> bool {
        if !self.dragging_sash {
            return false;
        }
        let raw = if self.is_horizontal {
            (pt.x - bounds.x) / bounds.width.max(1.0)
        } else {
            (pt.y - bounds.y) / bounds.height.max(1.0)
        };
        self.split_ratio = raw.clamp(self.min_ratio, self.max_ratio);
        true
    }

    pub fn on_pointer_up(&mut self) {
        self.dragging_sash = false;
    }
}

/// Floating or dockable Draggable Panel (`QDockWidget` / `AdwFlap` / Compose `DraggablePanel`).
#[derive(Debug, Clone)]
pub struct DraggableFloatingPanel {
    pub id: NodeId,
    pub title: String,
    pub bounds: Rect,
    pub is_dragging: bool,
    pub is_collapsed: bool,
    pub is_docked_left: bool,
    drag_grab_offset: Point,
}

impl DraggableFloatingPanel {
    pub fn new(id: NodeId, title: impl Into<String>, initial_bounds: Rect) -> Self {
        Self {
            id,
            title: title.into(),
            bounds: Rect::new(
                initial_bounds.x,
                initial_bounds.y,
                initial_bounds.width.max(180.0),
                initial_bounds.height.max(MIN_TOUCH_TARGET_DP * 2.0),
            ),
            is_dragging: false,
            is_collapsed: false,
            is_docked_left: false,
            drag_grab_offset: Point::new(0.0, 0.0),
        }
    }

    /// 48dp-tall draggable titlebar header rect.
    pub fn header_rect(&self) -> Rect {
        Rect::new(
            self.bounds.x,
            self.bounds.y,
            self.bounds.width,
            MIN_TOUCH_TARGET_DP,
        )
    }

    pub fn on_pointer_down(&mut self, pt: Point) -> bool {
        if self.header_rect().contains(pt) {
            self.is_dragging = true;
            self.drag_grab_offset = Point::new(pt.x - self.bounds.x, pt.y - self.bounds.y);
            return true;
        }
        false
    }

    pub fn on_pointer_move(&mut self, viewport: Rect, pt: Point) -> bool {
        if !self.is_dragging {
            return false;
        }
        let max_x = (viewport.x + viewport.width - self.bounds.width).max(viewport.x);
        let max_y = (viewport.y + viewport.height - MIN_TOUCH_TARGET_DP).max(viewport.y);
        self.bounds.x = (pt.x - self.drag_grab_offset.x).clamp(viewport.x, max_x);
        self.bounds.y = (pt.y - self.drag_grab_offset.y).clamp(viewport.y, max_y);
        // Snap-to-dock if dragged within 16dp of the left edge
        self.is_docked_left = (self.bounds.x - viewport.x).abs() < 16.0;
        if self.is_docked_left {
            self.bounds.x = viewport.x;
        }
        true
    }

    pub fn on_pointer_up(&mut self) {
        self.is_dragging = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qt_gtk_compose_widget_catalog_mappings() {
        // Verify Qt 6 widget & QML mappings
        assert_eq!(
            UnifiedWidgetKind::from_qt_class("QMainWindow"),
            Some(UnifiedWidgetKind::AppWindow)
        );
        assert_eq!(
            UnifiedWidgetKind::from_qt_class("QSplitter"),
            Some(UnifiedWidgetKind::SplitPane)
        );
        assert_eq!(
            UnifiedWidgetKind::from_qt_class("QDockWidget"),
            Some(UnifiedWidgetKind::DockWidget)
        );
        assert_eq!(
            UnifiedWidgetKind::from_qt_class("QTreeView"),
            Some(UnifiedWidgetKind::TreeView)
        );

        // Verify GTK 4 & Libadwaita mappings
        assert_eq!(
            UnifiedWidgetKind::from_gtk_class("AdwApplicationWindow"),
            Some(UnifiedWidgetKind::AppWindow)
        );
        assert_eq!(
            UnifiedWidgetKind::from_gtk_class("AdwNavigationSplitView"),
            Some(UnifiedWidgetKind::NavigationRail)
        );
        assert_eq!(
            UnifiedWidgetKind::from_gtk_class("AdwNavigationView"),
            Some(UnifiedWidgetKind::NavDisplay)
        );
        assert_eq!(
            UnifiedWidgetKind::from_gtk_class("GtkPaned"),
            Some(UnifiedWidgetKind::SplitPane)
        );
        assert_eq!(
            UnifiedWidgetKind::from_gtk_class("AdwFlap"),
            Some(UnifiedWidgetKind::DraggablePanel)
        );

        // Verify Compose & Navigation 3 mappings
        assert_eq!(
            UnifiedWidgetKind::from_compose_name("NavDisplay"),
            Some(UnifiedWidgetKind::NavDisplay)
        );
        assert_eq!(
            UnifiedWidgetKind::from_compose_name("NavigationRail"),
            Some(UnifiedWidgetKind::NavigationRail)
        );
        assert_eq!(
            UnifiedWidgetKind::from_compose_name("NavigationBar"),
            Some(UnifiedWidgetKind::NavigationBar)
        );
        assert_eq!(
            UnifiedWidgetKind::from_compose_name("DraggablePanel"),
            Some(UnifiedWidgetKind::DraggablePanel)
        );
    }

    #[test]
    fn test_navigation3_backstack_and_adaptive_rail_vs_bar() {
        let items = vec![
            NavigationItemSpec {
                route_key: "home".into(),
                label: "Home".into(),
                badge_count: None,
            },
            NavigationItemSpec {
                route_key: "analytics".into(),
                label: "Analytics".into(),
                badge_count: Some(3),
            },
            NavigationItemSpec {
                route_key: "settings".into(),
                label: "Settings".into(),
                badge_count: None,
            },
        ];
        let mut nav3 = Navigation3Controller::new("home", items);
        assert_eq!(nav3.current_key(), "home");
        assert!(!nav3.can_go_back());

        // Push detail route (Compose Navigation 3 `backStack.add(DetailKey("42"))`)
        nav3.push("detail/42");
        assert_eq!(nav3.current_key(), "detail/42");
        assert!(nav3.can_go_back());

        // Pop detail route (`backStack.removeLastOrNull()`)
        assert_eq!(nav3.pop().as_deref(), Some("detail/42"));
        assert_eq!(nav3.current_key(), "home");

        // Switch top-level route via NavigationRail / NavigationBar
        nav3.select_top_level(1);
        assert_eq!(nav3.current_key(), "analytics");

        // Landscape viewport -> Side NavigationRail
        let (rail_rect, rail_items, _) =
            nav3.compute_bar_or_rail_geometry(Rect::new(0.0, 0.0, 1280.0, 800.0));
        assert!(rail_rect.width >= MIN_TOUCH_TARGET_DP);
        assert_eq!(rail_rect.height, 800.0);
        for item_r in rail_items {
            assert!(item_r.width >= MIN_TOUCH_TARGET_DP && item_r.height >= MIN_TOUCH_TARGET_DP);
        }

        // Portrait viewport -> Bottom NavigationBar
        let (bar_rect, bar_items, _) =
            nav3.compute_bar_or_rail_geometry(Rect::new(0.0, 0.0, 600.0, 1280.0));
        assert_eq!(bar_rect.width, 600.0);
        assert!(bar_rect.height >= MIN_TOUCH_TARGET_DP);
        for item_r in bar_items {
            assert!(item_r.width >= MIN_TOUCH_TARGET_DP && item_r.height >= MIN_TOUCH_TARGET_DP);
        }
    }

    #[test]
    fn test_draggable_split_pane_and_floating_dock_panel() {
        let bounds = Rect::new(0.0, 0.0, 1000.0, 600.0);
        let mut split = DraggableSplitPane::new(NodeId::from_raw(500), true, 0.30);
        let (left, sash, right) = split.compute_panes(bounds);
        assert!(sash.width >= MIN_TOUCH_TARGET_DP);
        assert!(left.width > 250.0 && right.width > 650.0);

        // Grab sash at x=300 and drag to x=550
        assert!(split.on_pointer_down(bounds, Point::new(300.0, 300.0)));
        assert!(split.on_pointer_move(bounds, Point::new(550.0, 300.0)));
        split.on_pointer_up();
        assert!((split.split_ratio - 0.55).abs() < 0.01);

        // Drag floating panel to left edge -> snaps to docked state
        let mut panel = DraggableFloatingPanel::new(
            NodeId::from_raw(501),
            "Inspector Dock",
            Rect::new(200.0, 100.0, 280.0, 360.0),
        );
        assert!(panel.on_pointer_down(Point::new(250.0, 120.0)));
        assert!(panel.on_pointer_move(bounds, Point::new(55.0, 180.0)));
        panel.on_pointer_up();
        assert!(panel.is_docked_left);
        assert_eq!(panel.bounds.x, 0.0);
    }
}
