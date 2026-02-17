import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { FollowingPlanet } from '../types/following';

interface FollowPlanetDialogProps {
  open: boolean;
  onClose: () => void;
  onFollowed: (planet: FollowingPlanet) => void;
}

export function FollowPlanetDialog({ open, onClose, onFollowed }: FollowPlanetDialogProps) {
  const [link, setLink] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!open) return null;

  const handleFollow = async () => {
    if (!link.trim()) return;
    setLoading(true);
    setError(null);

    try {
      const planet = await invoke<FollowingPlanet>('planet_follow', { link: link.trim() });
      onFollowed(planet);
      setLink('');
      onClose();
    } catch (e: any) {
      setError(typeof e === 'string' ? e : e.message || '关注失败');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow-xl p-6 w-[480px]">
        <h2 className="text-lg font-semibold mb-4">关注 Planet</h2>

        <input
          type="text"
          value={link}
          onChange={(e) => setLink(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && handleFollow()}
          placeholder="输入 IPNS 地址、ENS 域名、.bit 域名或 RSS Feed URL"
          className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600
                     rounded-md bg-white dark:bg-gray-700 text-sm
                     focus:outline-none focus:ring-2 focus:ring-blue-500"
          disabled={loading}
          autoFocus
        />

        <p className="text-xs text-gray-500 mt-2">
          支持: k51... (IPNS) · vitalik.eth (ENS) · example.bit · https://blog.example.com/feed.xml
        </p>

        {error && (
          <p className="text-sm text-red-500 mt-2">{error}</p>
        )}

        <div className="flex justify-end gap-2 mt-4">
          <button
            onClick={onClose}
            className="px-4 py-2 text-sm rounded-md border border-gray-300
                       dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700"
            disabled={loading}
          >
            取消
          </button>
          <button
            onClick={handleFollow}
            className="px-4 py-2 text-sm rounded-md bg-blue-500 text-white
                       hover:bg-blue-600 disabled:opacity-50"
            disabled={loading || !link.trim()}
          >
            {loading ? '关注中...' : '关注'}
          </button>
        </div>
      </div>
    </div>
  );
}
