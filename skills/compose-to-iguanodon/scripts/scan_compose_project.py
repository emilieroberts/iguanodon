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
Scans an Android Jetpack Compose or Compose Multiplatform project, reporting:
  - @Composable functions
  - Compose UI imports that can be auto-rewritten by migrate_compose_imports.sh
  - Navigation 2 (NavHost) vs Navigation 3 (NavDisplay / rememberNavBackStack) usage
  - NavigationBar, NavigationRail, NavigationSuiteScaffold, and SplitPane usage
"""
import argparse
import os
import re
from collections import Counter

TRACKED_COMPOSABLES = [
    "MaterialTheme",
    "Scaffold",
    "NavigationSuiteScaffold",
    "NavigationBar",
    "ScrollableNavigationBar",
    "NavigationBarItem",
    "NavigationRail",
    "NavigationRailItem",
    "ModalNavigationDrawer",
    "PermanentNavigationDrawer",
    "ScrollableChipRow",
    "HorizontalActionRow",
    "HorizontalSplitPane",
    "VerticalSplitPane",
    "DraggablePanel",
    "FilterChip",
    "AssistChip",
    "resolveIguanodonColorScheme",
    "NavDisplay",
    "rememberNavBackStack",
    "entryProvider",
    "NavHost",
    "rememberNavController",
    "TopAppBar",
    "BottomAppBar",
    "TabRow",
    "ScrollableTabRow",
    "Row",
    "Column",
    "Box",
    "Surface",
    "Card",
    "ElevatedCard",
    "OutlinedCard",
    "LazyColumn",
    "LazyRow",
    "LazyVerticalGrid",
    "Button",
    "OutlinedButton",
    "FilledTonalButton",
    "TextButton",
    "IconButton",
    "FloatingActionButton",
    "Text",
    "TextField",
    "OutlinedTextField",
    "SearchBar",
    "Switch",
    "Checkbox",
    "RadioButton",
    "Slider",
    "LinearProgressIndicator",
    "CircularProgressIndicator",
    "DropdownMenu",
    "AlertDialog",
    "ModalBottomSheet",
]


def main() -> None:
    parser = argparse.ArgumentParser(description="Scan Compose / Compose Multiplatform project for Iguanodon porting.")
    parser.add_argument("project_dir", help="Path to source Compose project")
    args = parser.parse_args()

    kt_files = []
    composable_defs = 0
    ui_import_files = 0
    usage_counts: Counter = Counter()

    for root, _, files in os.walk(args.project_dir):
        if "/build/" in root:
            continue
        for f in files:
            if f.endswith(".kt"):
                full = os.path.join(root, f)
                kt_files.append(full)
                try:
                    with open(full, "r", encoding="utf-8", errors="ignore") as fp:
                        text = fp.read()
                    composable_defs += len(re.findall(r"@Composable\b", text))
                    if re.search(r"^import androidx\.compose\.(material3|material|foundation|ui)\b", text, re.M):
                        ui_import_files += 1
                    for comp in TRACKED_COMPOSABLES:
                        c = len(re.findall(r"\b" + re.escape(comp) + r"\b", text))
                        if c:
                            usage_counts[comp] += c
                except Exception:
                    pass

    print("==================================================================")
    print(f" 🦖 Iguanodon Compose Multiplatform Scanner — {args.project_dir}")
    print("==================================================================")
    print(f"  Kotlin (.kt) files scanned:          {len(kt_files)}")
    print(f"  @Composable functions detected:      {composable_defs}")
    print(f"  Files with Compose UI imports:       {ui_import_files}")
    print("  Detected Composable & Navigation 3 Elements:")
    for comp, cnt in usage_counts.most_common():
        print(f"    - {comp:28s} (x{cnt})  -->  Supported in iguanodon.compose.*")
    if usage_counts["NavHost"] > 0 and usage_counts["NavDisplay"] == 0:
        print("\n  [!] Note: Project uses Navigation 2 (NavHost). Upgrade to Navigation 3")
        print("      (rememberNavBackStack + NavDisplay + entryProvider) per references/compose-nav3-mapping.md.")
    if usage_counts["NavigationBarItem"] >= 5 and usage_counts["ScrollableNavigationBar"] == 0:
        print("\n  [!] Tip: 5+ NavigationBarItem instances detected. Pass scrollableBottomBar = true")
        print("      to NavigationSuiteScaffold or use ScrollableNavigationBar to prevent portrait tab clipping.")
    if (usage_counts["FilterChip"] + usage_counts["AssistChip"]) >= 3 and usage_counts["ScrollableChipRow"] == 0:
        print("\n  [!] Tip: Multiple FilterChip/AssistChip instances detected. Wrap chip rows in")
        print("      ScrollableChipRow (or HorizontalActionRow) so chips never wrap 1-char-per-line in portrait.")


if __name__ == "__main__":
    main()
