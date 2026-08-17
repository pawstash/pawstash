export type InitialImport = 'none' | 'latest' | 'all';
export type DownloadScope = 'primary' | 'all';

export interface CreatorSubscription {
  id: string;
  service: string;
  creator_id: string;
  creator_name: string;
  destination_collection_id: string;
  enabled: boolean;
  initial_import: InitialImport;
  auto_download: boolean;
  download_scope: DownloadScope;
  poll_interval_minutes: number;
  last_checked_at?: string;
  next_check_at: string;
  failure_count: number;
  last_error?: string;
}

export interface SubscriptionInput {
  service: string;
  creator_id: string;
  creator_name: string;
  destination_collection_id?: string;
  initial_import: InitialImport;
  auto_download: boolean;
  download_scope: DownloadScope;
  poll_interval_minutes: number;
}
