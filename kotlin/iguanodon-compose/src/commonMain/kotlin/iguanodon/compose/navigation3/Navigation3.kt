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

import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.snapshots.SnapshotStateList
import kotlin.reflect.KClass

/**
 * Full implementation of **Compose Multiplatform Navigation 3**
 * (`https://kotlinlang.org/docs/multiplatform/compose-navigation-3.html`)
 * powered by Iguanodon's 120Hz spring-animated Vulkan scene graph.
 *
 * Supports:
 * - [`NavKey`]: Marker interface for `@Serializable` route keys (`data object HomeKey : NavKey`, `data class DetailKey(val id: String) : NavKey`).
 * - [`rememberNavBackStack`]: Creates and remembers an observable [`NavBackStack`] state list.
 * - [`entryProvider`] & [`EntryProviderBuilder.entry`]: Type-safe DSL mapping route keys to `@Composable` screen lambdas.
 * - [`NavDisplay`]: Observes the back stack and renders the active destination (or two-pane master-detail split via [`TwoPaneSplitSceneStrategy`]).
 */
interface NavKey

/**
 * Observable Navigation 3 back-stack list backed by Compose's `SnapshotStateList`.
 * Push routes via `backStack.add(key)` and pop via `backStack.removeLastOrNull()`.
 */
class NavBackStack<T : Any>(
    initialElements: List<T>,
) : MutableList<T> {
    private val stateList: SnapshotStateList<T> =
        mutableStateListOf<T>().apply { addAll(initialElements) }

    val currentKey: T?
        get() = stateList.lastOrNull()

    val canGoBack: Boolean
        get() = stateList.size > 1

    fun removeLastOrNull(): T? =
        if (stateList.size > 1) stateList.removeAt(stateList.lastIndex) else null

    fun replaceTopLevel(rootKey: T) {
        stateList.clear()
        stateList.add(rootKey)
    }

    override val size: Int get() = stateList.size
    override fun contains(element: T): Boolean = stateList.contains(element)
    override fun containsAll(elements: Collection<T>): Boolean = stateList.containsAll(elements)
    override fun get(index: Int): T = stateList[index]
    override fun indexOf(element: T): Int = stateList.indexOf(element)
    override fun isEmpty(): Boolean = stateList.isEmpty()
    override fun iterator(): MutableIterator<T> = stateList.iterator()
    override fun lastIndexOf(element: T): Int = stateList.lastIndexOf(element)
    override fun add(element: T): Boolean = stateList.add(element)
    override fun add(index: Int, element: T) = stateList.add(index, element)
    override fun addAll(index: Int, elements: Collection<T>): Boolean = stateList.addAll(index, elements)
    override fun addAll(elements: Collection<T>): Boolean = stateList.addAll(elements)
    override fun clear() = stateList.clear()
    override fun listIterator(): MutableListIterator<T> = stateList.listIterator()
    override fun listIterator(index: Int): MutableListIterator<T> = stateList.listIterator(index)
    override fun remove(element: T): Boolean = stateList.remove(element)
    override fun removeAll(elements: Collection<T>): Boolean = stateList.removeAll(elements)
    override fun removeAt(index: Int): T = stateList.removeAt(index)
    override fun retainAll(elements: Collection<T>): Boolean = stateList.retainAll(elements)
    override fun set(index: Int, element: T): T = stateList.set(index, element)
    override fun subList(fromIndex: Int, toIndex: Int): MutableList<T> =
        stateList.subList(fromIndex, toIndex)
}

/**
 * Remembers a Navigation 3 back stack initialized with [elements].
 */
@Composable
fun <T : Any> rememberNavBackStack(vararg elements: T): NavBackStack<T> =
    remember { NavBackStack(elements.toList()) }

/**
 * Represents a single resolved destination entry in Navigation 3.
 */
class NavEntry<T : Any>(
    val key: T,
    val metadata: Map<String, Any> = emptyMap(),
    val content: @Composable (T) -> Unit,
)

/**
 * DSL builder for `entryProvider { entry<HomeKey> { ... } }`.
 */
