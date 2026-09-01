import type { Post } from './content';

export interface ProviderConfig {
  id: string;
  name: string;
  enabled: boolean;
  api_url: string;
  fallback_urls: string[];
  file_url?: string | null;
  image_url?: string | null;
  file_prefix?: string | null;
  image_prefix?: string | null;
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

export interface AuthField {
  key: string;
  label_key: string;
  field_type: 'text' | 'password' | 'textarea';
  placeholder?: string | null;
  help_text_key?: string | null;
  required: boolean;
}

export interface ProviderAuthSchema {
  provider_id: string;
  supports_auth: boolean;
  supports_remote_favorites: boolean;
  supports_push_favorites: boolean;
  auth_fields: AuthField[];
  help_url?: string | null;
}

export interface FavoritesSyncResult {
  provider_id: string;
  pulled_count: number;
  pushed_count: number;
  errors: string[];
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
  post?: Post;
  [key: string]: unknown;
}
