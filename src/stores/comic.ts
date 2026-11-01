import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'

export interface Comic {
  _id: string
  title: string
  author?: string
  pagesCount?: number
  epsCount?: number
  finished?: boolean
  categories: string[]
  thumb?: { path: string; fileServer: string }
  likesCount?: number
  viewsCount?: number
  isFavourite?: boolean
  isLiked?: boolean
  updated_at?: string
  description?: string
}

export interface Episode {
  _id: string
  title: string
  order: number
  updated_at?: string
}

export interface Picture {
  _id: string
  media: { path: string; fileServer: string }
}

export interface CommentItem {
  _id: string
  content: string
  likesCount: number
  commentsCount?: number
  isLiked?: boolean
  _user?: {
    _id: string
    name: string
    level: number
    slogan?: string
    avatar?: { path: string; fileServer: string }
  }
  created_at?: string
}

export interface ProfileUser {
  _id: string
  email?: string
  exp: number
  level: number
  name?: string
  title?: string
  isPunched?: boolean
  slogan?: string
  avatar?: { path: string; fileServer: string }
}

export interface Collection {
  _id: string
  title: string
  comics: Comic[]
}

export const useComicStore = defineStore('comic', {
  state: () => ({
    leaderboard: [] as Comic[],
    leaderboardPeriod: 'H24',
    latest: [] as Comic[],
    latestPage: 1,
    latestHasMore: false,
    favourites: [] as Comic[],
    favouriteSort: 'dd',
    favouritePage: 1,
    favouritePages: 1,
    randomComics: [] as Comic[],
    recommendations: [] as Comic[],
    keywords: [] as string[],
    collections: [] as Collection[],
    comments: [] as CommentItem[],
    commentPage: 1,
    commentPages: 1,
    searchResults: [] as Comic[],
    searchHasMore: false,
    currentComic: null as Comic | null,
    episodes: [] as Episode[],
    pictures: [] as Picture[],
    profile: null as ProfileUser | null,
    loading: false,
    requestId: 0,
    recommendationRequestId: 0,
    commentRequestId: 0,
    error: null as string | null
  }),
  actions: {
    cancelRequests() {
      this.requestId++
      this.recommendationRequestId++
      this.commentRequestId++
      this.loading = false
      this.error = null
    },
    async loadResource<T>(request: () => Promise<T>, apply: (data: T) => void) {
      const id = ++this.requestId
      this.loading = true
      this.error = null
      try {
        const data = await request()
        if (id === this.requestId) apply(data)
      } catch (e) {
        if (id === this.requestId) this.error = String(e)
      } finally {
        if (id === this.requestId) this.loading = false
      }
    },
    async fetchLeaderboard(period?: string) {
      this.leaderboardPeriod = period ?? this.leaderboardPeriod
      await this.loadResource(
        () => invoke<Comic[]>('get_leaderboard', { tt: this.leaderboardPeriod }),
        (data) => {
          this.leaderboard = data
        }
      )
    },
    async fetchRandom() {
      await this.loadResource(
        () => invoke<Comic[]>('get_random_comics'),
        (data) => {
          this.randomComics = data
        }
      )
    },
    async fetchRecommendations(comicId: string) {
      const id = ++this.recommendationRequestId
      try {
        const data = await invoke<Comic[]>('get_recommendations', { comicId })
        if (id === this.recommendationRequestId) this.recommendations = data
      } catch {
        if (id === this.recommendationRequestId) this.recommendations = []
      }
    },
    async fetchKeywords() {
      try {
        this.keywords = await invoke<string[]>('get_keywords')
      } catch {
        this.keywords = []
      }
    },
    async fetchCollections() {
      await this.loadResource(
        () => invoke<Collection[]>('get_collections'),
        (data) => {
          this.collections = data
        }
      )
    },
    async fetchComments(comicId: string, page = 1) {
      const id = ++this.commentRequestId
      try {
        const data = await invoke<{ docs: CommentItem[]; pages: number }>('get_comic_comments', {
          comicId,
          page
        })
        if (id !== this.commentRequestId) return
        this.comments = page === 1 ? data.docs : this.comments.concat(data.docs)
        this.commentPage = page
        this.commentPages = data.pages
      } catch {
        if (id === this.commentRequestId && page === 1) {
          this.comments = []
          this.commentPage = 1
          this.commentPages = 1
        }
      }
    },
    async likeComic(comicId: string) {
      await invoke('like_comic', { comicId })
      await this.fetchComic(comicId)
    },
    async fetchProfile() {
      try {
        this.profile = await invoke<ProfileUser>('get_my_profile')
      } catch {
        this.profile = null
      }
    },
    async fetchLatest(page = 1) {
      await this.loadResource(
        () => invoke<{ docs: Comic[]; pages: number }>('get_latest_comics', { page }),
        (data) => {
          this.latest =
            page === 1
              ? data.docs
              : [
                  ...this.latest,
                  ...data.docs.filter(
                    (comic) => !this.latest.some((existing) => existing._id === comic._id)
                  )
                ]
          this.latestPage = page
          this.latestHasMore = page < data.pages
        }
      )
    },
    async fetchFavourites(page = 1, sort?: string) {
      this.favouriteSort = sort ?? this.favouriteSort
      await this.loadResource(
        () =>
          invoke<{ docs: Comic[]; pages: number }>('get_my_favourites', {
            page,
            sort: this.favouriteSort
          }),
        (data) => {
          this.favourites = data.docs
          this.favouritePage = page
          this.favouritePages = data.pages || 1
        }
      )
    },
    async search(keyword: string, page: number, sort: string) {
      if (page === 1) this.searchResults = []
      await this.loadResource(
        () =>
          invoke<{ docs: Comic[]; pages?: number }>('search_comics', {
            req: { keyword, page, sort }
          }),
        (data) => {
          this.searchResults =
            page === 1
              ? data.docs
              : [
                  ...this.searchResults,
                  ...data.docs.filter(
                    (comic) => !this.searchResults.some((existing) => existing._id === comic._id)
                  )
                ]
          this.searchHasMore = data.pages ? page < data.pages : data.docs.length > 0
        }
      )
    },
    async fetchComic(id: string) {
      if (this.currentComic?._id !== id) {
        this.currentComic = null
        this.episodes = []
      }
      await this.loadResource(
        () =>
          Promise.all([
            invoke<Comic>('get_comic_info', { comicId: id }),
            invoke<Episode[]>('get_episodes', { comicId: id })
          ]),
        ([comic, episodes]) => {
          this.currentComic = comic
          this.episodes = episodes
        }
      )
    },
    async fetchPictures(comicId: string, epOrder: number) {
      this.pictures = []
      await this.loadResource(
        () => invoke<Picture[]>('get_pictures', { comicId, epOrder }),
        (data) => {
          this.pictures = data
        }
      )
    },
    getImageUrl(pic: Picture | { media: { path: string; fileServer: string } }): string {
      const path = pic.media.path
      if (path.startsWith('http://') || path.startsWith('https://')) return path
      return pic.media.fileServer.replace(/\/$/, '') + '/static/' + path
    },
    getThumbUrl(comic: Comic): string {
      if (!comic.thumb) return ''
      const path = comic.thumb.path
      if (path.startsWith('http://') || path.startsWith('https://')) return path
      return comic.thumb.fileServer.replace(/\/$/, '') + '/static/' + path
    },
    getAvatarUrl(avatar?: { path: string; fileServer: string }): string {
      if (!avatar) return ''
      const path = avatar.path
      if (path.startsWith('http://') || path.startsWith('https://')) return path
      return avatar.fileServer.replace(/\/$/, '') + '/static/' + path
    }
  }
})
