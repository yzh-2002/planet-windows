import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { FollowingPlanet, FollowingArticle } from '../types/following';

interface FollowingArticleListProps {
  planet: FollowingPlanet;
  onSelectArticle: (article: FollowingArticle) => void;
  selectedArticleId: string | null;
}

export function FollowingArticleList({
  planet,
  onSelectArticle,
  selectedArticleId,
}: FollowingArticleListProps) {
  const [articles, setArticles] = useState<FollowingArticle[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    setLoading(true);
    invoke<FollowingArticle[]>('following_articles', { id: planet.id })
      .then(setArticles)
      .catch((e) => console.error('获取文章列表失败:', e))
      .finally(() => setLoading(false));
  }, [planet.id]);

  const handleUpdate = async () => {
    try {
      await invoke('following_update', { id: planet.id });
      // 重新加载文章列表
      const updated = await invoke<FollowingArticle[]>('following_articles', {
        id: planet.id,
      });
      setArticles(updated);
    } catch (e) {
      console.error('更新失败:', e);
    }
  };

  const handleUnfollow = async () => {
    if (!confirm(`确认取消关注 "${planet.name}"？`)) return;
    try {
      await invoke('planet_unfollow', { id: planet.id });
      window.location.reload(); // 简单刷新
    } catch (e) {
      console.error('取消关注失败:', e);
    }
  };

  if (loading) {
    return (
      <div className="w-72 shrink-0 border-r border-gray-200 dark:border-gray-700 flex items-center justify-center">
        <span className="text-gray-400 text-sm">加载中...</span>
      </div>
    );
  }

  return (
    <div className="w-72 shrink-0 border-r border-gray-200 dark:border-gray-700 flex flex-col h-full">
      {/* 顶部操作栏 */}
      <div className="flex items-center justify-between p-3 border-b border-gray-200 dark:border-gray-700 gap-2">
        <div className="min-w-0 flex-1">
          <h3 className="text-sm font-semibold truncate" title={planet.name}>{planet.name}</h3>
          <p className="text-xs text-gray-500">{articles.length} 篇文章</p>
        </div>
        <div className="flex gap-2 shrink-0">
          <button
            onClick={handleUpdate}
            className="text-xs px-2 py-1 rounded border border-gray-300
                       dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700"
          >
            更新
          </button>
          <button
            onClick={handleUnfollow}
            className="text-xs px-2 py-1 rounded border border-red-300
                       text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20"
          >
            取消关注
          </button>
        </div>
      </div>

      {/* 文章列表 */}
      <div className="flex-1 overflow-y-auto">
        {articles.map((article) => (
          <div
            key={article.id}
            onClick={() => onSelectArticle(article)}
            className={`px-3 py-2 cursor-pointer border-b border-gray-100
                        dark:border-gray-700/50 hover:bg-gray-50
                        dark:hover:bg-gray-800 ${
                          selectedArticleId === article.id
                            ? 'bg-blue-50 dark:bg-blue-900/20'
                            : ''
                        } ${!article.read ? 'font-semibold' : ''}`}
          >
            <div className="text-sm truncate">{article.title}</div>
            {article.summary && (
              <div className="text-xs text-gray-500 mt-0.5 line-clamp-2">
                {article.summary}
              </div>
            )}
            <div className="text-xs text-gray-400 mt-0.5">
              {new Date(article.created).toLocaleDateString()}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
