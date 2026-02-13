import React from 'react'

export function ArticleDetail() {
  return (
    <div className="flex-1 bg-white h-full overflow-y-auto">
      <div className="max-w-4xl mx-auto p-8">
        <h1 className="text-3xl font-bold mb-4">Article Title</h1>
        <div className="text-gray-500 mb-6">2024-01-01</div>
        <div className="prose max-w-none">
          {/* 占位内容 */}
          <p>Article content will be displayed here...</p>
        </div>
      </div>
    </div>
  )
}