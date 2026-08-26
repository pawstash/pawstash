import { invoke } from '@tauri-apps/api/core';
import type { CloudFolderResult } from '$lib/types/cloud';
import type { AppSettings } from '$lib/types/config';
import type { DownloadItem } from '$lib/types/download';
import type { CreatorSubscription, SubscriptionInput } from '$lib/types/subscription';
import type { SyncDevice, SyncStatus } from '$lib/types/sync';
import type {
  LibraryCollection,
  LibraryPostIdentity,
  LibrarySaveResult
} from '$lib/types/library';
import type {
  Announcement,
  AccountSession,
  ApiActionResult,
  Comment,
  Creator,
  CreatorProfile,
  Fancard,
  Favorite,
  FavoriteType,
  FileSearchResult,
  PawchivePost,
  PostRevision,
  ResolvedPostLink
} from '$lib/types/pawchive';

export const apiGetAxumPort = () => invoke<number>('get_axum_port');
export const apiCheckAria2c = () => invoke<boolean>('check_aria2c_installed');
export const apiGetSettings = () => invoke<AppSettings>('get_settings');
export const apiGetDefaultSettings = () => invoke<AppSettings>('get_default_settings');
export interface CacheStats {
  total_bytes: number;
  metadata_bytes: number;
  protected_bytes: number;
  reclaimable_bytes: number;
  preview_bytes: number;
  avatar_bytes: number;
  banner_bytes: number;
  other_bytes: number;
  file_count: number;
}
export const apiGetCacheStats = () => invoke<CacheStats>('get_cache_stats');
export const apiClearContentCache = () => invoke<CacheStats>('clear_content_cache');
export const apiClearAllContentCache = () => invoke<CacheStats>('clear_all_content_cache');
export const apiWipeAllData = () => invoke<CacheStats>('wipe_all_data');
export const apiSaveSettings = (settings: AppSettings) => invoke<void>('save_settings', { settings });
export const apiOpenInBrowser = (url: string) => invoke<void>('open_in_browser', { url });
export async function openExternalUrl(url: string) {
  try {
    await invoke('open_in_browser', { url });
  } catch {
    window.open(url, '_blank');
  }
}
export const apiOpenDownloadsFolder = () => invoke<void>('open_downloads_folder');
export const apiOpenDownloadFile = (filePath: string) =>
  invoke<void>('open_download_file', { filePath });
export const apiShowInFolder = (path: string) =>
  invoke<void>('show_in_folder', { path });
export const apiShowMainWindow = () => invoke<void>('show_main_window');
export const apiPickFolder = () => invoke<string | null>('pick_folder');
export const apiGetAccountSession = () => invoke<AccountSession>('get_account_session');
export const apiLoginAccount = (username: string, password: string) =>
  invoke<AccountSession>('login_account', { username, password });
export const apiLogoutAccount = () => invoke<AccountSession>('logout_account');

export const apiFetchCreators = () => invoke<Creator[]>('fetch_creators');

export const apiFetchPosts = (service: string, userId: string, offset = 0) =>
  invoke<PawchivePost[]>('fetch_posts', { service, userId, offset });

export const apiFetchRecentPosts = (query?: string, offset = 0) =>
  invoke<PawchivePost[]>('fetch_recent_posts', { query, offset });

export const apiFetchPopularPosts = (
  period: 'day' | 'week' | 'month' = 'day',
  date?: string,
  offset = 0
) => invoke<PawchivePost[]>('fetch_popular_posts', { period, date, offset });

export const apiFetchCreatorPosts = (
  service: string,
  creatorId: string,
  query?: string,
  offset = 0
) => invoke<PawchivePost[]>('fetch_creator_posts', { service, creatorId, query, offset });

export const apiFetchCreatorProfile = (service: string, creatorId: string) =>
  invoke<CreatorProfile>('fetch_creator_profile', { service, creatorId });

export const apiFetchAnnouncements = (service: string, creatorId: string) =>
  invoke<Announcement[]>('fetch_announcements', { service, creatorId });

export const apiFetchFancards = (service: string, creatorId: string) =>
  invoke<Fancard[]>('fetch_fancards', { service, creatorId });

export const apiFetchCreatorLinks = (service: string, creatorId: string) =>
  invoke<CreatorProfile[]>('fetch_creator_links', { service, creatorId });

export const apiFetchSimilarCreators = (service: string, creatorId: string) =>
  invoke<CreatorProfile[]>('fetch_similar_creators', { service, creatorId });

