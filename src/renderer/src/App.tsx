import { useState, useEffect } from 'react'

function App() {
  const [selectedPlanet, setSelectedPlanet] = useState<string | null>(null)

  function getApiReady(): Promise<typeof window.api> {
    return new Promise(resolve => {
      const wait = () => {
        if (window.api) resolve(window.api)
        else setTimeout(wait, 50)
      }
      wait()
    })
  }
  
  // React 中调用
  useEffect(() => {
    getApiReady().then(api => {
      api.ipfs.getState().then(state => console.log(state))
    })
  }, [])

  return (
    <div className="flex h-screen bg-white text-gray-900">
      {/* 左栏: Sidebar */}
      <div className="w-60 min-w-[200px] border-r border-gray-200 bg-gray-50 flex flex-col">
        <div className="p-4 font-bold text-lg border-b border-gray-200">Planet</div>
        <div className="flex-1 overflow-y-auto p-2">
          <div className="text-xs text-gray-500 uppercase tracking-wide px-2 py-1">
            My Planets
          </div>
          <div className="text-sm text-gray-400 px-2 py-4">
            No planets yet
          </div>
          <div className="text-xs text-gray-500 uppercase tracking-wide px-2 py-1 mt-4">
            Following
          </div>
          <div className="text-sm text-gray-400 px-2 py-4">
            Not following anyone
          </div>
        </div>
        {/* IPFS 状态指示器（占位） */}
        <div className="p-3 border-t border-gray-200 flex items-center gap-2">
          <div className="w-2.5 h-2.5 rounded-full bg-red-500" />
          <span className="text-xs text-gray-500">IPFS Offline</span>
        </div>
      </div>

      {/* 中栏: Article List */}
      <div className="w-72 min-w-[240px] border-r border-gray-200 flex flex-col">
        <div className="p-3 border-b border-gray-200 text-sm font-medium">
          Articles
        </div>
        <div className="flex-1 overflow-y-auto flex items-center justify-center">
          <span className="text-sm text-gray-400">Select a planet</span>
        </div>
      </div>

      {/* 右栏: Article Detail */}
      <div className="flex-1 flex flex-col">
        <div className="flex-1 flex items-center justify-center">
          <span className="text-sm text-gray-400">Select an article to read</span>
        </div>
      </div>
    </div>
  )
}

export default App