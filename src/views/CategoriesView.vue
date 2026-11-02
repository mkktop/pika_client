<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useCategoryStore } from '../stores/categories'
import {
  categoryKey,
  categoryTitle,
  isSupportedCategory,
  type CategoryGroup
} from '../utils/categories'
import AppIcon from '../components/AppIcon.vue'
import ComicCard from '../components/ComicCard.vue'
import ComicSkeleton from '../components/ComicSkeleton.vue'
import EmptyState from '../components/EmptyState.vue'
import Paginator from '../components/Paginator.vue'

const store = useCategoryStore()
const route = useRoute()
const router = useRouter()
const filter = ref('')
const resultsTop = ref<HTMLElement>()

defineOptions({ name: 'CategoriesView' })
const selected = computed(() =>
  typeof route.query.category === 'string' ? route.query.category.trim() : ''
)
const supported = computed(() => !selected.value || isSupportedCategory(selected.value))
const sorts = [
  { value: 'dd', label: '最新更新' },
  { value: 'da', label: '最早发布' },
  { value: 'ld', label: '最多喜欢' },
  { value: 'vd', label: '最多浏览' }
]
const sort = computed(() =>
  sorts.some((option) => option.value === route.query.sort) ? String(route.query.sort) : 'dd'
)
const current = computed<CategoryGroup | null>(() => {
  if (!selected.value || !supported.value) return null
  const key = categoryKey(selected.value)
  return (
    store.groups.find((group) => group.key === key) || {
      key,
      title: categoryTitle(selected.value),
      aliases: [selected.value]
    }
  )
})
const groups = computed(() => {
  const term = categoryKey(filter.value)
  return store.groups.filter((group) => group.key.includes(term))
})

function choose(group?: CategoryGroup) {
  if (!group) filter.value = ''
  router.push({
    name: 'categories',
    query: {
      ...(group ? { category: group.title } : {}),
      ...(sort.value !== 'dd' ? { sort: sort.value } : {})
    }
  })
}
function changeSort(event: Event) {
  router.push({
    name: 'categories',
    query: { ...route.query, sort: (event.target as HTMLSelectElement).value }
  })
}
function goToPage(page: number) {
  if (!current.value || store.loading) return
  resultsTop.value?.scrollIntoView({ behavior: 'auto', block: 'start' })
  store.fetchComics(current.value.aliases, sort.value, page)
}
function retry() {
  if (current.value) {
    store.fetchComics(current.value.aliases, sort.value, store.comics.length ? store.page + 1 : 1)
  }
}
watch(
  [() => store.catalogLoaded, () => current.value?.aliases.join('\0'), sort],
  () => {
    store.resetResults()
    if (store.catalogLoaded && current.value) store.fetchComics(current.value.aliases, sort.value)
  },
  { immediate: true }
)
onMounted(() => store.fetchCatalog())
</script>

<template>
  <div class="page categories-view">
    <div class="page-header">
      <div>
        <div class="eyebrow">CATEGORIES</div>
        <h1>分类浏览</h1>
        <p>按站点分类找本，支持简繁名称匹配。</p>
      </div>
      <button
        class="btn btn-secondary btn-sm"
        :disabled="store.catalogLoading || store.loading"
        @click="store.fetchCatalog(true)"
      >
        <AppIcon name="refresh" :size="14" />刷新分类
      </button>
    </div>
    <section class="category-panel" :class="{ 'has-selection': !!selected }" aria-label="站点分类">
      <div class="category-panel-heading">
        <strong
          >全部分类 <span v-if="store.catalogLoaded">{{ store.groups.length }}</span></strong
        ><label class="category-filter"
          ><AppIcon name="search" :size="15" /><input
            v-model="filter"
            aria-label="筛选分类"
            placeholder="筛选分类名称，支持简繁体" /><button
            v-if="filter"
            class="icon-btn"
            title="清空分类筛选"
            @click="filter = ''"
          >
            <AppIcon name="close" :size="13" /></button
        ></label>
      </div>
      <div
        v-if="store.catalogLoading && !store.catalogLoaded"
        class="category-grid"
        aria-busy="true"
        aria-label="正在加载分类"
      >
        <span v-for="n in 12" :key="n" class="skeleton category-skeleton" />
      </div>
      <EmptyState
        v-else-if="store.catalogError"
        error
        title="分类加载失败"
        :description="store.catalogError"
        ><button class="btn btn-primary" @click="store.fetchCatalog(true)">
          重新加载分类
        </button></EmptyState
      >
      <div v-else-if="groups.length" class="category-grid">
        <button
          v-for="group in groups"
          :key="group.key"
          class="category-option"
          :class="{ active: current?.key === group.key }"
          :aria-pressed="current?.key === group.key"
          :title="group.aliases.join(' / ')"
          @click="choose(group)"
        >
          <AppIcon name="grid" :size="15" /><span>{{ group.title }}</span
          ><small v-if="group.aliases.length > 1">简繁</small
          ><AppIcon v-else name="chevron-right" :size="12" />
        </button>
      </div>
      <p v-else class="category-no-match">
        {{ filter ? '没有匹配的分类，试试其他名称。' : '暂无可浏览的分类。' }}
      </p>
    </section>
    <EmptyState v-if="selected && !supported" error title="该分类不可用"
      ><button class="btn btn-secondary" @click="choose()">查看其他分类</button></EmptyState
    >
    <template v-else-if="current && store.catalogLoaded && !store.catalogError">
      <div ref="resultsTop" class="section-heading category-results-heading">
        <div>
          <h2>
            {{ current.title
            }}<small v-if="!store.loading && store.pages > 1"
              >第 {{ store.page }} / {{ store.pages }} 页</small
            >
          </h2>
          <p v-if="current.aliases.length > 1" class="category-aliases">
            已合并简繁分类：{{ current.aliases.join(' / ') }}
          </p>
        </div>
        <div class="section-controls">
          <button class="text-btn" @click="choose()">全部分类</button
          ><select
            class="input category-sort"
            :value="sort"
            aria-label="分类排序"
            @change="changeSort"
          >
            <option v-for="option in sorts" :key="option.value" :value="option.value">
              {{ option.label }}
            </option>
          </select>
        </div>
      </div>
      <ComicSkeleton v-if="store.loading && !store.comics.length" />
      <template v-else>
        <div
          v-if="store.error && store.comics.length"
          role="alert"
          class="notice error category-load-error"
        >
          {{ store.error }}<button class="text-btn" @click="retry">重试本页</button>
        </div>
        <EmptyState
          v-if="store.error && !store.comics.length"
          error
          title="分类漫画加载失败"
          :description="store.error"
          ><button class="btn btn-primary" @click="retry">重新加载漫画</button></EmptyState
        >
        <div v-else-if="store.comics.length" class="comic-grid">
          <ComicCard
            v-for="comic in store.comics"
            :key="comic._id"
            :comic="comic"
            @click="router.push('/comic/' + comic._id)"
          />
        </div>
        <EmptyState
          v-else
          title="这个分类下暂无漫画"
          description="可以切换其他分类，或按关键词搜索。"
          icon="grid"
          ><router-link
            class="btn btn-secondary"
            :to="{ name: 'search', query: { q: current.title } }"
            >关键词搜索</router-link
          ></EmptyState
        >
        <Paginator
          v-if="!store.error"
          :page="store.page"
          :pages="store.pages"
          :loading="store.loading"
          @change="goToPage"
        />
      </template>
    </template>
  </div>
