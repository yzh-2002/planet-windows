import { useIPFS } from '../hooks/useIPFS'

export function IPFSStatusSidebar() {
  const { state, loading } = useIPFS()

  if (loading) {
    return (
      <div className="p-4 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900">
        <div className="flex items-center gap-2">
          <div className="w-2 h-2 bg-gray-400 rounded-full animate-pulse" />
          <span className="text-xs text-gray-500">Loading IPFS...</span>
        </div>
      </div>
    )
  }

  const isOnline = state.online
  const peers = state.server_info?.ipfs_peer_count ?? 0
  const gatewayPort = state.gateway_port

  return (
    <div className="p-3 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900">
      <div className="flex flex-col gap-1.5">
        {/* Status Indicator */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <div 
              className={`w-2 h-2 rounded-full ${
                isOnline ? 'bg-green-500' : 'bg-red-500'
              }`} 
            />
            <span className="text-xs font-medium text-gray-700 dark:text-gray-300">
              IPFS Daemon
            </span>
          </div>
          <span className={`text-[10px] px-1.5 py-0.5 rounded-full ${
            isOnline 
              ? 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400' 
              : 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400'
          }`}>
            {isOnline ? 'Online' : 'Offline'}
          </span>
        </div>

        {/* Info Rows */}
        {isOnline && (
          <div className="grid grid-cols-2 gap-x-2 gap-y-1 mt-1">
            <div className="flex items-center justify-between text-[10px] text-gray-500">
              <span>Peers</span>
              <span className="font-mono text-gray-700 dark:text-gray-300">{peers}</span>
            </div>
            <div className="flex items-center justify-between text-[10px] text-gray-500">
              <span>Gateway</span>
              <span className="font-mono text-gray-700 dark:text-gray-300">:{gatewayPort}</span>
            </div>
            <div className="col-span-2 flex items-center justify-between text-[10px] text-gray-500 border-t border-gray-200 dark:border-gray-800 pt-1 mt-1">
              <span>Version</span>
              <span className="font-mono text-gray-700 dark:text-gray-300 truncate max-w-[120px] text-right">
                {state.server_info?.ipfs_version ?? '-'}
              </span>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
