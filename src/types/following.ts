export interface FollowingPlanet {
    id: string;
    planetType: number;  // 0=planet, 1=ens, 2=dnslink, 3=dns, 4=dotbit
    name: string;
    about: string;
    link: string;
    cid: string | null;
    created: string;
    updated: string;
    lastRetrieved: string;
    isUpdating: boolean;
    articleCount: number;
    unreadCount: number;
    hasAvatar: boolean;
  }
  
  export interface FollowingArticle {
    id: string;
    link: string;
    title: string;
    content: string;
    summary: string | null;
    created: string;
    read: string | null;
    starred: string | null;
    videoFilename: string | null;
    audioFilename: string | null;
    attachments: string[] | null;
  }
  
  export type PlanetType = 'planet' | 'ens' | 'dnslink' | 'dns' | 'dotbit';
  
  export function planetTypeName(type: number): PlanetType {
    switch (type) {
      case 0: return 'planet';
      case 1: return 'ens';
      case 2: return 'dnslink';
      case 3: return 'dns';
      case 4: return 'dotbit';
      default: return 'planet';
    }
  }
  