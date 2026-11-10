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
Scans a Qt 5 / Qt 6 project (.ui XML, .qml, and C++ headers/sources), inventories all
Qt widgets and signals/slots, and generates a starter Iguanodon @Composable scaffold
for Linux Desktop and Android.
"""
import argparse
import os
import re
import xml.etree.ElementTree as ET
from collections import Counter

QT_TO_IGUANODON = {
    "QMainWindow": "QMainWindow",
    "QWidget": "Surface",
    "QDialog": "AlertDialog",
    "QSplitter": "QSplitter",
    "QDockWidget": "QDockWidget",
    "QTabWidget": "QTabWidget",
    "QStackedWidget": "NavDisplay",
    "QGroupBox": "QGroupBox",
    "QHBoxLayout": "Row",
    "QVBoxLayout": "Column",
    "QGridLayout": "LazyVerticalGrid",
    "QPushButton": "QPushButton",
    "QToolButton": "IconButton",
    "QCheckBox": "QCheckBox",
    "QRadioButton": "RadioButton",
    "QLabel": "Text",
    "QLineEdit": "QLineEdit",
    "QTextEdit": "ScrollableText",
    "QPlainTextEdit": "ScrollableText",
    "QSpinBox": "QSpinBox",
    "QDoubleSpinBox": "QSpinBox",
    "QSlider": "Slider",
    "QComboBox": "QComboBox",
    "QProgressBar": "QProgressBar",
    "QListView": "LazyColumn",
    "QListWidget": "LazyColumn",
    "QTreeView": "TreeView",
    "QTreeWidget": "TreeView",
    "QTableView": "DataTable",
    "QTableWidget": "DataTable",
    "QMenu": "ContextMenuRegion",
}


def scan_ui_file(path: str, counts: Counter) -> None:
    try:
        tree = ET.parse(path)
        for elem in tree.iter():
            cls = elem.attrib.get("class")
            if cls:
                counts[cls] += 1
    except Exception:
        pass


def scan_code_file(path: str, counts: Counter, connects: list[str]) -> None:
    try:
        with open(path, "r", encoding="utf-8", errors="ignore") as f:
            text = f.read()
        for qt_cls in QT_TO_IGUANODON:
            matches = len(re.findall(r"\b" + re.escape(qt_cls) + r"\b", text))
            if matches:
                counts[qt_cls] += matches
        for m in re.finditer(r"connect\s*\(([^;]+)\)", text):
            connects.append(m.group(1).strip())
    except Exception:
        pass


def main() -> None:
    parser = argparse.ArgumentParser(description="Scan Qt project and generate Iguanodon port scaffold.")
    parser.add_argument("project_dir", help="Path to source Qt project")
    parser.add_argument("--out", default="", help="Optional output file for generated Kotlin scaffold")
    args = parser.parse_args()

    counts: Counter = Counter()
    connects: list[str] = []
    ui_files, qml_files, cpp_files = [], [], []

    for root, _, files in os.walk(args.project_dir):
        for f in files:
            full = os.path.join(root, f)
            if f.endswith(".ui"):
                ui_files.append(full)
                scan_ui_file(full, counts)
            elif f.endswith(".qml"):
                qml_files.append(full)
                scan_code_file(full, counts, connects)
            elif f.endswith((".h", ".hpp", ".cpp", ".cc")):
                cpp_files.append(full)
                scan_code_file(full, counts, connects)

    print("==================================================================")
    print(f" 🦖 Iguanodon Qt Porting Scanner — {args.project_dir}")
    print("==================================================================")
    print(f"  .ui files: {len(ui_files)} | .qml files: {len(qml_files)} | C++ files: {len(cpp_files)}")
    print("  Detected Qt Widgets -> Iguanodon Targets:")
    for cls, cnt in counts.most_common():
        target = QT_TO_IGUANODON.get(cls, "CustomWidget")
        print(f"    - {cls:18s} (x{cnt})  -->  {target}")
    print(f"  Detected QObject::connect signal/slot bindings: {len(connects)}")

    if args.out:
        os.makedirs(os.path.dirname(os.path.abspath(args.out)), exist_ok=True)
        with open(args.out, "w", encoding="utf-8") as out_f:
            out_f.write(
                """package ported.qt.app

import androidx.compose.runtime.*
import iguanodon.compose.*

@Composable
fun PortedQtMainWindow() {
    var darkTheme by remember { mutableStateOf(true) }
    var splitRatio by remember { mutableStateOf(0.32f) }

    MaterialTheme(themeId = IguanodonThemeId.LinuxEliteObsidian, darkTheme = darkTheme) {
        QMainWindow(
            windowTitle = "Ported Qt Application on Iguanodon (Linux & Android)",
            toolBar = {
                Row {
                    QPushButton(text = if (darkTheme) "Mode: Dark" else "Mode: Light") {
                        darkTheme = !darkTheme
                    }
                }
            },
            dockWidgets = {
                QDockWidget(title = "Inspector Dock") {
                    Text("Docked / Floating Panel")
                }
            },
        ) {
            QSplitter(
                horizontal = true,
                splitRatio = splitRatio,
                first = {
                    LazyColumn(items = List(100) { "Qt Model Row #$it" }) { _, item ->
                        Text(item)
                    }
                },
                second = {
                    QGroupBox(title = "Main Workspace") {
                        Text("Ported from Qt to Iguanodon Vulkan UI")
                    }
                },
            )
        }
    }
}
"""
            )
        print(f"[+] Wrote starter Iguanodon Qt port scaffold to: {args.out}")


if __name__ == "__main__":
    main()
