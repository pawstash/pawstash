export type DownloadStatus =
  | 'queued'
  | 'resolving'
  | 'downloading'
  | 'paused'
  | 'verifying'
  | 'completed'
  | 'failed'
  | 'cancelled'
  | 'missing';

export interface DownloadItem {
  id: string;
  service: string;
  creator_id: string;
  post_id: string;
  media_id: string;
  url: string;
  filename: string;
  output_dir: string;
  temp_path: string;
  final_path: string;
  engine: 'native' | 'aria2c';
  status: DownloadStatus;
  downloaded_bytes: number;
  total_bytes: number;
  speed_bps: number;
  sha256?: string;
  error_code?: string;
  error_message?: string;
  retry_count: number;
  created_at: string;
  updated_at: string;
  completed_at?: string;
  post_title: string;
  creator_name: string;
  post_preview_path?: string;
  post_preview_url?: string;
  creator_avatar_path?: string;
}
