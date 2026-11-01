import { test, expect, type Page } from '@playwright/test'

async function capture(page: Page, name: string) {
  await expect(page.locator('.fade-enter-active, .fade-leave-active')).toHaveCount(0)
  const main = page.locator('.main-content')
  if (await main.count()) await main.evaluate((el) => el.scrollTo({ top: 0, behavior: 'instant' }))
  await page.screenshot({ animations: 'disabled', path: 'artifacts/ui/' + name + '.png' })
}

async function fixture(page: Page, loggedIn = true) {
  await page.route('https://pika.test/**', (route) => {
    const n = Number(new URL(route.request().url()).pathname.split('/').pop()) || 1
    const colors = ['#a3b79d', '#dbb18e', '#8ea7a0', '#b5a68c', '#a6ad87', '#cf9e8d']
    return route.fulfill({
      contentType: 'image/svg+xml',
      body:
        '<svg xmlns="http://www.w3.org/2000/svg" width="600" height="800"><rect width="600" height="800" fill="' +
        colors[n % colors.length] +
        '"/><circle cx="400" cy="230" r="140" fill="#f3e6ca"/><path d="M0 590Q130 260 390 610T700 500V800H0" fill="#405e50"/><text x="45" y="105" font-family="Georgia" font-size="46" fill="#fff9ec">STORY No. ' +
        n +
        '</text><text x="45" y="740" font-family="Georgia" font-size="21" fill="#fff9ec">A LITTLE ESCAPE · PIKA</text></svg>'
    })
  })
  await page.addInitScript(
    ({ loggedIn }) => {
      const comics = Array.from({ length: 12 }, (_, i) => ({
        _id: 'comic-' + i,
        title:
          ['雨后的书店', '山海之间', '漫长的午后', '森林来信', '直到星星亮起', '风经过的地方'][
            i % 6
          ] + (i > 5 ? ' · 续篇' : ''),
        author: ['青木', '林间', '白川'][i % 3],
        pagesCount: 36 + i * 8,
        epsCount: 3,
        finished: i % 2 === 0,
        categories: ['日常', '故事'],
        likesCount: 1250 + i * 532,
        thumb: { path: 'https://pika.test/cover/' + i, fileServer: '' },
        description:
          '在生活的留白里，遇见那些微小而明亮的瞬间。\n一个关于相遇、告别与重新出发的故事。'
      }))
      const state = window as any
      state.__calls = []
      state.__fail = ''
      state.isTauri = true
      state.__windowState = { maximized: false, minimized: false, closed: false }
      state.__TAURI_INTERNALS__ = {
        metadata: { currentWindow: { label: 'main' } },
        invoke: async (cmd: string, args: any = {}) => {
          state.__calls.push({ cmd, args })
          if (state.__fail === cmd) throw '连接暂时不可用，请稍后重试'
          switch (cmd) {
            case 'plugin:window|is_maximized':
              return state.__windowState.maximized
            case 'plugin:window|toggle_maximize':
              state.__windowState.maximized = !state.__windowState.maximized
              window.dispatchEvent(new Event('resize'))
              return
            case 'plugin:window|minimize':
              state.__windowState.minimized = true
              return
            case 'plugin:window|close':
              state.__windowState.closed = true
              return
            case 'is_logged_in':
              return loggedIn
            case 'login':
              return 'test-only'
            case 'get_my_profile':
              return { _id: 'reader', name: '故事收藏家', level: 12 }
            case 'get_download_dir':
              return 'D:/Books/Pika'
            case 'get_proxy':
              return ''
            case 'get_image_quality':
              return 'original'
            case 'get_leaderboard':
            case 'get_random_comics':
              return comics
            case 'get_latest_comics':
              return { docs: comics, page: 1, pages: 1 }
            case 'get_collections':
              return [{ _id: 'collection-1', title: '慢下来的日常', comics: comics.slice(0, 5) }]
            case 'get_keywords':
              return ['日常', '冒险', '治愈', '奇幻']
            case 'get_categories':
              return [
                { _id: 'c1', title: '漢化' },
                { _id: 'c2', title: '汉化' },
                { _id: 'c3', title: '原創' },
                { _id: 'c4', title: '原创' },
                { _id: 'c5', title: '日常' },
                { _id: 'c6', title: '完结' },
                { _id: 'excluded', title: '萝莉' },
                { _id: 'web', title: '外部活动', isWeb: true },
                { _id: 'inactive', title: '已下架分类', active: false }
              ]
            case 'get_comics_by_category':
              if (args.category === '漢化')
                return {
                  docs: args.page === 1 ? comics.slice(0, 2) : [comics[3], comics[0]],
                  pages: 2
                }
              if (args.category === '汉化') return { docs: comics.slice(1, 3), pages: 1 }
              if (args.category === '原創' || args.category === '原创')
                return { docs: comics.slice(4, 6), pages: 1 }
              return { docs: comics.slice(0, 4), pages: 1 }
            case 'search_comics':
              return args.req.keyword === '空结果'
                ? { docs: [], pages: 1 }
                : { docs: args.req.page === 1 ? comics.slice(0, 4) : comics.slice(4, 7), pages: 2 }
            case 'get_my_favourites':
              return { docs: args.page === 1 ? comics.slice(0, 4) : comics.slice(4, 7), pages: 2 }
            case 'get_comic_info':
              return comics.find((c) => c._id === args.comicId) || comics[0]
            case 'get_episodes':
              return [1, 2, 3].map((order) => ({
                _id: 'ep-' + order,
                order,
                title: ['相遇的那一天', '风从远方来', '故事仍在继续'][order - 1]
              }))
            case 'get_recommendations':
              return comics.slice(1, 6)
            case 'get_comic_comments':
              return {
                docs: [
                  {
                    _id: 'comment',
                    content: '很喜欢这个安静的故事。',
                    likesCount: 8,
                    created_at: '2026-11-01',
                    _user: { name: '林间的风', level: 6 }
                  }
                ],
                pages: 1
              }
            case 'get_pictures':
              return Array.from({ length: 5 }, (_, i) => ({
                _id: 'pic-' + i,
                media: { path: 'https://pika.test/page/' + i, fileServer: '' }
              }))
            case 'get_library':
              return []
            case 'get_library_stats':
              return { total_comics: 0, total_episodes: 0, total_images: 0 }
            case 'get_download_queue':
              return [
                {
                  id: 'task',
                  comic_title: comics[0].title,
                  episode_title: '相遇的那一天',
                  status: 'downloading',
                  total_images: 50,
                  downloaded_images: 25,
                  save_path: 'D:/Books/Pika/雨后的书店'
                }
              ]
            default:
              return null
          }
        }
      }
    },
    { loggedIn }
  )
}

