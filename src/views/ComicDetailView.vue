<script setup lang="ts">
import { onMounted, onUnmounted, computed, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useComicStore, type Episode } from '../stores/comic'
import { invoke } from '@tauri-apps/api/core'
import ComicCard from '../components/ComicCard.vue'
import AppIcon from '../components/AppIcon.vue'
import EmptyState from '../components/EmptyState.vue'
import Paginator from '../components/Paginator.vue'
import { readingProgress } from '../utils/readerPreferences'
import { isSupportedCategory } from '../utils/categories'
const route = useRoute()
const router = useRouter()
const store = useComicStore()
const comicId = String(route.params.id)
const comic = computed(() => (store.currentComic?._id === comicId ? store.currentComic : null))
const categories = computed(() => comic.value?.categories.filter(isSupportedCategory) || [])
const tags = computed(() => comic.value?.tags || [])
const downloading = ref<number[]>([])
const queued = ref<number[]>([])
const downloadedOrders = ref(new Set<number>())
const historyOrders = ref(new Set<number>())
const acting = ref(false)
const commentLoading = ref(false)
const notice = ref('')
const actionError = ref(false)
const failedCover = ref(false)
const commentsTop = ref<HTMLElement>()
const reversed = ref(false)
const episodes = computed(() => (reversed.value ? [...store.episodes].reverse() : store.episodes))
const saved = readingProgress(comicId)
const resumeEpisode = computed(() => saved && store.episodes.find((e) => e.order === saved.episode))
async function load() {
  await store.fetchComic(comicId)
}
onMounted(() => {
  store.recommendations = []
  store.comments = []
  load()
  store.fetchRecommendations(comicId)
  store.fetchComments(comicId)
  refreshDownloaded()
  // 用户可能去资源管理器删除/移动了文件，切回窗口时重新核对下载状态。
  window.addEventListener('focus', refreshDownloaded)
})
onUnmounted(() => window.removeEventListener('focus', refreshDownloaded))
async function refreshDownloaded() {
  try {
    const orders = await invoke<number[]>('get_downloaded_episode_orders', { comicId })
    downloadedOrders.value = new Set(orders)
  } catch {
    downloadedOrders.value = new Set()
  }
  try {
    const orders = await invoke<number[]>('get_history_episode_orders', { comicId })
    historyOrders.value = new Set(orders)
  } catch {
    historyOrders.value = new Set()
  }
}
async function toggle(kind: 'favourite' | 'like') {
  if (!comic.value || acting.value) return
  acting.value = true
  notice.value = ''
  try {
    await invoke(kind === 'favourite' ? 'toggle_favourite' : 'like_comic', { comicId })
    if (route.name === 'comic-detail' && route.params.id === comicId) await load()
  } catch (e) {
    actionError.value = true
    notice.value = '操作失败：' + e
  } finally {
    acting.value = false
  }
}
async function download(ep: Episode) {
  if (!comic.value || downloading.value.includes(ep.order) || queued.value.includes(ep.order))
    return
  downloading.value.push(ep.order)
  notice.value = ''
  try {
    await invoke('start_download', {
      req: {
        comic_id: comicId,
        comic_title: comic.value.title,
        episode_order: ep.order,
        episode_title: ep.title
      }
    })
    queued.value.push(ep.order)
    actionError.value = false
    notice.value = '「' + ep.title + '」已加入下载队列'
  } catch (e) {
    actionError.value = true
    notice.value = '下载失败：' + e
  } finally {
    downloading.value = downloading.value.filter((o) => o !== ep.order)
  }
}
const pendingEpisodes = computed(() =>
  store.episodes.filter(
    (ep) =>
      !downloadedOrders.value.has(ep.order) &&
      !downloading.value.includes(ep.order) &&
      !queued.value.includes(ep.order)
  )
)
async function downloadAll() {
  if (!comic.value || acting.value) return
  const targets = pendingEpisodes.value
  if (!targets.length) return
  notice.value = ''
  let ok = 0
  for (const ep of targets) {
    try {
      await invoke('start_download', {
        req: {
          comic_id: comicId,
          comic_title: comic.value.title,
          episode_order: ep.order,
          episode_title: ep.title
        }
      })
      queued.value.push(ep.order)
      ok++
    } catch {
      // 单章入队失败不阻塞其余章节
    }
  }
  if (ok) {
    actionError.value = false
    notice.value = `已把 ${ok} 个章节加入下载队列`
  } else {
    actionError.value = true
    notice.value = '批量下载失败，请稍后重试'
  }
}
function read(ep: Episode) {
  router.push('/reader/' + comicId + '/' + ep.order)
}
function goBack() {
  const previous = router.options.history.state.back
  if (typeof previous === 'string') {
    const target = router.resolve(previous)
    // Old reading sessions may have left a reader entry behind this detail page.
    if (target.matched.length && target.name !== 'reader') {
      router.back()
      return
    }
  }
  router.replace({ name: 'home' })
}
async function goToCommentPage(page: number) {
  if (commentLoading.value) return
  commentLoading.value = true
  commentsTop.value?.scrollIntoView({ behavior: 'instant', block: 'start' })
  await store.fetchComments(comicId, page)
  commentLoading.value = false
}
const count = (n = 0) => (n >= 10000 ? (n / 10000).toFixed(1) + '万' : n.toLocaleString())
</script>
<template>
  <div class="page comic-detail">
    <button class="text-btn detail-back" @click="goBack">
      <AppIcon name="back" :size="16" />返回上一页
    </button>
    <div v-if="store.loading && !comic" class="loading">
      <span class="spinner" />正在加载漫画详情…
    </div>
    <EmptyState
      v-else-if="!comic"
      error
      title="这部作品暂时没有打开"
      :description="store.error || '请稍后重新加载。'"
      ><button class="btn btn-primary" @click="load">重新加载</button></EmptyState
    >
    <template v-else
      ><section class="detail-header">
        <div class="cover-large">
          <img
            v-if="store.getThumbUrl(comic) && !failedCover"
            :src="store.getThumbUrl(comic)"
            :alt="comic.title"
            @error="failedCover = true"
          /><span v-else class="cover-fallback"
            ><AppIcon name="book" :size="42" /><span>PIKA LIBRARY</span></span
          >
        </div>
        <div class="detail-info">
          <div class="eyebrow">
            {{ comic.finished ? 'COMPLETED · 已完结' : 'ONGOING · 连载中' }}
          </div>
          <h1>{{ comic.title }}</h1>
          <button
            v-if="comic.author"
            class="detail-author author-link"
            :title="'搜索「' + comic.author + '」的作品'"
            @click="router.push({ name: 'search', query: { q: comic.author } })"
          >
            {{ comic.author }}
          </button>
          <p v-else class="detail-author">作者未署名</p>
          <p
            v-if="comic.created_at || comic.updated_at"
            class="detail-dates"
            :title="'发布 ' + (comic.created_at || '未知') + ' · 更新 ' + (comic.updated_at || '未知')"
          >
            <template v-if="comic.created_at">发布 {{ comic.created_at.slice(0, 10) }}</template
            ><template
              v-if="
                comic.updated_at &&
                comic.updated_at.slice(0, 10) !== (comic.created_at || '').slice(0, 10)
              "
              >　更新 {{ comic.updated_at.slice(0, 10) }}</template
            >
          </p>
          <div class="detail-stats">
            <div>
              <strong>{{ count(comic.pagesCount) }}</strong
              ><span>页</span>
            </div>
            <div>
              <strong>{{ count(comic.epsCount) }}</strong
              ><span>章</span>
            </div>
            <div>
              <strong>{{ count(comic.likesCount) }}</strong
              ><span>喜欢</span>
            </div>
            <div>
              <strong>{{ count(comic.viewsCount) }}</strong
              ><span>浏览</span>
            </div>
          </div>
          <div class="categories">
            <button
              v-for="cat in categories"
              :key="cat"
              class="tag"
              style="border: 0"
              @click="router.push({ name: 'categories', query: { category: cat } })"
            >
              {{ cat }}
            </button>
          </div>
          <div v-if="tags.length" class="categories detail-tags">
            <button
              v-for="tag in tags"
              :key="tag"
              class="tag tag-chip"
              style="border: 0"
              :title="'按标签「' + tag + '」浏览（无结果数上限）'"
              @click="router.push({ name: 'search', query: { tag } })"
            >
              # {{ tag }}
            </button>
          </div>
          <p v-if="comic.description" class="description">{{ comic.description }}</p>
          <div class="detail-actions">
            <button
              v-if="store.episodes[0]"
              class="btn btn-primary"
              @click="read(resumeEpisode || store.episodes[0])"
            >
              <AppIcon name="book" :size="16" />{{
                resumeEpisode ? '继续阅读 · 第 ' + resumeEpisode.order + ' 话' : '开始阅读'
              }}<AppIcon name="arrow" :size="15" /></button
            ><button
              class="btn btn-secondary"
              :class="{ selected: comic.isFavourite }"
              :disabled="acting"
              @click="toggle('favourite')"
            >
              <AppIcon name="heart" :size="16" />{{
                comic.isFavourite ? '已加入收藏' : '加入收藏'
              }}</button
            ><button
              class="btn btn-secondary"
              :class="{ selected: comic.isLiked }"
              :disabled="acting"
              @click="toggle('like')"
            >
              <AppIcon name="sparkles" :size="16" />{{ comic.isLiked ? '已喜欢' : '喜欢' }}
            </button>
          </div>
        </div>
      </section>
      <div
        v-if="notice"
        role="status"
        class="notice"
        :class="{ error: actionError }"
        style="margin-top: 17px"
      >
        <AppIcon :name="actionError ? 'alert' : 'check'" :size="15" />{{ notice
        }}<router-link v-if="!actionError" to="/downloads" class="text-btn">查看下载</router-link>
      </div>
      <section class="detail-section">
        <div class="section-heading">
          <h2>
            章节目录<small>{{ store.episodes.length }} CHAPTERS</small>
          </h2>
          <div class="section-controls">
            <button
              class="btn btn-secondary btn-sm"
              :disabled="!pendingEpisodes.length"
              :title="pendingEpisodes.length ? '下载全部未下载章节' : '所有章节都已下载或在队列中'"
              @click="downloadAll"
            >
              <AppIcon name="download" :size="14" />下载整本<template v-if="pendingEpisodes.length"
                >（{{ pendingEpisodes.length }} 话）</template
              >
            </button><button class="text-btn" @click="reversed = !reversed">
              <AppIcon name="list" :size="14" />{{ reversed ? '倒序排列' : '正序排列' }}
            </button>
          </div>
        </div>
        <EmptyState v-if="!episodes.length" title="暂无章节" description="稍后再来看看。" />
        <div v-else class="episodes-grid">
          <article v-for="ep in episodes" :key="ep._id" class="episode-card">
            <div class="ep-info">
              <span class="ep-order">{{ String(ep.order).padStart(2, '0') }}</span
              ><span class="ep-title" :title="ep.title">{{ ep.title }}</span
              ><span v-if="downloadedOrders.has(ep.order)" class="ep-badge">已下载</span
              ><span
                v-else-if="historyOrders.has(ep.order)"
                class="ep-badge muted"
                title="下载历史里有这一章，但本地文件已不在"
                >曾下载</span
              >
            </div>
            <div class="ep-actions">
              <button class="btn btn-secondary btn-sm" @click="read(ep)">阅读</button
              ><button
                class="icon-btn"
                :title="
                  downloadedOrders.has(ep.order)
                    ? '已下载到本地'
                    : queued.includes(ep.order)
                      ? '已加入队列'
                      : '下载 ' + ep.title
                "
                :disabled="
                  downloading.includes(ep.order) ||
                  queued.includes(ep.order) ||
                  downloadedOrders.has(ep.order)
                "
                @click="download(ep)"
              >
                <AppIcon
                  :name="
                    downloadedOrders.has(ep.order) || queued.includes(ep.order)
                      ? 'check'
                      : 'download'
                  "
                  :size="16"
                />
              </button>
            </div>
          </article>
        </div>
      </section>
      <section v-if="store.recommendations.length" class="detail-section">
        <div class="section-heading">
          <h2>你也许还喜欢<small>MORE TO EXPLORE</small></h2>
        </div>
        <div class="comic-grid">
          <ComicCard
            v-for="item in store.recommendations"
            :key="item._id"
            :comic="item"
            @click="router.push('/comic/' + item._id)"
          />
        </div>
      </section>
      <section ref="commentsTop" class="detail-section">
        <div class="section-heading">
          <h2>
            评论区<small
              >{{ store.commentPages > 1
                ? `第 ${store.commentPage} / ${store.commentPages} 页`
                : `${store.comments.length} COMMENTS` }}</small
            >
          </h2>
        </div>
        <div v-if="store.comments.length" class="comments-list">
          <article v-for="c in store.comments" :key="c._id" class="comment-item">
            <img
              v-if="store.getAvatarUrl(c._user?.avatar)"
              class="comment-avatar"
              :src="store.getAvatarUrl(c._user?.avatar)"
              alt="读者头像"
            /><span v-else class="comment-avatar"><AppIcon name="user" :size="16" /></span>
            <div class="comment-body">
              <div class="comment-head">
                <strong>{{ c._user?.name || '匿名读者' }}</strong
                ><span class="comment-level">Lv.{{ c._user?.level || 0 }}</span
                ><time class="comment-time">{{ c.created_at?.slice(0, 10) }}</time>
              </div>
              <p class="comment-content">{{ c.content }}</p>
              <div class="comment-foot">
                <AppIcon name="heart" :size="12" />{{ c.likesCount || 0
                }}<template v-if="c.commentsCount"
                  >　<AppIcon name="message" :size="12" />{{ c.commentsCount }}</template
                >
              </div>
            </div>
          </article>
        </div>
        <EmptyState v-else title="暂无评论" description="暂时没有可展示的评论。" icon="message" />
        <Paginator
          :page="store.commentPage"
          :pages="store.commentPages"
          :loading="commentLoading"
          @change="goToCommentPage"
        />
      </section>
    </template>
  </div>
</template>
