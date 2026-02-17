import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { FollowingPlanet } from '../types/following';
import { planetTypeName } from '../types/following';

interface FollowingListProps {
  onSelectPlanet: (planet: FollowingPlanet) => void;
  selectedId: string | null;
  onFollowNewPlanet?: () => void;
}

export function FollowingList({ onSelectPlanet, selectedId, onFollowNewPlanet }: FollowingListProps) {
  const [planets, setPlanets] = useState<FollowingPlanet[]>([]);
  const [loading, setLoading] = useState(true);

  const fetchList = useCallback(async () => {
    try {
      const list = await invoke<FollowingPlanet[]>('following_list');
      setPlanets(list);
    } catch (e) {
      console.error('获取关注列表失败:', e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchList();

    // 监听后台更新事件
    const unlisten = listen('following-updated', () => {
      fetchList();
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [fetchList]);

  const handleUpdateAll = async () => {
    try {
      await invoke('following_update_all');
    } catch (e) {
      console.error('更新失败:', e);
    }
  };

  if (loading) {
    return <div className="p-4 text-sm text-gray-500">加载中...</div>;
  }

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between p-3 border-b border-gray-200 dark:border-gray-700">
        <h3 className="text-sm font-semibold">关注 ({planets.length})</h3>
        <div className="flex items-center gap-2">
          {onFollowNewPlanet && (
            <button
              onClick={onFollowNewPlanet}
              className="w-6 h-6 flex items-center justify-center rounded hover:bg-gray-200 dark:hover:bg-gray-700 text-gray-500 dark:text-gray-400"
              title="关注新 Planet"
            >
              +
            </button>
          )}
          <button
            onClick={handleUpdateAll}
            className="text-xs text-blue-500 hover:text-blue-700 dark:text-blue-400 dark:hover:text-blue-300"
            title="更新所有"
          >
            ⟳ 更新全部
          </button>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto">
        {planets.map((planet) => (
          <div
            key={planet.id}
            onClick={() => onSelectPlanet(planet)}
            className={`flex items-center gap-3 px-3 py-2 cursor-pointer border-b
                        border-gray-100 dark:border-gray-700/50 hover:bg-gray-50
                        dark:hover:bg-gray-800 ${
                          selectedId === planet.id
                            ? 'bg-blue-50 dark:bg-blue-900/20'
                            : ''
                        }`}
          >
            {/* 头像 */}
            <div className="w-8 h-8 rounded-full bg-gray-200 dark:bg-gray-700
                            flex items-center justify-center text-xs font-semibold shrink-0">
              {planet.name.charAt(0).toUpperCase()}
            </div>

            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-1">
                <span className="text-sm font-medium truncate">{planet.name}</span>
                {planet.isUpdating && (
                  <span className="text-xs text-blue-500 animate-pulse">⟳</span>
                )}
              </div>
              <div className="text-xs text-gray-500 truncate">
                <span className="uppercase">{planetTypeName(planet.planetType)}</span>
                {' · '}
                {planet.articleCount} 篇
                {planet.unreadCount > 0 && (
                  <span className="text-blue-500 font-medium ml-1">
                    ({planet.unreadCount} 未读)
                  </span>
                )}
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}