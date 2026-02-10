import { app, shell, BrowserWindow, ipcMain, Tray, Menu, nativeImage, dialog } from 'electron'
import { join } from 'path'
import { electronApp, optimizer, is } from '@electron-toolkit/utils'
import icon from '../../resources/icon.png?asset'
import { getKuboPath, getIPFSRepoPath } from './utils/paths'

let mainWindow: BrowserWindow | null = null

function createWindow(): void {
  // Create the browser window.
  mainWindow = new BrowserWindow({
    width: 1200,
    height: 800,
    minWidth: 840,
    minHeight: 600,
    title: 'Planet',
    show: false,
    autoHideMenuBar: true,
    ...(process.platform === 'linux' ? { icon } : {}),
    webPreferences: {
      preload: join(__dirname, '../preload/index.js'),
      sandbox: false,
      contextIsolation: true,  // 启用上下文隔离，preload 脚本使用 contextBridge
      webviewTag: true,   // 后续文章渲染需要用 webview
    }
  })

  mainWindow.on('ready-to-show', () => {
    mainWindow!.show()
  })

  mainWindow.webContents.setWindowOpenHandler((details) => {
    shell.openExternal(details.url)
    return { action: 'deny' }
  })

  // HMR for renderer base on electron-vite cli.
  // Load the remote URL for development or the local html file for production.
  if (is.dev && process.env['ELECTRON_RENDERER_URL']) {
    mainWindow.loadURL(process.env['ELECTRON_RENDERER_URL'])
  } else {
    mainWindow.loadFile(join(__dirname, '../renderer/index.html'))
  }
}

let tray: Tray | null = null

function createTray() {
  // 用一个简单的 16x16 图标，后续替换为正式图标
  const icon = nativeImage.createEmpty()  // 临时用空图标
  tray = new Tray(icon)
  tray.setToolTip('Planet')
  const contextMenu = Menu.buildFromTemplate([
    { label: 'Show Planet', click: () => mainWindow?.show() },
    { type: 'separator' },
    { label: 'IPFS Status', enabled: false },
    { type: 'separator' },
    { label: 'Quit', click: () => app.quit() },
  ])
  tray.setContextMenu(contextMenu)
  tray.on('click', () => mainWindow?.show())
}

function createMenu() {
  const template: Electron.MenuItemConstructorOptions[] = [
    {
      label: 'File',
      submenu: [
        { label: 'New Planet...', accelerator: 'CmdOrCtrl+N', click: () => { /* TODO */ } },
        { label: 'Follow Planet...', click: () => { /* TODO */ } },
        { type: 'separator' },
        { label: 'Open IPFS Resource...', accelerator: 'CmdOrCtrl+O', click: () => { /* TODO */ } },
        { type: 'separator' },
        { label: 'Quit', accelerator: 'CmdOrCtrl+Q', role: 'quit' },
      ]
    },
    {
      label: 'Edit',
      submenu: [
        { role: 'undo' },
        { role: 'redo' },
        { type: 'separator' },
        { role: 'cut' },
        { role: 'copy' },
        { role: 'paste' },
        { role: 'selectAll' },
      ]
    },
    {
      label: 'View',
      submenu: [
        { role: 'reload' },
        { role: 'toggleDevTools' },
        { type: 'separator' },
        { role: 'zoomIn' },
        { role: 'zoomOut' },
        { role: 'resetZoom' },
      ]
    },
    {
      label: 'Help',
      submenu: [
        { label: 'About Planet', click: () => { /* TODO */ } },
      ]
    }
  ]
  const menu = Menu.buildFromTemplate(template)
  Menu.setApplicationMenu(menu)
}

function registerIpcHandlers() {
  // IPFS — Phase 1 再实现具体逻辑
  ipcMain.handle('ipfs:setup', async () => { console.log('TODO: ipfs setup') })
  ipcMain.handle('ipfs:launch', async () => { console.log('TODO: ipfs launch') })
  ipcMain.handle('ipfs:shutdown', async () => { console.log('TODO: ipfs shutdown') })
  ipcMain.handle('ipfs:getState', async () => {
    return { online: false, apiPort: 5981, gatewayPort: 18181, swarmPort: 4001 }
  })
  ipcMain.handle('ipfs:gc', async () => { console.log('TODO: ipfs gc') })

  // Planet — Phase 2 再实现
  ipcMain.handle('planet:list', async () => [])
  ipcMain.handle('planet:create', async () => ({}))
  ipcMain.handle('planet:publish', async () => {})
  ipcMain.handle('planet:follow', async () => ({}))

  // Article — Phase 2 再实现
  ipcMain.handle('article:list', async () => [])
  ipcMain.handle('article:create', async () => ({}))
  ipcMain.handle('article:delete', async () => {})

  // 通用
  ipcMain.handle('app:openExternal', async (_event, url: string) => {
    await shell.openExternal(url)
  })
  ipcMain.handle('app:showOpenDialog', async (_event, options) => {
    return dialog.showOpenDialog(options)
  })
}

// This method will be called when Electron has finished
// initialization and is ready to create browser windows.
// Some APIs can only be used after this event occurs.
app.whenReady().then(() => {
  console.log('Kubo path:', getKuboPath())
  console.log('IPFS repo path:', getIPFSRepoPath())

  // Set app user model id for windows
  electronApp.setAppUserModelId('com.electron')

  // Default open or close DevTools by F12 in development
  // and ignore CommandOrControl + R in production.
  // see https://github.com/alex8088/electron-toolkit/tree/master/packages/utils
  app.on('browser-window-created', (_, window) => {
    optimizer.watchWindowShortcuts(window)
  })

  // IPC test
  ipcMain.on('ping', () => console.log('pong'))

  createWindow()
  createTray()
  createMenu()
  registerIpcHandlers()

  app.on('activate', function () {
    // On macOS it's common to re-create a window in the app when the
    // dock icon is clicked and there are no other windows open.
    if (BrowserWindow.getAllWindows().length === 0) createWindow()
  })
})

// Quit when all windows are closed, except on macOS. There, it's common
// for applications and their menu bar to stay active until the user quits
// explicitly with Cmd + Q.
app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit()
  }
})

// In this file you can include the rest of your app's specific main process
// code. You can also put them in separate files and require them here.