export const apiFetchPost = (service: string, creatorId: string, postId: string) =>
  invoke<PawchivePost>('fetch_post', { service, creatorId, postId });

export const apiGetCachedPost = (service: string, creatorId: string, postId: string) =>
  invoke<PawchivePost | null>('get_cached_post', { service, creatorId, postId });

export const apiResolveExternalPostLink = (
  url: string,
  currentService?: string,
  currentCreatorId?: string
) => invoke<ResolvedPostLink | null>('resolve_external_post_link', {
  url,
  currentService,
  currentCreatorId
});

export const apiFetchAccountFavorites = (favoriteType?: FavoriteType) =>
  invoke<Favorite[]>('fetch_account_favorites', { favoriteType });

export const apiSetPostFavorite = (
  service: string,
  creatorId: string,
  postId: string,
  favorite: boolean
) => invoke<ApiActionResult>('set_post_favorite', { service, creatorId, postId, favorite });

export const apiSetCreatorFavorite = (
  service: string,
  creatorId: string,
  favorite: boolean
) => invoke<ApiActionResult>('set_creator_favorite', { service, creatorId, favorite });

export const apiFetchCreatorArtworkDataUrl = (
  service: string,
  creatorId: string,
  artworkKind: 'banner' | 'avatar'
) => invoke<string>('fetch_creator_artwork_data_url', { service, creatorId, artworkKind });

export const apiFetchCreatorTags = (service: string, creatorId: string) =>
  invoke<string[]>('fetch_creator_tags', { service, creatorId });

export const apiSearchHash = (fileHash: string) =>
  invoke<FileSearchResult>('search_hash', { fileHash });

export const apiFlagPost = (service: string, creatorId: string, postId: string) =>
  invoke<ApiActionResult>('flag_post', { service, creatorId, postId });

export const apiIsPostFlagged = (service: string, creatorId: string, postId: string) =>
  invoke<boolean>('is_post_flagged', { service, creatorId, postId });

export const apiFetchPostRevisions = (service: string, creatorId: string, postId: string) =>
  invoke<PostRevision[]>('fetch_post_revisions', { service, creatorId, postId });

export const apiFetchPostComments = (service: string, creatorId: string, postId: string) =>
  invoke<Comment[]>('fetch_post_comments', { service, creatorId, postId });

export const apiGetPawchiveAppVersion = () =>
  invoke<string>('get_pawchive_app_version');

export const apiSearchPosts = (query: string) =>
  invoke<PawchivePost[]>('search_posts', { query });

export const apiListLibraryCollections = () =>
  invoke<LibraryCollection[]>('list_library_collections');

export const apiCreateLibraryStash = (name: string) =>
  invoke<LibraryCollection>('create_library_stash', { name });

export const apiDeleteLibraryStash = (collectionId: string) =>
  invoke<boolean>('delete_library_stash', { collectionId });

export const apiRenameLibraryStash = (collectionId: string, name: string) =>
  invoke<boolean>('rename_library_stash', { collectionId, name });

export const apiReorderLibraryStashes = (collectionIds: string[]) =>
  invoke<boolean>('reorder_library_stashes', { collectionIds });

export const apiClearLibraryStash = (collectionId: string) =>
  invoke<number>('clear_library_stash', { collectionId });

export const apiRemoveLibraryPostFromStash = (
  collectionId: string,
  service: string,
  creatorId: string,
  postId: string
) =>
  invoke<boolean>('remove_library_post_from_stash', { collectionId, service, creatorId, postId });

export const apiListPostCollections = (service: string, creatorId: string, postId: string) =>
  invoke<string[]>('list_post_collections', { service, creatorId, postId });

export const apiSaveLibraryPost = (post: PawchivePost, collectionId?: string) =>
  invoke<LibrarySaveResult>('save_library_post', { post, collectionId });

export const apiRemoveLibraryPost = (service: string, creatorId: string, postId: string) =>
  invoke<boolean>('remove_library_post', { service, creatorId, postId });

export interface PostStashMembership {
  collection_id: string;
  service: string;
  creator_id: string;
  post_id: string;
}

export const apiListSavedPostIdentities = () =>
  invoke<LibraryPostIdentity[]>('list_saved_post_identities');

export const apiListPostStashMemberships = () =>
  invoke<PostStashMembership[]>('list_post_stash_memberships');

