import { Converter } from 'opencc-js/t2cn'
import type { Comic } from '../stores/comic'

const toSimplified = Converter({ from: 'tw', to: 'cn' })

export interface Category {
  _id: string
  title: string
  description?: string
  isWeb?: boolean
  active?: boolean
}
export interface CategoryGroup {
  key: string
  title: string
  aliases: string[]
}

export function categoryTitle(value: string) {
  return toSimplified(value.normalize('NFKC')).trim().replace(/\s+/g, ' ')
}
export function categoryKey(value: string) {
  return categoryTitle(value).toLowerCase()
}

// Adult browsing excludes categories describing sexualized minors.
export function isSupportedCategory(value: string) {
  const key = categoryKey(value).replace(/[\s\p{P}\p{S}]/gu, '')
  const excluded = ['没有']
  return !!key && !excluded.some((term) => key.includes(term)) && !['jk', 'jc', 'js'].includes(key)
}

export function groupCategories(categories: Category[]): CategoryGroup[] {
  const groups = new Map<string, CategoryGroup>()
  for (const category of categories) {
    if (category.isWeb || category.active === false || !isSupportedCategory(category.title))
      continue
    const title = category.title.trim()
    const key = categoryKey(title)
    const group = groups.get(key)
    if (group) {
      if (!group.aliases.includes(title)) group.aliases.push(title)
    } else groups.set(key, { key, title: categoryTitle(title), aliases: [title] })
  }
  return [...groups.values()]
}

export function mergeCategoryComics(comics: Comic[], sort: string): Comic[] {
  const unique = [
    ...new Map(
      comics
        .filter((comic) => comic.categories.every(isSupportedCategory))
        .map((comic) => [comic._id, comic])
    ).values()
  ]
  return unique.sort((a, b) => {
    if (sort === 'ld') return (b.likesCount || 0) - (a.likesCount || 0)
    if (sort === 'vd') return (b.viewsCount || 0) - (a.viewsCount || 0)
    const first = Date.parse(a.updated_at || '')
    const second = Date.parse(b.updated_at || '')
    return Number.isFinite(first) && Number.isFinite(second)
      ? sort === 'da'
        ? first - second
        : second - first
      : 0
  })
}
