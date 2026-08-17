<script lang="ts">
  import { onMount } from 'svelte';
  import { contentState, postCacheKey, type CachedPost } from '$lib/state/contentState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { configState } from '$lib/state/configState.svelte';
  import { libraryState } from '$lib/state/libraryState.svelte';
  import { downloadState } from '$lib/state/downloadState.svelte';
  import { accountState } from '$lib/state/accountState.svelte';
  import { themeState, getContrastColor } from '$lib/theme/themeState.svelte';
  import { creatorsState } from '$lib/state/creatorsState.svelte';
  import { apiFetchAccountFavorites, apiSetPostFavorite, apiFetchCreatorProfile, apiFetchCreatorArtworkDataUrl, apiOpenInBrowser, apiFetchPostComments, apiGetAxumPort, apiProbeDownloadSize } from '$lib/utils/ipc';
  import type { Attachment, Comment } from '$lib/types/pawchive';
  import type { DownloadItem } from '$lib/types/download';
  import { i18n } from '$lib/i18n';
  import { formatDate, formatBytes } from '$lib/utils/formatters';
  import { isImageUrl, isVideoUrl } from '$lib/utils/media';
  import PageShell from '$lib/components/layout/PageShell.svelte';
  import StickyHeader from '$lib/components/layout/StickyHeader.svelte';
  import HeroBackdrop from '$lib/components/ui/HeroBackdrop.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import ServiceIcon from './ServiceIcon.svelte';
  import RichContent from './RichContent.svelte';
  import PostPoll from './PostPoll.svelte';
  import MediaViewer, { type MediaViewerItem, type MediaViewerKind } from './MediaViewer.svelte';
  import pawchiveLogo from './pawchive-favicon.png';
  import IconArrowLeft from '~icons/fluent/arrow-left-24-regular';
  import IconDownload from '~icons/fluent/arrow-download-24-regular';
  import IconCheck from '~icons/fluent/checkmark-20-regular';
  import IconDelete from '~icons/fluent/delete-24-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';
  import IconChevronLeft from '~icons/fluent/chevron-left-24-regular';
  import IconChevronRight from '~icons/fluent/chevron-right-24-regular';
  import IconSave from '~icons/fluent/bookmark-add-24-regular';
  import IconSaved from '~icons/fluent/bookmark-24-filled';
  import IconHeart from '~icons/fluent/heart-24-regular';
  import IconHeartFilled from '~icons/fluent/heart-24-filled';
  import IconDocument from '~icons/fluent/document-24-regular';
  import IconEye from '~icons/fluent/eye-24-regular';
  import IconChevronDown from '~icons/fluent/chevron-down-24-regular';
  import IconChevronUp from '~icons/fluent/chevron-up-24-regular';
  import IconVideoOff from '~icons/fluent/video-off-24-regular';
  import IconWarning from '~icons/fluent/warning-24-regular';
  import IconGrid from '~icons/fluent/grid-24-regular';
  import IconVideo from '~icons/fluent/video-24-regular';
  import IconImage from '~icons/fluent/image-24-regular';
  import IconComment from '~icons/fluent/comment-24-regular';
  import IconFolder from '~icons/fluent/folder-24-regular';
  import IconFolderAdd from '~icons/fluent/folder-add-24-regular';
  import IconGlobe from '~icons/fluent/globe-24-regular';
  import IconOpen from '~icons/fluent/open-24-regular';
  import { toast } from 'svelte-sonner';

  interface PostEmbed {
    url?: string;
    subject?: string;
    description?: string;
    provider?: string;
    provider_url?: string;
    html?: string;
    [key: string]: unknown;
  }

  interface Props { service: string; creatorId: string; postId: string; }
  let { service, creatorId, postId }: Props = $props();

  const emptyEntry: CachedPost = { post: null, loading: false, loaded: false, error: null };
  let entry = $derived.by(() => contentState.posts[postCacheKey(service, creatorId, postId)] ?? emptyEntry);
  let post = $derived(entry.post);
  let postEmbed = $derived<PostEmbed | null>(
    (post?.embed && typeof post.embed === 'object' && Object.keys(post.embed).length > 0)
      ? (post.embed as PostEmbed)
      : null
  );
  let previousPost = $derived.by(() => post?.prev
    ? contentState.posts[postCacheKey(service, creatorId, post.prev)]?.post ?? null
    : null);
  let nextPost = $derived.by(() => post?.next
    ? contentState.posts[postCacheKey(service, creatorId, post.next)]?.post ?? null
    : null);
  let previousPostTitle = $derived(previousPost?.title?.trim() || i18n.t('post.previous'));
  let nextPostTitle = $derived(nextPost?.title?.trim() || i18n.t('post.next'));
  
  let richContent = $derived(post?.content || post?.substring || '');

  let media = $derived.by(() => {
    if (!post) return [];
    const items: Attachment[] = [];
    if (post.file && (post.file.path || post.file.name)) {
      items.push(post.file);
    }
    if (post.attachments && Array.isArray(post.attachments)) {
      for (const att of post.attachments) {
        if (!att || (!att.path && !att.name)) continue;
        const exists = items.some(
          (existing) => (existing.path && existing.path === att.path) || (!existing.path && existing.name === att.name)
        );
        if (!exists) {
          items.push(att);
        }
      }
    }
    return items;
  });

  let activeMediaTab = $state<'all' | 'video' | 'photo' | 'file'>('all');
  let mediaSort = $state<'default' | 'name_asc' | 'name_desc' | 'size_desc' | 'size_asc'>('default');
  let viewerIndex = $state<number | null>(null);
  let viewerFiles = $state<Attachment[]>([]);
  let contentExpanded = $state(false);
  let contentHeight = $state(0);
  const MAX_CONTENT_HEIGHT = 480;
  let isOverflowing = $derived(contentHeight > MAX_CONTENT_HEIGHT);

  let hevcSupported = $state(true);
  let videoFailures = $state<Record<number, boolean>>({});

  function isH265Video(filename?: string) {
    if (!filename) return false;
    const name = filename.toLowerCase();
    return name.includes('h265') || name.includes('hevc') || name.includes('x265') || name.includes('265');
  }

  function handleVideoMetadata(e: Event, index: number) {
    const video = e.target as HTMLVideoElement;
    if (video.videoWidth === 0 && video.videoHeight === 0) {
      videoFailures[index] = true;
    }
  }

  function isHtmlContentEmpty(html?: string) {
    if (!html) return true;
    const text = html.replace(/<[^>]*>/g, '').replace(/&nbsp;/g, '').trim();
    return text.length === 0;
  }

  let galleryExpanded = $state(false);
  let galleryHeight = $state(0);
  const MAX_GALLERY_HEIGHT = 960;
  let isGalleryOverflowing = $derived(galleryHeight > MAX_GALLERY_HEIGHT);

  $effect(() => {
    if (activeMediaTab) {
      galleryExpanded = false;
    }
  });

  let hasEmbed = $derived(Boolean(postEmbed && (postEmbed.url || postEmbed.subject || postEmbed.html)));

  let mediaCounts = $derived.by(() => {
    let videos = 0;
    let photos = 0;
    let files = 0;
    for (const file of media) {
      const url = file.path ? fileUrl(file) : (file.name || '');
      if (isVideoUrl(url) || /\.(mp4|mkv|webm|mov|avi|flv|wmv|m4v)(?:$|[?#])/i.test(file.name || '')) videos++;
      else if (isImageUrl(url) || /\.(png|jpe?g|gif|webp|bmp|avif)(?:$|[?#])/i.test(file.name || '')) photos++;
      else files++;
    }
    if (hasEmbed) {
      videos++;
    }
    return {
      all: media.length + (hasEmbed ? 1 : 0),
      video: videos,
      photo: photos,
      file: files
    };
  });

  let activeCategoriesCount = $derived.by(() => {
    let count = 0;
    if (mediaCounts.video > 0) count++;
    if (mediaCounts.photo > 0) count++;
    if (mediaCounts.file > 0) count++;
    return count;
  });

  let filteredMedia = $derived.by(() => {
    let list = [...media];
    if (activeMediaTab === 'video') {
      list = list.filter((file) => isVideoUrl(file.path ? fileUrl(file) : '') || /\.(mp4|mkv|webm|mov|avi|flv|wmv|m4v)(?:$|[?#])/i.test(file.name || ''));
    } else if (activeMediaTab === 'photo') {
      list = list.filter((file) => isImageUrl(file.path ? fileUrl(file) : '') || /\.(png|jpe?g|gif|webp|bmp|avif)(?:$|[?#])/i.test(file.name || ''));
    } else if (activeMediaTab === 'file') {
      list = list.filter((file) => {
        const isVid = isVideoUrl(file.path ? fileUrl(file) : '') || /\.(mp4|mkv|webm|mov|avi|flv|wmv|m4v)(?:$|[?#])/i.test(file.name || '');
        const isImg = isImageUrl(file.path ? fileUrl(file) : '') || /\.(png|jpe?g|gif|webp|bmp|avif)(?:$|[?#])/i.test(file.name || '');
        return !isVid && !isImg;
      });
    }

    if (mediaSort === 'name_asc') {
      list.sort((a, b) => (a.name || '').localeCompare(b.name || ''));
    } else if (mediaSort === 'name_desc') {
      list.sort((a, b) => (b.name || '').localeCompare(a.name || ''));
    } else if (mediaSort === 'size_desc') {
      list.sort((a, b) => (b.size || 0) - (a.size || 0));
    } else if (mediaSort === 'size_asc') {
      list.sort((a, b) => (a.size || 0) - (b.size || 0));
    }

    return list;
  });

  function mediaViewerKind(file: Attachment, url: string): MediaViewerKind {
    const filename = `${file.name ?? ''} ${file.path ?? ''}`.toLocaleLowerCase();
    if (isImageUrl(url)) return 'image';
    if (isVideoUrl(url)) return 'video';
    if (/\.(mp3|m4a|aac|wav|ogg|opus|flac)(?:$|[?#])/i.test(filename)) return 'audio';
    return 'file';
  }

  let embedAttachment = $derived(postEmbed ? ({
    name: postEmbed.subject || postEmbed.description || post?.title || 'Embed',
    path: postEmbed.url || `embed:${postId}`,
    server: '',
    html: postEmbed.html
  } as Attachment & { html?: string }) : null);

  let viewerItems = $derived.by((): MediaViewerItem[] => viewerFiles.map((file, itemIndex) => {
    const isEmbed = file === embedAttachment || Boolean((file as any)?.html);
    const url = file.path ? (file.path.startsWith('http') ? file.path : fileUrl(file)) : '';
    const job = attachmentDownload(file);
    const width = typeof file.width === 'number' && file.width > 0 ? file.width : undefined;
    const height = typeof file.height === 'number' && file.height > 0 ? file.height : undefined;
    return {
      id: file.path || `${file.name || 'media'}:${itemIndex}`,
      url,
      name: file.name || i18n.t('post.file'),
      kind: isEmbed ? 'video' : mediaViewerKind(file, url),
      size: file.size,
      width,
      height,
      html: (file as any)?.html,
      downloadStatus: job?.status,
      downloadedBytes: job?.downloaded_bytes,
      totalBytes: job?.total_bytes
    };
  }));

  let viewerInitialTime = $state(0);

  function handleVideoPlay(e: Event) {
    const currentVideo = e.currentTarget as HTMLVideoElement;
    document.querySelectorAll<HTMLVideoElement>('.media-gallery video, video, audio').forEach((other) => {
      if (other !== currentVideo && !other.paused) {
        other.pause();
      }
    });
  }

  function openMediaViewer(file: Attachment, source: Attachment[] = filteredMedia, originVideoEl?: HTMLVideoElement | null) {
    let time = 0;
    if (originVideoEl && !originVideoEl.paused) {
      time = originVideoEl.currentTime || 0;
      originVideoEl.pause();
    } else {
      const pageVideos = document.querySelectorAll<HTMLVideoElement>('.media-gallery video');
      for (const v of pageVideos) {
        if (!v.paused) {
          time = v.currentTime || 0;
          v.pause();
        }
      }
    }
    document.querySelectorAll<HTMLVideoElement>('video, audio').forEach((el) => {
      if (!el.paused) el.pause();
    });

    viewerInitialTime = time;

    const allSource = embedAttachment && !source.some((s) => s.path === embedAttachment!.path)
      ? [embedAttachment, ...source]
      : source;
    const sourceItems = [post?.file, ...allSource]
      .filter((item): item is Attachment => Boolean(item?.path || (item as any)?.html))
      .filter((item, itemIndex, list) => list.findIndex((candidate) => candidate.path === item.path) === itemIndex);
    const nextIndex = sourceItems.findIndex((item) => item.path === file.path);
    viewerFiles = nextIndex >= 0 ? sourceItems : [file, ...sourceItems];
    viewerIndex = nextIndex >= 0 ? nextIndex : 0;
  }

  function handleCloseViewer(finalIndex?: number, finalTime?: number) {
    const closedIndex = typeof finalIndex === 'number' ? finalIndex : viewerIndex;
    viewerIndex = null;
    viewerInitialTime = 0;

    if (typeof closedIndex === 'number' && typeof finalTime === 'number' && finalTime > 0) {
      const closedFile = viewerFiles[closedIndex];
      if (closedFile?.path) {
        const matchingVideo = document.querySelector<HTMLVideoElement>(`.media-gallery video[src*="${closedFile.path}"]`);
        if (matchingVideo) {
          matchingVideo.currentTime = finalTime;
        }
      }
    }
  }

  function openPreviewViewer() {
    if (!post?.file) return;
    const source = [post.file, ...media.filter((file) => file.path !== post?.file?.path)];
    openMediaViewer(post.file, source);
  }

  async function downloadViewerItem(item: MediaViewerItem) {
    const fileIndex = viewerFiles.findIndex((file) => (file.path || file.name) === item.id || file.name === item.name);
    const file = viewerFiles[fileIndex];
    if (!file) return;
    const job = attachmentDownload(file);
    if (job && !['failed', 'cancelled', 'missing'].includes(job.status)) return;
    await download(file, Math.max(0, fileIndex));
  }

  function getFileExtension(filename?: string) {
    if (!filename) return 'FILE';
    const parts = filename.split('.');
    if (parts.length <= 1) return 'FILE';
    const ext = parts.pop()!;
    return ext.length > 5 ? 'FILE' : ext.toUpperCase();
  }
  
  let saved = $derived(post ? libraryState.isSaved(post) : false);
  let saving = $derived(post ? libraryState.isPending(post) : false);
  let stashes = $derived(libraryState.collections.filter((collection) => collection.kind === 'stash'));
  let stashOptions = $derived(stashes.map((s) => ({ value: s.id, label: s.name })));
  let postStashes = $derived(post ? libraryState.getCustomPostStashes(post) : []);
  let postStashNames = $derived(post ? libraryState.getPostStashNames(post) : []);
  let stashSelectPlaceholder = $derived.by(() => {
    if (postStashes.length === 0) return i18n.t('library.add_to_stash') || 'Add to stash';
    if (postStashes.length === 1) return postStashNames[0] || (i18n.t('library.add_to_stash') || 'Add to stash');
    return i18n.t('library.in_stashes_count', { count: postStashes.length }) || `${postStashes.length} stashes`;
  });
  let authenticated = $derived(accountState.session.authenticated);

  let isFavorited = $state(false);
  let favoritingPending = $state(false);
  let mediaPort = $state<number | null>(null);
  let creatorAvatar = $state('');
  let creatorAvatarFailed = $state(false);
  let deletingDownloadId = $state<string | null>(null);
  let downloadingAll = $state(false);
  let probedMediaSizes = $state<Record<string, number>>({});
  const probingMediaPaths = new Set<string>();
  let totalMediaBytes = $derived.by(() => {
    if (media.length === 0) return 0;

    let total = 0;
    for (const file of media) {
      const job = attachmentDownload(file);
      const size = (file.size ?? 0) > 0
        ? file.size!
        : file.path && (probedMediaSizes[file.path] ?? 0) > 0
          ? probedMediaSizes[file.path]
          : Math.max(job?.total_bytes ?? 0, job?.downloaded_bytes ?? 0);
      if (size <= 0) return 0;
      total += size;
    }
    return total;
  });

  let creatorName = $state('');

  let deferredAttachments = $derived.by(() => {
    if (!post?.attachments) return [];
    return post.attachments.filter((att) => att.deferred === true || (!att.path && att.name));
  });

  let limitWarningText = $derived.by(() => {
    if (deferredAttachments.length === 0) {
      const explicitWarning = typeof post?.warning === 'string'
        ? post.warning
        : typeof (post as any)?.extra?.warning === 'string'
          ? (post as any).extra.warning
          : '';
      if (explicitWarning) return explicitWarning;
      if (
        post?.detail_fetched &&
        (post.attachment_count ?? 0) > 0 &&
        media.length === 0 &&
        !post.file?.path &&
        !richContent.includes('<img') &&
        !richContent.includes('<video')
      ) {
        return i18n.t('post.files_exceed_limit_warning', { details: '' }) || 'Some files exceed the archive size limit and were not saved.';
      }
      return '';
    }

    const videoCount = deferredAttachments.filter((f) => isVideoUrl(f.name || '') || /\.(mp4|mkv|webm|mov|avi|flv|wmv|m4v)(?:$|[?#])/i.test(f.name || '')).length;
    const photoCount = deferredAttachments.filter((f) => isImageUrl(f.name || '') || /\.(png|jpe?g|gif|webp|bmp|avif)(?:$|[?#])/i.test(f.name || '')).length;
    const otherCount = deferredAttachments.length - videoCount - photoCount;

    const parts: string[] = [];
    if (videoCount > 0) parts.push(`${videoCount} ${i18n.t('post.video_count', { count: videoCount }) || (videoCount === 1 ? 'video' : 'videos')}`);
    if (photoCount > 0) parts.push(`${photoCount} ${i18n.t('post.photo_count', { count: photoCount }) || (photoCount === 1 ? 'image' : 'images')}`);
    if (otherCount > 0) parts.push(`${otherCount} ${i18n.t('post.file_count', { count: otherCount }) || (otherCount === 1 ? 'file' : 'files')}`);
    const details = parts.join(', ') || `${deferredAttachments.length} ${i18n.t('post.file_count', { count: deferredAttachments.length }) || 'files'}`;

    return i18n.t('post.files_exceed_limit_warning', { details }) ||
      `Some files exceed the archive size limit and weren't saved: ${details}. Please note these limits are in place to keep this site running long term, without costing a fortune. You can favorite the creator though, certain milestones increase the limit.`;
  });

  $effect(() => {
    if (service && creatorId && postId) {
      void contentState.loadPost(service, creatorId, postId).then(() => {
        void checkFavoriteStatus();
      });
    }
  });

  $effect(() => {
    if (service && creatorId && post) {
      if (post.prev) {
        void contentState.loadPost(service, creatorId, post.prev);
      }
      if (post.next) {
        void contentState.loadPost(service, creatorId, post.next);
      }
    }
  });

  $effect(() => {
    if (service && creatorId) {
      const cacheKey = `${service.toLowerCase()}:${creatorId.toLowerCase()}`;
      const cachedName = creatorsState.creatorsMap.get(cacheKey) || contentState.creators[cacheKey]?.profile?.name;
      if (typeof cachedName === 'string' && cachedName) {
        creatorName = cachedName;
      } else {
        void apiFetchCreatorProfile(service, creatorId)
          .then((profile) => {
            if (typeof profile?.name === 'string' && profile.name) {
              creatorName = profile.name;
              const cachedCreator = contentState.getCreator(service, creatorId);
              cachedCreator.profile = profile;
            }
          })
          .catch((err) => {
            console.warn('Failed to load creator profile:', err);
          });
      }

      void apiFetchCreatorArtworkDataUrl(service, creatorId, 'avatar')
        .then((avatar) => {
          creatorAvatar = avatar;
          creatorAvatarFailed = false;
        })
        .catch(() => (creatorAvatar = ''));
    }
  });

  onMount(() => {
    void downloadState.init();
    void apiGetAxumPort().then((port) => (mediaPort = port)).catch(() => (mediaPort = null));

    try {
      const v = document.createElement('video');
      const canPlay = v.canPlayType('video/mp4; codecs="hevc"') || v.canPlayType('video/mp4; codecs="hvc1"');
      hevcSupported = canPlay === 'probably' || canPlay === 'maybe';
    } catch (e) {
      hevcSupported = false;
    }
  });

  function getAverageColor(url: string): Promise<string> {
    return new Promise((resolve) => {
      const img = new Image();
      img.crossOrigin = 'Anonymous';
      img.onload = () => {
        try {
          const canvas = document.createElement('canvas');
          canvas.width = 1;
          canvas.height = 1;
          const ctx = canvas.getContext('2d');
          if (!ctx) return resolve('');
          ctx.drawImage(img, 0, 0, 1, 1);
          const [r, g, b] = ctx.getImageData(0, 0, 1, 1).data;
          resolve(`rgb(${r}, ${g}, ${b})`);
        } catch (e) {
          console.warn('Canvas color extraction failed:', e);
          resolve('');
        }
      };
      img.onerror = () => resolve('');
      img.src = url;
    });
  }

  $effect(() => {
    if (post && post.file && configState.settings.dynamic_accent && isImageUrl(fileUrl(post.file))) {
      const url = fileUrl(post.file);
      void getAverageColor(url).then((color) => {
        if (color) {
          const root = document.documentElement;
          root.style.setProperty('--accent-primary', color);
          root.style.setProperty('--accent-primary-hover', color);
          const glowColor = color.replace('rgb', 'rgba').replace(')', ', 0.35)');
          root.style.setProperty('--accent-glow', glowColor);
          root.style.setProperty('--text-on-accent', getContrastColor(color));
        }
      });
    }

    return () => {
      themeState.applyCssTokens();
    };
  });

  async function checkFavoriteStatus() {
    try {
      const favorites = await accountState.fetchFavorites('post');
      isFavorited = favorites.some((fav) => String(fav.id) === String(postId) && fav.service === service);
    } catch (error) {
      console.error('Failed to check post favorite status:', error);
    }
  }

  async function toggleFavorite() {
    if (!post || favoritingPending) return;
    favoritingPending = true;
    const targetState = !isFavorited;
    try {
      await apiSetPostFavorite(service, creatorId, postId, targetState);
      isFavorited = targetState;
      if (!authenticated) {
        toast.success(i18n.t(targetState ? 'favorites.saved_locally' : 'favorites.removed_locally'));
      } else {
        toast.success(i18n.t(targetState ? 'post.added_to_favorites' : 'post.removed_from_favorites'));
      }
      if (post.favorite_count !== undefined) {
        post.favorite_count = Math.max(0, post.favorite_count + (targetState ? 1 : -1));
      }
      if (targetState) {
        accountState.addPostFavoriteOptimistic(post);
      } else {
        accountState.removePostFavoriteOptimistic(service, creatorId, postId);
      }
    } catch (error) {
      console.error('Failed to toggle post favorite:', error);
      toast.error(i18n.t('post.favorite_failed'));
    } finally {
      favoritingPending = false;
    }
  }

  function remoteFileUrl(file: { path?: string; server?: string }) {
    const cdn = file.server || `https://${configState.settings.file_domain}`;
    return `${cdn}/data${file.path}`;
  }

  function fileUrl(file: { path?: string; server?: string }) {
    const localDownload = downloadState.downloads.find((item) => item.service === service && item.creator_id === creatorId && item.post_id === postId && item.media_id === file.path && item.status === 'completed');
    const localPath = localDownload?.final_path || (file.path === post?.file?.path ? String(post?.local_preview_path || '') : '');
    if (localPath && mediaPort) {
      const encoded = localPath.replace(/\\/g, '/').split('/').map((part) => encodeURIComponent(part)).join('/');
      return `http://127.0.0.1:${mediaPort}/media/${encoded}`;
    }
    return remoteFileUrl(file);
  }

  function probeAttachmentSize(file?: Attachment) {
    const path = file?.path;
    if (!path || (file.size ?? 0) > 0 || probedMediaSizes[path] || probingMediaPaths.has(path)) return;
    probingMediaPaths.add(path);
    void apiProbeDownloadSize(remoteFileUrl(file))
      .then((size) => {
        if (size && size > 0) probedMediaSizes = { ...probedMediaSizes, [path]: size };
      })
      .catch(() => undefined);
  }

  $effect(() => {
    if (!post) return;
    probeAttachmentSize(post.file);
    for (const file of post.attachments ?? []) probeAttachmentSize(file);
  });

  function attachmentDownload(file?: { path?: string }) {
    if (!file?.path) return undefined;
    return downloadState.downloads.find((item) =>
      item.service === service &&
      item.creator_id === creatorId &&
      item.post_id === postId &&
      item.media_id === file.path
    );
  }

  async function deleteDownload(item: DownloadItem) {
    if (deletingDownloadId) return;
    deletingDownloadId = item.id;
    const filename = item.filename;
    try {
      await downloadState.remove(item.id);
      toast.success(i18n.t('post.download_deleted', { filename }));
    } catch (error) {
      toast.error(i18n.t('post.download_delete_failed'), { description: String(error) });
    } finally {
      deletingDownloadId = null;
    }
  }

  async function download(file: { path?: string; server?: string; name?: string }, index: number) {
    if (!file.path) return;
    try {
      if (!post) return;
      await downloadState.start(post, file.path, fileUrl(file), file.name || `${postId}_${index + 1}`);
      toast.success(i18n.t('feed.download_started', { title: file.name || postId }));
    } catch (error) {
      toast.error(i18n.t('feed.download_failed', { error: String(error) }));
    }
  }

  async function downloadAllMedia() {
    if (!post || downloadingAll) return;

    const pendingMedia = media.filter((file) => {
      const job = attachmentDownload(file);
      return file.path && (!job || ['failed', 'cancelled', 'missing'].includes(job.status));
    });
    if (pendingMedia.length === 0) return;

    downloadingAll = true;
    try {
      const results = await Promise.allSettled(
        pendingMedia.map((file, index) =>
          downloadState.start(post!, file.path!, remoteFileUrl(file), file.name || `${postId}_${index + 1}`)
        )
      );
      const started = results.filter((result) => result.status === 'fulfilled').length;
      const failed = results.length - started;

      if (started > 0) toast.success(i18n.t('post.download_all_started', { count: started }));
      if (failed > 0) toast.error(i18n.t('post.download_all_failed', { count: failed }));
    } finally {
      downloadingAll = false;
    }
  }

  async function toggleLibrary() {
    if (!post) return;
    const wasSaved = saved;
    try {
      await libraryState.toggle(post);
      toast.success(i18n.t(wasSaved ? 'library.removed' : 'library.saved'));
    } catch (error) {
      toast.error(i18n.t('library.save_error'), { description: String(error) });
    }
  }

  async function handleStashToggle(collectionId: string) {
    if (!post || !collectionId) return;
    const isCurrentlyIn = postStashes.includes(collectionId);
    try {
      if (isCurrentlyIn) {
        await libraryState.removeFromStash(collectionId, post);
        toast.success(i18n.t('library.removed_from_stash') || 'Removed from stash');
      } else {
        await libraryState.save(post, collectionId);
        toast.success(i18n.t('library.added_to_stash') || 'Added to stash');
      }
    } catch (error) {
      toast.error(error instanceof Error ? error.message : String(error));
    }
  }

  async function handleCreateStash(name: string) {
    if (!post || !name.trim()) return;
    try {
      const newStash = await libraryState.createStash(name.trim());
      await libraryState.save(post, newStash.id);
      toast.success(i18n.t('library.added_to_stash') || 'Added to stash');
    } catch (error) {
      toast.error(error instanceof Error ? error.message : String(error));
    }
  }

  $effect(() => {
    if (post) {
      void libraryState.loadPostStashes(post);
    }
  });

  let comments = $state<Comment[]>([]);
  let loadingComments = $state(false);
  let commentsError = $state<string | null>(null);
  let commentsExpanded = $state(false);
  let commentsHeight = $state(0);
  const MAX_COMMENTS_HEIGHT = 600;
  let isCommentsOverflowing = $derived(commentsHeight > MAX_COMMENTS_HEIGHT);
  let commentsSort = $state<'newest' | 'oldest'>('newest');

  async function loadComments() {
    if (!postId) return;
    loadingComments = true;
    commentsError = null;
    try {
      comments = await apiFetchPostComments(service, creatorId, postId);
    } catch (err) {
      const errMsg = String(err);
      if (errMsg.includes('404') || errMsg.toLowerCase().includes('not found')) {
        comments = [];
      } else {
        console.error('Failed to load comments:', err);
        commentsError = errMsg;
      }
    } finally {
      loadingComments = false;
    }
  }

  $effect(() => {
    if (postId) {
      void loadComments();
    }
  });

  $effect(() => {
    const previousId = post?.prev;
    const nextId = post?.next;
    if (previousId) void contentState.loadPost(service, creatorId, previousId);
    if (nextId) void contentState.loadPost(service, creatorId, nextId);
  });

  let rootComments = $derived.by(() => {
    const ids = new Set(comments.map(c => c.id));
    const list = comments.filter(c => !c.parent_id || !ids.has(c.parent_id));
    return list.sort((a, b) => {
      const timeA = new Date(a.published).getTime();
      const timeB = new Date(b.published).getTime();
      return commentsSort === 'newest' ? timeB - timeA : timeA - timeB;
    });
  });

  function getReplies(commentId: string) {
    return comments
      .filter(c => c.parent_id === commentId)
      .sort((a, b) => new Date(a.published).getTime() - new Date(b.published).getTime());
  }

</script>

{#snippet mediaDownloadAction(file: Attachment, index: number)}
  {@const job = attachmentDownload(file)}
  {@const downloaded = job?.status === 'completed' ? job : undefined}
  {@const active = job && ['queued', 'resolving', 'downloading', 'paused', 'verifying'].includes(job.status)}
  {@const verifying = job?.status === 'verifying'}
  {@const knownTotal = job?.total_bytes || 0}
  {@const declaredBytes = file.size && file.size > 0 ? file.size : file.path ? probedMediaSizes[file.path] ?? 0 : 0}
  {@const declaredSize = declaredBytes > 0 ? formatBytes(declaredBytes) : ''}
  {@const hasProgress = Boolean(active && !verifying && knownTotal > 0)}
  {@const progress = job && knownTotal > 0 ? Math.min(100, Math.round(job.downloaded_bytes / knownTotal * 100)) : 0}
  <Button
    variant="ghost"
    disabled={Boolean(active) || deletingDownloadId === downloaded?.id}
    onclick={() => downloaded ? void deleteDownload(downloaded) : void download(file, index)}
    class={`media-download-btn${downloaded ? ' is-downloaded' : ''}${active ? ' is-downloading' : ''}${deletingDownloadId === downloaded?.id ? ' is-deleting' : ''}`}
    title={i18n.t(downloaded ? 'post.delete_download' : verifying ? 'downloads.status_verifying' : active ? 'post.downloading' : 'post.download')}
  >
    {#if active}
      {#if hasProgress}<span class="attachment-progress-fill" style:width={`${progress}%`}></span>{/if}
      <span class="attachment-button-state downloading-state">
        {#if hasProgress}<IconDownload />{:else}<IconLoading />{/if}
        <span>{verifying ? i18n.t('downloads.status_verifying') : i18n.t('post.downloading')}{hasProgress ? ` · ${progress}%` : ''}</span>
      </span>
    {:else if downloaded && deletingDownloadId === downloaded.id}
      <span class="attachment-button-state"><IconLoading /><span>{i18n.t('post.deleting')}</span></span>
    {:else if downloaded}
      <span class="attachment-state-stack">
        <span class="attachment-button-state downloaded-state"><IconCheck /><span>{i18n.t('post.downloaded')} · {formatBytes(Math.max(downloaded.total_bytes, downloaded.downloaded_bytes))}</span></span>
        <span class="attachment-button-state delete-state"><IconDelete /><span>{i18n.t('post.delete_download')}</span></span>
      </span>
    {:else}
      <span class="attachment-button-state"><IconDownload /><span>{i18n.t('post.download')}{declaredSize ? ` · ${declaredSize}` : ''}</span></span>
    {/if}
  </Button>
{/snippet}

<PageShell scrollable={true} scrollKey={navigationState.entryKey}>
  {#snippet overlay()}
    <StickyHeader threshold={120}>
      <div class="sticky-post-info">
        <Button variant="ghost" onclick={() => navigationState.back()} class="sticky-back-btn" title={i18n.t('nav.back')}>
          <IconArrowLeft class="w-[20px] h-[20px]" />
        </Button>
        <span class="sticky-post-title">{post?.title || ''}</span>
      </div>

      <div class="sticky-post-actions">
        <Button
          variant="ghost"
          onclick={() => navigationState.openCreator(service, creatorId)}
          class="sticky-creator-btn"
        >
          {#if creatorAvatar && !creatorAvatarFailed}
            <span class="post-creator-avatar"><img src={creatorAvatar} alt="" onerror={() => creatorAvatarFailed = true} /></span>
          {:else}
            <ServiceIcon {service} />
          {/if}
          <span class="sticky-creator-name">{creatorName}</span>
        </Button>
        {#if post}
          <Button
            variant={isFavorited ? 'accent' : 'ghost'}
            disabled={favoritingPending}
            onclick={toggleFavorite}
            class="sticky-action-btn"
            title={i18n.t(isFavorited ? 'post.unfavorite' : 'post.favorite')}
          >
            {#if isFavorited}
              <IconHeartFilled class="w-[20px] h-[20px] fav-active-heart" />
            {:else}
              <IconHeart class="w-[20px] h-[20px]" />
            {/if}
            <span class="btn-text">{i18n.t(isFavorited ? 'post.unfavorite' : 'post.favorite')}</span>
          </Button>

          <Button
            variant={saved ? 'accent' : 'ghost'}
            disabled={saving}
            onclick={() => void toggleLibrary()}
            class="sticky-action-btn"
            title={i18n.t(saved ? 'library.saved' : 'library.save')}
          >
            {#if saved}
              <IconSaved class="w-[20px] h-[20px]" />
            {:else}
              <IconSave class="w-[20px] h-[20px]" />
            {/if}
            <span class="btn-text">{i18n.t(saved ? 'library.saved' : 'library.save')}</span>
          </Button>

          <div class="sticky-stash-select">
            <Select
              options={stashOptions}
              selectedValues={postStashes}
              placeholder={stashSelectPlaceholder}
              onchange={handleStashToggle}
              createLabel={i18n.t('library.new_stash')}
              onCreate={handleCreateStash}
              variant={postStashes.length > 0 ? 'accent' : 'ghost'}
              multi={true}
              closeOnChange={false}
              icon={IconFolder}
            />
          </div>

        {/if}
      </div>
    </StickyHeader>
  {/snippet}

  {#if post && post.file && isImageUrl(fileUrl(post.file))}
    <HeroBackdrop src={fileUrl(post.file)} />
  {/if}

  <div class="post-content-wrapper">
    <div class="post-actions-bar">
      <Button variant="ghost" onclick={() => navigationState.back()} class="action-btn">
        <IconArrowLeft class="w-[18px] h-[18px]" /> {i18n.t('nav.back')}
      </Button>

      {#if post}
        <Button
          variant="ghost"
          onclick={() => navigationState.openCreator(service, creatorId)}
          class="action-btn creator-btn"
        >
          {#if creatorAvatar && !creatorAvatarFailed}
            <span class="post-creator-avatar"><img src={creatorAvatar} alt="" onerror={() => creatorAvatarFailed = true} /></span>
          {:else}
            <ServiceIcon {service} />
          {/if}
          <span>{creatorName}</span>
        </Button>

        <Button
          variant={isFavorited ? 'accent' : 'ghost'}
          disabled={favoritingPending}
          onclick={toggleFavorite}
          class="action-btn"
        >
          {#if isFavorited}
            <IconHeartFilled class="w-[18px] h-[18px] fav-active-heart" />
          {:else}
            <IconHeart class="w-[18px] h-[18px]" />
          {/if}
          <span>{i18n.t(isFavorited ? 'post.unfavorite' : 'post.favorite')}</span>
        </Button>

        <div class="library-controls">
          <Button
            variant={saved ? 'accent' : 'primary'}
            disabled={saving}
            onclick={() => void toggleLibrary()}
            class="action-btn"
          >
            {#if saved}
              <IconSaved class="w-[18px] h-[18px]" />
            {:else}
              <IconSave class="w-[18px] h-[18px]" />
            {/if}
            <span>{i18n.t(saved ? 'library.saved' : 'library.save')}</span>
          </Button>

          <div class="stash-select-container">
            <Select
              options={stashOptions}
              selectedValues={postStashes}
              placeholder={stashSelectPlaceholder}
              onchange={handleStashToggle}
              createLabel={i18n.t('library.new_stash')}
              onCreate={handleCreateStash}
              variant={postStashes.length > 0 ? 'accent' : 'ghost'}
              multi={true}
              closeOnChange={false}
              icon={IconFolder}
              class="stash-select"
            />
          </div>
        </div>
      {/if}
    </div>

    {#if post}
      <header class="detail-header">
        <div class="min-w-0 flex-1">
          <h1>{post.title || i18n.t('feed.untitled')}</h1>
          <p class="post-date">{formatDate(post.published || post.added)}</p>
        </div>
      </header>

      {#if post.poll}
        <PostPoll poll={post.poll} />
      {/if}

      {#if entry.loading && !entry.loaded}
        <div class="detail-loading">{i18n.t('feed.loading')}</div>
      {/if}

      {#if media.length > 0 || hasEmbed}
        <div class="media-section">
          <div class="media-controls-row">
            {#if (media.length > 1 || (hasEmbed && media.length > 0)) && activeCategoriesCount > 1}
              <nav class="media-tabs" aria-label="Media categories">
                <Button variant={activeMediaTab === 'all' ? 'accent' : 'ghost'} onclick={() => activeMediaTab = 'all'}>
                  <IconGrid class="w-[16px] h-[16px]" />
                  <span>{i18n.t('post.tab_all')}</span>
                  <span class="media-tab-count">{mediaCounts.all}</span>
                </Button>
                {#if mediaCounts.video > 0}
                  <Button variant={activeMediaTab === 'video' ? 'accent' : 'ghost'} onclick={() => activeMediaTab = 'video'}>
                    <IconVideo class="w-[16px] h-[16px]" />
                    <span>{i18n.t('post.tab_video')}</span>
                    <span class="media-tab-count">{mediaCounts.video}</span>
                  </Button>
                {/if}
                {#if mediaCounts.photo > 0}
                  <Button variant={activeMediaTab === 'photo' ? 'accent' : 'ghost'} onclick={() => activeMediaTab = 'photo'}>
                    <IconImage class="w-[16px] h-[16px]" />
                    <span>{i18n.t('post.tab_photo')}</span>
                    <span class="media-tab-count">{mediaCounts.photo}</span>
                  </Button>
                {/if}
                {#if mediaCounts.file > 0}
                  <Button variant={activeMediaTab === 'file' ? 'accent' : 'ghost'} onclick={() => activeMediaTab = 'file'}>
                    <IconDocument class="w-[16px] h-[16px]" />
                    <span>{i18n.t('post.tab_file')}</span>
                    <span class="media-tab-count">{mediaCounts.file}</span>
                  </Button>
                {/if}
              </nav>
            {/if}

            {#if media.length > 1}
              <div class="media-sort-selector">
                <Select
                  options={[
                    { value: 'default', label: i18n.t('post.media_sort_default') || 'Default Order' },
                    { value: 'name_asc', label: i18n.t('post.media_sort_name_asc') || 'Name (A-Z)' },
                    { value: 'name_desc', label: i18n.t('post.media_sort_name_desc') || 'Name (Z-A)' },
                    { value: 'size_desc', label: i18n.t('post.media_sort_size_desc') || 'Size (Largest)' },
                    { value: 'size_asc', label: i18n.t('post.media_sort_size_asc') || 'Size (Smallest)' }
                  ]}
                  value={mediaSort}
                  onchange={(val) => mediaSort = val as any}
                  variant="ghost"
                />
              </div>
            {/if}
          </div>

          {#if filteredMedia.length > 0 || (hasEmbed && (activeMediaTab === 'all' || activeMediaTab === 'video'))}
            <div class="media-gallery-container" class:is-collapsed={isGalleryOverflowing && !galleryExpanded}>
              <section class="media-gallery" bind:clientHeight={galleryHeight} aria-label={i18n.t('post.media')}>
                {#if postEmbed && (activeMediaTab === 'all' || activeMediaTab === 'video')}
                  <div class="media-item is-embed-item">
                    <div class="media-header">
                      <span class="media-embed-tag">
                        <IconGlobe class="w-[12px] h-[12px]" />
                        <span>{postEmbed.provider || postEmbed.provider_url || 'Embed'}</span>
                      </span>
                      <span class="media-filename">{postEmbed.subject || postEmbed.description || post.title}</span>
                    </div>

                    {#if postEmbed.html}
                      <div class="media-embed-player">
                        {@html postEmbed.html}
                      </div>
                    {:else if postEmbed.url && isVideoUrl(postEmbed.url)}
                      <!-- svelte-ignore a11y_media_has_caption -->
                      <video src={postEmbed.url} controls preload="metadata"></video>
                    {:else}
                      <button
                        class="file-placeholder media-open-surface"
                        type="button"
                        onclick={() => embedAttachment && openMediaViewer(embedAttachment, filteredMedia)}
                        aria-label={postEmbed.subject || 'External Video'}
                      >
                        <IconVideo class="placeholder-icon" />
                        <p class="placeholder-text">{postEmbed.subject || postEmbed.description || 'External Video'}</p>
                      </button>
                    {/if}

                    {#if embedAttachment}
                      <button
                        class="media-viewer-open-btn"
                        type="button"
                        onclick={() => openMediaViewer(embedAttachment!, filteredMedia)}
                        title={i18n.t('post.viewer_open')}
                        aria-label={i18n.t('post.viewer_open')}
                      ><IconEye /></button>
                    {/if}

                    {#if postEmbed.url}
                      <Button
                        variant="ghost"
                        onclick={() => postEmbed?.url && void apiOpenInBrowser(postEmbed.url)}
                        class="media-download-btn is-embed-btn"
                        title={i18n.t('post.open_link')}
                      >
                        <span class="attachment-button-state">
                          <IconOpen class="w-[16px] h-[16px]" />
                          <span>{i18n.t('post.open_link')}</span>
                        </span>
                      </Button>
                    {/if}
                  </div>
                {/if}

                {#each filteredMedia as file, index}
                  {@const isDeferred = file?.deferred === true || (!file?.path && Boolean(file?.name))}
                  {@const url = file?.path ? fileUrl(file) : ''}
                  <div class="media-item" class:is-deferred={isDeferred}>
                    {#if isDeferred}
                      <div class="media-header">
                        <span class="media-filename">{file?.name || i18n.t('post.file')}</span>
                      </div>
                      <div class="file-placeholder media-open-surface is-deferred-placeholder" title={i18n.t('post.file_not_saved_desc')}>
                        <IconWarning class="placeholder-icon text-red-500" />
                        <p class="placeholder-text text-red-400">{i18n.t('post.file_not_saved')}</p>
                      </div>
                    {:else if isVideoUrl(url) || isImageUrl(url)}
                      <div class="media-header">
                        <span class="media-filename">{file?.name || i18n.t('post.file')}</span>
                        {#if file?.size}
                          <span class="media-filesize">({formatBytes(file.size)})</span>
                        {/if}
                      </div>
                      {#if isVideoUrl(url) && (videoFailures[index] || (isH265Video(file?.name) && !hevcSupported))}
                        <div class="video-placeholder">
                          <IconVideoOff class="placeholder-icon" />
                          <p class="placeholder-text">{i18n.t('post.unsupported_codec_desc')}</p>
                        </div>
                      {:else}
                        {#if isVideoUrl(url)}
                          <!-- svelte-ignore a11y_media_has_caption -->
                          <video
                            src={url}
                            controls
                            preload={index === 0 ? 'metadata' : 'none'}
                            onloadedmetadata={(e) => handleVideoMetadata(e, index)}
                            onplay={handleVideoPlay}
                          ></video>
                          <button
                            class="media-viewer-open-btn"
                            type="button"
                            onclick={(e) => openMediaViewer(file!, filteredMedia, e.currentTarget.parentElement?.querySelector('video'))}
                            title={i18n.t('post.viewer_open')}
                            aria-label={i18n.t('post.viewer_open')}
                          ><IconEye /></button>
                        {:else}
                          <button
                            class="media-open-surface"
                            type="button"
                            onclick={() => openMediaViewer(file!, filteredMedia)}
                            aria-label={`${i18n.t('post.viewer_open')}: ${file?.name || post.title}`}
                          >
                            <img src={url} alt={file?.name || post.title} loading={index < 2 ? 'eager' : 'lazy'} decoding="async" />
                          </button>
                        {/if}
                      {/if}
                      {@render mediaDownloadAction(file!, index)}
                    {:else}
                      {@const ext = getFileExtension(file?.name).toUpperCase()}
                      {@const sizeStr = file?.size ? formatBytes(file.size) : ''}
                      <div class="media-header">
                        <span class="media-filename">{file?.name || i18n.t('post.file')}</span>
                        {#if file?.size}
                          <span class="media-filesize">({formatBytes(file.size)})</span>
                        {/if}
                      </div>
                      <button class="file-placeholder media-open-surface" type="button" onclick={() => openMediaViewer(file!, filteredMedia)} aria-label={`${i18n.t('post.viewer_open')}: ${file?.name || i18n.t('post.file')}`}>
                        <IconDocument class="placeholder-icon" />
                        <p class="placeholder-text">{sizeStr ? `${ext} • ${sizeStr}` : ext}</p>
                      </button>
                      {@render mediaDownloadAction(file!, index)}
                    {/if}
                  </div>
                {/each}
              </section>
            </div>
            {#if isGalleryOverflowing && !galleryExpanded}
              <div class="gallery-fade-overlay">
                <Button variant="ghost" onclick={() => galleryExpanded = true} class="gallery-expand-btn">
                  <IconChevronDown class="w-[16px] h-[16px]" />
                  <span>{i18n.t('post.expand_gallery')}</span>
                </Button>
              </div>
            {:else if isGalleryOverflowing && galleryExpanded}
              <div class="gallery-collapse-action">
                <Button variant="ghost" onclick={() => galleryExpanded = false} class="gallery-collapse-btn">
                  <IconChevronUp class="w-[16px] h-[16px]" />
                  <span>{i18n.t('post.collapse_gallery')}</span>
                </Button>
              </div>
            {/if}
          {/if}
        </div>
      {/if}

      {#if richContent && !isHtmlContentEmpty(richContent)}
        <section class="post-content">
          <div class="html-content-container" class:is-collapsed={isOverflowing && !contentExpanded}>
            <div class="html-content" bind:clientHeight={contentHeight}>
              <RichContent html={richContent} currentService={service} currentCreatorId={creatorId} />
            </div>
          </div>

          {#if isOverflowing && !contentExpanded}
            <div class="content-fade-overlay">
              <Button variant="ghost" onclick={() => contentExpanded = true} class="expand-btn">
                <IconChevronDown class="w-[16px] h-[16px]" />
                <span>{i18n.t('post.read_more')}</span>
              </Button>
            </div>
          {:else if isOverflowing && contentExpanded}
            <div class="content-collapse-action">
              <Button variant="ghost" onclick={() => contentExpanded = false} class="collapse-btn">
                <IconChevronUp class="w-[16px] h-[16px]" />
                <span>{i18n.t('post.read_less')}</span>
              </Button>
            </div>
          {/if}
        </section>
      {/if}

      {#if post}
        <div class="post-footer-actions-row">
          {#if media.length > 0}
            <Button
              variant="ghost"
              disabled={downloadingAll || media.every((file) => {
                const job = attachmentDownload(file);
                return Boolean(job && !['failed', 'cancelled', 'missing'].includes(job.status));
              })}
              onclick={() => void downloadAllMedia()}
              class="post-footer-action"
            >
              {#if downloadingAll}
                <IconLoading class="w-[18px] h-[18px]" />
              {:else}
                <IconDownload class="w-[18px] h-[18px]" />
              {/if}
              <span>
                {i18n.t(downloadingAll ? 'post.downloading_all' : 'post.download_all')}{totalMediaBytes > 0 ? ` · ${formatBytes(totalMediaBytes)}` : ''}
              </span>
            </Button>
          {/if}

          {#if post.file}
            <Button variant="ghost" onclick={openPreviewViewer} class="post-footer-action">
              <IconEye class="w-[18px] h-[18px]" />
              <span>{i18n.t('post.view_preview')}</span>
            </Button>
          {/if}

          <Button
            variant="ghost"
            onclick={() => {
              const url = `https://${configState.settings.api_domain}/${service}/user/${creatorId}/post/${postId}`;
              void apiOpenInBrowser(url).catch((err) => console.warn('Failed to open post URL in browser:', err));
            }}
            class="post-footer-action"
            title={i18n.t('post.open_in_browser')}
          >
            <img src={pawchiveLogo} alt="" class="pawchive-action-icon" />
            <span>{i18n.t('post.open_in_browser')}</span>
          </Button>
        </div>

        <div class="post-footer-toolbar">
          <div class="footer-nav-left">
            <Button
              variant="ghost"
              disabled={!post.prev}
              onclick={() => post?.prev && navigationState.openPost(service, creatorId, post.prev)}
              class="footer-nav-btn"
              title={previousPostTitle}
            >
              <IconChevronLeft class="w-[18px] h-[18px]" />
              <span>{previousPostTitle}</span>
            </Button>
          </div>

          <div class="footer-nav-right">
            <Button
              variant="ghost"
              disabled={!post.next}
              onclick={() => post?.next && navigationState.openPost(service, creatorId, post.next)}
              class="footer-nav-btn"
              title={nextPostTitle}
            >
              <span>{nextPostTitle}</span>
              <IconChevronRight class="w-[18px] h-[18px]" />
            </Button>
          </div>
        </div>
      {/if}

      {#snippet commentNode(comment: Comment, depth = 0)}
        <div class="comment-item">
          <div class="comment-content-block">
            <div class="comment-meta">
              <span class="comment-author" class:is-creator={comment.commenter === creatorId}>
                {comment.commenter_name || comment.commenter}
              </span>
              {#if comment.commenter === creatorId}
                <span class="creator-badge">{i18n.t('post.creator_badge') || 'Creator'}</span>
              {/if}
              <span class="comment-date">{formatDate(comment.published)}</span>
              {#if comment.revisions && comment.revisions.length > 0}
                <span class="comment-edited" title={comment.revisions.map(r => `${formatDate(r.added)}: ${r.content}`).join('\n')}>
                  ({i18n.t('post.edited') || 'edited'})
                </span>
              {/if}
            </div>
            <div class="comment-body">
              <RichContent html={comment.content} currentService={service} currentCreatorId={creatorId} />
            </div>

            {#if getReplies(comment.id).length > 0}
              <div class="comment-replies-container">
                {#each getReplies(comment.id) as reply (reply.id)}
                  {@render commentNode(reply, depth + 1)}
                {/each}
              </div>
            {/if}
          </div>
        </div>
      {/snippet}

      <section class="comments-section">
        <div class="comments-header-row">
          <h2 class="comments-title">
            <IconComment class="w-[20px] h-[20px] opacity-75" />
            <span>{i18n.t('post.comments')}</span>
            {#if comments.length > 0}
              <span class="comments-count">({comments.length})</span>
            {/if}
          </h2>

          {#if comments.length > 1}
            <div class="comments-sort-selector">
              <Select
                options={[
                  { value: 'newest', label: i18n.t('post.sort_newest') || 'Newest' },
                  { value: 'oldest', label: i18n.t('post.sort_oldest') || 'Oldest' }
                ]}
                value={commentsSort}
                onchange={(val) => commentsSort = val as 'newest' | 'oldest'}
                variant="ghost"
              />
            </div>
          {/if}
        </div>

        {#if loadingComments}
          <div class="comments-empty">{i18n.t('feed.loading')}</div>
        {:else if commentsError}
          <div class="comments-empty">{commentsError}</div>
        {:else if comments.length === 0}
          <div class="comments-empty">{i18n.t('post.no_comments')}</div>
        {:else}
          <div class="comments-wrapper">
            <div
              class="comments-container"
              class:is-collapsed={isCommentsOverflowing && !commentsExpanded}
              class:is-expanded={commentsExpanded}
            >
              <div class="comments-list" bind:clientHeight={commentsHeight}>
                {#each rootComments as comment (comment.id)}
                  {@render commentNode(comment)}
                {/each}
              </div>
            </div>

            {#if isCommentsOverflowing && !commentsExpanded}
              <div class="comments-expand-action">
                <Button variant="ghost" onclick={() => commentsExpanded = true}>
                  <IconChevronDown class="w-[16px] h-[16px]" />
                  <span>{i18n.t('post.expand_comments') || 'Show All Comments'}</span>
                </Button>
              </div>
            {:else if isCommentsOverflowing && commentsExpanded}
              <div class="comments-expand-action">
                <Button variant="ghost" onclick={() => commentsExpanded = false}>
                  <IconChevronUp class="w-[16px] h-[16px]" />
                  <span>{i18n.t('post.collapse_comments') || 'Collapse Comments'}</span>
                </Button>
              </div>
            {/if}
          </div>
        {/if}
      </section>
    {:else if entry.error}
      <div class="detail-loading">{entry.error}</div>
    {:else}
      <div class="detail-loading">{i18n.t('feed.loading')}</div>
    {/if}
  </div>
</PageShell>

{#if viewerIndex !== null && viewerItems.length > 0}
  <MediaViewer
    items={viewerItems}
    initialIndex={viewerIndex}
    initialTime={viewerInitialTime}
    onclose={handleCloseViewer}
    ondownload={downloadViewerItem}
  />
{/if}

<style>
  .post-content-wrapper {
    position: relative;
    z-index: 2;
  }

  .post-actions-bar {
    position: relative;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 20px;
    padding-bottom: 14px;
    z-index: 10;
  }

  .post-actions-bar :global(.btn),
  .media-controls-row :global(.btn),
  .comments-header-row :global(.btn),
  .sticky-post-info :global(.btn),
  .sticky-post-actions :global(.btn) {
    height: 44px !important;
    padding: 0 18px !important;
    font-size: 13.5px !important;
    border-radius: var(--radius-full) !important;
    gap: 8px !important;
  }

  .post-actions-bar :global(.btn svg),
  .media-controls-row :global(.btn svg),
  .comments-header-row :global(.btn svg),
  .sticky-post-info :global(.btn svg),
  .sticky-post-actions :global(.btn svg) {
    width: 20px;
    height: 20px;
  }

  .post-actions-bar :global(.fav-active-heart) {
    color: var(--text-on-accent, var(--text-primary));
  }

  .post-creator-avatar {
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    flex: none;
    overflow: hidden;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.1);
  }

  .post-creator-avatar img {
    width: 100%;
    height: 100%;
    display: block;
    object-fit: cover;
  }

  .library-controls {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: auto;
  }

  .stash-select-container {
    width: 170px;
  }

  .stash-select-container :global(.select-trigger),
  .media-sort-selector :global(.select-trigger),
  .comments-header-row :global(.select-trigger) {
    height: 44px !important;
    font-size: 13.5px !important;
    padding: 0 18px !important;
    border-radius: var(--radius-full) !important;
  }

  .detail-header {
    position: relative;
    display: flex;
    align-items: flex-start;
    gap: 14px;
    padding-bottom: 20px;
    z-index: 10;
  }

  h1 {
    margin: 0;
    color: white;
    font-family: var(--font-sans);
    font-size: clamp(28px, 4.5vw, 42px);
    font-weight: var(--font-weight-normal);
    line-height: 1.12;
  }

  .post-date {
    margin-top: 6px;
    color: rgba(255, 255, 255, 0.4);
    font-size: 12px;
  }

  .detail-loading { min-height: 300px; display: grid; place-items: center; color: rgba(255,255,255,.5); font-size: 13px; }

  .media-controls-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 16px;
    margin-bottom: 20px;
  }

  .media-tabs {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .media-tabs :global(.btn) {
    gap: 6px !important;
  }

  .media-tab-count {
    opacity: 0.55;
    font-size: 12px;
    font-weight: 500;
  }

  .media-sort-selector {
    width: 200px;
    flex-shrink: 0;
  }

  .media-gallery {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 340px), 1fr));
    gap: 20px;
    max-width: 1000px;
    margin: 0 auto;
    padding: 16px 0;
  }
  
  .media-embed-tag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 7px;
    border-radius: var(--radius-full, 9999px);
    background: var(--accent-glow, rgba(255, 255, 255, 0.1));
    color: var(--accent-primary, #fff);
    font-size: 11px;
    font-weight: 600;
    flex-shrink: 0;
  }

  .media-embed-player {
    position: relative;
    width: 100%;
    border-radius: var(--radius-lg, 12px);
    overflow: hidden;
    background: #000;
    display: flex;
    justify-content: center;
    align-items: center;
    max-height: 520px;
  }

  .media-embed-player :global(iframe) {
    width: 100% !important;
    min-height: 280px;
    max-height: 520px;
    aspect-ratio: 16 / 9;
    border: 0;
    display: block;
  }

  .media-item {
    position: relative;
    width: 100%;
    height: auto;
    margin: 0;
    overflow: hidden;
    background: transparent;
    border: 0;
    border-radius: 0;
    box-shadow: none;
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    content-visibility: auto;
    contain-intrinsic-size: 320px 420px;
  }

  .media-viewer-open-btn {
    position: absolute;
    top: 44px;
    right: 10px;
    z-index: 15;
    width: 36px;
    height: 36px;
    padding: 0;
    border-radius: var(--radius-full, 9999px);
    background: rgba(0, 0, 0, 0.65);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    border: 1px solid rgba(255, 255, 255, 0.18);
    color: var(--text-primary, #fff);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    opacity: 0.85;
    transition: opacity var(--duration-fast, 150ms) ease, transform var(--duration-fast, 150ms) ease, background var(--duration-fast, 150ms) ease;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.35);
  }

  .media-viewer-open-btn:hover {
    opacity: 1;
    transform: scale(1.08);
    background: rgba(0, 0, 0, 0.85);
    border-color: rgba(255, 255, 255, 0.35);
  }

  .media-viewer-open-btn :global(svg) {
    width: 18px;
    height: 18px;
  }

  .media-header {
    width: 100%;
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 6px;
    padding: 8px 0px;
    box-sizing: border-box;
    margin-bottom: 6px;
  }

  .media-filename {
    font-size: 12.5px;
    color: var(--text-secondary);
    font-family: var(--font-sans);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    opacity: 0.85;
    min-width: 0;
  }

  .media-header:has(.media-filesize) .media-filename {
    flex-shrink: 1;
  }

  .media-filesize {
    font-size: 11.5px;
    color: var(--text-secondary);
    font-family: var(--font-sans);
    opacity: 0.55;
    flex-shrink: 0;
    white-space: nowrap;
  }

  .media-item img {
    position: relative;
    z-index: 2;
    display: block;
    width: 100%;
    height: auto;
    max-height: 520px;
    object-fit: contain;
    border-radius: 0;
    align-self: center;
  }

  .media-item video {
    position: relative;
    z-index: 2;
    display: block;
    width: auto;
    max-width: 100%;
    height: auto;
    min-height: 240px;
    max-height: 520px;
    object-fit: contain;
    border-radius: 0;
    background: transparent;
    align-self: center;
  }

  .video-placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    height: 240px;
    max-height: 520px;
    width: 100%;
    box-sizing: border-box;
    padding: 16px;
    background: transparent;
    border: 0;
  }

  .video-placeholder :global(.placeholder-icon) {
    width: 48px;
    height: 48px;
    color: var(--text-secondary);
    opacity: 0.6;
    display: block;
    margin: 0 auto;
  }

  .placeholder-text {
    margin: 0 auto;
    font-size: 13px;
    color: var(--text-secondary);
    max-width: 320px;
    line-height: 1.5;
    text-align: center;
    opacity: 0.8;
    transition: color var(--duration-fast) var(--ease-expo), opacity var(--duration-fast) var(--ease-expo);
  }

  .file-placeholder,
  .file-placeholder.media-open-surface {
    display: flex !important;
    flex-direction: column !important;
    align-items: center !important;
    justify-content: center !important;
    gap: 12px;
    height: 240px;
    max-height: 520px;
    width: 100%;
    box-sizing: border-box;
    padding: 16px;
    background: transparent;
    border: 0;
    cursor: pointer;
    text-align: center;
  }

  .file-placeholder :global(.placeholder-icon) {
    width: 48px;
    height: 48px;
    color: var(--text-secondary);
    opacity: 0.6;
    display: block;
    margin: 0 auto;
    transition: transform var(--duration-fast) var(--ease-expo), opacity var(--duration-fast) var(--ease-expo);
  }

  .file-placeholder:hover :global(.placeholder-icon) {
    opacity: 0.95;
    transform: scale(1.06);
  }

  .file-placeholder:hover .placeholder-text {
    color: var(--text-primary);
    opacity: 1;
  }

  .media-item :global(.media-download-btn) {
    margin-top: 12px;
    align-self: center;
  }

  :global(.media-download-btn) {
    position: relative;
    width: 220px;
    max-width: 100%;
    overflow: hidden;
    isolation: isolate;
  }

  :global(.media-download-btn.is-downloading:disabled) {
    opacity: 1;
  }

  .attachment-button-state {
    position: relative;
    z-index: 2;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    width: 100%;
    min-width: 0;
    white-space: nowrap;
  }

  .attachment-button-state :global(svg) {
    width: 16px;
    height: 16px;
    min-width: 16px;
    flex: 0 0 16px;
  }

  .attachment-button-state > span {
    min-width: 0;
  }

  .attachment-state-stack {
    display: grid;
    width: 100%;
    place-items: center;
  }

  .attachment-state-stack > .attachment-button-state {
    grid-area: 1 / 1;
  }

  .downloaded-state {
    color: var(--accent-primary);
    opacity: 1;
    visibility: visible;
    transition: opacity var(--duration-fast), visibility var(--duration-fast);
  }

  .delete-state {
    color: var(--danger, #ff626d);
    opacity: 0;
    visibility: hidden;
    transition: opacity var(--duration-fast), visibility var(--duration-fast);
  }

  :global(.media-download-btn.is-downloaded:hover) .downloaded-state {
    opacity: 0;
    visibility: hidden;
  }

  :global(.media-download-btn.is-downloaded:hover) .delete-state {
    opacity: 1;
    visibility: visible;
  }

  .attachment-progress-fill {
    position: absolute;
    z-index: 0;
    inset: 0 auto 0 0;
    border-radius: inherit;
    background: color-mix(in srgb, var(--accent-primary) 32%, transparent);
    pointer-events: none;
    transition: width 180ms linear;
  }

  .downloading-state {
    color: var(--text-primary);
  }

  .media-item :global(svg) {
    width: 16px;
    height: 16px;
  }

  .media-section {
    position: relative;
  }

  .media-gallery-container {
    position: relative;
    width: 100%;
    transition: max-height var(--duration-normal) var(--ease-expo);
  }

  .media-gallery-container.is-collapsed {
    max-height: 960px;
    overflow: hidden;
    mask-image: linear-gradient(to bottom, black 80%, transparent 98%);
    -webkit-mask-image: linear-gradient(to bottom, black 80%, transparent 98%);
  }

  .gallery-fade-overlay {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 120px;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    padding-bottom: 8px;
    z-index: 5;
    pointer-events: none;
  }

  .gallery-fade-overlay :global(.btn) {
    pointer-events: auto;
  }

  .gallery-collapse-action {
    display: flex;
    justify-content: center;
    margin-top: 14px;
  }

  .post-content {
    position: relative;
    margin-top: 24px;
  }

  .html-content-container {
    width: 100%;
    transition: max-height var(--duration-normal) var(--ease-expo);
  }

  .html-content-container.is-collapsed {
    max-height: 480px;
    overflow: hidden;
    mask-image: linear-gradient(to bottom, black 70%, transparent 95%);
    -webkit-mask-image: linear-gradient(to bottom, black 70%, transparent 95%);
  }

  .content-fade-overlay {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 120px;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    padding-bottom: 8px;
    z-index: 5;
    pointer-events: none;
  }

  .content-fade-overlay :global(.btn) {
    pointer-events: auto;
  }

  .content-collapse-action {
    display: flex;
    justify-content: center;
    margin-top: 14px;
  }

  .html-content {
    color: rgba(255, 255, 255, 0.85);
    font-size: 14.5px;
    line-height: 1.7;
    font-family: var(--font-sans);
    user-select: text;
  }

  .html-content :global(p) {
    margin: 0 0 16px 0;
  }

  .html-content :global(p:last-child) {
    margin-bottom: 0;
  }

  .html-content :global(a) {
    color: var(--accent-primary, #ffffff);
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  .html-content :global(img) {
    max-width: 100%;
    height: auto;
    border-radius: var(--radius-md);
    margin: 16px 0;
  }

  .html-content :global(h1),
  .html-content :global(h2),
  .html-content :global(h3),
  .html-content :global(h4) {
    font-family: var(--font-sans);
    font-weight: 600;
    color: var(--text-primary);
    margin: 24px 0 12px 0;
  }

  .html-content :global(ul),
  .html-content :global(ol) {
    margin: 0 0 16px 0;
    padding-left: 24px;
  }

  .html-content :global(li) {
    margin-bottom: 6px;
  }

  .html-content :global(blockquote) {
    margin: 16px 0;
    padding: 8px 16px;
    border-left: 3px solid var(--border-color);
    color: var(--text-secondary);
  }

  .html-content :global(pre) {
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    padding: 12px 16px;
    overflow-x: auto;
    font-family: monospace;
    font-size: 13px;
    margin: 16px 0;
  }

  .html-content :global(code) {
    font-family: monospace;
    font-size: 13px;
    background: var(--bg-card);
    padding: 2px 6px;
    border-radius: var(--radius-sm);
  }

  .comments-section {
    margin-top: 48px;
    padding-top: 32px;
    border-top: 1px solid rgba(255, 255, 255, 0.08);
  }

  .comments-header-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 24px;
    gap: 16px;
  }

  .comments-title {
    font-family: var(--font-sans);
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .comments-count {
    font-size: 14px;
    color: var(--text-secondary);
    font-weight: 400;
    opacity: 0.7;
  }

  .comments-sort-selector {
    width: 180px;
    flex-shrink: 0;
  }

  .comments-wrapper {
    position: relative;
    width: 100%;
  }

  .comments-container {
    max-height: 600px;
    overflow: hidden;
    transition: max-height 0.4s var(--ease-expo);
  }

  .comments-container.is-expanded {
    max-height: none;
  }

  .comments-container.is-collapsed {
    mask-image: linear-gradient(to bottom, black 80%, transparent 100%);
    -webkit-mask-image: linear-gradient(to bottom, black 80%, transparent 100%);
  }

  .comments-expand-action {
    display: flex;
    justify-content: center;
    padding: 24px 0 12px;
    position: relative;
    z-index: 10;
  }

  .comments-list {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  .comment-item {
    display: block;
    position: relative;
  }

  .comment-content-block {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .comment-meta {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    font-size: 13px;
    line-height: 1.4;
  }

  .comment-author {
    font-weight: 600;
    color: var(--text-primary);
    user-select: text;
    cursor: text;
  }

  .comment-author.is-creator {
    color: var(--accent-primary, #f97316);
  }

  .creator-badge {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 2px 6px;
    border-radius: 4px;
    background: var(--accent-primary, rgba(249, 115, 22, 0.2));
    color: var(--text-on-accent, white);
  }

  .comment-date {
    color: var(--text-secondary);
    opacity: 0.5;
    font-size: 12px;
    user-select: text;
    cursor: text;
  }

  .comment-edited {
    color: var(--text-secondary);
    opacity: 0.35;
    font-size: 11px;
    font-style: italic;
    cursor: help;
    user-select: text;
  }

  .comment-body {
    font-size: 14px;
    line-height: 1.55;
    color: var(--text-primary);
    opacity: 0.88;
    word-break: break-word;
    user-select: text;
    cursor: text;
  }

  .comment-body :global(*) {
    user-select: text;
  }

  .comment-body :global(a) {
    color: var(--accent-primary, #38bdf8);
    text-decoration: none;
    cursor: pointer;
  }

  .comment-body :global(a:hover) {
    text-decoration: underline;
  }

  .comment-body :global(p) {
    margin: 0 0 8px;
  }

  .comment-body :global(p:last-child) {
    margin-bottom: 0;
  }

  .comment-replies-container {
    display: flex;
    flex-direction: column;
    gap: 20px;
    margin-top: 16px;
    padding-left: 16px;
    position: relative;
  }

  .comment-replies-container::before {
    content: '';
    position: absolute;
    left: -8px;
    top: 0;
    bottom: 0;
    width: 2px;
    background: rgba(255, 255, 255, 0.06);
    border-radius: 1px;
  }

  .comments-empty {
    text-align: center;
    padding: 40px;
    color: var(--text-secondary);
    opacity: 0.5;
    font-size: 14px;
  }

  .sticky-post-info {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    flex: 1;
  }

  .sticky-post-info :global(.sticky-back-btn) {
    flex: 0 0 44px !important;
    width: 44px !important;
    height: 44px !important;
    min-width: 44px !important;
    border-radius: 50% !important;
    padding: 0 !important;
    display: flex !important;
    align-items: center !important;
    justify-content: center !important;
    flex-shrink: 0 !important;
  }

  .sticky-post-info :global(.sticky-back-btn svg) {
    width: 20px !important;
    height: 20px !important;
    flex-shrink: 0 !important;
  }

  .sticky-post-title {
    font-family: var(--font-sans);
    color: var(--text-primary);
    font-size: 15px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    opacity: 0.95;
    text-align: left;
    min-width: 0;
    flex: 1;
  }

  .sticky-post-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .sticky-post-actions :global(.btn-text) {
    display: none;
  }

  .sticky-stash-select {
    display: none;
  }

  :global(.sticky-header-bar.is-mobile) .sticky-post-actions :global(.btn) {
    width: 44px !important;
    height: 44px !important;
    min-width: 44px !important;
    flex: 0 0 44px !important;
    border-radius: 50% !important;
    padding: 0 !important;
    display: flex !important;
    align-items: center !important;
    justify-content: center !important;
  }

  :global(.sticky-header-bar.is-mobile) .sticky-creator-name {
    display: none !important;
  }

  :global(.sticky-header-bar.is-mobile) .sticky-post-actions :global(.btn svg) {
    width: 20px !important;
    height: 20px !important;
    flex-shrink: 0 !important;
  }

  :global(.sticky-header-bar:not(.is-mobile)) .sticky-post-actions {
    gap: 8px;
  }

  :global(.sticky-header-bar:not(.is-mobile)) .sticky-post-actions :global(.btn-text) {
    display: inline;
  }

  :global(.sticky-header-bar:not(.is-mobile)) .sticky-stash-select {
    display: block;
    width: 170px;
  }

  :global(.sticky-header-bar:not(.is-mobile)) .sticky-post-title {
    font-size: 18px !important;
    font-weight: 700 !important;
  }

  .post-footer-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    min-height: 44px;
    margin-top: 0;
    margin-bottom: 24px;
    padding-top: 0;
    border-top: none;
  }

  .detail-header h1,
  .post-date,
  .media-filename {
    user-select: text;
    cursor: text;
  }

  .post-footer-actions-row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin-top: var(--space-3);
    padding: var(--space-3) 0;
  }

  .footer-nav-left,
  .footer-nav-right {
    display: flex;
    align-items: center;
    flex: 1 1 0;
    min-width: 0;
  }

  .footer-nav-right {
    justify-content: flex-end;
  }

  .post-footer-toolbar :global(.footer-nav-btn) {
    min-width: 0;
    max-width: 100%;
  }

  .post-footer-toolbar :global(.footer-nav-btn span) {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.btn.post-footer-action),
  .post-footer-toolbar :global(.btn) {
    height: 44px !important;
    padding: 0 16px !important;
    font-size: 13.5px !important;
    border-radius: var(--radius-full) !important;
    gap: 6px !important;
  }

  :global(.btn.post-footer-action svg),
  .post-footer-toolbar :global(.btn svg) {
    width: 18px;
    height: 18px;
  }

  .pawchive-action-icon {
    width: 18px;
    height: 18px;
    flex: 0 0 18px;
    object-fit: contain;
    filter: brightness(0) invert(1);
  }

  .is-deferred-placeholder {
    cursor: default;
    background: transparent !important;
    border: 0 !important;
  }

  .is-deferred-placeholder :global(.placeholder-icon) {
    color: var(--text-muted);
    opacity: 0.6;
    width: 48px;
    height: 48px;
  }

  .is-deferred-placeholder .placeholder-text {
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 500;
    text-align: center;
    padding: 0 12px;
    line-height: 1.4;
  }
</style>