class EntryProviderBuilder<T : Any>(
    private val fallback: (T) -> NavEntry<T>,
) {
    @PublishedApi
    internal val providers = mutableMapOf<KClass<out T>, (T) -> NavEntry<T>>()
    private val predicateProviders = mutableListOf<Pair<(T) -> Boolean, (T) -> NavEntry<T>>>()

    inline fun <reified K : T> entry(
        metadata: Map<String, Any> = emptyMap(),
        noinline content: @Composable (K) -> Unit,
    ) {
        providers[K::class] = { rawKey ->
            @Suppress("UNCHECKED_CAST")
            val typedKey = rawKey as K
            NavEntry(
                key = rawKey,
                metadata = metadata,
                content = { content(typedKey) },
            )
        }
    }

    fun <K : T> addEntryProvider(
        clazz: KClass<K>,
        metadata: Map<String, Any> = emptyMap(),
        content: @Composable (K) -> Unit,
    ) {
        providers[clazz] = { rawKey ->
            @Suppress("UNCHECKED_CAST")
            val typedKey = rawKey as K
            NavEntry(key = rawKey, metadata = metadata, content = { content(typedKey) })
        }
    }

    fun entryForPredicate(
        predicate: (T) -> Boolean,
        metadata: Map<String, Any> = emptyMap(),
        content: @Composable (T) -> Unit,
    ) {
        predicateProviders.add(
            predicate to { rawKey -> NavEntry(key = rawKey, metadata = metadata, content = content) },
        )
    }

    internal fun build(): (T) -> NavEntry<T> = { key ->
        val direct = providers[key::class]
        if (direct != null) {
            direct(key)
        } else {
            val matched = predicateProviders.firstOrNull { it.first(key) }?.second
            matched?.invoke(key) ?: fallback(key)
        }
    }
}

/**
 * Creates a Navigation 3 `entryProvider` function mapping keys to [`NavEntry`] instances.
 */
fun <T : Any> entryProvider(
    fallback: (T) -> NavEntry<T> = { key ->
        NavEntry(key) { Label("Unknown Navigation 3 Key: $key") }
    },
    builder: EntryProviderBuilder<T>.() -> Unit,
): (T) -> NavEntry<T> = EntryProviderBuilder(fallback).apply(builder).build()

/**
 * Scene layout strategy for [`NavDisplay`] (e.g., single-pane vs. two-pane split master-detail).
 */
interface SceneStrategy<T : Any> {
    @Composable
    fun RenderScene(
        entries: List<NavEntry<T>>,
        modifier: Modifier,
    )
}

class SinglePaneSceneStrategy<T : Any> : SceneStrategy<T> {
    @Composable
    override fun RenderScene(entries: List<NavEntry<T>>, modifier: Modifier) {
        val current = entries.lastOrNull() ?: return
        Box(modifier = modifier) {
            current.content(current.key)
        }
    }
}

/**
 * Adaptive Two-Pane Scene Strategy for Navigation 3 on Linux Desktop & Android Tablets/Foldables:
 * Renders the previous destination (e.g. list/master) and current destination (e.g. detail)
 * side-by-side inside a [`HorizontalSplitPane`] when in Landscape mode (`!isPortrait`) and at least
 * 2 entries are on the back stack; falls back to single-pane in Portrait mode.
 */
class TwoPaneSplitSceneStrategy<T : Any>(
    private val initialSplitRatio: Float = 0.38f,
) : SceneStrategy<T> {
    @Composable
    override fun RenderScene(entries: List<NavEntry<T>>, modifier: Modifier) {
        val session = LocalIguanodonSession.current
        val isPortrait = session?.isPortrait == true
        if (!isPortrait && entries.size >= 2) {
            val primary = entries[entries.size - 2]
            val detail = entries[entries.size - 1]
            HorizontalSplitPane(
                splitRatio = initialSplitRatio,
                modifier = modifier,
                first = { primary.content(primary.key) },
                second = { detail.content(detail.key) },
            )
        } else {
            val current = entries.lastOrNull() ?: return
            Box(modifier = modifier) {
                current.content(current.key)
            }
        }
    }
}

/**
 * Navigation 3 [`NavDisplay`] composable that observes [backStack] and renders the active
 * destination(s) using [entryProvider] and [sceneStrategy].
 */
@Composable
fun <T : Any> NavDisplay(
    backStack: List<T>,
    modifier: Modifier = Modifier,
    onBack: () -> Unit = {
        if (backStack is MutableList<*> && backStack.size > 1) {
            backStack.removeAt(backStack.size - 1)
        }
    },
    sceneStrategy: SceneStrategy<T> = SinglePaneSceneStrategy(),
    entryProvider: (T) -> NavEntry<T>,
) {
    if (backStack.isEmpty()) return
    val resolvedEntries = backStack.map { entryProvider(it) }
    sceneStrategy.RenderScene(resolvedEntries, modifier)
}
