import { useState, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Sidebar } from './components/Sidebar'
import { ArticleList } from './components/ArticleList'
import { ArticleDetail } from './components/ArticleDetail'
import { NewPlanetDialog } from './components/NewPlanetDialog'
import { NewArticleDialog } from './components/NewArticleDialog'
import { usePlanetStore, useArticles } from './hooks/usePlanetStore'
import type { MyArticle } from './types/planet'

function App() {
  // 全局状态
  const { myPlanets, loading: planetsLoading, createPlanet, deletePlanet } = usePlanetStore()

  // 选中状态
  const [selectedPlanetId, setSelectedPlanetId] = useState<string | null>(null)
  const [selectedArticle, setSelectedArticle] = useState<MyArticle | null>(null)

  // 文章列表
  const {
    articles,
    loading: articlesLoading,
    createArticle,
    deleteArticle,
  } = useArticles(selectedPlanetId)

  // 对话框状态
  const [showNewPlanet, setShowNewPlanet] = useState(false)
  const [showNewArticle, setShowNewArticle] = useState(false)

  // 选中文章
  const handleSelectArticle = useCallback(
    (articleId: string) => {
      const article = articles.find((a) => a.id === articleId)
      setSelectedArticle(article || null)
    },
    [articles]
  )

  // 创建 Planet
  const handleCreatePlanet = useCallback(
    async (name: string, about: string) => {
      const planet = await createPlanet(name, about)
      setSelectedPlanetId(planet.id)
    },
    [createPlanet]
  )

  // 创建文章
  const handleCreateArticle = useCallback(
    async (title: string, content: string) => {
      const article = await createArticle(title, content)
      setSelectedArticle(article)
    },
    [createArticle]
  )

  // 删除文章
  const handleDeleteArticle = useCallback(
    async (articleId: string) => {
      await deleteArticle(articleId)
      setSelectedArticle(null)
    },
    [deleteArticle]
  )

  if (planetsLoading) {
    return (
      <div className="h-screen flex items-center justify-center bg-white dark:bg-gray-950">
        <span className="text-gray-400">Loading...</span>
      </div>
    )
  }

  return (
    <div className="h-screen flex bg-white dark:bg-gray-950">
      {/* 左侧：Planet 列表 */}
      <Sidebar
        planets={myPlanets}
        selectedPlanetId={selectedPlanetId}
        onSelectPlanet={setSelectedPlanetId}
        onCreatePlanet={() => setShowNewPlanet(true)}
      />

      {/* 中间：文章列表 */}
      {selectedPlanetId && (
        <ArticleList
          articles={articles}
          selectedArticleId={selectedArticle?.id || null}
          onSelectArticle={handleSelectArticle}
          onCreateArticle={() => setShowNewArticle(true)}
          loading={articlesLoading}
        />
      )}

      {/* 右侧：文章详情 */}
      {selectedPlanetId ? (
        <ArticleDetail
          article={selectedArticle}
          onDelete={handleDeleteArticle}
        />
      ) : (
        <div className="flex-1 flex items-center justify-center text-gray-400">
          <div className="text-center">
            <div className="text-4xl mb-4">🪐</div>
            <div className="text-lg">选择一个 Planet 开始</div>
            <div className="text-sm mt-2">或点击左侧 + 创建新 Planet</div>
          </div>
        </div>
      )}

      {/* 对话框 */}
      <NewPlanetDialog
        open={showNewPlanet}
        onClose={() => setShowNewPlanet(false)}
        onCreate={handleCreatePlanet}
      />
      <NewArticleDialog
        open={showNewArticle}
        onClose={() => setShowNewArticle(false)}
        onCreate={handleCreateArticle}
      />
    </div>
  )
}

export default App