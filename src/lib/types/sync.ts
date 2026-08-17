export interface SyncStatus {
  configured: boolean;
  enabled: boolean;
  unlocked: boolean;
  syncing: boolean;
  account_id?: string;
  server_url?: string;
  device_id?: string;
  revision: number;
  cursor: number;
  conflict: boolean;
  last_synced_at?: string;
  last_error?: string;
}

export interface SyncDevice {
  id: string;
  name: string;
  platform: string;
  created_at: string;
  revoked_at?: string;
}
