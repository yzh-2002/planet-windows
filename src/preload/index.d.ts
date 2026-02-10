import { ElectronAPI } from '@electron-toolkit/preload'

export interface PlanetAPI {
  ipfs: {
    setup: () => Promise<void>
    launch: () => Promise<void>
    shutdown: () => Promise<void>
    getState: () => Promise<any>
    gc: () => Promise<void>
    onStateChange: (callback: (state: any) => void) => void
  }
  planet: {
    list: () => Promise<any[]>
    create: (data: any) => Promise<any>
    publish: (id: string) => Promise<void>
    follow: (link: string) => Promise<any>
  }
  article: {
    list: (planetId: string) => Promise<any[]>
    create: (data: any) => Promise<any>
    delete: (id: string) => Promise<void>
  }
  app: {
    openExternal: (url: string) => Promise<void>
    showOpenDialog: (options: any) => Promise<any>
  }
}

declare global {
  interface Window {
    electron: ElectronAPI
    api: PlanetAPI
  }
}
