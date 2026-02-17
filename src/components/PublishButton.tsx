import React from 'react';
import { usePublish } from '../hooks/usePublish';

interface PublishButtonProps {
  planetId: string;
}

const STEP_LABELS: Record<string, string> = {
  idle: '发布',
  saving: '生成站点...',
  uploading: '上传到 IPFS...',
  publishing: '发布到 IPNS...',
  pinning: '远程 Pinning...',
  done: '发布完成',
  error: '发布失败',
};

export const PublishButton: React.FC<PublishButtonProps> = ({ planetId }) => {
  const { publishState, publish, isPublishing } = usePublish(planetId);

  const step = publishState?.step ?? 'idle';
  const label = STEP_LABELS[step] ?? '发布';

  // 调试日志
  React.useEffect(() => {
    console.log('PublishButton 渲染，planetId:', planetId, 'step:', step, 'isPublishing:', isPublishing);
  }, [planetId, step, isPublishing]);

  return (
    <div className="flex flex-col gap-2">
      <button
        onClick={publish}
        disabled={isPublishing}
        className={`px-4 py-2 rounded-lg text-white font-medium transition-colors ${
          isPublishing
            ? 'bg-blue-400 cursor-not-allowed'
            : 'bg-blue-600 hover:bg-blue-700'
        }`}
      >
        {isPublishing && (
          <span className="inline-block w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin mr-2" />
        )}
        {label}
      </button>

      {/* 发布状态详情 */}
      {publishState && step !== 'idle' && (
        <div className="text-sm">
          {step === 'done' && publishState.cid && (
            <div className="text-green-600 dark:text-green-400">
              <div>✅ 发布成功</div>
              <div className="font-mono text-xs mt-1 break-all">
                CID: {publishState.cid}
              </div>
            </div>
          )}

          {step === 'error' && (
            <div className="text-red-600 dark:text-red-400">
              ❌ {publishState.error}
            </div>
          )}
        </div>
      )}
    </div>
  );
};
