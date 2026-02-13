import React from 'react'

export function Sidebar() {
  return (
    <div className="w-64 bg-gray-100 border-r border-gray-300 h-full overflow-y-auto">
      <div className="p-4">
        <h2 className="text-lg font-semibold mb-4">My Planets</h2>
        <div className="space-y-2">
          {/* 占位内容 */}
          <div className="p-2 bg-white rounded cursor-pointer hover:bg-gray-50">
            <div className="font-medium">Planet Name</div>
            <div className="text-sm text-gray-500">Last updated: ...</div>
          </div>
        </div>
      </div>
      <div className="p-4 border-t border-gray-300">
        <h2 className="text-lg font-semibold mb-4">Following</h2>
        <div className="space-y-2">
          {/* 占位内容 */}
        </div>
      </div>
    </div>
  )
}