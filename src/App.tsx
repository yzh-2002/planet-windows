import { IPFSStatusPanel } from './components/IPFSStatusPanel'

function App() {
  return (
    <div className="flex h-screen bg-gray-100 dark:bg-gray-900">
      {/* 左侧边栏 */}
      <div className="w-60 bg-white dark:bg-gray-800 border-r border-gray-200 dark:border-gray-700 flex flex-col">
        <div className="p-4 text-lg font-bold text-gray-800 dark:text-gray-200">
          Planet
        </div>
        <div className="flex-1 overflow-y-auto">
          {/* Phase 2 中填充 Planet 列表 */}
          <div className="p-4 text-sm text-gray-400">
            Planets will appear here...
          </div>
        </div>
      </div>

      {/* 中间内容区 */}
      <div className="flex-1 flex flex-col">
        <div className="flex-1 flex items-center justify-center text-gray-400">
          Select a planet to view articles
        </div>
      </div>

      {/* 右侧边栏 — IPFS 状态面板 */}
      <div className="w-72 border-l border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-4">
        <IPFSStatusPanel />
      </div>
    </div>
  )
}

export default App