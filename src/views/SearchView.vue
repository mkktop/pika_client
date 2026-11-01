<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useComicStore } from '../stores/comic'
import ComicCard from '../components/ComicCard.vue'
import ComicSkeleton from '../components/ComicSkeleton.vue'
import EmptyState from '../components/EmptyState.vue'
import AppIcon from '../components/AppIcon.vue'
import Paginator from '../components/Paginator.vue'
const route = useRoute()
const router = useRouter()

defineOptions({ name: 'SearchView' })
const store = useComicStore()
const keyword = ref(String(route.query.q || ''))
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
async function doSearch() {
  if (!keyword.value.trim()) return
  searched.value = true
  submittedKeyword.value = keyword.value.trim()
  page.value = 1
  await store.search(keyword.value.trim(), 1, sort.value)
}
async function goToPage(target: number) {
  if (store.loading) return
  page.value = target
  resultsTop.value?.scrollIntoView({ behavior: 'instant', block: 'start' })
  await store.search(submittedKeyword.value, target, sort.value)
}
function retry() {
  store.search(submittedKeyword.value, page.value, sort.value)
}
function chooseKeyword(kw: string) {
  keyword.value = kw
  doSearch()
}
watch(
  () => route.query.q,
  (q) => {
    if (q && String(q) !== keyword.value) {
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
        <p>按漫画名称、作者或关键词搜索。</p>
      </div>
      <router-link to="/categories" class="btn btn-secondary btn-sm"
        ><AppIcon name="grid" :size="14" />分类浏览</router-link
      >
    </div>
    <form class="search-panel" @submit.prevent="doSearch">
      <div class="search-field">
        <AppIcon name="search" :size="22" /><input
          v-model="keyword"
          aria-label="搜索关键词"
          placeholder="输入漫画名称、作者或关键词"
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
        搜索漫画 <AppIcon name="arrow" :size="15" />
      </button>
    </form>
    <div v-if="store.keywords.length" class="hot-keywords">
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
    <div v-if="searched" ref="resultsTop" class="section-heading search-results-heading">
      <h2>
        搜索结果<small v-if="!store.loading && store.searchPages > 1"
          >第 {{ store.searchPage }} / {{ store.searchPages }} 页</small
        >
      </h2>
      <span class="header-note">按所选方式排序</span>
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
      title="没有找到相关漫画"
      description="试试更短的关键词，或者换一位作者搜索。"
      icon="search"
    />
    <div v-else class="search-welcome">
      <div class="search-welcome-art"><AppIcon name="search" :size="42" /><span>?</span></div>
      <h2>想看什么，搜一下。</h2>
      <p>输入关键词开始搜索，也可以点击上方热门搜索词。</p>
      <span class="eyebrow">SEARCH BY TITLE · AUTHOR · KEYWORD</span>
    </div>
  </div>
</template>
