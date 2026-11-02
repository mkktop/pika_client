<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useComicStore } from '../stores/comic'
import ComicCard from '../components/ComicCard.vue'
import ComicSkeleton from '../components/ComicSkeleton.vue'
import EmptyState from '../components/EmptyState.vue'
import AppIcon from '../components/AppIcon.vue'
import Paginator from '../components/Paginator.vue'
import { readPreference, writePreference } from '../utils/readerPreferences'
const route = useRoute()
const router = useRouter()

defineOptions({ name: 'SearchView' })
const store = useComicStore()
// 两种模式：keyword 走全文搜索（每次查询上限 1 万条）；tag 走 t= 精确过滤（无上限）。
type SearchMode = 'keyword' | 'tag'
const mode = ref<SearchMode>(route.query.tag ? 'tag' : 'keyword')
const keyword = ref(String(route.query.tag || route.query.q || ''))
const sort = ref('dd')
const page = ref(1)
const searched = ref(false)
const submittedKeyword = ref('')
const resultsTop = ref<HTMLElement>()
const sortOptions = [
  { value: 'dd', label: '最新更新' },
  { value: 'da', label: '最早发布' },
  { value: 'ld', label: '最多喜欢' },
  { value: 'vd', label: '最多浏览' }
]
// 标签模式的最近搜索记录，本地持久化，最多留 10 个。
const TAG_HISTORY_KEY = 'pika-tag-history'
const tagHistory = ref<string[]>(readPreference<string[]>(TAG_HISTORY_KEY, []))
function rememberTag(tag: string) {
  tagHistory.value = [tag, ...tagHistory.value.filter((t) => t !== tag)].slice(0, 10)
  writePreference(TAG_HISTORY_KEY, tagHistory.value)
}
// 关键词模式的最近搜索记录，同样本地持久化。
const KW_HISTORY_KEY = 'pika-kw-history'
const kwHistory = ref<string[]>(readPreference<string[]>(KW_HISTORY_KEY, []))
function rememberKeyword(kw: string) {
  kwHistory.value = [kw, ...kwHistory.value.filter((t) => t !== kw)].slice(0, 10)
  writePreference(KW_HISTORY_KEY, kwHistory.value)
}
function clearKwHistory() {
  kwHistory.value = []
  writePreference(KW_HISTORY_KEY, kwHistory.value)
}
function chooseTag(tag: string) {
  keyword.value = tag
  doSearch()
}
function clearTagHistory() {
  tagHistory.value = []
  writePreference(TAG_HISTORY_KEY, tagHistory.value)
}
async function doSearch() {
  if (!keyword.value.trim()) return
  searched.value = true
  submittedKeyword.value = keyword.value.trim()
  page.value = 1
  if (mode.value === 'tag') {
    rememberTag(submittedKeyword.value)
    await store.searchByTag(submittedKeyword.value, 1, sort.value)
  } else {
    rememberKeyword(submittedKeyword.value)
    await store.search(submittedKeyword.value, 1, sort.value)
  }
}
async function goToPage(target: number) {
  if (store.loading) return
  page.value = target
  resultsTop.value?.scrollIntoView({ behavior: 'auto', block: 'start' })
  if (mode.value === 'tag') {
    await store.searchByTag(submittedKeyword.value, target, sort.value)
  } else {
    await store.search(submittedKeyword.value, target, sort.value)
  }
}
function retry() {
  if (mode.value === 'tag') {
    store.searchByTag(submittedKeyword.value, page.value, sort.value)
  } else {
    store.search(submittedKeyword.value, page.value, sort.value)
  }
}
function chooseKeyword(kw: string) {
  keyword.value = kw
  mode.value = 'keyword'
  doSearch()
}
function switchMode(next: SearchMode) {
  if (mode.value === next) return
  mode.value = next
  searched.value = false
  submittedKeyword.value = ''
  store.searchResults = []
  store.error = null
}
watch(
  () => [route.query.q, route.query.tag],
  ([q, t]) => {
    if (t && String(t) !== keyword.value) {
      mode.value = 'tag'
      keyword.value = String(t)
      doSearch()
    } else if (q && mode.value === 'keyword' && String(q) !== keyword.value) {
      keyword.value = String(q)
      doSearch()
    }
  }
)
onMounted(() => {
  store.searchResults = []
  store.error = null
  store.fetchKeywords()
  if (keyword.value) doSearch()
})
</script>
<template>
  <div class="page search-view">
    <div class="page-header">
      <div>
        <div class="eyebrow">SEARCH · PIKA COMICS</div>
        <h1>搜索与探索</h1>
        <p>按关键词全文搜索，或用标签精确过滤（标签无结果数上限）。</p>
      </div>
      <router-link to="/categories" class="btn btn-secondary btn-sm"
        ><AppIcon name="grid" :size="14" />分类浏览</router-link
      >
    </div>
    <div class="search-modes" role="tablist" aria-label="搜索方式">
      <button
        type="button"
        role="tab"
        :aria-selected="mode === 'keyword'"
        :class="{ active: mode === 'keyword' }"
        @click="switchMode('keyword')"
      >
        关键词
      </button>
      <button
        type="button"
        role="tab"
        :aria-selected="mode === 'tag'"
        :class="{ active: mode === 'tag' }"
        @click="switchMode('tag')"
      >
        标签
      </button>
    </div>
    <form class="search-panel" @submit.prevent="doSearch">
      <div class="search-field">
        <AppIcon name="search" :size="22" /><input
          v-model="keyword"
          aria-label="搜索关键词"
          :placeholder="
            mode === 'tag' ? '输入完整标签名，如：全彩、純愛、原神' : '输入漫画名称、作者或关键词'
          "
        /><button
          v-if="keyword"
          type="button"
          title="清空关键词"
          class="icon-btn"
          @click="keyword = ''"
        >
          <AppIcon name="close" :size="16" />
        </button>
      </div>
      <select
        v-model="sort"
        class="input search-sort"
        aria-label="搜索排序"
        :disabled="store.loading"
        @change="searched && doSearch()"
      >
        <option v-for="opt in sortOptions" :key="opt.value" :value="opt.value">
          {{ opt.label }}
        </option></select
      ><button class="btn btn-primary" :disabled="store.loading || !keyword.trim()">
        {{ mode === 'tag' ? '按标签查找' : '搜索漫画' }} <AppIcon name="arrow" :size="15" />
      </button>
    </form>
    <div v-if="mode === 'keyword' && kwHistory.length" class="hot-keywords">
      <span><AppIcon name="clock" :size="15" /> 最近搜索</span
      ><button
        v-for="kw in kwHistory"
        :key="kw"
        class="hot-tag"
        :disabled="store.loading"
        @click="chooseKeyword(kw)"
      >
        {{ kw }}
      </button><button class="text-btn tag-history-clear" :disabled="store.loading" @click="clearKwHistory">
        清除
      </button>
    </div>
    <div v-if="mode === 'keyword' && store.keywords.length" class="hot-keywords">
      <span><AppIcon name="trend" :size="15" /> 大家在搜</span
      ><button
        v-for="kw in store.keywords.slice(0, 14)"
        :key="kw"
        class="hot-tag"
        :disabled="store.loading"
        @click="chooseKeyword(kw)"
      >
        {{ kw }}
      </button>
    </div>
    <div v-else-if="mode === 'tag' && tagHistory.length" class="hot-keywords">
      <span><AppIcon name="clock" :size="15" /> 最近标签</span
      ><button
        v-for="t in tagHistory"
        :key="t"
        class="hot-tag"
        :disabled="store.loading"
        @click="chooseTag(t)"
      >
        {{ t }}
      </button><button class="text-btn tag-history-clear" :disabled="store.loading" @click="clearTagHistory">
        清除
      </button>
    </div>
    <div v-if="searched" ref="resultsTop" class="section-heading search-results-heading">
      <h2>
        {{ mode === 'tag' ? '标签结果' : '搜索结果'
        }}<small v-if="!store.loading && store.searchPages > 1"
          >第 {{ store.searchPage }} / {{ store.searchPages }} 页</small
        >
      </h2>
      <span class="header-note">{{
        mode === 'tag' ? `精确匹配标签「${submittedKeyword}」` : '按所选方式排序'
      }}</span>
    </div>
    <ComicSkeleton v-if="store.loading && !store.searchResults.length" />
    <EmptyState
      v-else-if="store.error"
      title="搜索暂时遇到了一点问题"
      :description="store.error"
      error
      ><button class="btn btn-primary" @click="retry">重新搜索</button></EmptyState
    >
    <template v-else-if="store.searchResults.length"
      ><div class="comic-grid">
        <ComicCard
          v-for="comic in store.searchResults"
          :key="comic._id"
          :comic="comic"
          @click="router.push('/comic/' + comic._id)"
        />
      </div>
      <Paginator
        :page="store.searchPage"
        :pages="store.searchPages"
        :loading="store.loading"
        @change="goToPage"
      />
    </template>
    <EmptyState
      v-else-if="searched"
      :title="mode === 'tag' ? '没有命中这个标签' : '没有找到相关漫画'"
      :description="
        mode === 'tag'
          ? '标签需要精确匹配完整名称，可以从漫画详情页点击标签进入。'
          : '试试更短的关键词，或者换一位作者搜索。'
      "
      icon="search"
    />
    <div v-else class="search-welcome">
      <div class="search-welcome-art"><AppIcon name="search" :size="42" /><span>?</span></div>
      <h2>{{ mode === 'tag' ? '按标签浏览全部作品。' : '想看什么，搜一下。' }}</h2>
      <p>{{
        mode === 'tag'
          ? '输入完整标签名开始过滤，也可以从漫画详情页点击标签跳转过来。'
          : '输入关键词开始搜索，也可以点击上方热门搜索词。'
      }}</p>
      <span class="eyebrow">{{ mode === 'tag' ? 'FILTER BY TAG · EXACT MATCH' : 'SEARCH BY TITLE · AUTHOR · KEYWORD' }}</span>
    </div>
  </div>
</template>
