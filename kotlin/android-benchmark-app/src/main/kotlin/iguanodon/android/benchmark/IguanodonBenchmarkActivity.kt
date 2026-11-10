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

package iguanodon.android.benchmark

import android.app.Activity
import android.graphics.Color
import android.os.Build
import android.os.Bundle
import android.view.Window
import android.view.WindowManager
import iguanodon.compose.android.IguanodonVulkanSurfaceView
import iguanodon.compose.sample.StandardBenchmarkApp

/**
 * Android host `Activity` for the Iguanodon Standard Benchmark Sample App.
 *
 * Configures full Android Edge-to-Edge display (drawing behind transparent system bars
 * and camera punch-hole cutouts) and supports both Portrait (single-column) and Landscape
 * (3-column) orientations using the shared `@Composable StandardBenchmarkApp()` UX code.
 */
class IguanodonBenchmarkActivity : Activity() {
    private var surfaceView: IguanodonVulkanSurfaceView? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        requestWindowFeature(Window.FEATURE_NO_TITLE)

        // Enable Android Edge-to-Edge rendering behind status bar, camera cutout, and navigation bar
        if (Build.VERSION.SDK_INT >= 30) {
            window.setDecorFitsSystemWindows(false)
            val lp = window.attributes
            lp.layoutInDisplayCutoutMode =
                WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_ALWAYS
            window.attributes = lp
        } else if (Build.VERSION.SDK_INT >= 28) {
            val lp = window.attributes
            lp.layoutInDisplayCutoutMode =
                WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES
            window.attributes = lp
        }
        @Suppress("DEPRECATION")
        window.statusBarColor = Color.TRANSPARENT
        @Suppress("DEPRECATION")
        window.navigationBarColor = Color.TRANSPARENT

        val view = IguanodonVulkanSurfaceView(this) {
            StandardBenchmarkApp()
        }
        surfaceView = view
        setContentView(view)
    }

    override fun onDestroy() {
        surfaceView?.releaseEngine()
        surfaceView = null
        super.onDestroy()
    }
}
