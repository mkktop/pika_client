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
  tags?: string[]
  thumb?: { path: string; fileServer: string }
  likesCount?: number
  viewsCount?: number
  isFavourite?: boolean
  isLiked?: boolean
  updated_at?: string
  created_at?: string
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

export interface PicturePage {
  docs: Picture[]
  total?: number
  pages?: number
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

export const useComicStore = defineStore('comic', {
  state: () => ({
    leaderboard: [] as Comic[],
    leaderboardPeriod: 'H24',
    latest: [] as Comic[],
    latestPage: 1,
    latestPages: 1,
    favourites: [] as Comic[],
    favouriteSort: 'dd',
    favouritePage: 1,
    favouritePages: 1,
    randomComics: [] as Comic[],
    recommendations: [] as Comic[],
    keywords: [] as string[],
    comments: [] as CommentItem[],
    commentPage: 1,
    commentPages: 1,
    searchResults: [] as Comic[],
    searchPage: 1,
    searchPages: 1,
    currentComic: null as Comic | null,
    episodes: [] as Episode[],
    pictures: [] as Picture[],
    pictureTotal: 0,
    picturePages: 0,
    pictureCursor: 0,
    fetchingPictures: false,
    picturesError: null as string | null,
    historyIds: new Set<string>(),
    localIds: new Set<string>(),
    downloadStatesFetchedAt: 0,
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
    async fetchDownloadStates(force = false) {
      // 卡片徽标数据：节流拉取，路由切换时反复调用也不产生额外查询。
      if (!force && Date.now() - this.downloadStatesFetchedAt < 15000) return
      this.downloadStatesFetchedAt = Date.now()
      try {
        const sets = await invoke<{ history: string[]; local: string[] }>('get_download_state_sets')
        this.historyIds = new Set(sets.history)
        this.localIds = new Set(sets.local)
      } catch {
        // 状态集合拉不到时不影响列表本身
      }
    },
    async fetchComments(comicId: string, page = 1) {
      const id = ++this.commentRequestId
      try {
        const data = await invoke<{ docs: CommentItem[]; pages: number }>('get_comic_comments', {
          comicId,
          page
        })
        if (id !== this.commentRequestId) return
        this.comments = data.docs
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
          this.latest = data.docs
          this.latestPage = page
          this.latestPages = data.pages || page
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
          this.searchResults = data.docs
          this.searchPage = page
          this.searchPages = data.pages || (data.docs.length ? page + 1 : page)
        }
      )
    },
    // 标签精确过滤（t= 链路）：结果与关键词搜索共用一套分页状态。
    async searchByTag(tag: string, page: number, sort: string) {
      if (page === 1) this.searchResults = []
      await this.loadResource(
        () =>
          invoke<{ docs: Comic[]; pages?: number }>('get_comics_by_tag', {
            page,
            tag,
            sort
          }),
        (data) => {
          this.searchResults = data.docs
          this.searchPage = page
          this.searchPages = data.pages || (data.docs.length ? page + 1 : page)
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
      const id = ++this.requestId
      this.pictures = []
      this.pictureTotal = 0
      this.picturePages = 0
      this.pictureCursor = 0
      this.fetchingPictures = false
      this.picturesError = null
      this.loading = true
      this.error = null
      try {
        // 首批先上屏，后续 API 分页在后台继续追加，长章节不用等全量。
        const first = await invoke<PicturePage>('get_pictures_page', { comicId, epOrder, page: 1 })
        if (id !== this.requestId) return
        this.pictures = first.docs
        this.pictureTotal = first.total || first.docs.length
        this.picturePages = Math.max(1, first.pages || 1)
        this.pictureCursor = 1
        this.loading = false
        if (this.picturePages > 1) await this.continuePictures(comicId, epOrder)
      } catch (e) {
        if (id === this.requestId) {
          this.error = String(e)
          this.loading = false
        }
      }
    },
    async continuePictures(comicId: string, epOrder: number) {
      const id = this.requestId
      if (this.fetchingPictures || this.pictureCursor >= this.picturePages) return
      this.picturesError = null
      this.fetchingPictures = true
      try {
        while (this.pictureCursor < this.picturePages) {
          const page = this.pictureCursor + 1
          const next = await invoke<PicturePage>('get_pictures_page', { comicId, epOrder, page })
          if (id !== this.requestId) return
          this.pictures = this.pictures.concat(next.docs)
          this.pictureCursor = page
        }
        this.fetchingPictures = false
      } catch (e) {
        if (id === this.requestId) {
          this.fetchingPictures = false
          this.picturesError = String(e)
        }
      }
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
