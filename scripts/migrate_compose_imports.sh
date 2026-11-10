#!/usr/bin/env bash
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

set -euo pipefail

# ==============================================================================
#  🦖 Iguanodon Compose Porting Assistant (`migrate_compose_imports.sh`)
# ==============================================================================
# Rewrites standard Jetpack Compose / Compose Multiplatform UI imports
# (`androidx.compose.material3.*`, `androidx.compose.foundation.*`, `androidx.compose.ui.*`)
# in Kotlin `.kt` source files to `import iguanodon.compose.*` while keeping
# `androidx.compose.runtime.*` (state, remember, LaunchedEffect, Flow) untouched.
#
# Usage:
#   ./scripts/migrate_compose_imports.sh <path-to-your-compose-project> [--apply]
# ==============================================================================

if [[ $# -lt 1 ]]; then
    echo "Usage: $0 <target-compose-project-dir> [--apply]"
    echo "  (Without --apply, runs in dry-run preview mode)"
    exit 1
fi

TARGET_DIR="$1"
APPLY_MODE="${2:-}"

if [[ ! -d "${TARGET_DIR}" ]]; then
    echo "[!] Directory not found: ${TARGET_DIR}"
    exit 2
fi

echo "=================================================================="
echo " 🦖 Iguanodon Compose Multiplatform Porting Assistant"
echo " Target Project: ${TARGET_DIR}"
echo "=================================================================="

KT_FILES=()
while IFS= read -r -d '' f; do
    KT_FILES+=("$f")
done < <(find "${TARGET_DIR}" -type f -name "*.kt" -not -path "*/build/*" -print0)

echo "[*] Scanned ${#KT_FILES[@]} Kotlin source files."

MATCHED=0
for f in "${KT_FILES[@]}"; do
    if grep -Eq '^import androidx\.compose\.(material3|material|foundation|ui)\.' "$f"; then
        MATCHED=$((MATCHED + 1))
        echo "  -> $f"
        if [[ "${APPLY_MODE}" == "--apply" ]]; then
            python3 - "$f" << 'PY'
import re, sys
path = sys.argv[1]
with open(path, "r", encoding="utf-8") as fp:
    lines = fp.readlines()

out = []
added_iguanodon = False
for line in lines:
    if re.match(r"^import androidx\.compose\.(material3|material|foundation|ui)\b", line):
        if not added_iguanodon:
            out.append("import iguanodon.compose.*\n")
            out.append("import iguanodon.compose.compat.*\n")
            out.append("import iguanodon.compose.navigation3.*\n")
            added_iguanodon = True
    else:
        out.append(line)

with open(path, "w", encoding="utf-8") as fp:
    fp.writelines(out)
PY
        fi
    fi
done

if [[ "${APPLY_MODE}" == "--apply" ]]; then
    echo "[+] Migrated ${MATCHED} Kotlin files to 'import iguanodon.compose.*' (preserving androidx.compose.runtime.*)."
else
    echo "[*] Dry-run found ${MATCHED} Kotlin files with Compose UI imports."
    echo "    Re-run with '--apply' to rewrite imports in-place:"
    echo "    $0 \"${TARGET_DIR}\" --apply"
fi
