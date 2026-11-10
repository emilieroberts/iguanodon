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

package iguanodon.compose

import iguanodon.compose.bridge.IguanodonSession
import iguanodon.compose.sample.Navigation3ComplexAppSample
import iguanodon.compose.sample.StandardBenchmarkApp
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class IguanodonApplierTest {
    @Test
    fun testPureRuntimeApplierMountsStandardBenchmarkAndNavigation3Apps() {
        val session = IguanodonSession(initialWidth = 1180f, initialHeight = 760f)
        val host = IguanodonCompositionHost(session)
        try {
            assertEquals(IguanodonPowerMode.Performance, session.powerMode)
            assertEquals(120, session.targetFps)
            assertEquals(IguanodonRendererBackend.Iguanodon, session.rendererBackend)

            host.setContent {
                StandardBenchmarkApp()
            }
            host.doFrame(16_000_000L)
            assertTrue(
                host.rootNode.children.isNotEmpty(),
                "IguanodonApplier rootNode should contain mounted child nodes for StandardBenchmarkApp",
            )

            // Toggle Power Mode (120 FPS -> 30 FPS -> 120 FPS) and Renderer Backend (Iguanodon -> Compose-UI -> Iguanodon)
            assertEquals(IguanodonPowerMode.PowerSaving, session.togglePowerMode())
            assertEquals(30, session.targetFps)
            assertEquals(33, session.targetFrameIntervalMs)
            assertEquals(IguanodonPowerMode.Performance, session.togglePowerMode())

            assertEquals(IguanodonRendererBackend.ComposeUi, session.toggleRendererBackend())
            assertEquals(IguanodonRendererBackend.Iguanodon, session.toggleRendererBackend())

            // Switch to Navigation3ComplexAppSample and run a frame
            host.setContent {
                Navigation3ComplexAppSample()
            }
            host.doFrame(32_000_000L)
            assertTrue(
                host.rootNode.children.isNotEmpty(),
                "IguanodonApplier rootNode should contain mounted child nodes for Navigation3ComplexAppSample",
            )

            // Simulate pointer move/click and poll reverse events
            session.onPointerMove(120f, 120f)
            session.onPointerDown(120f, 120f, false)
            session.onPointerUp(120f, 120f)
            session.pollEvents()
            host.doFrame(48_000_000L)
        } finally {
            host.dispose()
            session.close()
        }
    }
}
