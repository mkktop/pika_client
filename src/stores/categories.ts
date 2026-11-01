import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import type { Comic } from './comic'
import {
  groupCategories,
  isSupportedCategory,
  mergeCategoryComics,
  type Category,
  type CategoryGroup
} from '../utils/categories'

interface CategoryPage {
  docs: Comic[]
  pages: number
}

export const useCategoryStore = defineStore('categories', {
  state: () => ({
    groups: [] as CategoryGroup[],
    catalogLoaded: false,
    catalogLoading: false,
    catalogError: '',
    catalogRequestId: 0,
    comics: [] as Comic[],
    loading: false,
    loadingMore: false,
    error: '',
    page: 0,
    hasMore: false,
    pagesByAlias: {} as Record<string, number>,
    requestId: 0
  }),
  actions: {
    async fetchCatalog(force = false) {
      if (this.catalogLoaded && !force) return
      const id = ++this.catalogRequestId
      this.catalogLoading = true
      this.catalogError = ''
      try {
        const categories = await invoke<Category[]>('get_categories')
        if (id === this.catalogRequestId) {
          this.groups = groupCategories(categories)
          this.catalogLoaded = true
        }
      } catch (e) {
        if (id === this.catalogRequestId) this.catalogError = String(e)
      } finally {
        if (id === this.catalogRequestId) this.catalogLoading = false
      }
    },
    resetResults() {
      this.requestId++
      this.comics = []
      this.loading = false
      this.loadingMore = false
      this.error = ''
      this.page = 0
      this.hasMore = false
      this.pagesByAlias = {}
    },
    async fetchComics(aliases: string[], sort: string, append = false) {
      if (!append) this.resetResults()
      if (!aliases.length || aliases.some((alias) => !isSupportedCategory(alias))) {
        this.error = '该分类不可用'
        return
      }
      const id = ++this.requestId
      const page = append ? this.page + 1 : 1
      this.loading = true
      this.loadingMore = append
      this.error = ''
      try {
        const remaining = aliases.filter(
          (alias) => page === 1 || page <= (this.pagesByAlias[alias] || 0)
        )
        const results = await Promise.all(
          remaining.map(async (category) => ({
            category,
            data: await invoke<CategoryPage>('get_comics_by_category', { page, category, sort })
          }))
        )
        if (id !== this.requestId) return
        for (const result of results) this.pagesByAlias[result.category] = result.data.pages
        this.comics = mergeCategoryComics(
          [...(append ? this.comics : []), ...results.flatMap((result) => result.data.docs)],
          sort
        )
        this.page = page
        this.hasMore = aliases.some((alias) => page < (this.pagesByAlias[alias] || 0))
      } catch (e) {
        if (id === this.requestId) this.error = String(e)
      } finally {
        if (id === this.requestId) {
          this.loading = false
          this.loadingMore = false
        }
      }
    }
  }
})
