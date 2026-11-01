<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore } from '../stores/app'
import AppIcon from '../components/AppIcon.vue'
import BrandMark from '../components/BrandMark.vue'
const app = useAppStore()
const downloadDir = ref('')
const proxy = ref('')
const imageQuality = ref('original')
const saving = ref('')
const messages = ref<Record<string, string>>({})
const qualityOptions = [
  { value: 'original', label: '原始画质', note: '保留每一处细节' },
  { value: 'lower', label: '均衡画质', note: '清晰与速度兼顾' },
  { value: 'lowest', label: '轻量画质', note: '更快加载，更省流量' }
]
onMounted(async () => {
  await app.loadDownloadDir()
  downloadDir.value = app.downloadDir
  const results = await Promise.allSettled([
    invoke<string | null>('get_proxy'),
    invoke<string>('get_image_quality')
  ])
  if (results[0].status === 'fulfilled') proxy.value = results[0].value || ''
  if (results[1].status === 'fulfilled') imageQuality.value = results[1].value
})
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
            <AppIcon name="folder" />
            <div>
              <h2>下载目录</h2>
              <p>设置漫画图片的保存位置</p>
            </div>
          </div>
          <label class="form-field"
            ><span class="eyebrow">存储位置</span
            ><input v-model="downloadDir" class="input" placeholder="输入本地文件夹路径"
          /></label>
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
              app.isLoggedIn ? '已登录 · 登录状态保存在本机' : '尚未登录'
            }}</span
          >
        </section>
      </div>
      <aside class="settings-aside">
        <BrandMark />
        <h3>PIKA DESKTOP</h3>
        <p>专注看本的桌面客户端。支持在线浏览、账号收藏、章节下载和阅读进度记忆。</p>
        <small>PIKA DESKTOP · v0.1.0</small>
      </aside>
    </div>
  </div>
</template>
