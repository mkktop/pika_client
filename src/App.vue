<script setup lang="ts">
import { onMounted, computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useAppStore } from './stores/app'
import { useComicStore } from './stores/comic'
import Sidebar from './components/Sidebar.vue'
import AppIcon from './components/AppIcon.vue'
import LoginView from './views/LoginView.vue'
import WindowControls from './components/WindowControls.vue'
import BrandMark from './components/BrandMark.vue'
import { desktopWindow, dragWindow, syncWindowTheme } from './utils/desktopWindow'
const appStore = useAppStore()
const comicStore = useComicStore()
const route = useRoute()
const router = useRouter()
const ready = ref(false)
const query = ref('')
const main = ref<HTMLElement>()
const isReader = computed(() => route.name === 'reader')
const nativeWindow = !!desktopWindow()
watch(
  [() => appStore.theme, isReader],
  ([theme, reader]) => syncWindowTheme(reader ? 'dark' : theme),
  { immediate: true }
)
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
  () => route.fullPath,
  () => {
    comicStore.cancelRequests()
    main.value?.scrollTo({ top: 0, behavior: 'instant' })
    query.value = String(route.query.q || '')
  }
)
function search() {
  if (query.value.trim()) router.push({ path: '/search', query: { q: query.value.trim() } })
}
onMounted(async () => {
  await appStore.checkLogin()
  ready.value = true
  if (appStore.isLoggedIn) await appStore.loadDownloadDir()
})
</script>
<template>
  <div
    class="app-container"
    :data-theme="appStore.theme"
    :class="{ 'compact-cards': appStore.compactCards }"
  >
    <header
      v-if="nativeWindow && (!ready || !appStore.isLoggedIn)"
      class="standalone-windowbar"
      @mousedown="dragWindow"
    >
      <WindowControls />
    </header>
    <div v-if="!ready" class="app-start">
      <BrandMark /><span class="spinner" />
      <p>正在打开哔咔漫画</p>
    </div>
    <LoginView v-else-if="!appStore.isLoggedIn" />
    <template v-else>
      <Sidebar v-if="!isReader" />
      <div
        class="workspace"
        :class="{ collapsed: appStore.sidebarCollapsed, fullscreen: isReader }"
      >
        <header v-if="!isReader" class="topbar" @mousedown="dragWindow">
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
        <main ref="main" class="main-content" :class="{ fullscreen: isReader }">
          <router-view v-slot="{ Component }"
            ><transition name="fade" mode="out-in"
              ><component :is="Component" :key="route.path" /></transition
          ></router-view>
        </main>
      </div>
    </template>
  </div>
</template>
