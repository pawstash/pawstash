export interface CloudNode {
  id: string;
  parent_id?: string | null;
  name: string;
  size?: number | null;
  is_folder: boolean;
  mime_type?: string | null;
  download_url?: string | null;
  stream_url?: string | null;
  thumbnail_url?: string | null;
  children?: CloudNode[] | null;
}

export interface CloudFolderResult {
  provider: 'mega' | 'dropbox' | 'pixeldrain' | 'googledrive' | string;
  url: string;
  title: string;
  total_files: number;
  total_size: number;
  is_single_file: boolean;
  nodes: CloudNode[];
}
