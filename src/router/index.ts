import { createRouter, createWebHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'
import { useAppStore } from '../stores/app'

const routes: RouteRecordRaw[] = [
  {
    path: '/login',
    name: 'login',
    component: () => import('../views/LoginView.vue')
  },
  {
    path: '/categories',
    name: 'categories',
    component: () => import('../views/CategoriesView.vue'),
    meta: { requiresAuth: true }
  },
  {
    path: '/',
    name: 'home',
    component: () => import('../views/HomeView.vue'),
    meta: { requiresAuth: true }
  },
  {
    path: '/search',
    name: 'search',
    component: () => import('../views/SearchView.vue'),
    meta: { requiresAuth: true }
  },
  {
    path: '/comic/:id',
    name: 'comic-detail',
    component: () => import('../views/ComicDetailView.vue')
  },
  {
    path: '/reader/:comicId/:epOrder',
    name: 'reader',
    component: () => import('../views/ReaderView.vue')
  },
  {
    path: '/favorites',
    name: 'favorites',
    component: () => import('../views/FavoritesView.vue'),
    meta: { requiresAuth: true }
  },
  {
    path: '/downloads',
    name: 'downloads',
    component: () => import('../views/DownloadsView.vue')
  },
  {
    path: '/library',
    name: 'library',
    component: () => import('../views/LibraryView.vue')
  },
  {
    path: '/settings',
    name: 'settings',
    component: () => import('../views/SettingsView.vue')
  }
]

const router = createRouter({
  history: createWebHistory(),
  routes
})

/**
 * 登录门禁只拦纯在线页面；本地书库、下载管理、阅读器、设置离线可用。
 * 未登录的默认落地页是本地书库（纯本地数据），登录页留有跳转入口。
 */
router.beforeEach((to) => {
  const auth = useAppStore()
  if (to.name === 'login') return auth.isLoggedIn ? { path: '/' } : true
  // 未登录不落在线首页，改去本地书库。
  if (!auth.isLoggedIn && to.path === '/') return { path: '/library' }
  if (to.meta.requiresAuth && !auth.isLoggedIn) {
    return { path: '/login', query: { redirect: to.fullPath } }
  }
  return true
})

export default router
