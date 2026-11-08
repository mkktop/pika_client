import { invoke as tauriInvoke, type InvokeArgs } from '@tauri-apps/api/core'

/** 后端会话失效且无法自动重登时，错误字符串统一带此前缀。 */
export const AUTH_EXPIRED_PREFIX = 'AUTH_REQUIRED:'

export const AUTH_EXPIRED_EVENT = 'pika:auth-expired'

/**
 * 统一的命令调用出口：识别后端的会话失效错误，广播事件让壳层引导重新登录，
 * 再抛出去掉前缀的干净文案。其余错误原样透传。
 */
export async function invoke<T>(cmd: string, args?: InvokeArgs): Promise<T> {
  try {
    return await tauriInvoke<T>(cmd, args)
  } catch (e) {
    const message = typeof e === 'string' ? e : e instanceof Error ? e.message : String(e)
    if (message.startsWith(AUTH_EXPIRED_PREFIX)) {
      window.dispatchEvent(new CustomEvent(AUTH_EXPIRED_EVENT))
      throw message.slice(AUTH_EXPIRED_PREFIX.length)
    }
    throw e
  }
}
