import { app } from 'electron'
import path from 'path'
import fs from 'fs'
import os from 'os'

/** 判断是否在开发模式 */
const isDev = !app.isPackaged

/**
 * 获取当前平台的 Kubo 二进制路径
 */
export function getKuboPath(): string {
  const platform = os.platform() // 'darwin' | 'win32' | 'linux'
  const arch = os.arch() // 'arm64' | 'x64' | 'ia32'

  // 构建文件名
  let filename: string
  if (platform === 'darwin') {
    filename = arch === 'arm64' ? 'ipfs-darwin-arm64' : 'ipfs-darwin-amd64'
  } else if (platform === 'win32') {
    filename = arch === 'arm64' ? 'ipfs-windows-arm64.exe' : 'ipfs-windows-amd64.exe'
  } else {
    // Linux 支持（可选）
    filename = arch === 'arm64' ? 'ipfs-linux-arm64' : 'ipfs-linux-amd64'
  }

  // 构建完整路径
  if (isDev) {
    return path.join(process.cwd(), 'resources', 'bin', filename)
  } else {
    return path.join(process.resourcesPath, 'bin', filename)
  }
}

/**
 * 验证 Kubo 二进制是否存在且可执行
 */
export function validateKuboPath(): { valid: boolean; path: string; error?: string } {
  const kuboPath = getKuboPath()
  const platform = os.platform()

  if (!fs.existsSync(kuboPath)) {
    return {
      valid: false,
      path: kuboPath,
      error: `Kubo binary not found at: ${kuboPath}`
    }
  }

  // macOS/Linux: 检查可执行权限
  if (platform !== 'win32') {
    try {
      fs.accessSync(kuboPath, fs.constants.X_OK)
    } catch {
      // 尝试修复权限
      fs.chmodSync(kuboPath, 0o755)
    }
  }

  return { valid: true, path: kuboPath }
}

/** IPFS repo 路径 */
export function getIPFSRepoPath(): string {
  const appData = app.getPath('appData')
  // macOS: ~/Library/Application Support/Planet/ipfs
  // Windows: %APPDATA%\Planet\ipfs
  const repoPath = path.join(appData, 'Planet', 'ipfs')
  fs.mkdirSync(repoPath, { recursive: true })
  return repoPath
}

/** 数据根目录 */
export function getDataPath(): string {
  const appData = app.getPath('appData')
  const dataPath = path.join(appData, 'Planet')
  fs.mkdirSync(dataPath, { recursive: true })
  return dataPath
}

/** 文档目录 */
export function getDocumentsPath(): string {
  const docs = app.getPath('documents')
  const planetDocs = path.join(docs, 'Planet')
  fs.mkdirSync(planetDocs, { recursive: true })
  return planetDocs
}

/** 临时目录 */
export function getTempPath(): string {
  const temp = app.getPath('temp')
  const planetTemp = path.join(temp, 'Planet')
  fs.mkdirSync(planetTemp, { recursive: true })
  return planetTemp
}
