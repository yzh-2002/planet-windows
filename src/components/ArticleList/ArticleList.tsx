import React from 'react'

export function ArticleList() {
  return (
    <div className="w-80 bg-white border-r border-gray-300 h-full overflow-y-auto">
      <div className="p-4">
        <h2 className="text-lg font-semibold mb-4">Articles</h2>
        <div className="space-y-2">
          {/* 占位内容 */}
          <div className="p-3 border-b border-gray-200 cursor-pointer hover:bg-gray-50">
            <div className="font-medium">Article Title</div>
            <div className="text-sm text-gray-500 mt-1">2024-01-01</div>
          </div>
        </div>
      </div>
    </div>
  )
}