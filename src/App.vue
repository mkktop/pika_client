<script setup lang="ts">
import { onMounted, onUnmounted, computed, ref, watch, nextTick } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useAppStore } from './stores/app'
import { useComicStore } from './stores/comic'
import Sidebar from './components/Sidebar.vue'
import AppIcon from './components/AppIcon.vue'
import WindowControls from './components/WindowControls.vue'
import BrandMark from './components/BrandMark.vue'
import { desktopWindow, dragWindow, syncWindowTheme } from './utils/desktopWindow'
import { AUTH_EXPIRED_EVENT } from './utils/invoke'
const appStore = useAppStore()
const comicStore = useComicStore()
const route = useRoute()
const router = useRouter()
const ready = ref(false)
const query = ref('')
const main = ref<HTMLElement>()
// 阅读器和登录页都是全屏视图：不显示侧边栏和顶栏。
const isReader = computed(() => route.name === 'reader')
const isChromeHidden = computed(() => isReader.value || route.name === 'login')
const nativeWindow = !!desktopWindow()
const pageName = computed(
  () =>
    ({
      home: '发现',
      categories: '分类浏览',
      search: '搜索',
      favorites: '我的收藏',
      library: '本地书库',
      downloads: '下载管理',
      settings: '偏好设置',
      'comic-detail': '漫画详情'
    })[String(route.name)] || '发现'
)
watch(
  [() => appStore.theme, isChromeHidden],
  ([theme, hidden]) => syncWindowTheme(hidden && !isReader.value ? 'light' : isReader.value ? 'dark' : theme),
  { immediate: true }
)
// 列表页用 keep-alive 缓存；离开时记录滚动位置，返回时恢复而不是重置。
const LIST_VIEW_NAMES = new Set([
  'home',
  'categories',
  'search',
  'favorites',
  'library',
  'downloads'
])
const scrollPositions = new Map<string, number>()
watch(
  () => route.fullPath,
  async (_, from) => {
    const fromName = String(router.resolve(from).name || '')
    if (fromName && LIST_VIEW_NAMES.has(fromName)) {
      scrollPositions.set(from, main.value?.scrollTop ?? 0)
    }
    comicStore.cancelRequests()
    comicStore.fetchDownloadStates()
    query.value = String(route.query.q || '')
    await nextTick()
    const isListView = LIST_VIEW_NAMES.has(String(route.name || ''))
    const restore = isListView ? scrollPositions.get(route.fullPath) : undefined
    main.value?.scrollTo({ top: restore ?? 0, behavior: 'auto' })
  }
)
function search() {
  if (query.value.trim()) router.push({ path: '/search', query: { q: query.value.trim() } })
}
/** 会话失效：已在在线页面时引导去登录页，本地页面则只更新状态（侧边栏出现登录入口）。 */
function handleAuthExpired() {
  appStore.sessionExpired()
  if (route.meta.requiresAuth) {
    router.push({ path: '/login', query: { expired: '1', redirect: route.fullPath } })
  }
}
onMounted(async () => {
  window.addEventListener(AUTH_EXPIRED_EVENT, handleAuthExpired)
  await appStore.loadDownloadDir()
  comicStore.fetchDownloadStates()
  ready.value = true
})
onUnmounted(() => window.removeEventListener(AUTH_EXPIRED_EVENT, handleAuthExpired))
</script>
<template>
  <div
    class="app-container"
    :data-theme="appStore.theme"
    :class="{ 'compact-cards': appStore.compactCards }"
  >
    <header
      v-if="nativeWindow && (!ready || route.name === 'login')"
      class="standalone-windowbar"
      @mousedown="dragWindow"
    >
      <WindowControls />
    </header>
    <div v-if="!ready" class="app-start">
      <BrandMark /><span class="spinner" />
      <p>正在打开哔咔漫画</p>
    </div>
    <template v-else>
      <Sidebar v-if="!isChromeHidden" />
      <div
        class="workspace"
        :class="{ collapsed: appStore.sidebarCollapsed, fullscreen: isChromeHidden }"
      >
        <header v-if="!isChromeHidden" class="topbar" @mousedown="dragWindow">
          <div class="breadcrumb">
            <span>哔咔漫画</span><AppIcon name="chevron-right" :size="13" /><strong>{{
              pageName
            }}</strong>
          </div>
          <form class="global-search" @submit.prevent="search">
            <AppIcon name="search" :size="17" /><input
              v-model="query"
              aria-label="全局搜索"
              placeholder="搜索漫画、作者或关键词"
            /><kbd>↵</kbd>
          </form>
          <div class="topbar-actions">
            <button
              class="icon-btn"
              :title="appStore.theme === 'light' ? '切换深色外观' : '切换浅色外观'"
              @click="appStore.toggleTheme"
            >
              <AppIcon :name="appStore.theme === 'light' ? 'moon' : 'sun'" />
            </button>
          </div>
          <WindowControls />
        </header>
        <main ref="main" class="main-content" :class="{ fullscreen: isChromeHidden }">
          <router-view v-slot="{ Component }"
            ><transition name="fade" mode="out-in"
              ><keep-alive
                :include="[
                  'HomeView',
                  'CategoriesView',
                  'SearchView',
                  'FavoritesView',
                  'LibraryView',
                  'DownloadsView'
                ]"
                :max="12"
                ><component :is="Component" :key="route.path" /></keep-alive
              ></transition
          ></router-view>
        </main>
      </div>
    </template>
  </div>
</template>
