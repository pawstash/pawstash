import type { PawchivePost } from './pawchive';

export interface LibraryCollection {
  id: string;
  kind: 'inbox' | 'stash' | 'folder';
  name: string;
  parent_id?: string;
  item_count: number;
  is_system: boolean;
}

export interface LibraryPostIdentity {
  service: string;
  creator_id: string;
  post_id: string;
}

export interface LibrarySaveResult {
  entry_id: string;
  created: boolean;
  membership_added: boolean;
}

export interface LibraryPage {
  posts: PawchivePost[];
  hasMore: boolean;
}
