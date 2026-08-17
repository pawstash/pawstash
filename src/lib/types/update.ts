export interface UpdateInfo {
  available: boolean;
  current_version: string;
  latest_version: string;
  is_prerelease: boolean;
  release_name: string;
  release_notes: string;
  published_at: string;
  release_url: string;
  download_url: string | null;
  asset_name: string | null;
  asset_size: number | null;
}

export interface UpdateProgressPayload {
  downloaded: number;
  total: number;
  percentage: number;
  speed_bytes_per_sec: number;
}

