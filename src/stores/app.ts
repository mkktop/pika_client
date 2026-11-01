import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'

function preference(key: string, fallback: string) {
  try {
    return localStorage.getItem(key) || fallback
  } catch {
    return fallback
  }
}
function savePreference(key: string, value: string) {
  try {
    localStorage.setItem(key, value)
  } catch {
    /* Preferences remain usable for this session. */
  }
}

export const useAppStore = defineStore('app', {
  state: () => ({
    isLoggedIn: false,
    isLoading: false,
    downloadDir: '',
    sidebarCollapsed: preference('pika-sidebar', 'false') === 'true',
    theme: preference('pika-theme', 'light') === 'dark' ? 'dark' : 'light',
    compactCards: preference('pika-compact', 'false') === 'true'
  }),
  actions: {
    async checkLogin() {
      try {
        this.isLoggedIn = await invoke<boolean>('is_logged_in')
      } catch {
        this.isLoggedIn = false
      }
      return this.isLoggedIn
    },
    async login(account: string, password: string) {
      this.isLoading = true
      try {
        // The Pica API accepts both username and email in the `email` field.
        await invoke<string>('login', { req: { email: account, password } })
        this.isLoggedIn = true
        return true
      } catch (e) {
        throw e
      } finally {
        this.isLoading = false
      }
    },
    async loadDownloadDir() {
      try {
        this.downloadDir = await invoke<string>('get_download_dir')
      } catch {
        this.downloadDir = ''
      }
    },
    toggleSidebar() {
      this.sidebarCollapsed = !this.sidebarCollapsed
      savePreference('pika-sidebar', String(this.sidebarCollapsed))
    },
    toggleTheme() {
      this.theme = this.theme === 'light' ? 'dark' : 'light'
      savePreference('pika-theme', this.theme)
    },
    toggleCompactCards() {
      this.compactCards = !this.compactCards
      savePreference('pika-compact', String(this.compactCards))
    }
  }
})
