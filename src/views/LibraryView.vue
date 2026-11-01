<script setup lang="ts">
import { onMounted, ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useRouter } from 'vue-router'
import AppIcon from '../components/AppIcon.vue'
import EmptyState from '../components/EmptyState.vue'
interface LocalComic {
  comic_id: string
  title: string
  author?: string
  finished?: boolean
  pages_count?: number
  eps_count?: number
  downloaded_episodes?: string[]
  cover_path?: string
}
const router = useRouter()
const comics = ref<LocalComic[]>([])
const stats = ref({ total_comics: 0, total_episodes: 0, total_images: 0 })
const loading = ref(false)
const error = ref('')
const query = ref('')
const offset = ref(0)
const more = ref(false)
const failedCovers = ref(new Set<string>())
const filtered = computed(() =>
  comics.value.filter((c) =>
    (c.title + ' ' + (c.author || '')).toLowerCase().includes(query.value.toLowerCase())
  )
)
async function loadLibrary(append = false) {
  if (loading.value) return
  loading.value = true
  error.value = ''
  try {
    const nextOffset = append ? offset.value + 100 : 0
    const [items, counts] = await Promise.all([
      invoke<LocalComic[]>('get_library', { limit: 100, offset: nextOffset }),
      invoke<typeof stats.value>('get_library_stats')
    ])
    comics.value = append ? [...comics.value, ...items] : items
    stats.value = counts
    offset.value = nextOffset
    more.value = items.length === 100
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
}
onMounted(() => loadLibrary())
</script>
<template>
  <div class="page library-view">
    <div class="page-header">
      <div>
        <div class="eyebrow">LOCAL COMICS LIBRARY</div>
        <h1>本地书库</h1>
        <p>查看已下载到本地的漫画和章节。</p>
      </div>
      <button class="btn btn-secondary btn-sm" :disabled="loading" @click="loadLibrary()">
        <AppIcon name="refresh" :size="14" />刷新书库
      </button>
    </div>
    <div class="stats-bar">
      <div class="stat-card">
        <span class="stat-icon"><AppIcon name="book" /></span>
        <div>
          <strong class="num">{{ stats.total_comics }}</strong
          ><span class="label">收藏于本地的漫画</span>
        </div>
      </div>
      <div class="stat-card">
        <span class="stat-icon"><AppIcon name="layers" /></span>
        <div>
          <strong class="num">{{ stats.total_episodes }}</strong
          ><span class="label">已下载章节</span>
        </div>
      </div>
      <div class="stat-card">
        <span class="stat-icon"><AppIcon name="image" /></span>
        <div>
          <strong class="num">{{ stats.total_images.toLocaleString() }}</strong
          ><span class="label">已保存图片</span>
        </div>
      </div>
    </div>
    <div v-if="comics.length" class="library-toolbar">
      <input
        v-model="query"
        class="input"
        placeholder="在已加载的书库中搜索…"
        aria-label="搜索本地书库"
      /><span class="header-note">{{ filtered.length }} 部作品</span>
    </div>
    <div v-if="loading && !comics.length" class="loading">
      <span class="spinner" />正在整理你的书架…
    </div>
    <EmptyState v-else-if="error" error title="书库暂时没有打开" :description="error"
      ><button class="btn btn-primary" @click="loadLibrary()">重新打开</button></EmptyState
    ><EmptyState
      v-else-if="!comics.length"
      title="本地书库为空"
      description="在漫画的章节列表中点击下载，完成后就能在这里找到它。"
      icon="library"
      ><router-link class="btn btn-primary" to="/"
        >发现漫画 <AppIcon name="arrow" :size="15" /></router-link></EmptyState
    ><EmptyState
      v-else-if="!filtered.length"
      title="书架上还没有这部作品"
      description="试试其他关键词。"
      icon="search"
    />
    <div v-else class="library-grid">
      <button
        v-for="comic in filtered"
        :key="comic.comic_id"
        class="local-comic"
        @click="router.push('/comic/' + comic.comic_id)"
      >
        <span class="local-cover"
          ><img
            v-if="comic.cover_path && !failedCovers.has(comic.comic_id)"
            :src="comic.cover_path"
            :alt="comic.title"
            loading="lazy"
            @error="failedCovers.add(comic.comic_id)" /><AppIcon
            v-else
            name="book"
            :size="27" /></span
        ><span class="local-info"
          ><strong>{{ comic.title }}</strong
          ><span>{{ comic.author || '作者未署名' }}</span
          ><span>{{ comic.eps_count || 0 }} 章节 · {{ comic.pages_count || 0 }} 页</span
          ><small>已下载 {{ comic.downloaded_episodes?.length || 0 }} 章</small></span
        >
      </button>
    </div>
    <div v-if="more" class="load-more">
      <button class="btn btn-secondary" :disabled="loading" @click="loadLibrary(true)">
        {{ loading ? '加载中…' : '加载更多漫画' }}
      </button>
    </div>
  </div>
</template>
