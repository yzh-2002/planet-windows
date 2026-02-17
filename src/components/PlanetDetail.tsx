import type { MyPlanet } from '../types/planet';
import { PublishButton } from './PublishButton';
import { PublishInfo } from './PublishInfo';
import { FilebaseSettings } from './FilebaseSettings';
import { PinnableSettings } from './PinnableSettings';
import { useIPFS } from '../hooks/useIPFS';

interface PlanetDetailProps {
  planet: MyPlanet;
}

export function PlanetDetail({ planet }: PlanetDetailProps) {
  const { state: ipfsState } = useIPFS();
  const gatewayPort = ipfsState?.gateway_port ?? null;

  return (
    <div className="flex-1 overflow-y-auto bg-white dark:bg-gray-950">
      <div className="max-w-3xl mx-auto p-8">
        {/* 基本信息 */}
        <div className="mb-6">
          <h2 className="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-2">
            {planet.name}
          </h2>
          <p className="text-gray-600 dark:text-gray-400">
            {planet.about || 'No description'}
          </p>
        </div>

        {/* 发布信息 */}
        <div className="mb-6 p-4 bg-gray-50 dark:bg-gray-900 rounded-lg">
          <PublishInfo planet={planet} gatewayPort={gatewayPort} />
        </div>

        {/* 发布按钮 */}
        <div className="mb-6">
          <PublishButton planetId={planet.id} />
        </div>

        {/* Pinning 设置 (折叠面板) */}
        <details className="mt-4">
          <summary className="cursor-pointer text-sm font-medium text-gray-600 dark:text-gray-400 mb-3">
            ⚙️ 远程 Pinning 设置
          </summary>
          <div className="mt-3 space-y-4">
            <FilebaseSettings
              planetId={planet.id}
              initialEnabled={planet.filebase_enabled ?? false}
              initialPinName={planet.filebase_pin_name ?? ''}
              initialApiToken={planet.filebase_api_token ?? ''}
            />
            <PinnableSettings
              planetId={planet.id}
              initialEnabled={planet.pinnable_enabled ?? false}
              initialApiEndpoint={planet.pinnable_api_endpoint ?? ''}
            />
          </div>
        </details>
      </div>
    </div>
  );
}
