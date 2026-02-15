// ============================================================
// Planet 相关类型定义
// ============================================================

export interface MyPlanet {
    id: string
    name: string
    about: string
    domain?: string
    author_name?: string
    created: string
    ipns: string
    updated: string
    template_name: string
    last_published?: string
    last_published_cid?: string
    archived?: boolean
    twitter_username?: string
    github_username?: string
    telegram_username?: string
    mastodon_username?: string
    discord_link?: string
  }
  
  export interface FollowingPlanet {
    id: string
    name: string
    about: string
    created: string
    planet_type: string
    link: string
    cid?: string
    updated: string
    last_retrieved: string
    archived?: boolean
  }
  
  export interface MyArticle {
    id: string
    planet_id: string
    title: string
    content: string
    created: string
    updated: string
    link: string
    slug?: string
    hero_image?: string
    external_link?: string
    attachments: Attachment[]
    tags: Record<string, string>
    pinned?: string
    article_type?: string
    summary?: string
  }
  
  export interface Attachment {
    name: string
    url?: string
    mime_type?: string
    size?: number
  }
  
  export interface Draft {
    id: string
    planet_id: string
    article_id?: string
    date: string
    title: string
    content: string
    attachments: Attachment[]
    hero_image?: string
    external_link?: string
    tags: Record<string, string>
  }
  
  export interface PlanetStoreSnapshot {
    my_planets: MyPlanet[]
    following_planets: FollowingPlanet[]
    selected_view?: SelectedView
  }
  
  export type SelectedView =
    | { type: 'Today' }
    | { type: 'Unread' }
    | { type: 'Starred' }
    | { type: 'MyPlanet'; value: string }
    | { type: 'FollowingPlanet'; value: string }