import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface PinnableSettingsProps {
  planetId: string;
  initialEnabled: boolean;
  initialApiEndpoint: string;
}

export const PinnableSettings: React.FC<PinnableSettingsProps> = ({
  planetId,
  initialEnabled,
  initialApiEndpoint,
}) => {
  const [enabled, setEnabled] = useState(initialEnabled);
  const [apiEndpoint, setApiEndpoint] = useState(initialApiEndpoint);
  const [saving, setSaving] = useState(false);

  const save = async () => {
    setSaving(true);
    try {
      await invoke('planet_update_pinnable', {
        planetId,
        enabled,
        apiEndpoint: apiEndpoint || null,
      });
    } catch (error) {
      console.error('保存 Pinnable 设置失败:', error);
    }
    setSaving(false);
  };

  return (
    <div className="space-y-3 p-4 border rounded-lg dark:border-gray-700">
      <div className="flex items-center gap-2">
        <input
          type="checkbox"
          checked={enabled}
          onChange={(e) => setEnabled(e.target.checked)}
          id="pinnable-enabled"
        />
        <label htmlFor="pinnable-enabled" className="font-medium">
          启用 Pinnable.xyz
        </label>
      </div>

      {enabled && (
        <div>
          <label className="block text-sm text-gray-600 dark:text-gray-400 mb-1">
            API Endpoint
          </label>
          <input
            type="text"
            value={apiEndpoint}
            onChange={(e) => setApiEndpoint(e.target.value)}
            className="w-full px-3 py-1.5 border rounded dark:bg-gray-800 dark:border-gray-600"
            placeholder="https://dev.pinnable.xyz/pin/xxxxxxxx"
          />
        </div>
      )}

      <button
        onClick={save}
        disabled={saving}
        className="px-3 py-1 bg-gray-200 dark:bg-gray-700 rounded hover:bg-gray-300 dark:hover:bg-gray-600 text-sm"
      >
        {saving ? '保存中...' : '保存设置'}
      </button>
    </div>
  );
};