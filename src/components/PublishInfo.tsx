import React from 'react';
import type { MyPlanet } from '../types/planet';

interface PublishInfoProps {
  planet: MyPlanet;
  gatewayPort: number | null;
}

export const PublishInfo: React.FC<PublishInfoProps> = ({ planet, gatewayPort }) => {
  const ipnsUrl = gatewayPort && planet.ipns
    ? `http://127.0.0.1:${gatewayPort}/ipns/${planet.ipns}`
    : null;

  return (
    <div className="space-y-2 text-sm">
      {/* IPNS */}
      {planet.ipns ? (
        <div>
          <span className="text-gray-500 dark:text-gray-400">IPNS: </span>
          <span className="font-mono text-xs break-all">{planet.ipns}</span>
        </div>
      ) : (
        <div>
          <span className="text-gray-500 dark:text-gray-400">IPNS: </span>
          <span className="text-gray-400 dark:text-gray-500 italic">未生成（首次发布后生成）</span>
        </div>
      )}

      {/* 上次发布时间 */}
      {planet.last_published && (
        <div>
          <span className="text-gray-500 dark:text-gray-400">上次发布: </span>
          <span>{new Date(planet.last_published).toLocaleString()}</span>
        </div>
      )}

      {/* 最新 CID */}
      {planet.last_published_cid && (
        <div>
          <span className="text-gray-500 dark:text-gray-400">CID: </span>
          <span className="font-mono text-xs break-all">{planet.last_published_cid}</span>
        </div>
      )}

      {/* 访问链接 */}
      {ipnsUrl && planet.last_published && (
        <div>
          <a
            href={ipnsUrl}
            target="_blank"
            rel="noreferrer"
            className="text-blue-600 dark:text-blue-400 hover:underline text-xs"
          >
            🌐 在浏览器中预览
          </a>
        </div>
      )}
    </div>
  );
};