import { defineStore } from 'pinia'
import { invoke } from '../utils/invoke'
import { useComicStore } from './comic'
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
    error: '',
    page: 0,
    pages: 0,
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
      this.error = ''
      this.page = 0
      this.pages = 0
      this.pagesByAlias = {}
    },
    async fetchComics(aliases: string[], sort: string, page = 1) {
      if (!aliases.length || aliases.some((alias) => !isSupportedCategory(alias))) {
        this.resetResults()
        this.error = '该分类不可用'
        return
      }
      const id = ++this.requestId
      this.loading = true
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
        this.comics = useComicStore().filterBlocked(
          mergeCategoryComics(results.flatMap((result) => result.data.docs), sort),
          'categories'
        )
        this.page = page
        this.pages = Math.max(0, ...aliases.map((alias) => this.pagesByAlias[alias] || 0))
      } catch (e) {
        if (id === this.requestId) this.error = String(e)
      } finally {
        if (id === this.requestId) {
          this.loading = false
        }
      }
    }
  }
})
