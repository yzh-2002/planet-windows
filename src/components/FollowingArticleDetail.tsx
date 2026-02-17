import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { FollowingArticle } from '../types/following';

interface FollowingArticleDetailProps {
  planetId: string;
  article: FollowingArticle;
}

export function FollowingArticleDetail({ planetId, article }: FollowingArticleDetailProps) {
  const [fullArticle, setFullArticle] = useState<FollowingArticle | null>(null);

  useEffect(() => {
    invoke<FollowingArticle>('following_article_get', {
      planetId,
      articleId: article.id,
    })
      .then(setFullArticle)
      .catch((e) => console.error('获取文章详情失败:', e));
  }, [planetId, article.id]);

  const displayArticle = fullArticle || article;

  return (
    <div className="flex-1 overflow-y-auto p-6">
      <article className="max-w-3xl mx-auto">
        <h1 className="text-2xl font-bold mb-2">{displayArticle.title}</h1>

        <div className="text-sm text-gray-500 mb-6">
          {new Date(displayArticle.created).toLocaleString()}
          {displayArticle.read && (
            <span className="ml-2 text-green-500">✓ 已读</span>
          )}
        </div>

        {/* 音频播放器 */}
        {displayArticle.audioFilename && (
          <div className="mb-4 p-3 bg-gray-50 dark:bg-gray-800 rounded-lg">
            <audio controls className="w-full">
              <source src={displayArticle.audioFilename} />
            </audio>
          </div>
        )}

        {/* 视频播放器 */}
        {displayArticle.videoFilename && (
          <div className="mb-4">
            <video controls className="w-full rounded-lg">
              <source src={displayArticle.videoFilename} />
            </video>
          </div>
        )}

        {/* 文章内容 */}
        <div
          className="prose dark:prose-invert max-w-none"
          dangerouslySetInnerHTML={{ __html: displayArticle.content }}
        />

        {/* 附件 */}
        {displayArticle.attachments && displayArticle.attachments.length > 0 && (
          <div className="mt-6 pt-4 border-t border-gray-200 dark:border-gray-700">
            <h3 className="text-sm font-semibold mb-2">附件</h3>
            <ul className="text-sm">
              {displayArticle.attachments.map((att, i) => (
                <li key={i} className="text-blue-500 hover:underline">
                  <a href={att} target="_blank" rel="noopener noreferrer">
                    {att}
                  </a>
                </li>
              ))}
            </ul>
          </div>
        )}
      </article>
    </div>
  );
}
