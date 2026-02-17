import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { PublishState } from '../types/publish';

export function usePublish(planetId: string | null) {
  const [publishState, setPublishState] = useState<PublishState | null>(null);

  // 监听发布状态变化事件
  useEffect(() => {
    console.log('注册发布状态监听器，planetId:', planetId);
    const unlisten = listen<PublishState>('publish-state-changed', (event) => {
      console.log('收到发布状态事件:', event.payload);
      if (event.payload.planetId === planetId) {
        console.log('状态匹配，更新状态:', event.payload);
        setPublishState(event.payload);
      } else {
        console.log('状态不匹配，忽略事件。期望:', planetId, '收到:', event.payload.planetId);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [planetId]);

  // 触发发布
  const publish = useCallback(async () => {
    if (!planetId) {
      console.warn('发布失败: planetId 为空');
      return;
    }
    console.log('开始发布，planetId:', planetId);
    try {
      await invoke('planet_publish', { planetId });
      console.log('发布命令调用成功');
    } catch (error) {
      console.error('发布失败:', error);
      setPublishState({
        planetId,
        isPublishing: false,
        step: 'error',
        cid: null,
        error: String(error),
        startedAt: null,
      });
    }
  }, [planetId]);

  // 查询当前发布状态
  const refreshState = useCallback(async () => {
    if (!planetId) return;
    try {
      const state = await invoke<PublishState>('planet_get_publish_state', { planetId });
      setPublishState(state);
    } catch (error) {
      console.error('查询发布状态失败:', error);
    }
  }, [planetId]);

  return {
    publishState,
    publish,
    refreshState,
    isPublishing: publishState?.isPublishing ?? false,
  };
}