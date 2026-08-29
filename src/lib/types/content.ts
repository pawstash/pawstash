export interface Attachment {
  name?: string;
  path?: string;
  server?: string;
  size?: number;
  [key: string]: unknown;
}

export interface Post {
  id: string;
  user: string;
  service: string;
  title: string;
  content?: string;
  substring?: string;
  published?: string;
  added?: string;
  library_added_at?: string;
  edited?: string;
  embed?: unknown;
  shared_file?: boolean;
  attachments?: Attachment[];
  file?: Attachment;
  poll?: unknown;
  captions?: unknown;
  tags?: unknown;
  origin?: string;
  preview_state?: string;
  has_full?: boolean;
  detail_fetched?: boolean;
  next?: string;
  prev?: string;
  favorite_count?: number;
  attachment_count?: number;
  [key: string]: unknown;
}

export interface ResolvedPostLink {
  service: string;
  creator_id: string;
  post_id?: string;
  link_type?: 'post' | 'creator';
  platform: string;
  source: 'cache' | 'remote';
}

export interface Creator {
  id: string;
  name: string;
  service: string;
  public_id?: string;
  relation_id?: string;
  updated?: number;
  indexed?: number;
  favorited?: number;
  kemono_favorited?: number;
  ever_imported?: boolean;
  [key: string]: unknown;
}

export interface CreatorProfile extends Omit<Creator, 'updated' | 'indexed'> {
  updated?: string;
  indexed?: string;
}

export interface Announcement {
  service: string;
  user_id: string;
  hash: string;
  content: string;
  added: string;
}

export interface Fancard {
  id: number;
  user_id: string;
  file_id: number;
  hash: string;
  mtime: string;
  ctime: string;
  mime: string;
  ext: string;
  added: string;
  size: number;
  ihash?: string;
}

export interface Favorite {
  faved_seq?: number;
  faved_at?: string;
  id: string;
  service?: string;
  name?: string;
  indexed?: string;
  last_imported?: string;
  updated?: string;
  [key: string]: unknown;
}

export interface FileSearchPost {
  file_id?: number;
  id: string;
  user?: string;
  service?: string;
  title?: string;
  server?: string;
  channel?: string;
  substring?: string;
  published?: string;
  file?: Attachment;
  attachments?: Attachment[];
  embeds?: unknown[];
  mentions?: unknown[];
  [key: string]: unknown;
}

export interface FileSearchResult {
  id: number;
  hash: string;
  mtime: string;
  ctime: string;
  mime: string;
  ext: string;
  added: string;
  size: number;
  ihash?: string;
  posts: FileSearchPost[];
  discord_posts: FileSearchPost[];
}

export interface PostRevision extends Post {
  revision_id: number;
}

export interface CommentRevision {
  id: number;
  content: string;
  added: string;
}

export interface Comment {
  id: string;
  parent_id?: string;
  commenter: string;
  commenter_name?: string;
  content: string;
  published: string;
  revisions: CommentRevision[];
  [key: string]: unknown;
}

export interface ApiActionResult {
  status: number;
  success: boolean;
}

export interface AccountSession {
  authenticated: boolean;
  username?: string;
}

export type FavoriteType = 'post' | 'artist';
