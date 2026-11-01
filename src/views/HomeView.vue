<script setup lang="ts">
import { onMounted, computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useComicStore } from '../stores/comic'
import { useAppStore } from '../stores/app'
import ComicCard from '../components/ComicCard.vue'
import AppIcon from '../components/AppIcon.vue'
import ComicSkeleton from '../components/ComicSkeleton.vue'
import EmptyState from '../components/EmptyState.vue'
import Paginator from '../components/Paginator.vue'
const router = useRouter()

defineOptions({ name: 'HomeView' })
const store = useComicStore()
const app = useAppStore()
type Tab = 'leaderboard' | 'latest' | 'random'
const activeTab = ref<Tab>('leaderboard')
const resultsTop = ref<HTMLElement>()
const tabs: { value: Tab; label: string; icon: string; en: string }[] = [
  { value: 'leaderboard', label: '热门排行', icon: 'trend', en: 'TRENDING NOW' },
  { value: 'latest', label: '最新更新', icon: 'clock', en: 'FRESH UPDATES' },
  { value: 'random', label: '随机本子', icon: 'shuffle', en: 'RANDOM PICKS' }
]
const periods = [
  { value: 'H24', label: '今日' },
  { value: 'D7', label: '本周' },
  { value: 'D30', label: '本月' }
]
const comics = computed(() =>
  activeTab.value === 'leaderboard'
    ? store.leaderboard
    : activeTab.value === 'latest'
      ? store.latest
      : store.randomComics
)
async function load() {
  if (activeTab.value === 'leaderboard') await store.fetchLeaderboard()
  if (activeTab.value === 'latest') await store.fetchLatest()
  if (activeTab.value === 'random') await store.fetchRandom()
}
async function switchTab(tab: Tab) {
  if (store.loading || activeTab.value === tab) return
  activeTab.value = tab
  await load()
}
function goToLatestPage(page: number) {
  if (store.loading) return
  resultsTop.value?.scrollIntoView({ behavior: 'instant', block: 'start' })
  store.fetchLatest(page)
}
onMounted(load)
</script>
<template>
  <div class="page home-view">
    <div class="page-header">
      <div>
        <div class="eyebrow">PIKA · MANGA DISCOVERY</div>
        <h1>发现本子</h1>
        <p>热门排行、最新更新，快速找到想看的本子。</p>
      </div>
      <router-link to="/categories" class="btn btn-secondary btn-sm"
        ><AppIcon name="grid" :size="14" />按分类找本</router-link
      >
    </div>
    <nav class="discovery-tabs" aria-label="发现内容">
      <button
        v-for="tab in tabs"
        :key="tab.value"
        :class="{ active: activeTab === tab.value }"
        :aria-pressed="activeTab === tab.value"
        :disabled="store.loading"
        @click="switchTab(tab.value)"
      >
        <AppIcon :name="tab.icon" :size="17" />{{ tab.label }}</button
      ><button class="refresh-button" title="刷新内容" :disabled="store.loading" @click="load">
        <AppIcon name="refresh" :size="15" /><span>换一批</span>
      </button>
    </nav>
    <div ref="resultsTop" class="section-heading">
      <h2>
        {{ tabs.find((t) => t.value === activeTab)?.label
        }}<small>{{ tabs.find((t) => t.value === activeTab)?.en }}</small>
      </h2>
      <div class="section-controls">
        <div v-if="activeTab === 'leaderboard'" class="segmented">
          <button
            v-for="p in periods"
            :key="p.value"
            :class="{ active: store.leaderboardPeriod === p.value }"
            :disabled="store.loading"
            @click="store.fetchLeaderboard(p.value)"
          >
            {{ p.label }}
          </button>
        </div>
        <button
          class="icon-btn"
          :title="app.compactCards ? '使用舒展布局' : '使用紧凑布局'"
          @click="app.toggleCompactCards"
        >
          <AppIcon name="grid" :size="16" />
        </button>
      </div>
    </div>
    <ComicSkeleton v-if="store.loading && !comics.length" />
    <EmptyState v-else-if="store.error" title="内容加载失败" :description="store.error" error
      ><button class="btn btn-primary" @click="load">
        <AppIcon name="refresh" :size="15" />重新加载</button
      ><router-link class="btn btn-secondary" to="/settings">检查网络设置</router-link></EmptyState
    >
    <EmptyState
      v-else-if="!comics.length"
      title="暂无漫画"
      description="稍后刷新，或试试搜索其他作品。"
      icon="book"
      ><router-link to="/search" class="btn btn-primary">去搜索</router-link></EmptyState
    >
    <div v-else class="comic-grid">
      <ComicCard
        v-for="(comic, index) in comics"
        :key="comic._id"
        :comic="comic"
        :rank="activeTab === 'leaderboard' ? index + 1 : undefined"
        @click="router.push('/comic/' + comic._id)"
      />
    </div>
    <Paginator
      v-if="activeTab === 'latest' && !store.error"
      :page="store.latestPage"
      :pages="store.latestPages"
      :loading="store.loading"
      @change="goToLatestPage"
    />
    <footer class="page-footer">
      <span>PIKA · 哔咔漫画桌面客户端</span><span>BROWSE · COLLECT · DOWNLOAD</span>
    </footer>
  </div>
</template>
