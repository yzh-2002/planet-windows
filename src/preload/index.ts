import { contextBridge, ipcRenderer } from 'electron'
import { electronAPI } from '@electron-toolkit/preload'

// Custom APIs for renderer
const api = {
  // IPFS 相关
  ipfs: {
    setup: () => ipcRenderer.invoke('ipfs:setup'),
    launch: () => ipcRenderer.invoke('ipfs:launch'),
    shutdown: () => ipcRenderer.invoke('ipfs:shutdown'),
    getState: () => ipcRenderer.invoke('ipfs:getState'),
    gc: () => ipcRenderer.invoke('ipfs:gc'),
    // 监听状态变化
    onStateChange: (callback: (state: any) => void) => {
      ipcRenderer.on('ipfs:stateChanged', (_event, state) => callback(state))
    }
  },
  // Planet 相关 (Phase 2 再实现)
  planet: {
    list: () => ipcRenderer.invoke('planet:list'),
    create: (data: any) => ipcRenderer.invoke('planet:create', data),
    publish: (id: string) => ipcRenderer.invoke('planet:publish', id),
    follow: (link: string) => ipcRenderer.invoke('planet:follow', link)
  },
  // 文章相关 (Phase 2 再实现)
  article: {
    list: (planetId: string) => ipcRenderer.invoke('article:list', planetId),
    create: (data: any) => ipcRenderer.invoke('article:create', data),
    delete: (id: string) => ipcRenderer.invoke('article:delete', id)
  },
  // 通用
  app: {
    openExternal: (url: string) => ipcRenderer.invoke('app:openExternal', url),
    showOpenDialog: (options: any) => ipcRenderer.invoke('app:showOpenDialog', options)
  }
}

// Use `contextBridge` APIs to expose Electron APIs to
// renderer only if context isolation is enabled, otherwise
// just add to the DOM global.
// if (process.contextIsolated) {
//   // if (typeof contextBridge !== 'undefined') {
//   try {
//     contextBridge.exposeInMainWorld('electron', electronAPI)
//     contextBridge.exposeInMainWorld('api', api)
//     console.log('Preload: Bridge exposed successfully')
//   } catch (error) {
//     console.error(error)
//   }
// } else {
//   console.log('contextBridge is not defined')
//   // @ts-ignore (define in dts)
//   window.electron = electronAPI
//   // @ts-ignore (define in dts)
//   window.api = api
// }

contextBridge.exposeInMainWorld('electronAPI', {
  // 获取系统默认桌面路径
  getDesktopPath: async () => {
    try {
      return await ipcRenderer.invoke('get-desktop-path')
    } catch (error) {
      console.error('Failed to get desktop path:', error.message)
    }
  }
})

contextBridge.exposeInMainWorld('myAPI', {
  desktop: true
});
