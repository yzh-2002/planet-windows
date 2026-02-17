import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface FilebaseSettingsProps {
  planetId: string;
  initialEnabled: boolean;
  initialPinName: string;
  initialApiToken: string;
}

export const FilebaseSettings: React.FC<FilebaseSettingsProps> = ({
  planetId,
  initialEnabled,
  initialPinName,
  initialApiToken,
}) => {
  const [enabled, setEnabled] = useState(initialEnabled);
  const [pinName, setPinName] = useState(initialPinName);
  const [apiToken, setApiToken] = useState(initialApiToken);
  const [saving, setSaving] = useState(false);

  const save = async () => {
    setSaving(true);
    try {
      await invoke('planet_update_filebase', {
        planetId,
        enabled,
        pinName: pinName || null,
        apiToken: apiToken || null,
      });
    } catch (error) {
      console.error('保存 Filebase 设置失败:', error);
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
          id="filebase-enabled"
        />
        <label htmlFor="filebase-enabled" className="font-medium">
          启用 Filebase Pinning
        </label>
      </div>

      {enabled && (
        <>
          <div>
            <label className="block text-sm text-gray-600 dark:text-gray-400 mb-1">Pin Name</label>
            <input
              type="text"
              value={pinName}
              onChange={(e) => setPinName(e.target.value)}
              className="w-full px-3 py-1.5 border rounded dark:bg-gray-800 dark:border-gray-600"
              placeholder="my-planet"
            />
          </div>
          <div>
            <label className="block text-sm text-gray-600 dark:text-gray-400 mb-1">API Token</label>
            <input
              type="password"
              value={apiToken}
              onChange={(e) => setApiToken(e.target.value)}
              className="w-full px-3 py-1.5 border rounded dark:bg-gray-800 dark:border-gray-600"
              placeholder="Bearer token"
            />
          </div>
        </>
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