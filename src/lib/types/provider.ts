import type { PawchivePost } from './pawchive';

export interface ProviderConfig {
  id: string;
  name: string;
  enabled: boolean;
  api_url: string;
  fallback_urls: string[];
  file_url?: string | null;
  image_url?: string | null;
  session_cookie: string;
  username: string;
  services: string[];
  is_custom: boolean;
  priority: number;
}

export interface ProviderHealth {
  provider_id: string;
  active_endpoint: string;
  is_healthy: boolean;
  latency_ms: number;
  error?: string | null;
  last_checked_at: string;
}

export interface PostRevisionData {
  revision_id: number;
  id?: string;
  user?: string;
  service?: string;
  title?: string;
  content?: string | null;
  substring?: string | null;
  published?: string | null;
  added?: string | null;
  edited?: string | null;
  embed?: any;
  shared_file?: boolean | null;
  attachments?: any[];
  file?: any;
  poll?: any;
  captions?: any;
  tags?: any;
  post?: PawchivePost;
  [key: string]: unknown;
}