</template>

<style scoped>
.category-panel {
  padding: 22px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 13px;
  margin-bottom: 28px;
}
.category-panel-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 20px;
  margin-bottom: 20px;
}
.category-panel-heading strong {
  font-size: 13px;
  font-weight: 600;
  white-space: nowrap;
}
.category-panel-heading strong span {
  font-size: 10px;
  color: var(--text-muted);
  margin-left: 8px;
  font-weight: 400;
}
.category-filter {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  border: 1px solid var(--border);
  border-radius: 7px;
  width: 270px;
  background: var(--bg-primary);
  color: var(--text-secondary);
}
.category-filter:focus-within {
  border-color: var(--accent);
}
.category-filter input {
  border: 0;
  background: none;
  color: var(--text-primary);
  outline: none;
  width: 100%;
  min-width: 0;
  font-size: 11px;
  height: 24px;
}
.category-filter .icon-btn {
  width: 24px;
  height: 24px;
}
.category-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  gap: 10px;
}
.category-option {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 52px;
  padding: 14px;
  border: 1px solid var(--border);
  background: var(--bg-primary);
  color: var(--text-secondary);
  border-radius: 8px;
  text-align: left;
  transition: var(--transition);
  min-width: 0;
}
.category-option > span {
  font-size: 12px;
  color: var(--text-primary);
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.category-option small {
  font-size: 9px;
  color: var(--text-muted);
}
.category-option:hover {
  border-color: var(--accent);
  color: var(--accent);
}
.category-option.active {
  background: var(--accent-glow);
  border-color: var(--accent);
  color: var(--accent);
}
.category-option.active > span,
.category-option.active small {
  color: var(--accent);
}
.has-selection .category-grid {
  grid-template-columns: repeat(auto-fill, minmax(125px, 1fr));
  max-height: 185px;
  overflow-y: auto;
  padding: 2px;
  margin: -2px;
}
.has-selection .category-option {
  min-height: 40px;
  padding: 10px;
  gap: 7px;
}
.has-selection .category-option > svg:first-child {
  display: none;
}
.category-skeleton {
  height: 52px;
}
.category-no-match {
  font-size: 12px;
  color: var(--text-secondary);
  padding: 23px 0;
  text-align: center;
}
.category-results-heading {
  margin-top: 32px;
}
.category-sort {
  width: 122px;
}
.category-aliases {
  font-size: 10px;
  color: var(--text-secondary);
  line-height: 1.8;
  margin-top: 8px;
}
.category-load-error {
  margin-bottom: 20px;
  justify-content: space-between;
}
@media (max-width: 760px) {
  .category-panel {
    padding: 16px;
  }
  .category-panel-heading {
    flex-wrap: wrap;
    gap: 12px;
  }
  .category-filter {
    width: 100%;
  }
  .category-grid {
    grid-template-columns: repeat(auto-fill, minmax(125px, 1fr));
  }
  .category-results-heading {
    align-items: flex-start;
    flex-wrap: wrap;
  }
}
</style>
