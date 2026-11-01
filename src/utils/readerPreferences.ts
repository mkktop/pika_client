export interface ReadingProgress {
  episode: number
  page: number
}
export function readPreference<T>(key: string, fallback: T): T {
  try {
    const value = localStorage.getItem(key)
    return value ? (JSON.parse(value) as T) : fallback
  } catch {
    return fallback
  }
}
export function writePreference(key: string, value: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(value))
  } catch {
    /* Continue reading when storage is unavailable. */
  }
}
export function readingProgress(comicId: string): ReadingProgress | null {
  const saved = readPreference<ReadingProgress | null>('pika-progress-' + comicId, null)
  return saved && Number.isInteger(saved.episode) && Number.isInteger(saved.page) && saved.page >= 0
    ? saved
    : null
}
