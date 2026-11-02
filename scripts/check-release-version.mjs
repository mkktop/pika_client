import { readFileSync } from 'node:fs'
import assert from 'node:assert/strict'

const tag = process.argv[2]
assert.match(
  tag || '',
  /^v(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)$/,
  'Release 标签应使用 v主版本.次版本.修订号'
)
const version = tag.slice(1)
const json = (path) => JSON.parse(readFileSync(new URL('../' + path, import.meta.url), 'utf8'))
assert.equal(json('package.json').version, version, 'package.json 版本与标签不一致')
assert.equal(json('package-lock.json').version, version, 'package-lock.json 版本与标签不一致')
assert.equal(json('package-lock.json').packages[''].version, version, 'npm 锁文件根包版本不一致')
assert.equal(json('src-tauri/tauri.conf.json').version, version, 'Tauri 版本与标签不一致')
const cargo = readFileSync(new URL('../src-tauri/Cargo.toml', import.meta.url), 'utf8')
assert.equal(cargo.match(/^version = "([^"]+)"/m)?.[1], version, 'Cargo.toml 版本与标签不一致')
const lock = readFileSync(new URL('../src-tauri/Cargo.lock', import.meta.url), 'utf8')
assert.equal(
  lock.match(/name = "app"\r?\nversion = "([^"]+)"/)?.[1],
  version,
  'Cargo.lock 版本与标签不一致'
)
console.log(`版本校验通过：${tag}`)
