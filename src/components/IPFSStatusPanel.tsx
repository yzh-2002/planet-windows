import { useState } from 'react'
import { useIPFS } from '../hooks/useIPFS'

/**
 * IPFS 状态面板
 * 对应原项目 Planet/IPFS/Status Views/IPFSStatusView.swift
 *
 * 显示内容：
 * - Online/Offline 状态指示灯
 * - Local Gateway 地址
 * - Repo Size
 * - Peers 数量
 * - IPFS Version
 * - Launch/Shutdown 切换按钮
 * - GC 按钮
 */
export function IPFSStatusPanel() {
  const { state, loading, launch, shutdown, gc, refresh } = useIPFS()
  const [showGCConfirm, setShowGCConfirm] = useState(false)
  const [gcResult, setGcResult] = useState<string | null>(null)

  if (loading) {
    return (
      <div className="p-4 bg-gray-50 dark:bg-gray-800 rounded-lg">
        <div className="animate-pulse text-gray-400">Loading IPFS status...</div>
      </div>
    )
  }

  const gatewayUrl = `http://127.0.0.1:${state.gateway_port}`

  /** 格式化字节 */
  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return '0 B'
    const k = 1024
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
  }

  /** 切换 daemon 状态 */
  const handleToggle = async () => {
    if (state.online) {
      await shutdown()
    } else {
      await launch()
    }
  }

  /** 执行 GC */
  const handleGC = async () => {
    setShowGCConfirm(false)
    const count = await gc()
    if (count !== null) {
      setGcResult(`Removed ${count} unused objects`)
      setTimeout(() => setGcResult(null), 5000)
    }
  }

  return (
    <div className="w-72 bg-white dark:bg-gray-800 rounded-lg shadow-md overflow-hidden">
      {/* 状态信息区域 */}
      <div className="p-3 space-y-2 text-sm">
        {/* Local Gateway */}
        <div className="flex justify-between items-center">
          <span className="text-gray-500 dark:text-gray-400">Local Gateway</span>
          <a
            href={gatewayUrl}
            target="_blank"
            rel="noopener noreferrer"
            className={`text-blue-500 hover:underline text-xs ${
              !state.online ? 'opacity-50 pointer-events-none' : ''
            }`}
          >
            {gatewayUrl}
          </a>
        </div>

        {/* Repo Size */}
        <div className="flex justify-between items-center">
          <span className="text-gray-500 dark:text-gray-400">Repo Size</span>
          <span className="text-gray-700 dark:text-gray-300">
            {state.repo_size !== null ? formatBytes(state.repo_size) : '—'}
          </span>
        </div>

        {/* Peers */}
        <div className="flex justify-between items-center">
          <span className="text-gray-500 dark:text-gray-400">Peers</span>
          <span className="text-gray-700 dark:text-gray-300">
            {state.online && state.server_info
              ? state.server_info.ipfs_peer_count
              : '—'}
          </span>
        </div>

        {/* IPFS Version */}
        <div className="flex justify-between items-center">
          <span className="text-gray-500 dark:text-gray-400">IPFS Version</span>
          <span className="text-gray-700 dark:text-gray-300">
            {state.server_info?.ipfs_version || '—'}
          </span>
        </div>
      </div>

      {/* 分割线 */}
      <div className="border-t border-gray-200 dark:border-gray-700" />

      {/* 错误信息 */}
      {state.error_message && (
        <div className="px-3 py-2 bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 text-xs">
          {state.error_message}
        </div>
      )}

      {/* GC 结果 */}
      {gcResult && (
        <div className="px-3 py-2 bg-green-50 dark:bg-green-900/20 text-green-600 dark:text-green-400 text-xs">
          {gcResult}
        </div>
      )}

      {/* 操作栏 */}
      <div className="px-3 py-2 flex items-center justify-between">
        <div className="flex items-center gap-2">
          {state.is_operating ? (
            <div className="flex items-center gap-2">
              <div className="w-2.5 h-2.5 rounded-full bg-yellow-400 animate-pulse" />
              <span className="text-xs text-gray-500">Processing...</span>
            </div>
          ) : (
            <>
              <div
                className={`w-2.5 h-2.5 rounded-full ${
                  state.online ? 'bg-green-500' : 'bg-red-500'
                }`}
              />
              <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
                {state.online ? 'Online' : 'Offline'}
              </span>
            </>
          )}
        </div>

        <div className="flex items-center gap-2">
          {/* GC 按钮 */}
          {!state.is_operating && (
            <button
              onClick={() => setShowGCConfirm(true)}
              disabled={!state.online}
              className="p-1 rounded hover:bg-gray-100 dark:hover:bg-gray-700 disabled:opacity-30"
              title="Run IPFS garbage collection"
            >
              <svg className="w-4 h-4 text-gray-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2}
                  d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
              </svg>
            </button>
          )}

          {/* Launch/Shutdown 切换 */}
          {!state.is_operating && (
            <label className="relative inline-flex items-center cursor-pointer">
              <input
                type="checkbox"
                className="sr-only peer"
                checked={state.online}
                onChange={handleToggle}
              />
              <div className="w-9 h-5 bg-gray-200 peer-focus:outline-none rounded-full peer dark:bg-gray-600
                peer-checked:after:translate-x-full peer-checked:after:border-white
                after:content-[''] after:absolute after:top-[2px] after:left-[2px]
                after:bg-white after:border-gray-300 after:border after:rounded-full
                after:h-4 after:w-4 after:transition-all
                peer-checked:bg-green-500" />
            </label>
          )}

          {/* 操作中转圈 */}
          {state.is_operating && (
            <div className="w-5 h-5 border-2 border-gray-300 border-t-blue-500 rounded-full animate-spin" />
          )}
        </div>
      </div>

      {/* GC 确认弹窗 */}
      {showGCConfirm && (
        <div className="px-3 py-2 bg-yellow-50 dark:bg-yellow-900/20 border-t border-yellow-200 dark:border-yellow-800">
          <p className="text-xs text-yellow-700 dark:text-yellow-400 mb-2">
            Run garbage collection to free disk space?
          </p>
          <div className="flex gap-2">
            <button
              onClick={handleGC}
              className="px-2 py-1 text-xs bg-red-500 text-white rounded hover:bg-red-600"
            >
              Run GC
            </button>
            <button
              onClick={() => setShowGCConfirm(false)}
              className="px-2 py-1 text-xs bg-gray-200 dark:bg-gray-600 rounded hover:bg-gray-300 dark:hover:bg-gray-500"
            >
              Cancel
            </button>
          </div>
        </div>
      )}
    </div>
  )
}