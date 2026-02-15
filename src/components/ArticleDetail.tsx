import type { MyArticle } from '../types/planet'

interface ArticleDetailProps {
  article: MyArticle | null
  onDelete?: (articleId: string) => void
}

export function ArticleDetail({ article, onDelete }: ArticleDetailProps) {
  if (!article) {
    return (
      <div className="flex-1 flex items-center justify-center text-gray-400">
        选择一篇文章查看
      </div>
    )
  }

  return (
    <div className="flex-1 overflow-y-auto bg-white dark:bg-gray-950">
      <div className="max-w-3xl mx-auto p-8">
        {/* 标题 */}
        <h1 className="text-3xl font-bold text-gray-900 dark:text-gray-100 mb-4">
          {article.title}
        </h1>

        {/* 元信息 */}
        <div className="flex items-center gap-4 text-sm text-gray-500 dark:text-gray-400 mb-8 pb-4 border-b border-gray-200 dark:border-gray-700">
          <span>创建于 {new Date(article.created).toLocaleString()}</span>
          <span>更新于 {new Date(article.updated).toLocaleString()}</span>
          {Object.keys(article.tags).length > 0 && (
            <div className="flex gap-1">
              {Object.keys(article.tags).map((tag) => (
                <span
                  key={tag}
                  className="px-2 py-0.5 bg-gray-100 dark:bg-gray-800 rounded text-xs"
                >
                  {tag}
                </span>
              ))}
            </div>
          )}
        </div>

        {/* Markdown 内容（Phase 2 先直接展示原文，Phase 3 再渲染 HTML） */}
        <div className="prose dark:prose-invert max-w-none">
          <pre className="whitespace-pre-wrap text-sm text-gray-800 dark:text-gray-200 font-mono bg-gray-50 dark:bg-gray-900 p-4 rounded-lg">
            {article.content}
          </pre>
        </div>

        {/* 操作按钮 */}
        {onDelete && (
          <div className="mt-8 pt-4 border-t border-gray-200 dark:border-gray-700">
            <button
              onClick={() => onDelete(article.id)}
              className="px-4 py-2 text-sm text-red-600 border border-red-300 rounded hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors"
            >
              删除文章
            </button>
          </div>
        )}
      </div>
    </div>
  )
}