test('discovery, appearance persistence, collapsed navigation and minimum window', async ({
  page
}) => {
  await fixture(page)
  await page.goto('/')
  await page.waitForFunction(() =>
    (window as any).__calls?.some((c: any) => c.cmd === 'get_leaderboard')
  )
  await expect(page.locator('.comic-card')).toHaveCount(12)
  await page.getByTitle('切换深色外观').click()
  await expect(page.locator('.app-container')).toHaveAttribute('data-theme', 'dark')
  await page.reload()
  await expect(page.locator('.app-container')).toHaveAttribute('data-theme', 'dark')
  await page.getByTitle('收起导航').click()
  await expect(page.locator('.sidebar')).toHaveClass(/collapsed/)
  await page.setViewportSize({ width: 960, height: 640 })
  await expect(page.locator('.global-search')).toBeVisible()
  expect(
    await page.locator('.workspace').evaluate((el) => el.scrollWidth <= el.clientWidth)
  ).toBeTruthy()
  await page.getByTitle('切换浅色外观').click()
  await page.getByTitle('展开导航').click()
  await page.setViewportSize({ width: 1280, height: 820 })
  await capture(page, 'home')
  await page.getByRole('button', { name: '精选合集', exact: true }).click()
  await expect(page.getByRole('heading', { name: /慢下来的日常/ })).toBeVisible()
})

test('global search, appended pages, sort reset and empty state', async ({ page }) => {
  await fixture(page)
  await page.goto('/')
  await page.getByLabel('全局搜索').fill('日常')
  await page.getByLabel('全局搜索').press('Enter')
  await expect(page.locator('.comic-card')).toHaveCount(4)
  await page.getByRole('button', { name: '加载更多作品' }).click()
  await expect(page.locator('.comic-card')).toHaveCount(7)
  await expect(page.getByText('已经看到全部结果了')).toBeVisible()
  await page.getByLabel('搜索排序').selectOption('ld')
  await expect(page.locator('.comic-card')).toHaveCount(4)
  await page.getByLabel('搜索关键词').fill('空结果')
  await page.getByRole('button', { name: '搜索漫画' }).click()
  await expect(page.getByText('没有找到相关漫画')).toBeVisible()
})

