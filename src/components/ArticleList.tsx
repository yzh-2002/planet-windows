import type { MyArticle } from '../types/planet'

interface ArticleListProps {
  articles: MyArticle[]
  selectedArticleId: string | null
  onSelectArticle: (id: string) => void
  onCreateArticle: () => void
  loading: boolean
}

export function ArticleList({
  articles,
  selectedArticleId,
  onSelectArticle,
  onCreateArticle,
  loading,
}: ArticleListProps) {
  if (loading) {
    return (
      <div className="w-72 border-r border-gray-200 dark:border-gray-700 flex items-center justify-center">
        <span className="text-gray-400">加载中...</span>
      </div>
    )
  }

  return (
    <div className="w-72 border-r border-gray-200 dark:border-gray-700 flex flex-col h-full bg-white dark:bg-gray-950">
      {/* 头部 */}
      <div className="p-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
        <h2 className="text-sm font-semibold text-gray-600 dark:text-gray-400">
          Articles ({articles.length})
        </h2>
        <button
          onClick={onCreateArticle}
          className="px-3 py-1 text-xs bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors"
        >
          New
        </button>
      </div>

      {/* 文章列表 */}
      <div className="flex-1 overflow-y-auto">
        {articles.length === 0 ? (
          <div className="p-4 text-sm text-gray-400 text-center">
            暂无文章
          </div>
        ) : (
          articles.map((article) => (
            <div
              key={article.id}
              onClick={() => onSelectArticle(article.id)}
              className={`px-4 py-3 cursor-pointer border-b border-gray-100 dark:border-gray-800 transition-colors ${
                selectedArticleId === article.id
                  ? 'bg-blue-50 dark:bg-blue-900/20'
                  : 'hover:bg-gray-50 dark:hover:bg-gray-900'
              }`}
            >
              <div className="text-sm font-medium text-gray-900 dark:text-gray-100 truncate">
                {article.title || 'Untitled'}
              </div>
              <div className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                {new Date(article.created).toLocaleDateString()}
              </div>
              {article.summary && (
                <div className="text-xs text-gray-400 mt-1 line-clamp-2">
                  {article.summary}
                </div>
              )}
            </div>
          ))
        )}
      </div>
    </div>
  )
}