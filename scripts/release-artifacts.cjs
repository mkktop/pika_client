const fs = require('node:fs')
const path = require('node:path')
const crypto = require('node:crypto')
const { execFileSync } = require('node:child_process')

function tree(ref) {
  return execFileSync('git', ['rev-parse', `${ref}^{tree}`], { encoding: 'utf8' }).trim()
}

function validRun(run, context) {
  return (
    run.status === 'completed' &&
    run.conclusion === 'success' &&
    run.path === '.github/workflows/check.yml' &&
    run.head_repository.full_name === `${context.repo.owner}/${context.repo.repo}` &&
    /^[0-9a-f]{40}$/.test(run.head_sha)
  )
}

async function findVerifiedRun({ github, context, tag, runId }) {
  if (!/^v\d+\.\d+\.\d+$/.test(tag)) throw new Error('Invalid release tag')
  execFileSync('node', ['scripts/check-release-version.mjs', tag], { stdio: 'inherit' })
  if (runId) {
    if (!/^\d+$/.test(String(runId))) throw new Error('Invalid workflow run ID')
    const { data: run } = await github.rest.actions.getWorkflowRun({
      ...context.repo,
      run_id: Number(runId)
    })
    if (!validRun(run, context)) throw new Error('Build must have succeeded in this repository')
    return String(run.id)
  }
  const { data } = await github.rest.actions.listWorkflowRuns({
    ...context.repo,
    workflow_id: 'check.yml',
    status: 'success',
    per_page: 50
  })
  const expected = tree('HEAD')
  for (const run of data.workflow_runs) {
    if (!validRun(run, context)) continue
    try {
      if (tree(run.head_sha) === expected) return String(run.id)
    } catch {
      /* Unrelated workflow commits may not be in this tag's history. */
    }
  }
  throw new Error(
    'No successful desktop build matches this tag; finish the build and rerun this workflow'
  )
}

async function uploadInstallers({ github, context, core, tag, directory, platforms }) {
  const version = tag.slice(1)
  const expectedTree = tree('HEAD')
  const suffixes = { 'windows-x64': '_x64-setup.exe', 'macos-arm64': '_aarch64.dmg' }
  // Validate every artifact before writing anything to a Release.
  const installers = platforms.map((platform) => {
    if (!suffixes[platform]) throw new Error('Unsupported installer platform')
    const folder = path.join(directory, `${platform}-installer`)
    const manifest = JSON.parse(fs.readFileSync(path.join(folder, 'build-info.json'), 'utf8'))
    if (
      manifest.platform !== platform ||
      manifest.version !== version ||
      manifest.source_tree !== expectedTree ||
      path.basename(manifest.filename) !== manifest.filename ||
      !manifest.filename.endsWith(`_${version}${suffixes[platform]}`)
    ) {
      throw new Error(`Installer provenance or version mismatch: ${platform}`)
    }
    const data = fs.readFileSync(path.join(folder, manifest.filename))
    const digest = `sha256:${crypto.createHash('sha256').update(data).digest('hex')}`
    if (
      data.length !== manifest.size ||
      digest !== `sha256:${manifest.sha256}` ||
      (platform === 'windows-x64' && data.subarray(0, 2).toString() !== 'MZ') ||
      (platform === 'macos-arm64' && data.subarray(-512, -508).toString() !== 'koly')
    ) {
      throw new Error(`Installer checksum or format mismatch: ${platform}`)
    }
    return { platform, name: manifest.filename, suffix: suffixes[platform], data, digest }
  })
  const { data: releases } = await github.rest.repos.listReleases({
    ...context.repo,
    per_page: 100
  })
  let release = releases.find((item) => item.tag_name === tag)
  if (release && !release.draft) throw new Error('Refusing to modify a published release')
  if (!release) {
    ;({ data: release } = await github.rest.repos.createRelease({
      ...context.repo,
      tag_name: tag,
      name: `Pika Client ${tag}`,
      draft: true,
      prerelease: false,
      body: 'Windows x64 与 macOS ARM64 安装包。Mac 版适用于 Apple Silicon（M 系列，macOS 11+），采用本地签名，未进行 Apple 公证。'
    }))
  }
  for (const installer of installers) {
    const existing = release.assets.filter((asset) =>
      asset.name.endsWith(`_${version}${installer.suffix}`)
    )
    if (existing.length) {
      if (
        existing.length !== 1 ||
        existing[0].digest !== installer.digest ||
        existing[0].size !== installer.data.length
      ) {
        throw new Error('Release draft contains a different installer')
      }
      // GitHub 会把资产名里的空格清洗成点，规范名以服务器返回为准。
      installer.uploadedName = existing[0].name
    } else {
      const upload = await github.rest.repos.uploadReleaseAsset({
        ...context.repo,
        release_id: release.id,
        name: installer.name,
        data: installer.data,
        headers: { 'content-type': 'application/octet-stream' }
      })
      installer.uploadedName = upload.data.name
    }
    core.info(`${installer.uploadedName}: ${installer.data.length} bytes, ${installer.digest}`)
  }
  // 更新清单随安装包一起上传：应用改从 releases/latest/download/latest.json 读取
  // 版本元数据，绕开 api.github.com 每 IP 每小时 60 次的匿名限流。
  const manifestData = Buffer.from(
    JSON.stringify(
      {
        version,
        notes: release.body || '',
        published_at: new Date().toISOString(),
        installers: Object.fromEntries(
          installers.map((installer) => [
            installer.platform,
            {
              name: installer.uploadedName,
              url: `https://github.com/${context.repo.owner}/${context.repo.repo}/releases/download/${tag}/${encodeURIComponent(installer.uploadedName)}`,
              size: installer.data.length,
              sha256: installer.digest.slice('sha256:'.length)
            }
          ])
        )
      },
      null,
      2
    ) + '\n'
  )
  // manifest 内容含发布时间，重跑不会逐字节一致，草稿上直接重传。
  for (const asset of release.assets.filter((asset) => asset.name === 'latest.json')) {
    await github.rest.repos.deleteReleaseAsset({ ...context.repo, asset_id: asset.id })
  }
  await github.rest.repos.uploadReleaseAsset({
    ...context.repo,
    release_id: release.id,
    name: 'latest.json',
    data: manifestData,
    headers: { 'content-type': 'application/json' }
  })
  core.info(`Verified installers uploaded to Release draft ${release.id}`)
}

module.exports = { findVerifiedRun, uploadInstallers }