test('favorites pages and download refresh preserve the list', async ({ page }) => {
  await fixture(page)
  await page.goto('/favorites')
  await expect(page.locator('.comic-card')).toHaveCount(4)
  await page.getByRole('button', { name: '下一页', exact: true }).click()
  await expect(page.locator('.comic-card')).toHaveCount(3)
  await page.goto('/downloads')
  await expect(page.locator('.task-card')).toHaveCount(1)
  await page.waitForFunction(
    () => (window as any).__calls.filter((c: any) => c.cmd === 'get_download_queue').length >= 2
  )
  await expect(page.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '50')
  await page.getByRole('button', { name: '已完成', exact: true }).click()
  await expect(page.getByText('这个分类下还没有任务')).toBeVisible()
})

test('settings save actual field values and local library empty state', async ({ page }) => {
  await fixture(page)
  await page.goto('/settings')
  await page.getByRole('switch', { name: '紧凑封面布局' }).click()
  await expect(page.locator('.app-container')).toHaveClass(/compact-cards/)
  await page.getByRole('button', { name: '均衡画质' }).click()
  await page.getByRole('button', { name: '保存画质' }).click()
  await expect(page.getByText('设置已保存').first()).toBeVisible()
  const quality = await page.evaluate(() =>
    (window as any).__calls.find((c: any) => c.cmd === 'set_image_quality')
  )
  expect(quality.args.quality).toBe('lower')
  await capture(page, 'settings')
  await page.goto('/library')
  await expect(page.getByText('本地书库为空')).toBeVisible()
})

test('reader paging, zen mode, saved page and chapter navigation', async ({ page }) => {
  await fixture(page)
  await page.goto('/comic/comic-0')
  await expect(page.locator('.episode-card')).toHaveCount(3)
  await capture(page, 'detail')
  await page.getByRole('button', { name: '开始阅读' }).click()
  await expect(page.locator('.scroll-image')).toHaveCount(5)
  await page.getByRole('button', { name: '翻页', exact: true }).click()
  await page.keyboard.press('ArrowRight')
  await expect(page.locator('.reader-footer')).toContainText('2 / 5 页')
  await page.keyboard.press('f')
  await expect(page.locator('.reader-view')).toHaveClass(/zen/)
  await page.keyboard.press('Escape')
  await expect(page.locator('.reader-view')).not.toHaveClass(/zen/)
  await page.reload()
  await expect(page.locator('.reader-footer')).toContainText('2 / 5 页')
  await expect(page.locator('.reader-view')).toBeVisible()
  await expect
    .poll(() => page.locator('.reader-view').evaluate((el) => getComputedStyle(el).opacity))
    .toBe('1')
  await capture(page, 'reader')
  await page.getByRole('button', { name: '滚动', exact: true }).click()
  await page.getByLabel('跳转阅读页码').fill('3')
  await expect
    .poll(() => page.locator('.scroll-container').evaluate((el) => el.scrollTop))
    .toBeGreaterThan(100)
  await page.getByRole('button', { name: '返回作品', exact: true }).click()
  await expect(page.getByRole('button', { name: '继续阅读 · 第 1 话' })).toBeVisible()
})

for (const origin of ['/', '/search?q=日常', '/favorites', '/categories?category=日常']) {
  test(`returning from the reader returns to the original list: ${origin}`, async ({ page }) => {
    await fixture(page)
    await page.goto(origin)
    await page.locator('.comic-card').first().click()
    await page.getByRole('button', { name: '开始阅读' }).click()
    await expect(page.locator('.reader-view')).toBeVisible()
    await page.reload()
    await page.getByRole('button', { name: '返回作品', exact: true }).click()
    await expect(page).toHaveURL(/\/comic\/comic-0$/)
    await page.getByRole('button', { name: '返回上一页', exact: true }).click()
    await expect(page).toHaveURL('http://127.0.0.1:1420' + origin)
    await expect(page.locator('.comic-card').first()).toBeVisible()
  })
}

