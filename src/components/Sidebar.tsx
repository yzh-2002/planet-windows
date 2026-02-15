import { useState } from 'react'
import type { MyPlanet } from '../types/planet'

interface SidebarProps {
  planets: MyPlanet[]
  selectedPlanetId: string | null
  onSelectPlanet: (id: string) => void
  onCreatePlanet: () => void
}

export function Sidebar({
  planets,
  selectedPlanetId,
  onSelectPlanet,
  onCreatePlanet,
}: SidebarProps) {
  return (
    <div className="w-60 bg-gray-50 dark:bg-gray-900 border-r border-gray-200 dark:border-gray-700 flex flex-col h-full">
      {/* 头部 */}
      <div className="p-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
        <h2 className="text-sm font-semibold text-gray-600 dark:text-gray-400 uppercase tracking-wider">
          My Planets
        </h2>
        <button
          onClick={onCreatePlanet}
          className="w-6 h-6 flex items-center justify-center rounded hover:bg-gray-200 dark:hover:bg-gray-700 text-gray-500"
          title="New Planet"
        >
          +
        </button>
      </div>

      {/* Planet 列表 */}
      <div className="flex-1 overflow-y-auto">
        {planets.length === 0 ? (
          <div className="p-4 text-sm text-gray-400 text-center">
            暂无 Planet，点击 + 创建
          </div>
        ) : (
          planets.map((planet) => (
            <div
              key={planet.id}
              onClick={() => onSelectPlanet(planet.id)}
              className={`px-4 py-3 cursor-pointer border-b border-gray-100 dark:border-gray-800 transition-colors ${
                selectedPlanetId === planet.id
                  ? 'bg-blue-50 dark:bg-blue-900/30 border-l-2 border-l-blue-500'
                  : 'hover:bg-gray-100 dark:hover:bg-gray-800'
              }`}
            >
              <div className="flex items-center gap-3">
                {/* 头像占位 */}
                <div className="w-8 h-8 rounded-full bg-gradient-to-br from-blue-400 to-purple-500 flex items-center justify-center text-white text-xs font-bold">
                  {planet.name.charAt(0).toUpperCase()}
                </div>
                <div className="flex-1 min-w-0">
                  <div className="text-sm font-medium text-gray-900 dark:text-gray-100 truncate">
                    {planet.name}
                  </div>
                  <div className="text-xs text-gray-500 dark:text-gray-400 truncate">
                    {planet.about || 'No description'}
                  </div>
                </div>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  )
}