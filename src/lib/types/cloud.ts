export interface CloudNode {
  id: string;
  parent_id?: string | null;
  name: string;
  size?: number | null;
  is_folder: boolean;
  is_encrypted?: boolean;
  mime_type?: string | null;
  download_url?: string | null;
  stream_url?: string | null;
  thumbnail_url?: string | null;
  children?: CloudNode[] | null;
}

export interface CloudFolderResult {
  provider: 'mega' | 'dropbox' | 'pixeldrain' | 'googledrive' | 'filen' | 'archive' | string;
  url: string;
  title: string;
  total_files: number;
  total_size: number;
  is_single_file: boolean;
  nodes: CloudNode[];
}

export interface CommentCloudLinks {
  all: string[];
  author: string[];
  others: string[];
}

export interface CloudLinkContext {
  service: string;
  creator_id: string;
  post_text?: string;
}

export type CloudLockKind = 'password' | 'key';

export interface ArchiveOpenRequest {
  url?: string | null;
  local_path?: string | null;
  name: string;
  service: string;
  creator_id: string;
  post_id: string;
  post_text?: string | null;
}

export type ArchiveFormat = 'zip' | 'rar' | '7z' | 'unknown';
export type ArchiveStatus = 'ready' | 'password_required' | 'unsupported';

export interface ArchiveOpenResult {
  archive_id: string;
  format: ArchiveFormat;
  status: ArchiveStatus;
  encrypted: boolean;
  password_source?: 'post' | 'creator' | 'user' | null;
  password_hints: string[];
  folder?: CloudFolderResult | null;
}

// `media_id` is what the attachment's download job was started with, so the modal can find it.
export interface ArchiveAttachment {
  name: string;
  url: string;
  media_id: string;
  size?: number;
}
