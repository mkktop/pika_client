<script setup lang="ts">
import { onMounted, onUnmounted, ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import AppIcon from '../components/AppIcon.vue'
import EmptyState from '../components/EmptyState.vue'
interface DownloadTask {
  id: string
  comic_id: string
  comic_title: string
  episode_title: string
  status: string
  total_images: number
  downloaded_images: number
  save_path: string
}
const tasks = ref<DownloadTask[]>([])
const loading = ref(true)
const refreshing = ref(false)
const error = ref('')
const filter = ref('all')
const activeTasks = computed(() =>
  tasks.value.filter((t) => ['downloading', 'pending', 'paused'].includes(t.status))
)
const completedTasks = computed(() => tasks.value.filter((t) => t.status === 'completed'))
const failedTasks = computed(() => tasks.value.filter((t) => t.status === 'failed'))
const filtered = computed(() =>
  filter.value === 'active'
    ? activeTasks.value
    : filter.value === 'completed'
      ? completedTasks.value
      : filter.value === 'failed'
        ? failedTasks.value
        : tasks.value
)
async function loadTasks() {
  if (refreshing.value) return
  refreshing.value = true
  try {
    tasks.value = await invoke<DownloadTask[]>('get_download_queue')
    error.value = ''
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
    refreshing.value = false
  }
}
const statusText = (s: string) =>
  ({
    pending: '等待中',
    downloading: '下载中',
    paused: '已暂停',
    completed: '已完成',
    failed: '下载失败'
  })[s] || s
const progress = (t: DownloadTask) =>
  t.total_images
    ? Math.max(0, Math.min(100, Math.round((t.downloaded_images / t.total_images) * 100)))
    : 0
let timer: ReturnType<typeof setInterval> | undefined
onMounted(() => {
  loadTasks()
  timer = setInterval(loadTasks, 3000)
})
onUnmounted(() => clearInterval(timer))
</script>
<template>
  <div class="page downloads-view">
    <div class="page-header">
      <div>
        <div class="eyebrow">DOWNLOAD QUEUE</div>
        <h1>下载管理</h1>
        <p>查看下载队列、章节进度与完成状态。</p>
      </div>
      <span class="header-note"><span class="status-dot" />每 3 秒自动更新</span>
    </div>
    <div class="stats-bar">
      <div class="stat-card">
        <span class="stat-icon"><AppIcon name="download" /></span>
        <div>
          <strong class="num">{{ activeTasks.length }}</strong
          ><span class="label">进行中 / 等待中</span>
        </div>
      </div>
      <div class="stat-card">
        <span class="stat-icon"><AppIcon name="check" /></span>
        <div>
          <strong class="num">{{ completedTasks.length }}</strong
          ><span class="label">已完成任务</span>
        </div>
      </div>
      <div class="stat-card">
        <span class="stat-icon"><AppIcon name="alert" /></span>
        <div>
          <strong class="num">{{ failedTasks.length }}</strong
          ><span class="label">未完成任务</span>
        </div>
      </div>
    </div>
    <div class="section-heading">
      <div class="segmented">
        <button
          v-for="tab in [
            { value: 'all', label: '全部任务' },
            { value: 'active', label: '进行中' },
            { value: 'completed', label: '已完成' },
            { value: 'failed', label: '失败' }
          ]"
          :key="tab.value"
          :class="{ active: filter === tab.value }"
          @click="filter = tab.value"
        >
          {{ tab.label }}
        </button>
      </div>
      <router-link to="/library" class="text-btn"
        >打开本地书库 <AppIcon name="arrow" :size="14"
      /></router-link>
    </div>
    <div v-if="loading" class="loading"><span class="spinner" />正在读取下载队列…</div>
    <template v-else
      ><div v-if="error && tasks.length" class="notice error" role="alert">
        {{ error }}<button class="text-btn" @click="loadTasks">刷新</button>
      </div>
      <EmptyState
        v-if="error && !tasks.length"
        error
        title="暂时无法读取下载任务"
        :description="error"
        ><button class="btn btn-primary" @click="loadTasks">重试</button></EmptyState
      ><EmptyState
        v-else-if="!filtered.length"
        icon="download"
        :title="tasks.length ? '这个分类下还没有任务' : '暂无下载任务'"
        :description="
          tasks.length
            ? '切换分类查看其他下载任务。'
            : '在漫画详情的章节列表中选择下载，任务会显示在这里。'
        "
        ><router-link v-if="!tasks.length" to="/" class="btn btn-primary"
          >去发现漫画 <AppIcon name="arrow" :size="15" /></router-link
      ></EmptyState>
      <div v-else class="task-list">
        <article v-for="task in filtered" :key="task.id" class="task-card">
          <span class="task-icon"
            ><AppIcon :name="task.status === 'completed' ? 'check' : 'download'"
          /></span>
          <div class="task-info">
            <div class="task-title">{{ task.comic_title }}</div>
            <p class="task-episode">{{ task.episode_title }}</p>
            <div class="task-path" :title="task.save_path">{{ task.save_path }}</div>
          </div>
          <div class="task-progress">
            <div
              class="progress-bar"
              role="progressbar"
              :aria-label="task.episode_title + ' 下载进度'"
              :aria-valuenow="progress(task)"
              :aria-valuemin="0"
              :aria-valuemax="100"
            >
              <div
                class="progress-fill"
                :style="{
                  width: progress(task) + '%',
                  background:
                    task.status === 'completed'
                      ? 'var(--success)'
                      : task.status === 'failed'
                        ? 'var(--error)'
                        : undefined
                }"
              />
            </div>
            <div class="progress-text">
              <span>{{ task.downloaded_images }} / {{ task.total_images }} 张</span
              ><span>{{ progress(task) }}%</span>
            </div>
          </div>
          <span class="task-status" :class="task.status">{{ statusText(task.status) }}</span>
        </article>
      </div></template
    >
  </div>
</template>
