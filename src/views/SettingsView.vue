<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '../utils/invoke'
import { useAppStore } from '../stores/app'
import { useComicStore, type BlockedTagScope } from '../stores/comic'
import AppIcon from '../components/AppIcon.vue'
import BrandMark from '../components/BrandMark.vue'
import AppUpdates from '../components/AppUpdates.vue'
import { useUpdateStore } from '../stores/updates'
const app = useAppStore()
const updates = useUpdateStore()
const comic = useComicStore()
const router = useRouter()
const blockInput = ref('')
function addBlocked() {
  const value = blockInput.value.trim()
  if (!value) return
  comic.blockTag(value)
  blockInput.value = ''
}
function removeBlocked(tag: string) {
  comic.unblockTag(tag)
}
function toggleScope(scope: BlockedTagScope) {
  comic.setBlockedTagScope(scope, !comic.blockedTagScopes[scope])
}
const downloadDir = ref('')
const proxy = ref('')
const imageQuality = ref('original')
const saving = ref('')
const messages = ref<Record<string, string>>({})
const cacheEnabled = ref(false)
const cacheLimit = ref(256)
const cacheUsed = ref(0)
function formatBytes(n: number): string {
  if (n < 1024) return n + ' B'
  if (n < 1024 * 1024) return (n / 1024).toFixed(1) + ' KB'
  if (n < 1024 ** 3) return (n / 1024 / 1024).toFixed(1) + ' MB'
  return (n / 1024 / 1024 / 1024).toFixed(2) + ' GB'
}
async function loadCacheStatus() {
  try {
    const status = await invoke<{ enabled: boolean; limit_mb: number; used_bytes: number }>(
      'get_image_cache_status'
    )
    cacheEnabled.value = status.enabled
    cacheLimit.value = status.limit_mb
    cacheUsed.value = status.used_bytes
  } catch {
    // 状态拉不到时保持默认展示
  }
}
async function persistCacheSettings(): Promise<boolean> {
  if (saving.value) return false
  saving.value = 'cache'
  messages.value.cache = ''
  try {
    await invoke('set_image_cache_settings', {
      enabled: cacheEnabled.value,
      limitMb: Math.max(16, Math.min(8192, Number(cacheLimit.value) || 256))
    })
    await loadCacheStatus()
    messages.value.cache = cacheEnabled.value ? '封面缓存已启用' : '封面缓存已关闭'
    return true
  } catch (e) {
    messages.value.cache = '保存失败：' + e
    return false
  } finally {
    saving.value = ''
  }
}
async function toggleCache() {
  const next = !cacheEnabled.value
  cacheEnabled.value = next
  if (!(await persistCacheSettings())) cacheEnabled.value = !next
}
async function saveCacheLimit() {
  await persistCacheSettings()
}
async function clearCache() {
  if (saving.value) return
  saving.value = 'cache'
  messages.value.cache = ''
  try {
    const freed = await invoke<number>('clear_image_cache')
    messages.value.cache = '已清空 ' + formatBytes(freed)
    await loadCacheStatus()
  } catch (e) {
    messages.value.cache = '清空失败：' + e
  } finally {
    saving.value = ''
  }
}
const qualityOptions = [
  { value: 'original', label: '原始画质', note: '保留每一处细节' },
  { value: 'lower', label: '均衡画质', note: '清晰与速度兼顾' },
  { value: 'lowest', label: '轻量画质', note: '更快加载，更省流量' }
]
const authStatus = ref<{ logged_in: boolean; saved_account: string | null }>({
  logged_in: false,
  saved_account: null
})
const loggingOut = ref(false)
const confirmLogout = ref(false)
let confirmTimer: number | undefined
async function loadAuthStatus() {
  try {
    authStatus.value = await invoke<{ logged_in: boolean; saved_account: string | null }>(
      'get_auth_status'
    )
  } catch {
    // 状态拉不到时保持默认展示
  }
}
/** 两步确认退出：第一次点击进入确认态，5 秒内再点一次才真正退出。 */
async function handleLogout() {
  if (loggingOut.value) return
  if (!confirmLogout.value) {
    confirmLogout.value = true
    confirmTimer = window.setTimeout(() => (confirmLogout.value = false), 5000)
    return
  }
  window.clearTimeout(confirmTimer)
  loggingOut.value = true
  try {
    await app.logout()
  } finally {
    loggingOut.value = false
    confirmLogout.value = false
    router.push('/login')
  }
}
onMounted(async () => {
  await app.loadDownloadDir()
  downloadDir.value = app.downloadDir
  const results = await Promise.allSettled([
    invoke<string | null>('get_proxy'),
    invoke<string>('get_image_quality'),
    loadAuthStatus(),
    loadCacheStatus()
  ])
  if (results[0].status === 'fulfilled') proxy.value = results[0].value || ''
  if (results[1].status === 'fulfilled') imageQuality.value = results[1].value
})
async function pickDir() {
  if (saving.value) return
  try {
    const dir = await invoke<string | null>('pick_download_dir')
    if (!dir) return
    downloadDir.value = dir
    await save('directory')
  } catch (e) {
    messages.value.directory = '选择目录失败：' + e
  }
}
async function exportHistory() {
  if (saving.value) return
  saving.value = 'history'
  messages.value.history = ''
  try {
    const path = await invoke<string | null>('export_download_history')
    messages.value.history = path
      ? '已导出到 ' + path
      : '已取消导出'
  } catch (e) {
    messages.value.history = '导出失败：' + e
  } finally {
    saving.value = ''
  }
}
async function importHistory() {
  if (saving.value) return
  saving.value = 'history'
  messages.value.history = ''
  try {
    const result = await invoke<string | null>('import_download_history')
    messages.value.history = result || '已取消导入'
  } catch (e) {
    messages.value.history = '导入失败：' + e
  } finally {
    saving.value = ''
  }
}
async function save(kind: string) {
  if (saving.value) return
  saving.value = kind
  messages.value[kind] = ''
  try {
    if (kind === 'directory') {
      if (!downloadDir.value.trim()) throw new Error('请填写下载目录')
      await invoke('set_download_dir', { dir: downloadDir.value.trim() })
      app.downloadDir = downloadDir.value.trim()
    }
    if (kind === 'proxy') await invoke('set_proxy', { proxy: proxy.value.trim() })
    if (kind === 'quality') await invoke('set_image_quality', { quality: imageQuality.value })
    messages.value[kind] =
      kind === 'proxy' ? (proxy.value.trim() ? '代理已启用' : '已切换为直接连接') : '设置已保存'
  } catch (e) {
    messages.value[kind] = '保存失败：' + e
  } finally {
    saving.value = ''
  }
}
</script>
<template>
  <div class="page settings-view">
    <div class="page-header">
      <div>
        <div class="eyebrow">PREFERENCES</div>
        <h1>偏好设置</h1>
        <p>调整界面外观、图片质量、下载目录和网络连接。</p>
      </div>
    </div>
    <div class="settings-layout">
      <div class="settings-form">
        <section class="setting-group">
          <div class="setting-title">
            <AppIcon name="sun" />
            <div>
              <h2>外观与布局</h2>
              <p>选择最舒服的浏览方式</p>
            </div>
          </div>
          <div class="setting-row">
            <div>
              <strong>界面外观</strong>
              <p>阅读器始终使用独立的深色背景。</p>
            </div>
            <div class="segmented">
              <button
                :class="{ active: app.theme === 'light' }"
                @click="app.theme !== 'light' && app.toggleTheme()"
              >
                <AppIcon name="sun" :size="13" /> 浅色</button
              ><button
                :class="{ active: app.theme === 'dark' }"
                @click="app.theme !== 'dark' && app.toggleTheme()"
              >
                <AppIcon name="moon" :size="13" /> 深色
              </button>
            </div>
          </div>
          <div class="setting-row">
            <div>
              <strong>紧凑封面布局</strong>
              <p>在同一屏幕中展示更多漫画。</p>
            </div>
            <button
              class="switch"
              :class="{ on: app.compactCards }"
              role="switch"
              :aria-checked="app.compactCards"
              aria-label="紧凑封面布局"
              @click="app.toggleCompactCards"
            >
              <span />
            </button>
          </div>
        </section>
        <section class="setting-group">
          <div class="setting-title">
            <AppIcon name="image" />
            <div>
              <h2>图片质量</h2>
              <p>浏览和下载共用此设置</p>
            </div>
          </div>
          <div class="quality-options">
            <button
              v-for="opt in qualityOptions"
              :key="opt.value"
              class="quality-option"
              :class="{ active: imageQuality === opt.value }"
              :aria-pressed="imageQuality === opt.value"
              @click="imageQuality = opt.value"
            >
              <strong>{{ opt.label }}</strong
              ><span>{{ opt.note }}</span>
            </button>
          </div>
          <div class="setting-actions">
            <button class="btn btn-secondary btn-sm" :disabled="!!saving" @click="save('quality')">
              {{ saving === 'quality' ? '保存中…' : '保存画质' }}</button
            ><span
              role="status"
              class="setting-message"
              :class="{ error: messages.quality?.includes('失败') }"
              >{{ messages.quality }}</span
            >
          </div>
        </section>
        <section class="setting-group">
          <div class="setting-title">
            <AppIcon name="image" />
            <div>
              <h2>封面缓存</h2>
              <p>封面与头像的本地磁盘缓存，默认关闭</p>
            </div>
          </div>
          <div class="setting-row">
            <div>
              <strong>启用封面缓存</strong>
              <p>关闭时每次从网络加载（仍走代理设置）。</p>
            </div>
            <button
              class="switch"
              :class="{ on: cacheEnabled }"
              role="switch"
              :aria-checked="cacheEnabled"
              aria-label="启用封面缓存"
              @click="toggleCache"
            >
              <span />
            </button>
          </div>
          <div class="setting-row">
            <div>
              <strong>容量上限</strong>
              <p>超出后按最久未使用自动清理，单位 MB。</p>
            </div>
            <div class="cache-limit-field">
              <input
                v-model.number="cacheLimit"
                type="number"
                min="16"
                max="8192"
                step="16"
                class="input cache-limit"
                aria-label="缓存容量上限（MB）"
                :disabled="!cacheEnabled"
              /><span>MB</span>
            </div>
          </div>
          <div class="setting-actions">
            <button
              class="btn btn-secondary btn-sm"
              :disabled="!!saving || !cacheEnabled"
              @click="saveCacheLimit"
            >
              保存上限</button
            ><button class="btn btn-secondary btn-sm" :disabled="!!saving" @click="clearCache">
              清空缓存</button
            ><span role="status" class="setting-message cache-usage"
              >当前占用 {{ formatBytes(cacheUsed) }}</span
            ><span
              role="status"
              class="setting-message"
              :class="{ error: messages.cache?.includes('失败') }"
              >{{ messages.cache }}</span
            >
          </div>
          <p class="hint">
            缓存仅包含封面与头像缩略图（单张超过 8MB 的异常响应不落盘），阅读内容不进入缓存。
          </p>
        </section>
        <section class="setting-group">
          <div class="setting-title">
            <AppIcon name="folder" />
            <div>
              <h2>下载目录</h2>
              <p>设置漫画图片的保存位置</p>
            </div>
          </div>
          <div class="form-field">
            <span class="eyebrow">存储位置</span>
            <div class="dir-field">
              <input v-model="downloadDir" class="input" placeholder="输入本地文件夹路径" />
              <button
                class="btn btn-secondary dir-browse"
                :disabled="!!saving"
                title="打开系统目录选择框"
                @click="pickDir"
              >
                <AppIcon name="folder" :size="14" />浏览…
              </button>
            </div>
          </div>
          <p class="hint">新的下载任务会保存到此目录，已下载的文件保留在原位置。</p>
          <div class="setting-actions">
            <button
              class="btn btn-secondary btn-sm"
              :disabled="!!saving"
              @click="save('directory')"
            >
              {{ saving === 'directory' ? '保存中…' : '保存目录' }}</button
            ><span
              role="status"
              class="setting-message"
              :class="{ error: messages.directory?.includes('失败') }"
              >{{ messages.directory }}</span
            >
          </div>
        </section>
        <section class="setting-group">
          <div class="setting-title">
            <AppIcon name="download" />
            <div>
              <h2>下载记录备份</h2>
              <p>下载历史独立于本地文件，可导出保存，重装后导入恢复</p>
            </div>
          </div>
          <div class="setting-actions">
            <button class="btn btn-secondary btn-sm" :disabled="!!saving" @click="exportHistory">
              {{ saving === 'history' ? '处理中…' : '导出记录' }}</button
            ><button class="btn btn-secondary btn-sm" :disabled="!!saving" @click="importHistory">
              导入记录</button
            ><span
              role="status"
              class="setting-message"
              :class="{ error: messages.history?.includes('失败') }"
              >{{ messages.history }}</span
            >
          </div>
          <p class="hint">
            导出为 JSON 文件；导入时与现有记录合并（章节取并集），不会覆盖或删除已有记录。
          </p>
        </section>
        <section class="setting-group">
          <div class="setting-title">
            <AppIcon name="close" />
            <div>
              <h2>屏蔽标签</h2>
              <p>携带所选标签的漫画会从对应列表中隐藏</p>
            </div>
          </div>
          <div class="setting-row">
            <div>
              <strong>搜索页结果</strong>
              <p>关键词搜索与标签过滤的结果应用屏蔽。</p>
            </div>
            <button
              class="switch"
              :class="{ on: comic.blockedTagScopes.search }"
              role="switch"
              :aria-checked="comic.blockedTagScopes.search"
              aria-label="屏蔽标签作用于搜索页"
              @click="toggleScope('search')"
            >
              <span />
            </button>
          </div>
          <div class="setting-row">
            <div>
              <strong>首页列表</strong>
              <p>排行榜、最新上线与随机推荐应用屏蔽。</p>
            </div>
            <button
              class="switch"
              :class="{ on: comic.blockedTagScopes.home }"
              role="switch"
              :aria-checked="comic.blockedTagScopes.home"
              aria-label="屏蔽标签作用于首页"
              @click="toggleScope('home')"
            >
              <span />
            </button>
          </div>
          <div class="setting-row">
            <div>
              <strong>分类浏览</strong>
              <p>分类与多分类合并的列表应用屏蔽。</p>
            </div>
            <button
              class="switch"
              :class="{ on: comic.blockedTagScopes.categories }"
              role="switch"
              :aria-checked="comic.blockedTagScopes.categories"
              aria-label="屏蔽标签作用于分类浏览"
              @click="toggleScope('categories')"
            >
              <span />
            </button>
          </div>
          <div class="form-field">
            <span class="eyebrow">标签列表</span>
            <div class="blocked-tags">
              <template v-if="comic.blockedTags.length"
                ><button
                  v-for="t in comic.blockedTags"
                  :key="t"
                  class="hot-tag blocked-chip"
                  :title="'点击取消屏蔽「' + t + '」'"
                  @click="removeBlocked(t)"
                >
                  {{ t }} ×
                </button></template
              ><span v-else class="setting-message">尚未屏蔽任何标签</span
              ><input
                v-model="blockInput"
                class="block-input"
                aria-label="添加屏蔽标签"
                placeholder="+ 添加屏蔽标签（回车确认）"
                @keydown.enter.prevent="addBlocked"
              />
            </div>
          </div>
          <p class="hint">
            标签需与漫画携带的标签精确一致；改动在对应列表下次加载时生效（重新搜索或重新进入页面）。
          </p>
        </section>
        <section class="setting-group">
          <div class="setting-title">
            <AppIcon name="network" />
            <div>
              <h2>网络连接</h2>
              <p>配置漫画服务与图片下载的连接方式</p>
            </div>
          </div>
          <label class="form-field"
            ><span class="eyebrow">代理地址</span
            ><input
              v-model="proxy"
              class="input"
              placeholder="http://127.0.0.1:7890 或 socks5://127.0.0.1:7891"
          /></label>
          <p class="hint">支持 HTTP / SOCKS5 代理。留空后应用，即使用直接连接。</p>
          <div class="setting-actions">
            <button class="btn btn-secondary btn-sm" :disabled="!!saving" @click="save('proxy')">
              {{ saving === 'proxy' ? '应用中…' : '应用连接设置' }}</button
            ><span
              role="status"
              class="setting-message"
              :class="{ error: messages.proxy?.includes('失败') }"
              >{{ messages.proxy }}</span
            >
          </div>
        </section>
        <section class="setting-group">
          <div class="setting-title">
            <AppIcon name="user" />
            <div>
              <h2>账号状态</h2>
              <p>你的收藏会跟随哔咔账号同步</p>
            </div>
          </div>
          <span class="header-note"
            ><span class="status-dot" />{{
              app.isLoggedIn
                ? authStatus.saved_account
                  ? `已登录 · 已记住密码（${authStatus.saved_account}），过期自动重登`
                  : '已登录 · 未记住密码，token 过期后需重新登录'
                : '尚未登录 · 本地书库与已下载漫画仍可阅读'
            }}</span
          >
          <div class="setting-actions">
            <button
              v-if="app.isLoggedIn"
              class="btn btn-secondary btn-sm"
              :class="{ danger: confirmLogout }"
              :disabled="loggingOut"
              @click="handleLogout"
            >
              {{ loggingOut ? '退出中…' : confirmLogout ? '再点一次确认退出' : '退出登录' }}</button
            ><button v-else class="btn btn-secondary btn-sm" @click="router.push('/login')">
              去登录
            </button>
          </div>
          <p class="hint">哔咔 token 有效期 7 天；记住密码后过期会自动重登。退出登录会同时清除本机保存的密码。</p>
        </section>
        <AppUpdates />
      </div>
      <aside class="settings-aside">
        <BrandMark />
        <h3>PIKA DESKTOP</h3>
        <p>专注看本的桌面客户端。支持在线浏览、账号收藏、章节下载和阅读进度记忆。</p>
        <small>PIKA DESKTOP{{ updates.currentVersion ? ' · v' + updates.currentVersion : '' }}</small>
      </aside>
    </div>
  </div>
</template>