test('switching chapters keeps the original detail as the reader return target', async ({
  page
}) => {
  await fixture(page)
  await page.goto('/favorites')
  await page.locator('.comic-card').first().click()
  await page.getByRole('button', { name: '开始阅读' }).click()
  await page.getByRole('button', { name: '下一章', exact: true }).click()
  await expect(page).toHaveURL(/\/reader\/comic-0\/2$/)
  await page.getByRole('button', { name: '下一章', exact: true }).click()
  await expect(page).toHaveURL(/\/reader\/comic-0\/3$/)
  await expect(page.locator('.reader-header p')).toContainText('第 3 话')
  await page.keyboard.press('Escape')
  await expect(page).toHaveURL(/\/comic\/comic-0$/)
  await page.getByRole('button', { name: '返回上一页', exact: true }).click()
  await expect(page).toHaveURL('http://127.0.0.1:1420/favorites')
})

for (const entry of ['/comic/comic-0', '/reader/comic-0/1']) {
  test(`directly opened pages have a safe return target: ${entry}`, async ({ page }) => {
    await fixture(page)
    await page.goto(entry)
    if (entry.startsWith('/reader/')) {
      await page.getByRole('button', { name: '返回作品', exact: true }).click()
      await expect(page).toHaveURL(/\/comic\/comic-0$/)
    }
    await page.getByRole('button', { name: '返回上一页', exact: true }).click()
    await expect(page).toHaveURL('http://127.0.0.1:1420/')
    await expect(page.locator('.comic-card').first()).toBeVisible()
  })
}

test('recoverable request failures and login validation', async ({ page }) => {
  await fixture(page)
  await page.goto('/')
  await page.evaluate(() => {
    ;(window as any).__fail = 'get_leaderboard'
  })
  await page.getByTitle('刷新内容').click()
  await expect(page.getByRole('alert')).toContainText('内容加载失败')
  await page.evaluate(() => {
    ;(window as any).__fail = ''
  })
  await page.getByRole('button', { name: '重新加载', exact: true }).click()
  await expect(page.locator('.comic-card')).toHaveCount(12)
  const context = await page
    .context()
    .browser()!
    .newContext({ viewport: { width: 1280, height: 820 } })
  const login = await context.newPage()
  await fixture(login, false)
  await login.goto('http://127.0.0.1:1420/')
  await login.getByRole('button', { name: '登录哔咔' }).click()
  await expect(login.getByRole('alert')).toContainText('请输入账号和密码')
  await capture(login, 'login')
  await context.close()
})

test('late responses cannot replace the active page or merge another keyword', async ({ page }) => {
  await fixture(page)
  await page.goto('/')
  await expect(page.locator('.comic-card')).toHaveCount(12)
  await page.evaluate(() => {
    const win = window as any
    const original = win.__TAURI_INTERNALS__.invoke
    win.__TAURI_INTERNALS__.invoke = (cmd: string, args: any) =>
      cmd === 'get_leaderboard'
        ? new Promise((_, reject) => {
            win.__releaseOldRequest = () => reject('旧请求失败')
          })
        : original(cmd, args)
  })
  await page.getByTitle('刷新内容').click()
  await page.getByLabel('全局搜索').fill('日常')
  await page.getByLabel('全局搜索').press('Enter')
  await expect(page.locator('.comic-card')).toHaveCount(4)
  await page.evaluate(() => (window as any).__releaseOldRequest())
  await expect(page.locator('.comic-card')).toHaveCount(4)
  await expect(page.getByRole('alert')).toHaveCount(0)
  await page.getByLabel('搜索关键词').fill('尚未提交的新关键词')
  await page.getByRole('button', { name: '加载更多作品' }).click()
  const requests = await page.evaluate(() =>
    (window as any).__calls.filter((c: any) => c.cmd === 'search_comics')
  )
  expect(requests.at(-1).args.req.keyword).toBe('日常')
  await expect(page.locator('.comic-card')).toHaveCount(7)
  await capture(page, 'search')
})

