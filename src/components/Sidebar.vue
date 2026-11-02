<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore } from '../stores/app'
import { useComicStore } from '../stores/comic'
import AppIcon from './AppIcon.vue'
import BrandMark from './BrandMark.vue'
const app = useAppStore()
const comics = useComicStore()
const explore = [
  { path: '/', icon: 'compass', label: '发现漫画' },
  { path: '/categories', icon: 'grid', label: '分类浏览' },
  { path: '/search', icon: 'search', label: '搜索与探索' }
]
const personal = [
  { path: '/favorites', icon: 'heart', label: '我的收藏' },
  { path: '/library', icon: 'library', label: '本地书库' },
  { path: '/downloads', icon: 'download', label: '下载管理' }
]
const punching = ref(false)
const punchNotice = ref('')
async function punchIn() {
  if (punching.value) return
  punching.value = true
  punchNotice.value = ''
  try {
    const res = await invoke<{ res?: { punchInLastDay?: string } }>('punch_in')
    await comics.fetchProfile()
    punchNotice.value = res?.res?.punchInLastDay
      ? '已连续签到 ' + res.res.punchInLastDay + ' 天'
      : '签到成功'
  } catch (e) {
    punchNotice.value = '签到失败：' + e
  } finally {
    punching.value = false
    setTimeout(() => (punchNotice.value = ''), 4000)
  }
}
onMounted(() => comics.fetchProfile())
</script>
<template>
  <aside class="sidebar" :class="{ collapsed: app.sidebarCollapsed }">
    <router-link to="/" class="sidebar-brand" title="Pika 首页">
      <BrandMark />
      <span class="brand-type">pika<span>PIKA COMICS</span></span>
    </router-link>
    <div class="sidebar-navigation">
      <div class="nav-group">
        <span class="nav-heading">探索 / EXPLORE</span
        ><router-link
          v-for="item in explore"
          :key="item.path"
          :to="item.path"
          class="nav-item"
          :title="item.label"
          exact-active-class="active"
          ><AppIcon :name="item.icon" /><span>{{ item.label }}</span
          ><span class="nav-active-dot"
        /></router-link>
      </div>
      <div class="nav-group">
        <span class="nav-heading">我的 / COLLECTION</span
        ><router-link
          v-for="item in personal"
          :key="item.path"
          :to="item.path"
          class="nav-item"
          :title="item.label"
          active-class="active"
          ><AppIcon :name="item.icon" /><span>{{ item.label }}</span></router-link
        >
      </div>
    </div>
    <div class="sidebar-note">
      <AppIcon name="sparkles" :size="24" />
      <p>收藏好本，随时开看。</p>
      <span>在线浏览 · 收藏同步 · 本地下载</span>
      <div class="note-line" />
    </div>
    <div class="sidebar-bottom">
      <span v-if="punchNotice" class="punch-notice" role="status">{{ punchNotice }}</span
      ><button
        v-if="comics.profile && !comics.profile.isPunched"
        class="nav-item punch-btn"
        :disabled="punching"
        :title="'每日签到 · 今日还未签到'"
        @click="punchIn"
      >
        <AppIcon name="check" /><span>{{ punching ? '签到中…' : '每日签到' }}</span>
      </button>
      <router-link to="/settings" class="nav-item" active-class="active" title="偏好设置"
        ><AppIcon name="settings" /><span>偏好设置</span></router-link
      ><button
        class="nav-item collapse-btn"
        :title="app.sidebarCollapsed ? '展开导航' : '收起导航'"
        @click="app.toggleSidebar"
      >
        <AppIcon :name="app.sidebarCollapsed ? 'chevron-right' : 'chevron-left'" /><span
          >收起导航</span
        >
      </button>
    </div>
    <router-link to="/settings" class="sidebar-profile" title="账号信息"
      ><img
        v-if="comics.getAvatarUrl(comics.profile?.avatar)"
        :src="comics.getAvatarUrl(comics.profile?.avatar)"
        alt="头像"
        class="avatar" /><span v-else class="avatar"><AppIcon name="user" :size="19" /></span
      ><span class="profile-info"
        ><strong>{{ comics.profile?.name || '漫画爱好者' }}</strong
        ><small>{{
          comics.profile ? 'Lv.' + comics.profile.level + ' · 哔咔账号' : '欢迎回来'
        }}</small></span
      ><AppIcon name="chevron-right" :size="15"
    /></router-link>
  </aside>
</template>
