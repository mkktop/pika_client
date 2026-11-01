<script setup lang="ts">
import { onMounted, computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useComicStore } from '../stores/comic'
import { useAppStore } from '../stores/app'
import ComicCard from '../components/ComicCard.vue'
import AppIcon from '../components/AppIcon.vue'
import ComicSkeleton from '../components/ComicSkeleton.vue'
import EmptyState from '../components/EmptyState.vue'
const router = useRouter()
const store = useComicStore()
const app = useAppStore()
type Tab = 'leaderboard' | 'latest' | 'random' | 'collections'
const activeTab = ref<Tab>('leaderboard')
const loadingMore = ref(false)
const tabs: { value: Tab; label: string; icon: string; en: string }[] = [
  { value: 'leaderboard', label: '热门排行', icon: 'trend', en: 'TRENDING NOW' },
  { value: 'latest', label: '最新更新', icon: 'clock', en: 'FRESH UPDATES' },
  { value: 'random', label: '随机本子', icon: 'shuffle', en: 'RANDOM PICKS' },
  { value: 'collections', label: '精选合集', icon: 'layers', en: 'CURATED COLLECTIONS' }
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
  if (activeTab.value === 'collections') await store.fetchCollections()
}
async function switchTab(tab: Tab) {
  if (store.loading || activeTab.value === tab) return
  activeTab.value = tab
  await load()
}
async function loadMore() {
  if (store.loading) return
  loadingMore.value = true
  await store.fetchLatest(store.latestPage + 1)
  loadingMore.value = false
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
    <div class="section-heading">
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
    <ComicSkeleton v-if="store.loading && !loadingMore" />
    <EmptyState v-else-if="store.error" title="内容加载失败" :description="store.error" error
      ><button class="btn btn-primary" @click="load">
        <AppIcon name="refresh" :size="15" />重新加载</button
      ><router-link class="btn btn-secondary" to="/settings">检查网络设置</router-link></EmptyState
    >
    <template v-else-if="activeTab === 'collections'"
      ><EmptyState
        v-if="!store.collections.length"
        title="暂无推荐合集"
        description="可以先浏览热门排行，或稍后刷新。"
        icon="layers"
        ><button class="btn btn-primary" @click="switchTab('leaderboard')">
          浏览热门排行
        </button></EmptyState
      >
      <section v-for="col in store.collections" :key="col._id" class="collection-section">
        <div class="section-heading">
          <h2>
            {{ col.title || '推荐合集' }}<small>{{ col.comics.length }} 部作品</small>
          </h2>
        </div>
        <div class="comic-grid">
          <ComicCard
            v-for="comic in col.comics"
            :key="comic._id"
            :comic="comic"
            @click="router.push('/comic/' + comic._id)"
          />
        </div></section
    ></template>
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
    <div v-if="activeTab === 'latest' && store.latestHasMore && !store.error" class="load-more">
      <button class="btn btn-secondary" :disabled="store.loading" @click="loadMore">
        {{ loadingMore ? '正在加载更多…' : '加载更多作品'
        }}<AppIcon name="chevron-right" :size="14" />
      </button>
    </div>
    <footer class="page-footer">
      <span>PIKA · 哔咔漫画桌面客户端</span><span>BROWSE · COLLECT · DOWNLOAD</span>
    </footer>
  </div>
</template>
