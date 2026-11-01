<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useComicStore } from '../stores/comic'
import ComicCard from '../components/ComicCard.vue'
import ComicSkeleton from '../components/ComicSkeleton.vue'
import EmptyState from '../components/EmptyState.vue'
import AppIcon from '../components/AppIcon.vue'
import Paginator from '../components/Paginator.vue'
const router = useRouter()
const store = useComicStore()
const resultsTop = ref<HTMLElement>()
function goToPage(page: number) {
  resultsTop.value?.scrollIntoView({ behavior: 'instant', block: 'start' })
  store.fetchFavourites(page)
}
onMounted(() => store.fetchFavourites())
</script>
<template>
  <div class="page favourites-view">
    <div class="page-header">
      <div>
        <div class="eyebrow">YOUR PERSONAL COLLECTION</div>
        <h1>我的收藏</h1>
        <p>管理哔咔账号中收藏的漫画。</p>
      </div>
      <div class="segmented">
        <button
          :class="{ active: store.favouriteSort === 'dd' }"
          :disabled="store.loading"
          @click="store.fetchFavourites(1, 'dd')"
        >
          最近收藏</button
        ><button
          :class="{ active: store.favouriteSort === 'da' }"
          :disabled="store.loading"
          @click="store.fetchFavourites(1, 'da')"
        >
          最早收藏
        </button>
      </div>
    </div>
    <div ref="resultsTop" class="collection-banner">
      <span class="collection-banner-icon"><AppIcon name="heart" :size="24" /></span>
      <div>
        <strong>我的收藏书架</strong>
        <p>按收藏时间查看，快速找到想重看的本子。</p>
      </div>
      <span v-if="!store.loading" class="collection-count"
        >{{ store.favourites.length }} <small>部作品 · 当前页</small></span
      >
    </div>
    <ComicSkeleton v-if="store.loading" /><EmptyState
      v-else-if="store.error"
      error
      title="收藏暂时没有加载成功"
      :description="store.error"
      ><button class="btn btn-primary" @click="store.fetchFavourites()">
        重新加载
      </button></EmptyState
    ><EmptyState
      v-else-if="!store.favourites.length"
      title="还没有收藏漫画"
      description="在漫画详情页点击收藏，就能在这里找到它。"
      icon="heart"
      ><router-link to="/" class="btn btn-primary"
        >去找本子 <AppIcon name="arrow" :size="15" /></router-link></EmptyState
    ><template v-else
      ><div class="comic-grid">
        <ComicCard
          v-for="comic in store.favourites"
          :key="comic._id"
          :comic="comic"
          @click="router.push('/comic/' + comic._id)"
        />
      </div>
      <Paginator
        :page="store.favouritePage"
        :pages="store.favouritePages"
        :loading="store.loading"
        @change="goToPage"
      /></template
    >
  </div>
</template>
