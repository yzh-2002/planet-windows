import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Sidebar } from './components/Sidebar/Sidebar'
import { ArticleList } from './components/ArticleList/ArticleList'

function App() {
  const [kuboPath, setKuboPath] = useState<string>('')
  const [message, setMessage] = useState<string>('')

  const handleTestInvoke = async () => {
    try {
      // 测试 get_kubo_path
      const path = await invoke<string>('get_kubo_path')
      setKuboPath(path)

      // 测试 hello_world
      const msg = await invoke<string>('hello_world', { name: 'World' })
      setMessage(msg)
    } catch (error) {
      console.error('Invoke error:', error)
    }
  }

  return (
    <div className="flex h-screen bg-gray-50">
      <Sidebar />
      <ArticleList />
      <div className="flex-1 bg-white h-full overflow-y-auto">
        <div className="max-w-4xl mx-auto p-8">
          <h1 className="text-3xl font-bold mb-4">Planet Desktop</h1>
          <button
            onClick={handleTestInvoke}
            className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700"
          >
            Test Tauri Invoke
          </button>
          {kuboPath && (
            <div className="mt-4 p-4 bg-gray-100 rounded">
              <p className="font-semibold">Kubo Path:</p>
              <p className="text-sm text-gray-600">{kuboPath}</p>
            </div>
          )}
          {message && (
            <div className="mt-4 p-4 bg-green-100 rounded">
              <p>{message}</p>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}

export default App