test('expanded navigation and page layouts fit the minimum desktop window', async ({ page }) => {
  await fixture(page)
  await page.setViewportSize({ width: 960, height: 640 })
  await page.goto('/')
  await expect(page.locator('.comic-card')).toHaveCount(12)
  const profile = await page.locator('.sidebar-profile').boundingBox()
  expect(profile!.y + profile!.height).toBeLessThanOrEqual(640)
  await capture(page, 'minimum')
  await page.getByTitle('切换深色外观').click()
  await capture(page, 'dark')
  for (const path of [
    '/search',
    '/favorites',
    '/library',
    '/downloads',
    '/settings',
    '/comic/comic-0'
  ]) {
    await page.goto(path)
    await expect(page.locator('.page')).toBeVisible()
    expect(
      await page.locator('.main-content').evaluate((el) => el.scrollWidth <= el.clientWidth)
    ).toBeTruthy()
  }
})

test('integrated window controls operate without turning form interactions into drags', async ({
  page
}) => {
  await fixture(page)
  await page.goto('/')
  await expect(page.getByRole('button', { name: '最大化窗口', exact: true })).toBeVisible()
  await page.getByRole('button', { name: '最大化窗口', exact: true }).click()
  await expect(page.getByRole('button', { name: '还原窗口', exact: true })).toBeVisible()
  await page.getByRole('button', { name: '还原窗口', exact: true }).click()
  await page.getByRole('button', { name: '最小化窗口', exact: true }).click()
  expect(await page.evaluate(() => (window as any).__windowState.minimized)).toBe(true)
  await page.locator('.breadcrumb').dblclick()
  await expect(page.getByRole('button', { name: '还原窗口', exact: true })).toBeVisible()
  const before = await page.evaluate(
    () =>
      (window as any).__calls.filter((c: any) => c.cmd === 'plugin:window|start_dragging').length
  )
  expect(before).toBeGreaterThan(0)
  await page.getByLabel('全局搜索').fill('日常')
  await page.getByTitle('切换深色外观').click()
  const after = await page.evaluate(
    () =>
      (window as any).__calls.filter((c: any) => c.cmd === 'plugin:window|start_dragging').length
  )
  expect(after).toBe(before)
  await capture(page, 'window-dark')
  await page.getByRole('button', { name: '关闭窗口', exact: true }).click()
  expect(await page.evaluate(() => (window as any).__windowState.closed)).toBe(true)
})

test('login and immersive reading retain accessible window controls', async ({ page }) => {
  await fixture(page, false)
  await page.goto('/')
  await expect(
    page.locator('.standalone-windowbar').getByRole('button', { name: '关闭窗口' })
  ).toBeVisible()
  await capture(page, 'window-login')
  const context = await page.context().browser()!.newContext()
  const reader = await context.newPage()
  await fixture(reader)
  await reader.goto('http://127.0.0.1:1420/reader/comic-0/1')
  await expect(
    reader.locator('.reader-header').getByRole('button', { name: '关闭窗口' })
  ).toBeVisible()
  await reader.getByTitle('沉浸模式 (F)').click()
  await expect(
    reader.locator('.reader-zen-titlebar').getByRole('button', { name: '关闭窗口' })
  ).toBeVisible()
  await reader.getByTitle('退出沉浸 (Esc)').click()
  await expect(
    reader.locator('.reader-header').getByRole('button', { name: '关闭窗口' })
  ).toBeVisible()
  await context.close()
})

test('category browsing merges simplified and traditional names, deduplicates pages and sorts', async ({
  page
}) => {
  await fixture(page)
  await page.goto('/')
  await page.getByTitle('分类浏览', { exact: true }).click()
  await expect(page.locator('.category-option')).toHaveCount(4)
  await capture(page, 'categories')
  await page.getByLabel('筛选分类').fill('漢化')
  await expect(page.locator('.category-option')).toHaveCount(1)
  await page.locator('.category-option').click()
  await expect(page.locator('.comic-card')).toHaveCount(3)
  const firstCalls = await page.evaluate(() =>
    (window as any).__calls.filter((c: any) => c.cmd === 'get_comics_by_category')
  )
  expect(firstCalls.map((c: any) => c.args.category)).toEqual(['漢化', '汉化'])
  await page.getByRole('button', { name: '加载更多漫画', exact: true }).click()
  await expect(page.locator('.comic-card')).toHaveCount(4)
  await expect(page.getByText('已经看到这个分类的全部结果了')).toBeVisible()
  const calls = await page.evaluate(() =>
    (window as any).__calls.filter((c: any) => c.cmd === 'get_comics_by_category')
  )
  expect(calls.at(-1).args).toEqual({ page: 2, category: '漢化', sort: 'dd' })
  expect(calls).toHaveLength(3)
  await page.getByLabel('分类排序').selectOption('ld')
  await expect(page.locator('.comic-card')).toHaveCount(3)
  await expect(page.locator('.comic-title').first()).toHaveText('漫长的午后')
  await capture(page, 'category-results')
  await page.getByRole('button', { name: '全部分类', exact: true }).click()
  await expect(page.locator('.comic-card')).toHaveCount(0)
  await expect(page.locator('.category-option')).toHaveCount(4)
})

