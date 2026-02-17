export interface PublishState {
    planetId: string;
    isPublishing: boolean;
    step: 'idle' | 'saving' | 'uploading' | 'publishing' | 'pinning' | 'done' | 'error';
    cid: string | null;
    error: string | null;
    startedAt: string | null;
  }
  
  export interface FilebaseSettings {
    enabled: boolean;
    pinName: string;
    apiToken: string;
    requestId?: string;
    pinCid?: string;
  }
  
  export interface PinnableSettings {
    enabled: boolean;
    apiEndpoint: string;
    pinCid?: string;
  }
  
  export interface FilebasePin {
    cid: string;
    requestId: string;
    status: string;
  }