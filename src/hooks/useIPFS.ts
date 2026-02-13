import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { IpfsStateSnapshot } from '../types/ipfs'

/** 默认初始状态 */
const defaultState: IpfsStateSnapshot = {
  online: false,
  is_operating: false,
  api_port: 5981,
  gateway_port: 18181,
  swarm_port: 4001,
  repo_size: null,
  server_info: null,
  error_message: null,
}

/**
 * IPFS 状态管理 Hook
 *
 * 功能：
 * 1. 监听后端推送的 "ipfs:state-changed" 事件，自动更新状态
 * 2. 提供 setup / launch / shutdown / gc / refresh 操作方法
 *
 * 对应原项目 SwiftUI 中的 @EnvironmentObject IPFSState
 */
export function useIPFS() {
  const [state, setState] = useState<IpfsStateSnapshot>(defaultState)
  const [loading, setLoading] = useState(true)

  // 初始加载：获取当前状态
  useEffect(() => {
    invoke<IpfsStateSnapshot>('ipfs_get_state')
      .then((s) => {
        setState(s)
        setLoading(false)
      })
      .catch((e) => {
        console.error('Failed to get IPFS state:', e)
        setLoading(false)
      })
  }, [])

  // 监听后端事件
  useEffect(() => {
    const unlisten = listen<IpfsStateSnapshot>('ipfs:state-changed', (event) => {
      setState(event.payload)
    })

    return () => {
      unlisten.then((fn) => fn())
    }
  }, [])

  // 操作方法
  const setup = useCallback(async () => {
    try {
      await invoke('ipfs_setup')
    } catch (e) {
      console.error('IPFS setup failed:', e)
    }
  }, [])

  const launch = useCallback(async () => {
    try {
      await invoke('ipfs_launch')
    } catch (e) {
      console.error('IPFS launch failed:', e)
    }
  }, [])

  const shutdown = useCallback(async () => {
    try {
      await invoke('ipfs_shutdown')
    } catch (e) {
      console.error('IPFS shutdown failed:', e)
    }
  }, [])

  const gc = useCallback(async (): Promise<number | null> => {
    try {
      return await invoke<number>('ipfs_gc')
    } catch (e) {
      console.error('IPFS GC failed:', e)
      return null
    }
  }, [])

  const refresh = useCallback(async () => {
    try {
      const s = await invoke<IpfsStateSnapshot>('ipfs_refresh_status')
      setState(s)
    } catch (e) {
      console.error('IPFS refresh failed:', e)
    }
  }, [])

  return {
    state,
    loading,
    setup,
    launch,
    shutdown,
    gc,
    refresh,
  }
}