export const apiListLibraryPosts = (collectionId?: string, offset = 0, limit = 50) =>
  invoke<PawchivePost[]>('list_library_posts', { collectionId, offset, limit });

export const apiStartDownload = (post: PawchivePost, mediaId: string, url: string, filename: string) =>
  invoke<DownloadItem>('start_download', { post, mediaId, url, filename });

export const apiProbeDownloadSize = (url: string) =>
  invoke<number | null>('probe_download_size', { url });

export const apiListDownloads = () => invoke<DownloadItem[]>('list_downloads');
export const apiPauseDownload = (downloadId: string) =>
  invoke<DownloadItem>('pause_download', { downloadId });
export const apiCancelDownload = (downloadId: string) =>
  invoke<DownloadItem>('cancel_download', { downloadId });
export const apiResumeDownload = (downloadId: string) =>
  invoke<DownloadItem>('resume_download', { downloadId });
export const apiRetryDownload = (downloadId: string) =>
  invoke<DownloadItem>('retry_download', { downloadId });
export const apiRemoveDownload = (downloadId: string) =>
  invoke<boolean>('remove_download', { downloadId });

export const apiListSubscriptions = () => invoke<CreatorSubscription[]>('list_subscriptions');
export const apiUpsertSubscription = (input: SubscriptionInput) =>
  invoke<CreatorSubscription>('upsert_subscription', { input });
export const apiSetSubscriptionEnabled = (subscriptionId: string, enabled: boolean) =>
  invoke<CreatorSubscription>('set_subscription_enabled', { subscriptionId, enabled });
export const apiRefreshSubscription = (subscriptionId: string) =>
  invoke<CreatorSubscription>('refresh_subscription', { subscriptionId });
export const apiDeleteSubscription = (subscriptionId: string) =>
  invoke<boolean>('delete_subscription', { subscriptionId });

export const apiGetSyncStatus = () => invoke<SyncStatus>('get_sync_status');
export const apiCreateSyncAccount = (serverUrl: string, accountId: string, masterPassword: string, deviceName: string) =>
  invoke<SyncStatus>('create_sync_account', { serverUrl, accountId, masterPassword, deviceName });
export const apiConnectSyncAccount = (serverUrl: string, accountId: string, masterPassword: string, deviceName: string) =>
  invoke<SyncStatus>('connect_sync_account', { serverUrl, accountId, masterPassword, deviceName });
export const apiUnlockSync = (masterPassword: string) => invoke<SyncStatus>('unlock_sync', { masterPassword });
export const apiLockSync = () => invoke<SyncStatus>('lock_sync');
export const apiDisconnectSync = () => invoke<SyncStatus>('disconnect_sync');
export const apiChangeSyncPassword = (currentPassword: string, newPassword: string) =>
  invoke<SyncStatus>('change_sync_password', { currentPassword, newPassword });
export const apiListSyncDevices = () => invoke<SyncDevice[]>('list_sync_devices');
export const apiRevokeSyncDevice = (deviceId: string) =>
  invoke<SyncDevice[]>('revoke_sync_device', { deviceId });
export const apiGetSyncRecoveryKit = () => invoke<string>('get_sync_recovery_kit');
export const apiCopySyncRecoveryKit = () => invoke<void>('copy_sync_recovery_kit');
export const apiRecoverSyncAccount = (recoveryKit: string, newPassword: string, deviceName: string) =>
  invoke<SyncStatus>('recover_sync_account', { recoveryKit, newPassword, deviceName });
export const apiRunSync = () => invoke<SyncStatus>('run_sync');
export const apiSetSyncEnabled = (enabled: boolean) =>
  invoke<SyncStatus>('set_sync_enabled', { enabled });
export const apiResolveSyncConflict = (resolution: 'local' | 'remote') =>
  invoke<SyncStatus>('resolve_sync_conflict', { resolution });

export const apiGetPendingDeepLink = () => invoke<string | null>('get_pending_deep_link');

export const apiHideToTray = () => invoke<void>('hide_to_tray');

export const apiUpdatePanicKey = (shortcut: string, enabled: boolean) =>
  invoke<void>('update_panic_key', { shortcut, enabled });

export const apiUpdateBossKey = (shortcut: string, enabled: boolean) =>
  apiUpdatePanicKey(shortcut, enabled);

export const apiResolveCloudLink = (url: string) =>
  invoke<CloudFolderResult>('resolve_cloud_link', { url });