test('detail tags open category results instead of keyword search', async ({ page }) => {
  await fixture(page)
  await page.goto('/comic/comic-0')
  await page.getByRole('button', { name: '日常', exact: true }).click()
  await expect(page).toHaveURL(/\/categories\?category=/)
  await expect(page.locator('.comic-card')).toHaveCount(4)
  const calls = await page.evaluate(() => (window as any).__calls)
  expect(
    calls.some((c: any) => c.cmd === 'get_comics_by_category' && c.args.category === '日常')
  ).toBe(true)
  expect(calls.some((c: any) => c.cmd === 'search_comics')).toBe(false)
  await page.reload()
  await expect(page.locator('.comic-card')).toHaveCount(4)
})

test('category errors retain existing results and can be retried', async ({ page }) => {
  await fixture(page)
  await page.goto('/categories?category=汉化')
  await expect(page.locator('.comic-card')).toHaveCount(3)
  await page.evaluate(() => {
    ;(window as any).__fail = 'get_comics_by_category'
  })
  await page.getByRole('button', { name: '加载更多漫画', exact: true }).click()
  await expect(page.getByRole('alert')).toContainText('连接暂时不可用')
  await expect(page.locator('.comic-card')).toHaveCount(3)
  await page.evaluate(() => {
    ;(window as any).__fail = ''
  })
  await page.getByRole('button', { name: '重试加载更多', exact: true }).click()
  await expect(page.locator('.comic-card')).toHaveCount(4)
  await page.evaluate(() => {
    ;(window as any).__fail = 'get_categories'
  })
  await page.getByRole('button', { name: '刷新分类', exact: true }).click()
  await expect(page.getByRole('alert')).toContainText('分类加载失败')
  await page.evaluate(() => {
    ;(window as any).__fail = ''
  })
  await page.getByRole('button', { name: '重新加载分类', exact: true }).click()
  await expect(page.locator('.category-option')).toHaveCount(4)
})

test('unsupported categories do not reach the category query endpoint', async ({ page }) => {
  await fixture(page)
  await page.goto('/categories?category=' + encodeURIComponent('蘿莉'))
  await expect(page.getByRole('heading', { name: '该分类不可用' })).toBeVisible()
  await expect(page.locator('.category-option')).toHaveCount(4)
  const calls = await page.evaluate(() =>
    (window as any).__calls.filter((c: any) => c.cmd === 'get_comics_by_category')
  )
  expect(calls).toHaveLength(0)
})

test('switching categories ignores late responses from the previous selection', async ({
  page
}) => {
  await fixture(page)
  await page.goto('/categories')
  await expect(page.locator('.category-option')).toHaveCount(4)
  await page.evaluate(() => {
    const win = window as any
    const original = win.__TAURI_INTERNALS__.invoke
    win.__TAURI_INTERNALS__.invoke = (cmd: string, args: any) =>
      cmd === 'get_comics_by_category' && args.category === '漢化'
        ? new Promise((resolve) => {
            win.__releaseCategory = () => resolve(original(cmd, args))
          })
        : original(cmd, args)
  })
  await page.getByRole('button', { name: /^汉化/ }).click()
  await page.getByRole('button', { name: /^原创/ }).click()
  await expect(page.locator('.comic-card')).toHaveCount(2)
  await expect(page.locator('.category-results-heading h2')).toContainText('原创')
  await page.evaluate(async () => {
    await (window as any).__releaseCategory()
    await Promise.resolve()
  })
  await expect(page.locator('.comic-card')).toHaveCount(2)
  await expect(page.locator('.comic-title').first()).toHaveText('直到星星亮起')
})
