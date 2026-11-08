import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import router from './router'
import { useAppStore } from './stores/app'
import './style.css'
import './styles/pages.css'

async function bootstrap() {
  const pinia = createPinia()
  const app = createApp(App)
  app.use(pinia)
  // 先确认登录态再做首次路由跳转，避免守卫在 is_logged_in 返回前误判。
  await useAppStore(pinia).checkLogin()
  app.use(router)
  app.mount('#app')
}

void bootstrap()
