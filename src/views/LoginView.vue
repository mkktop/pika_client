<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useAppStore } from '../stores/app'
import AppIcon from '../components/AppIcon.vue'
import ReadingArtwork from '../components/ReadingArtwork.vue'
import BrandMark from '../components/BrandMark.vue'
const router = useRouter()
const app = useAppStore()
const account = ref('')
const password = ref('')
const visible = ref(false)
const error = ref('')
async function handleLogin() {
  if (app.isLoading) return
  if (!account.value.trim() || !password.value) {
    error.value = '请输入账号和密码'
    return
  }
  error.value = ''
  try {
    await app.login(account.value.trim(), password.value)
    password.value = ''
    router.push('/')
  } catch (e) {
    error.value = String(e)
  }
}
</script>
<template>
  <div class="login-page">
    <section class="login-story">
      <div class="login-brand"><BrandMark /><span>pika</span></div>
      <div class="login-story-text">
        <div class="hero-kicker">PIKA · COMICS</div>
        <h1>你的私人<br />本子漫画库。</h1>
        <p>哔咔成人漫画桌面客户端</p>
      </div>
      <ReadingArtwork /><span class="login-story-footer"
        >BROWSE · COLLECT · DOWNLOAD <span>PIKA COMICS · 18+</span></span
      >
    </section>
    <section class="login-panel">
      <button
        class="icon-btn login-theme"
        :title="app.theme === 'light' ? '切换深色外观' : '切换浅色外观'"
        @click="app.toggleTheme"
      >
        <AppIcon :name="app.theme === 'light' ? 'moon' : 'sun'" />
      </button>
      <div class="login-form-wrap">
        <span class="eyebrow">SIGN IN TO PIKA</span>
        <h2>欢迎回来。</h2>
        <p class="login-intro">登录哔咔账号，同步收藏，浏览和下载漫画。</p>
        <form class="login-form" @submit.prevent="handleLogin">
          <div class="form-field">
            <label for="account">账号</label>
            <div class="field-with-icon">
              <AppIcon name="user" :size="17" /><input
                id="account"
                v-model="account"
                class="input"
                autocomplete="username"
                autocapitalize="off"
                :spellcheck="false"
                placeholder="用户名或邮箱"
                :disabled="app.isLoading"
              />
            </div>
          </div>
          <div class="form-field">
            <label for="password">密码</label>
            <div class="field-with-icon">
              <AppIcon name="lock" :size="17" /><input
                id="password"
                v-model="password"
                class="input"
                :type="visible ? 'text' : 'password'"
                autocomplete="current-password"
                placeholder="输入你的密码"
                :disabled="app.isLoading"
              /><button
                type="button"
                class="icon-btn password-toggle"
                :title="visible ? '隐藏密码' : '显示密码'"
                :aria-pressed="visible"
                @click="visible = !visible"
              >
                <AppIcon name="eye" :size="17" />
              </button>
            </div>
          </div>
          <div v-if="error" role="alert" class="notice error">
            <AppIcon name="alert" :size="16" />{{ error }}
          </div>
          <button class="btn btn-primary login-submit" :disabled="app.isLoading">
            <span v-if="app.isLoading" class="spinner" />{{
              app.isLoading ? '正在登录…' : '登录哔咔'
            }}<AppIcon v-if="!app.isLoading" name="arrow" :size="17" />
          </button>
        </form>
        <div class="login-help">
          <AppIcon name="lock" :size="13" /><span>登录状态会保存在本机，下次打开即可阅读。</span>
        </div>
      </div>
      <footer class="login-panel-footer">
        <span>PIKA DESKTOP</span><span>哔咔漫画桌面客户端</span>
      </footer>
    </section>
  </div>
</template>
