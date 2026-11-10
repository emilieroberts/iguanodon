#!/usr/bin/env python3
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

"""
Scans a GTK 3 / GTK 4 / Libadwaita project (.ui XML, Blueprint .blp, C, Rust gtk4-rs, Python),
inventories all Gtk* and Adw* widgets, and generates a starter Iguanodon @Composable scaffold
for Linux Desktop and Android.
"""
import argparse
import os
import re
import xml.etree.ElementTree as ET
from collections import Counter

GTK_TO_IGUANODON = {
    "AdwApplicationWindow": "AdwApplicationWindow",
    "GtkApplicationWindow": "AdwApplicationWindow",
    "GtkWindow": "AdwApplicationWindow",
    "AdwHeaderBar": "AdwHeaderBar",
    "GtkHeaderBar": "AdwHeaderBar",
    "AdwNavigationSplitView": "AdwNavigationSplitView",
    "AdwOverlaySplitView": "AdwNavigationSplitView",
    "AdwNavigationView": "NavDisplay",
    "AdwViewSwitcherBar": "NavigationSuiteScaffold",
    "GtkPaned": "GtkPaned",
    "AdwFlap": "AdwFlap",
    "GtkNotebook": "TabRow",
    "AdwTabView": "TabRow",
    "GtkStack": "NavDisplay",
    "AdwPreferencesPage": "AdwPreferencesPage",
    "AdwPreferencesGroup": "AdwPreferencesGroup",
    "AdwActionRow": "AdwActionRow",
    "AdwSwitchRow": "AdwSwitchRow",
    "AdwEntryRow": "AdwEntryRow",
    "AdwComboRow": "AdwComboRow",
    "AdwExpanderRow": "ExpandableCard",
    "GtkBox": "Column",
    "GtkGrid": "LazyVerticalGrid",
    "GtkFlowBox": "LazyVerticalGrid",
    "GtkScrolledWindow": "ScrollableText",
    "GtkButton": "Button",
    "GtkToggleButton": "FilledTonalButton",
    "GtkMenuButton": "IconButton",
    "GtkSwitch": "Switch",
    "GtkCheckButton": "Checkbox",
    "GtkLabel": "Text",
    "GtkEntry": "TextField",
    "GtkSearchEntry": "SearchBar",
    "GtkTextView": "ScrollableText",
    "GtkSpinButton": "QSpinBox",
    "GtkScale": "Slider",
    "GtkDropDown": "QComboBox",
    "GtkProgressBar": "ProgressBar",
    "GtkLevelBar": "ProgressBar",
    "GtkListView": "LazyColumn",
    "GtkListBox": "LazyColumn",
    "GtkColumnView": "DataTable",
    "GtkPopoverMenu": "ContextMenuRegion",
}


def scan_gtk_ui_file(path: str, counts: Counter) -> None:
    try:
        tree = ET.parse(path)
        for elem in tree.iter():
            cls = elem.attrib.get("class")
            if cls:
                counts[cls] += 1
    except Exception:
        pass


def scan_source_file(path: str, counts: Counter, signals: list[str]) -> None:
    try:
        with open(path, "r", encoding="utf-8", errors="ignore") as f:
            text = f.read()
        for gtk_cls in GTK_TO_IGUANODON:
            matches = len(re.findall(r"\b" + re.escape(gtk_cls) + r"\b", text))
            if matches:
                counts[gtk_cls] += matches
        for m in re.finditer(r"(?:g_signal_connect|connect_\w+)\s*\(([^)]+)\)", text):
            signals.append(m.group(1).strip())
    except Exception:
        pass


def main() -> None:
    parser = argparse.ArgumentParser(description="Scan GTK/Libadwaita project and generate Iguanodon port scaffold.")
    parser.add_argument("project_dir", help="Path to source GTK/Libadwaita project")
    parser.add_argument("--out", default="", help="Optional output file for generated Kotlin scaffold")
    args = parser.parse_args()

    counts: Counter = Counter()
    signals: list[str] = []
    ui_files, blp_files, code_files = [], [], []

    for root, _, files in os.walk(args.project_dir):
        for f in files:
            full = os.path.join(root, f)
            if f.endswith(".ui"):
                ui_files.append(full)
                scan_gtk_ui_file(full, counts)
            elif f.endswith(".blp"):
                blp_files.append(full)
                scan_source_file(full, counts, signals)
            elif f.endswith((".c", ".h", ".rs", ".py", ".vala")):
                code_files.append(full)
                scan_source_file(full, counts, signals)

    print("==================================================================")
    print(f" 🦖 Iguanodon GTK / Libadwaita Porting Scanner — {args.project_dir}")
    print("==================================================================")
    print(f"  .ui files: {len(ui_files)} | .blp files: {len(blp_files)} | Code files: {len(code_files)}")
    print("  Detected GTK / Adw Widgets -> Iguanodon Targets:")
    for cls, cnt in counts.most_common():
        target = GTK_TO_IGUANODON.get(cls, "CustomWidget")
        print(f"    - {cls:24s} (x{cnt})  -->  {target}")
    print(f"  Detected GTK signal bindings: {len(signals)}")

    if args.out:
        os.makedirs(os.path.dirname(os.path.abspath(args.out)), exist_ok=True)
        with open(args.out, "w", encoding="utf-8") as out_f:
            out_f.write(
                """package ported.gtk.app

import androidx.compose.runtime.*
import iguanodon.compose.*

@Composable
fun PortedGtkApplication() {
    var darkTheme by remember { mutableStateOf(true) }
    var notificationsEnabled by remember { mutableStateOf(true) }

    MaterialTheme(themeId = IguanodonThemeId.LinuxEliteObsidian, darkTheme = darkTheme) {
        AdwApplicationWindow(
            title = "Ported GTK4 / Libadwaita App on Iguanodon",
            headerBar = {
                AdwHeaderBar(
                    title = "Ported GTK4 / Libadwaita App",
                    endWidgets = {
                        Button(label = if (darkTheme) "Mode: Dark" else "Mode: Light") {
                            darkTheme = !darkTheme
                        }
                    },
                )
            },
        ) {
            AdwNavigationSplitView(
                sidebar = {
                    LazyColumn(items = listOf("General", "Appearance", "Vulkan GPU")) { _, section ->
                        Text(section)
                    }
                },
                content = {
                    AdwPreferencesPage {
                        AdwPreferencesGroup(
                            title = "Appearance & Shaders",
                            description = "Rendered by Iguanodon Vulkan SDF Engine on Linux & Android",
                        ) {
                            AdwSwitchRow(
                                title = "Dark Polarity Mode",
                                subtitle = "Switch between Dark Obsidian and Light Solar Alabaster",
                                active = darkTheme,
                                onActiveChanged = { darkTheme = it },
                            )
                            AdwSwitchRow(
                                title = "Enable GPU Notifications",
                                active = notificationsEnabled,
                                onActiveChanged = { notificationsEnabled = it },
                            )
                        }
                    }
                },
            )
        }
    }
}
"""
            )
        print(f"[+] Wrote starter Iguanodon GTK/Libadwaita port scaffold to: {args.out}")


if __name__ == "__main__":
    main()
