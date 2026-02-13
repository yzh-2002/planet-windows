/** IPFS 状态快照 — 与 Rust IpfsStateSnapshot 一一对应 */
export interface IpfsStateSnapshot {
    online: boolean
    is_operating: boolean
    api_port: number
    gateway_port: number
    swarm_port: number
    repo_size: number | null
    server_info: ServerInfo | null
    error_message: string | null
  }
  
  /** 服务器信息 — 与 Rust ServerInfo 一一对应 */
  export interface ServerInfo {
    host_name: string
    version: string
    ipfs_peer_id: string
    ipfs_version: string
    ipfs_peer_count: number
  }