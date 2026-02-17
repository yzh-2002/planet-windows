import { useState, useCallback } from 'react'
import { Sidebar } from './components/Sidebar'
import { ArticleList } from './components/ArticleList'
import { ArticleDetail } from './components/ArticleDetail'
import { PlanetDetail } from './components/PlanetDetail'
import { NewPlanetDialog } from './components/NewPlanetDialog'
import { NewArticleDialog } from './components/NewArticleDialog'
import { FollowingList } from './components/FollowingList'
import { FollowingArticleList } from './components/FollowingArticleList'
import { FollowingArticleDetail } from './components/FollowingArticleDetail'
import { FollowPlanetDialog } from './components/FollowPlanetDialog'
import { usePlanetStore, useArticles } from './hooks/usePlanetStore'
import type { MyArticle } from './types/planet'
import type { FollowingPlanet, FollowingArticle } from './types/following'

type ViewMode = 'my' | 'following'

function App() {
  // 视图模式：'my' 表示我的 Planet，'following' 表示关注的 Planet
  const [viewMode, setViewMode] = useState<ViewMode>('my')

  // 全局状态
  const { myPlanets, loading: planetsLoading, createPlanet } = usePlanetStore()

  // 我的 Planet 选中状态
  const [selectedPlanetId, setSelectedPlanetId] = useState<string | null>(null)
  const [selectedArticle, setSelectedArticle] = useState<MyArticle | null>(null)

  // 关注的 Planet 选中状态
  const [selectedFollowingPlanet, setSelectedFollowingPlanet] = useState<FollowingPlanet | null>(null)
  const [selectedFollowingArticle, setSelectedFollowingArticle] = useState<FollowingArticle | null>(null)

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
  const [showFollowPlanet, setShowFollowPlanet] = useState(false)

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

  // 切换视图时重置选中状态
  const handleViewModeChange = useCallback((mode: ViewMode) => {
    setViewMode(mode)
    if (mode === 'my') {
      setSelectedFollowingPlanet(null)
      setSelectedFollowingArticle(null)
    } else {
      setSelectedPlanetId(null)
      setSelectedArticle(null)
    }
  }, [])

  // 处理关注 Planet
  const handleFollowed = useCallback((planet: FollowingPlanet) => {
    setSelectedFollowingPlanet(planet)
    setViewMode('following')
  }, [])

  if (planetsLoading) {
    return (
      <div className="h-screen flex items-center justify-center bg-white dark:bg-gray-950">
        <span className="text-gray-400">Loading...</span>
      </div>
    )
  }

  return (
    <div className="h-screen flex flex-col bg-white dark:bg-gray-950">
      {/* 顶部：视图切换标签 */}
      <div className="flex border-b border-gray-200 dark:border-gray-700">
        <button
          onClick={() => handleViewModeChange('my')}
          className={`px-6 py-3 text-sm font-medium border-b-2 transition-colors ${
            viewMode === 'my'
              ? 'border-blue-500 text-blue-600 dark:text-blue-400'
              : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300'
          }`}
        >
          我的 Planet
        </button>
        <button
          onClick={() => handleViewModeChange('following')}
          className={`px-6 py-3 text-sm font-medium border-b-2 transition-colors ${
            viewMode === 'following'
              ? 'border-blue-500 text-blue-600 dark:text-blue-400'
              : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300'
          }`}
        >
          关注的 Planet
        </button>
      </div>

      {/* 主内容区域 */}
      <div className="flex-1 flex overflow-hidden">
        {viewMode === 'my' ? (
          <>
            {/* 左侧：我的 Planet 列表 */}
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

            {/* 右侧：文章详情或 Planet 详情 */}
            {selectedPlanetId ? (
              selectedArticle ? (
                <ArticleDetail
                  article={selectedArticle}
                  onDelete={handleDeleteArticle}
                />
              ) : (
                <PlanetDetail
                  planet={myPlanets.find((p) => p.id === selectedPlanetId)!}
                />
              )
            ) : (
              <div className="flex-1 flex items-center justify-center text-gray-400">
                <div className="text-center">
                  <div className="text-4xl mb-4">🪐</div>
                  <div className="text-lg">选择一个 Planet 开始</div>
                  <div className="text-sm mt-2">或点击左侧 + 创建新 Planet</div>
                </div>
              </div>
            )}
          </>
        ) : (
          <>
            {/* 左侧：关注的 Planet 列表 */}
            <div className="w-64 border-r border-gray-200 dark:border-gray-700">
              <FollowingList
                onSelectPlanet={setSelectedFollowingPlanet}
                selectedId={selectedFollowingPlanet?.id || null}
                onFollowNewPlanet={() => setShowFollowPlanet(true)}
              />
            </div>

            {/* 中间：关注的文章列表 + 右侧：文章详情 */}
            {selectedFollowingPlanet ? (
              <>
                <FollowingArticleList
                  planet={selectedFollowingPlanet}
                  onSelectArticle={setSelectedFollowingArticle}
                  selectedArticleId={selectedFollowingArticle?.id || null}
                />
                {selectedFollowingArticle ? (
                  <FollowingArticleDetail
                    planetId={selectedFollowingPlanet.id}
                    article={selectedFollowingArticle}
                  />
                ) : (
                  <div className="flex-1 flex items-center justify-center text-gray-400">
                    <div className="text-center">
                      <div className="text-4xl mb-4">📄</div>
                      <div className="text-lg">选择一篇文章查看详情</div>
                    </div>
                  </div>
                )}
              </>
            ) : (
              <div className="flex-1 flex items-center justify-center text-gray-400">
                <div className="text-center">
                  <div className="text-4xl mb-4">⭐</div>
                  <div className="text-lg">选择一个关注的 Planet</div>
                  <div className="text-sm mt-2">或点击左侧 + 关注新 Planet</div>
                </div>
              </div>
            )}
          </>
        )}
      </div>

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
      <FollowPlanetDialog
        open={showFollowPlanet}
        onClose={() => setShowFollowPlanet(false)}
        onFollowed={handleFollowed}
      />
    </div>
  )
}

export default App