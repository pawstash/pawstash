<script module lang="ts">
  const globalCloudFolderCache = new Map<string, import('$lib/types/cloud').CloudFolderResult>();
  const globalPostCloudNodes = new Map<string, import('$lib/types/pawchive').Attachment[]>();
  const globalProbedSizes = new Map<string, number>();
</script>

<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { contentState, postCacheKey, creatorCacheKey, type CachedPost } from '$lib/state/contentState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { configState } from '$lib/state/configState.svelte';
  import { libraryState } from '$lib/state/libraryState.svelte';
  import { downloadState } from '$lib/state/downloadState.svelte';
  import { accountState } from '$lib/state/accountState.svelte';
  import { themeState, getContrastColor } from '$lib/theme/themeState.svelte';
  import { creatorsState } from '$lib/state/creatorsState.svelte';
  import { apiFetchAccountFavorites, apiSetPostFavorite, apiFetchCreatorProfile, apiFetchCreatorArtworkDataUrl, apiOpenInBrowser, apiFetchPostComments, apiGetAxumPort, apiProbeDownloadSize, apiShowInFolder, apiStartDownload } from '$lib/utils/ipc';
  import type { Attachment, Comment, PawchivePost } from '$lib/types/pawchive';
  import type { DownloadItem } from '$lib/types/download';
  import type { LibraryCollection } from '$lib/types/library';
  import { i18n } from '$lib/i18n';
  import { toast } from 'svelte-sonner';
  import { formatDate, formatBytes, parseTags, cleanPostTitle, parseDateTimestamp } from '$lib/utils/formatters';
  import { isImageUrl, isVideoUrl, attachmentMediaUrl, attachmentThumbnailUrl, isAttachmentVideo, isAttachmentImage, postPageUrl, formatProviderName } from '$lib/utils/media';
  import { extractCloudLinks } from './RichContent.svelte';
  import { apiResolveCloudLink } from '$lib/utils/ipc';
  import { logger } from '$lib/utils/logger';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { handleGlobalPanicKey, panicCapture } from '$lib/utils/panic';
  import PageShell from '$lib/components/layout/PageShell.svelte';
  import StickyHeader from '$lib/components/layout/StickyHeader.svelte';
  import HeroBackdrop from '$lib/components/ui/HeroBackdrop.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { tooltip } from '$lib/motion';
  import Button from '$lib/components/ui/Button.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import SearchBar from '$lib/components/ui/SearchBar.svelte';
  import TagList from '$lib/components/ui/TagList.svelte';
  import CountBadge from '$lib/components/ui/CountBadge.svelte';
  import ServiceIcon from './ServiceIcon.svelte';
  import RichContent from './RichContent.svelte';
  import PostPoll from './PostPoll.svelte';
  import MediaViewer, { type MediaViewerItem, type MediaViewerKind } from './MediaViewer.svelte';
  import IconCloud from '~icons/fluent/cloud-24-regular';
  import IconSearch from '~icons/fluent/search-24-regular';
  import IconArrowLeft from '~icons/fluent/arrow-left-24-regular';
  import IconDownload from '~icons/fluent/arrow-download-24-regular';
  import IconArrowDownload from '~icons/fluent/arrow-download-24-regular';
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
  import IconSparkle from '~icons/fluent/sparkle-24-regular';
  import PopoverMenu from '$lib/components/ui/PopoverMenu.svelte';
  import CloudFolderModal from '$lib/components/content/CloudFolderModal.svelte';
  import type { CloudFolderResult, CloudNode } from '$lib/types/cloud';
  import { providerState } from '$lib/state/providerState.svelte';
  import { notify } from '$lib/utils/toast';

  interface PostEmbed {
    url?: string;
    subject?: string;
    description?: string;
    provider?: string;
    provider_url?: string;
    html?: string;
    [key: string]: unknown;
  }

  interface Props {
    service: string;
    creatorId: string;
    postId: string;
    initialMedia?: string;
    openViewer?: boolean;
  }
  let { service, creatorId, postId, initialMedia, openViewer }: Props = $props();

  const emptyEntry: CachedPost = { post: null, loading: false, loaded: false, error: null };
  let entry = $derived.by(() => contentState.posts[postCacheKey(service, creatorId, postId)] ?? emptyEntry);
  let rawPost = $derived(entry.post);

  let postKey = $derived(providerState.getPostKey(service, creatorId, postId));
  let postRevisions = $derived(providerState.postRevisions[postKey] || []);
  let selectedRevId = $derived(providerState.selectedRevision[postKey] ?? null);
  let candidateProviders = $derived(providerState.getProvidersForService(service));
  let providerSelectOptions = $derived.by(() => {
    if (candidateProviders.length <= 1) {
      return candidateProviders.map((p) => ({
        value: p.id,
        label: formatProviderName(p.name)
      }));
    }
    return [
      { value: 'auto', label: i18n.t('post.source_auto') || 'Merged' },
      ...candidateProviders.map((p) => ({
        value: p.id,
        label: formatProviderName(p.name)
      }))
    ];
  });
  let activeProviderId = $derived(
    candidateProviders.length === 1
      ? candidateProviders[0].id
      : providerState.getSelectedProvider(service, creatorId, postId)
  );

  let post = $derived.by(() => {
    if (selectedRevId !== null) {
      const found = postRevisions.find((r) => r.revision_id === selectedRevId);
      if (found) return ((found as any).post || found) as PawchivePost;
    }
    return rawPost;
  });

  let postEmbed = $derived<PostEmbed | null>(
    (post?.embed && typeof post.embed === 'object' && Object.keys(post.embed).length > 0)
      ? (post.embed as PostEmbed)
      : null
  );
  let creatorPosts = $derived.by(() => {
    const key = creatorCacheKey(service, creatorId);
    return contentState.creators[key]?.posts || [];
  });

  let currentPostIndexInCreator = $derived.by(() => {
    if (creatorPosts.length === 0) return -1;
    return creatorPosts.findIndex((p) => String(p.id) === String(postId));
  });

  let effectivePrevId = $derived.by(() => {
    if (post?.prev) return String(post.prev);
    if (currentPostIndexInCreator > 0) {
      return String(creatorPosts[currentPostIndexInCreator - 1].id);
    }
    return '';
  });

  let effectiveNextId = $derived.by(() => {
    if (post?.next) return String(post.next);
    if (currentPostIndexInCreator >= 0 && currentPostIndexInCreator < creatorPosts.length - 1) {
      return String(creatorPosts[currentPostIndexInCreator + 1].id);
    }
    return '';
  });

  let newerPost = $derived.by(() => {
    if (!effectivePrevId) return null;
    const prevKey = postCacheKey(service, creatorId, effectivePrevId);
    const cached = contentState.posts[prevKey]?.post;
    if (cached?.title) return cached;
    return creatorPosts.find((p) => String(p.id) === effectivePrevId) ?? cached ?? null;
  });

  let olderPost = $derived.by(() => {
    if (!effectiveNextId) return null;
    const nextKey = postCacheKey(service, creatorId, effectiveNextId);
    const cached = contentState.posts[nextKey]?.post;
    if (cached?.title) return cached;
    return creatorPosts.find((p) => String(p.id) === effectiveNextId) ?? cached ?? null;
  });

  let leftPostTitle = $derived(newerPost?.title?.trim() || i18n.t('post.next'));
  let rightPostTitle = $derived(olderPost?.title?.trim() || i18n.t('post.previous'));
  
  let richContent = $derived(post?.content || post?.substring || '');
  let postTags = $derived(parseTags(post?.tags));

  let publishedDateStr = $derived(formatDate(post?.published || post?.added));
  let editedDateStr = $derived(post?.edited ? formatDate(post.edited) : '');
  let addedDateStr = $derived(post?.added ? formatDate(post.added) : '');

  let showEdited = $derived(Boolean(
    post?.edited &&
    editedDateStr &&
    editedDateStr !== publishedDateStr
  ));

  let showImported = $derived(Boolean(
    post?.added &&
    post?.published &&
    addedDateStr &&
    addedDateStr !== publishedDateStr &&
    (!showEdited || addedDateStr !== editedDateStr)
  ));

  let revisionSelectOptions = $derived.by(() => {
    return [
      { value: 'latest', label: `${i18n.t('post.revision_current') || 'Latest'} [current]` },
      ...postRevisions.map((rev, idx) => {
        const revPost = (rev as any).post || rev;
        const revDate = revPost.edited || revPost.added || revPost.published;
        const providerName = (rev as any).provider_id ? ` • ${(rev as any).provider_id}` : '';
        return {
          value: String(rev.revision_id),
          label: `v${rev.revision_id || postRevisions.length - idx}${providerName} (${formatDate(revDate)})`
        };
      })
    ];
  });

  function onRevisionChange(val: string) {
    if (val === 'latest') {
      providerState.setSelectedRevision(service, creatorId, postId, null);
    } else {
      const revId = Number(val);
      providerState.setSelectedRevision(service, creatorId, postId, revId);
      const found = postRevisions.find((r) => r.revision_id === revId);
      const provId = (found as any)?.provider_id;
      if (provId && candidateProviders.some((p) => p.id === provId)) {
        providerState.setSelectedProvider(service, creatorId, postId, provId);
      }
    }
  }

  let resolvedCloudAttachments = $state<Attachment[]>([]);
  let cloudResolving = $state(false);
  let cloudResolvedVersion = $state(0);
  let activeCloudModalFolder = $state<CloudFolderResult | null>(null);
  let activeCloudModalInitialFolderId = $state<string | null>(null);
  let isCloudModalOpen = $state(false);
  let cloudFolderResults = $state<Map<string, CloudFolderResult>>(new Map(globalCloudFolderCache));

  function openCloudFolderModal(folderResult?: CloudFolderResult, folderId?: string | null) {
    if (!folderResult) return;
    activeCloudModalFolder = folderResult;
    activeCloudModalInitialFolderId = folderId || null;
    isCloudModalOpen = true;
  }

  async function downloadCloudSubfolder(folderResult?: CloudFolderResult, folderId?: string | null) {
    if (!folderResult) return;
    const targetNodes: CloudNode[] = [];
    const queue = [folderId];
    while (queue.length > 0) {
      const cur = queue.shift();
      for (const n of folderResult.nodes) {
        if (n.parent_id === cur) {
          if (n.is_folder) queue.push(n.id);
          else targetNodes.push(n);
        }
      }
    }
    if (targetNodes.length === 0) {
      targetNodes.push(...folderResult.nodes.filter((n) => !n.is_folder && (folderId ? n.parent_id === folderId : true)));
    }

    const targetPost = post || {
      id: folderResult.title,
      service: folderResult.provider,
      user: 'cloud',
      title: folderResult.title,
      content: folderResult.url
    };

    let started = 0;
    for (const node of targetNodes) {
      const port = mediaPort ?? 0;
      const streamUrl = node.stream_url?.startsWith('/cloud_stream/') && port > 0
        ? `http://127.0.0.1:${port}${node.stream_url}`
        : (node.download_url || '');
      if (!streamUrl) continue;
      try {
        await apiStartDownload(targetPost, node.id, streamUrl, node.name);
        started++;
      } catch (err) {
        console.error('Failed to queue download:', node.name, err);
      }
    }
    toast.success(
      i18n.t('feed.download_started') || 'Download started',
      { description: `${started} ${started === 1 ? 'file' : 'files'} added to queue` }
    );
  }

  async function handleOpenCloudFromText(url: string) {
    let existing = cloudFolderResults.get(url) || globalCloudFolderCache.get(url);
    if (!existing) {
      try {
        existing = await apiResolveCloudLink(url);
        cloudFolderResults.set(url, existing);
        globalCloudFolderCache.set(url, existing);
        cloudResolvedVersion++;
      } catch (err) {
        console.warn('Failed to resolve cloud link from text:', url, err);
      }
    }
    if (existing) {
      openCloudFolderModal(existing, null);
    } else {
      void apiOpenInBrowser(url);
    }
  }

  let lastResolvedPostKey = '';
  $effect(() => {
    const currentPostKey = postKey;
    const content = post?.content || post?.substring || '';
    let sources = content;
    if (postEmbed?.url) sources += ' ' + postEmbed.url;
    if (postEmbed?.html) sources += ' ' + postEmbed.html;
    if (postEmbed?.description) sources += ' ' + postEmbed.description;

    untrack(() => {
      const cachedNodes = globalPostCloudNodes.get(currentPostKey);
      if (cachedNodes && cachedNodes.length > 0) {
        resolvedCloudAttachments = cachedNodes;
        cloudResolving = false;
        return;
      }

      const cloudUrls = extractCloudLinks(sources);
      if (cloudUrls.length === 0) {
        resolvedCloudAttachments = [];
        globalPostCloudNodes.delete(currentPostKey);
        cloudResolving = false;
        return;
      }

      if (lastResolvedPostKey === currentPostKey && resolvedCloudAttachments.length > 0) {
        return;
      }
      lastResolvedPostKey = currentPostKey;
      cloudResolving = true;

      (async () => {
        const allCloudNodes: Attachment[] = [];
        for (const url of cloudUrls) {
          try {
            let res = globalCloudFolderCache.get(url);
            if (!res) {
              res = await apiResolveCloudLink(url);
              globalCloudFolderCache.set(url, res);
              logger.info(`[Cloud] Resolved link: ${url} (${res.nodes.length} items)`);
            }
            cloudFolderResults.set(url, res);

            const root = res.nodes.find((n) => n.is_folder && !n.parent_id);
            const rootId = root ? root.id : null;

            let topNodes = res.nodes.filter((n) => (rootId ? n.parent_id === rootId : !n.parent_id));
            if (topNodes.length === 0) {
              topNodes = res.nodes;
            }

            for (const node of topNodes) {
              if (node.is_folder) {
                const childrenCount = res.nodes.filter((n) => n.parent_id === node.id).length;
                allCloudNodes.push({
                  name: node.name,
                  path: `cloud_folder:${res.provider}:${node.id}`,
                  size: undefined,
                  server: '',
                  is_cloud: true,
                  is_cloud_folder: true,
                  cloud_provider: res.provider,
                  cloud_node_id: node.id,
                  cloud_folder_title: res.title,
                  cloud_folder_result: res,
                  cloud_child_count: childrenCount
                } as any);
              } else {
                allCloudNodes.push({
                  name: node.name,
                  path: node.stream_url || node.download_url || `cloud:${res.provider}:${node.id}`,
                  size: typeof node.size === 'number' && node.size > 0 ? node.size : undefined,
                  server: '',
                  is_cloud: true,
                  is_cloud_folder: false,
                  cloud_provider: res.provider,
                  cloud_node_id: node.id,
                  cloud_folder_title: res.title,
                  cloud_folder_result: res
                } as any);
              }
            }
          } catch (err) {
            console.warn('Failed to auto-resolve cloud link for post gallery:', url, err);
          }
        }

        if (postKey === currentPostKey) {
          resolvedCloudAttachments = allCloudNodes;
          if (allCloudNodes.length > 0) {
            globalPostCloudNodes.set(currentPostKey, allCloudNodes);
          }
          cloudResolving = false;
          cloudResolvedVersion++;
        }
      })();
    });
  });

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
    if (resolvedCloudAttachments.length > 0) {
      for (const att of resolvedCloudAttachments) {
        const exists = items.some(
          (existing) => (existing.path && existing.path === att.path) || (!existing.path && existing.name === att.name)
        );
        if (!exists) {
          items.push(att);
        }
      }
    }

    // Include completed downloaded items for this post (enables 100% offline playback)
    const postDownloads = downloadState.downloads.filter((d) =>
      d.service === service &&
      d.creator_id === creatorId &&
      d.post_id === postId &&
      d.status === 'completed' &&
      d.final_path
    );
    for (const d of postDownloads) {
      const port = mediaPort ?? 0;
      const encoded = d.final_path!.replace(/\\/g, '/').split('/').map((part) => encodeURIComponent(part)).join('/');
      const streamUrl = port > 0 ? `http://127.0.0.1:${port}/media/${encoded}` : d.url;
      const exists = items.some(
        (existing) => (existing.path && (existing.path === streamUrl || existing.path === d.url || existing.path === d.media_id)) ||
                      (existing.name && existing.name === d.filename)
      );
      if (!exists) {
        items.push({
          name: d.filename,
          path: streamUrl,
          size: d.total_bytes || d.downloaded_bytes,
          server: '',
          is_cloud: true,
          is_cloud_folder: false,
          cloud_provider: 'download'
        } as any);
      }
    }

    return items;
  });

  let activeMediaTab = $state<'all' | 'video' | 'photo' | 'file' | 'cloud'>('all');
  let mediaSort = $state<'default' | 'name_asc' | 'name_desc' | 'size_desc' | 'size_asc'>('default');
  let mediaSearchQuery = $state('');
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
    return /\b(h265|hevc|x265)\b|\.(h265|hevc)$/i.test(name);
  }

  function handleVideoError(e: Event, file?: Attachment | null, index?: number) {
    if (typeof index === 'number' && file) {
      const video = e.currentTarget as HTMLVideoElement;
      const remote = remoteFileUrl(file);
      if (remote && video.src !== remote) {
        console.warn('Local video playback failed, falling back to remote stream:', file.name);
        video.src = remote;
        return;
      }
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

  let isEmbedResolvedToCloud = $derived(Boolean(
    postEmbed?.url && (
      postEmbed.url.includes('iframely.net') ||
      postEmbed.url.includes('iframe.ly') ||
      postEmbed.url.includes('mega.nz') ||
      postEmbed.url.includes('dropbox.com') ||
      postEmbed.url.includes('pixeldrain.com')
    ) && resolvedCloudAttachments.length > 0
  ));

  let hasEmbed = $derived(Boolean(!isEmbedResolvedToCloud && postEmbed && (postEmbed.url || postEmbed.subject || postEmbed.html)));
  let embedMatchesSearch = $derived.by(() => {
    const q = mediaSearchQuery.trim().toLowerCase();
    if (!q) return true;
    const title = (postEmbed?.subject || postEmbed?.description || post?.title || '').toLowerCase();
    return title.includes(q);
  });

  let mediaCounts = $derived.by(() => {
    let videos = 0;
    let photos = 0;
    let files = 0;
    let cloud = 0;
    for (const file of media) {
      if ((file as any).is_cloud) {
        cloud++;
      }
      const url = file.path ? fileUrl(file) : (file.name || '');
      if (isAttachmentVideo(file, url)) videos++;
      else if (isAttachmentImage(file, url)) photos++;
      else files++;
    }
    if (hasEmbed) {
      videos++;
    }
    return {
      all: media.length + (hasEmbed ? 1 : 0),
      video: videos,
      photo: photos,
      file: files,
      cloud
    };
  });

  let activeCategoriesCount = $derived.by(() => {
    let count = 0;
    if (mediaCounts.video > 0) count++;
    if (mediaCounts.photo > 0) count++;
    if (mediaCounts.file > 0) count++;
    if (mediaCounts.cloud > 0) count++;
    return count;
  });

  let probedMediaSizes = $state<Record<string, number>>({});
  const probingMediaPaths = new Set<string>();

  function attachmentDownload(file?: { path?: string; name?: string; cloud_node_id?: string }) {
    if (!file) return undefined;
    return downloadState.downloads.find((item) =>
      item.service === service &&
      item.creator_id === creatorId &&
      item.post_id === postId &&
      (
        (Boolean(file.path) && item.media_id === file.path) ||
        (Boolean((file as any).cloud_node_id) && item.media_id === (file as any).cloud_node_id) ||
        (Boolean(file.name) && item.filename === file.name)
      )
    );
  }

  function getEffectiveFileSize(file?: Attachment | null): number {
    if (!file) return 0;
    if (typeof file.size === 'number' && file.size > 0) return file.size;
    if (typeof (file as any).filesize === 'number' && (file as any).filesize > 0) return (file as any).filesize;
    if (typeof (file as any).file_size === 'number' && (file as any).file_size > 0) return (file as any).file_size;
    if (typeof (file as any).bytes === 'number' && (file as any).bytes > 0) return (file as any).bytes;
    if (typeof file.size === 'string' && Number(file.size) > 0) return Number(file.size);
    if (file.path) {
      if (probedMediaSizes[file.path] && probedMediaSizes[file.path] > 0) return probedMediaSizes[file.path];
      const cached = globalProbedSizes.get(file.path);
      if (cached && cached > 0) return cached;
    }
    const job = attachmentDownload(file);
    if (job && (job.total_bytes > 0 || job.downloaded_bytes > 0)) {
      return Math.max(job.total_bytes, job.downloaded_bytes);
    }
    return 0;
  }

  let filteredMedia = $derived.by(() => {
    let list = [...media];
    if (activeMediaTab === 'video') {
      list = list.filter((file) => isAttachmentVideo(file, file.path ? fileUrl(file) : ''));
    } else if (activeMediaTab === 'photo') {
      list = list.filter((file) => isAttachmentImage(file, file.path ? fileUrl(file) : ''));
    } else if (activeMediaTab === 'file') {
      list = list.filter((file) => {
        const url = file.path ? fileUrl(file) : '';
        const isVid = isAttachmentVideo(file, url);
        const isImg = isAttachmentImage(file, url);
        return !isVid && !isImg;
      });
    } else if (activeMediaTab === 'cloud') {
      list = list.filter((file) => (file as any).is_cloud === true);
    }

    const query = mediaSearchQuery.trim().toLowerCase();
    if (query) {
      list = list.filter((file) => {
        const name = (file.name || '').toLowerCase();
        const path = (file.path || '').toLowerCase();
        const provider = String((file as any).cloud_provider || '').toLowerCase();
        return name.includes(query) || path.includes(query) || provider.includes(query);
      });
    }

    if (mediaSort === 'name_asc') {
      list.sort((a, b) => (a.name || '').localeCompare(b.name || '', undefined, { numeric: true, sensitivity: 'base' }));
    } else if (mediaSort === 'name_desc') {
      list.sort((a, b) => (b.name || '').localeCompare(a.name || '', undefined, { numeric: true, sensitivity: 'base' }));
    } else if (mediaSort === 'size_desc') {
      list.sort((a, b) => {
        const sizeA = getEffectiveFileSize(a);
        const sizeB = getEffectiveFileSize(b);
        if (sizeB !== sizeA) {
          return sizeB - sizeA;
        }
        return (a.name || '').localeCompare(b.name || '', undefined, { numeric: true, sensitivity: 'base' });
      });
    } else if (mediaSort === 'size_asc') {
      list.sort((a, b) => {
        const sizeA = getEffectiveFileSize(a);
        const sizeB = getEffectiveFileSize(b);
        if (sizeA === 0 && sizeB > 0) return 1;
        if (sizeB === 0 && sizeA > 0) return -1;
        if (sizeA !== sizeB) {
          return sizeA - sizeB;
        }
        return (a.name || '').localeCompare(b.name || '', undefined, { numeric: true, sensitivity: 'base' });
      });
    }

    return list;
  });

  function mediaViewerKind(file: Attachment, url: string): MediaViewerKind {
    const filename = `${file.name ?? ''} ${file.path ?? ''}`.toLocaleLowerCase();
    if (isAttachmentImage(file, url)) return 'image';
    if (isAttachmentVideo(file, url)) return 'video';
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
    const url = file.path ? fileUrl(file) : '';
    const job = attachmentDownload(file);
    const width = typeof file.width === 'number' && file.width > 0 ? file.width : undefined;
    const height = typeof file.height === 'number' && file.height > 0 ? file.height : undefined;
    return {
      id: file.path || `${file.name || 'media'}:${itemIndex}`,
      url,
      poster: attachmentThumbnailUrl(file, service),
      name: file.name || i18n.t('post.file'),
      kind: isEmbed ? 'video' : mediaViewerKind(file, url),
      size: getEffectiveFileSize(file) || file.size,
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

    let allSource = embedAttachment && !source.some((s) => s.path === embedAttachment!.path)
      ? [embedAttachment, ...source]
      : [...source];

    if ((file as any)?.cloud_folder_result) {
      const fRes = (file as any).cloud_folder_result as CloudFolderResult;
      const port = mediaPort ?? 0;
      const cloudFiles = fRes.nodes.filter((n) => !n.is_folder).map((n) => {
        const streamUrl = n.stream_url?.startsWith('/cloud_stream/') && port > 0
          ? `http://127.0.0.1:${port}${n.stream_url}`
          : (n.download_url || '');
        return {
          name: n.name,
          path: streamUrl || n.stream_url || n.download_url || `cloud:${fRes.provider}:${n.id}`,
          size: n.size,
          server: '',
          is_cloud: true,
          cloud_provider: fRes.provider,
          cloud_node_id: n.id,
          cloud_folder_title: fRes.title,
          cloud_folder_result: fRes
        } as Attachment;
      });
      for (const cf of cloudFiles) {
        if (!allSource.some((s) => s.path === cf.path || (s.name && cf.name && s.name === cf.name))) {
          allSource.push(cf);
        }
      }
    }

    const sourceItems = allSource
      .filter((item): item is Attachment => Boolean(item?.path || (item as any)?.html))
      .filter((item, itemIndex, list) => list.findIndex((candidate) => candidate.path === item.path) === itemIndex);

    if (post?.file?.path && !sourceItems.some((s) => s.path === post?.file?.path)) {
      sourceItems.push(post.file);
    }

    const nextIndex = sourceItems.findIndex((item) => (item.path && item.path === file.path) || (item.name && file.name && item.name === file.name));
    viewerFiles = nextIndex >= 0 ? sourceItems : [file, ...sourceItems];
    viewerIndex = nextIndex >= 0 ? nextIndex : 0;
  }

  let initialViewerHandled = $state(false);
  let lastHandledPostId = $state('');

  function findTargetAttachment(needleStr: string): Attachment | undefined {
    const raw = decodeURIComponent(needleStr).trim();
    const normNeedle = raw.toLowerCase().split('?')[0].split('#')[0].replace(/\\/g, '/');
    const needleBase = normNeedle.split('/').pop() || normNeedle;

    // 1. Direct search in post attachments / media
    const directMatch = media.find((f) => {
      const fPath = decodeURIComponent(f.path || '').toLowerCase().split('?')[0].split('#')[0].replace(/\\/g, '/');
      const fName = (f.name || '').toLowerCase();
      const fBase = fPath.split('/').pop() || fPath;
      return (
        fPath === normNeedle ||
        fName === normNeedle ||
        fBase === needleBase ||
        (fName && (normNeedle.endsWith(fName) || normNeedle.includes(fName) || fName.includes(needleBase) || needleBase.includes(fName))) ||
        (fPath && (normNeedle.endsWith(fPath) || normNeedle.includes(fPath) || fPath.includes(needleBase)))
      );
    });
    if (directMatch) return directMatch;

    // 2. Search across all resolved cloud folder results
    for (const [_, res] of cloudFolderResults) {
      const matchingNode = res.nodes.find((n) => {
        if (n.is_folder) return false;
        const nName = (n.name || '').toLowerCase();
        const nPath = decodeURIComponent(n.stream_url || n.download_url || n.id || '').toLowerCase().split('?')[0].split('#')[0].replace(/\\/g, '/');
        const nBase = nPath.split('/').pop() || nName;
        return (
          n.id === raw ||
          n.id === normNeedle ||
          nName === normNeedle ||
          nName === needleBase ||
          nName.includes(needleBase) ||
          needleBase.includes(nName) ||
          normNeedle.includes(nName) ||
          (nPath && (nPath === normNeedle || nPath.endsWith(needleBase) || normNeedle.endsWith(nPath)))
        );
      });

      if (matchingNode) {
        const port = mediaPort ?? 0;
        const streamUrl = matchingNode.stream_url?.startsWith('/cloud_stream/') && port > 0
          ? `http://127.0.0.1:${port}${matchingNode.stream_url}`
          : (matchingNode.download_url || '');
        return {
          name: matchingNode.name,
          path: streamUrl || matchingNode.stream_url || matchingNode.download_url || `cloud:${res.provider}:${matchingNode.id}`,
          size: matchingNode.size,
          server: '',
          is_cloud: true,
          is_cloud_folder: false,
          cloud_provider: res.provider,
          cloud_node_id: matchingNode.id,
          cloud_folder_title: res.title,
          cloud_folder_result: res,
        } as any;
      }
    }

    // 3. Search across completed downloaded files in downloadState (for 100% offline access)
    const downloadedMatch = downloadState.downloads.find((d) => {
      if (d.service !== service || d.creator_id !== creatorId || d.post_id !== postId) return false;
      const dName = (d.filename || '').toLowerCase();
      const dPath = (d.final_path || d.media_id || d.url || '').toLowerCase().split('?')[0].split('#')[0].replace(/\\/g, '/');
      const dBase = dPath.split('/').pop() || dName;
      return (
        d.media_id === raw ||
        d.media_id === normNeedle ||
        dName === normNeedle ||
        dName === needleBase ||
        dName.includes(needleBase) ||
        needleBase.includes(dName) ||
        normNeedle.includes(dName) ||
        (dPath && (dPath === normNeedle || dPath.endsWith(needleBase) || normNeedle.endsWith(dPath)))
      );
    });

    if (downloadedMatch && downloadedMatch.status === 'completed' && downloadedMatch.final_path) {
      const port = mediaPort ?? 0;
      const encoded = downloadedMatch.final_path.replace(/\\/g, '/').split('/').map((part) => encodeURIComponent(part)).join('/');
      const streamUrl = port > 0 ? `http://127.0.0.1:${port}/media/${encoded}` : downloadedMatch.url;
      return {
        name: downloadedMatch.filename,
        path: streamUrl,
        size: downloadedMatch.total_bytes || downloadedMatch.downloaded_bytes,
        server: '',
        is_cloud: true,
        is_cloud_folder: false,
        cloud_provider: 'download'
      } as any;
    }

    return undefined;
  }

  $effect(() => {
    if (postId !== lastHandledPostId) {
      lastHandledPostId = postId;
      initialViewerHandled = false;
    }
  });

  $effect(() => {
    const _ver = cloudResolvedVersion;
    const isPostLoading = entry.loading;
    const isCloudResolving = cloudResolving;
    const mediaCount = media.length;

    if (!initialViewerHandled && (openViewer || initialMedia)) {
      if (isPostLoading && mediaCount === 0) {
        return;
      }

      let targetFile: Attachment | undefined;
      if (initialMedia) {
        targetFile = findTargetAttachment(initialMedia);
      }

      // If looking for a specific media item and cloud link resolution is still ongoing, wait
      if (initialMedia && !targetFile && (isPostLoading || isCloudResolving)) {
        return;
      }

      // Fallback: if specific item was not found but openViewer was requested, open first media item
      if (!targetFile && openViewer && mediaCount > 0) {
        targetFile = (post?.attachments && post.attachments.length > 0 ? post.attachments[0] : null) || post?.file || media[0];
      }

      if (targetFile) {
        initialViewerHandled = true;
        openMediaViewer(targetFile, media);
      }
    }
  });

  $effect(() => {
    if (viewerIndex !== null && media.length > 0) {
      const currentSelected = viewerFiles[viewerIndex];
      const allSource = embedAttachment && !media.some((s) => s.path === embedAttachment!.path)
        ? [embedAttachment, ...media]
        : media;
      const updatedSourceItems = allSource
        .filter((item): item is Attachment => Boolean(item?.path || (item as any)?.html))
        .filter((item, itemIndex, list) => list.findIndex((candidate) => candidate.path === item.path) === itemIndex);

      if (post?.file?.path && !updatedSourceItems.some((s) => s.path === post?.file?.path)) {
        updatedSourceItems.push(post.file);
      }

      if (updatedSourceItems.length !== viewerFiles.length) {
        viewerFiles = updatedSourceItems;
        if (currentSelected) {
          const idx = updatedSourceItems.findIndex((item) => (item.path && item.path === currentSelected.path) || (currentSelected.name && item.name === currentSelected.name));
          if (idx >= 0) {
            viewerIndex = idx;
          }
        }
      }
    }
  });

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
  let stashes = $derived(libraryState.allStashes);
  let stashOptions = $derived(stashes.map((s) => ({ value: s.id, label: libraryState.getStashDisplayName(s) })));
  let postStashes = $derived(post ? libraryState.getPostStashes(post) : []);
  let customStashes = $derived(post ? libraryState.getCustomPostStashes(post) : []);
  let customStashNames = $derived(
    customStashes
      .map((id) => libraryState.collections.find((c) => c.id === id))
      .filter((c): c is LibraryCollection => Boolean(c))
      .map((c) => libraryState.getStashDisplayName(c))
  );
  let libraryButtonLabel = $derived.by(() => {
    if (!saved && postStashes.length === 0) {
      return i18n.t('library.save') || 'Save to library';
    }
    if (customStashNames.length === 1) {
      return customStashNames[0];
    }
    if (customStashNames.length > 1) {
      return i18n.t('library.in_stashes_count', { count: customStashes.length }) || `${customStashes.length} stashes`;
    }
    return i18n.t('library.saved') || 'Saved';
  });
  let authenticated = $derived(accountState.session.authenticated);

  let isFavorited = $state(false);
  let favoritingPending = $state(false);
  let mediaPort = $state<number | null>(null);
  let creatorAvatar = $state('');
  let creatorAvatarFailed = $state(false);
  let deletingDownloadId = $state<string | null>(null);
  let downloadingAll = $state(false);
  let totalMediaBytes = $derived.by(() => {
    if (media.length === 0) return 0;

    let total = 0;
    for (const file of media) {
      const size = getEffectiveFileSize(file);
      if (size <= 0) return 0;
      total += size;
    }
    return total;
  });

  let creatorName = $state('');

  let deferredAttachments = $derived.by(() => {
    if (!post?.attachments) return [];
    return post.attachments.filter((att: Attachment) => (att as any).deferred === true || (!att.path && att.name));
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

    const videoCount = deferredAttachments.filter((f: Attachment) => isVideoUrl(f.name || '') || /\.(mp4|mkv|webm|mov|avi|flv|wmv|m4v)(?:$|[?#])/i.test(f.name || '')).length;
    const photoCount = deferredAttachments.filter((f: Attachment) => isImageUrl(f.name || '') || /\.(png|jpe?g|gif|webp|bmp|avif)(?:$|[?#])/i.test(f.name || '')).length;
    const otherCount = deferredAttachments.length - videoCount - photoCount;

    const parts: string[] = [];
    if (videoCount > 0) parts.push(`${videoCount} ${i18n.t('post.video_count', { count: videoCount }) || (videoCount === 1 ? 'video' : 'videos')}`);
    if (photoCount > 0) parts.push(`${photoCount} ${i18n.t('post.photo_count', { count: photoCount }) || (photoCount === 1 ? 'image' : 'images')}`);
    if (otherCount > 0) parts.push(`${otherCount} ${i18n.t('post.file_count', { count: otherCount }) || (otherCount === 1 ? 'file' : 'files')}`);
    const details = parts.join(', ') || `${deferredAttachments.length} ${i18n.t('post.file_count', { count: deferredAttachments.length }) || 'files'}`;

    return i18n.t('post.files_exceed_limit_warning', { details }) ||
      `Some files exceed the archive size limit and weren't saved: ${details}. Please note these limits are in place to keep this site running long term, without costing a fortune. You can favorite the creator though, certain milestones increase the limit.`;
  });

  let lastLoadedPostKey = '';
  $effect(() => {
    const currentService = service;
    const currentCreatorId = creatorId;
    const currentPostId = postId;
    const currentKey = `${currentService}:${currentCreatorId}:${currentPostId}`;
    if (currentService && currentCreatorId && currentPostId) {
      if (lastLoadedPostKey !== currentKey) {
        lastLoadedPostKey = currentKey;
        probingMediaPaths.clear();
        probeQueue = [];
      }
      untrack(() => {
        void contentState.loadPost(currentService, currentCreatorId, currentPostId).then(() => {
          void checkFavoriteStatus();
        });
        void providerState.loadPostRevisions(currentService, currentCreatorId, currentPostId);
      });
    }
  });

  $effect(() => {
    const currentService = service;
    const currentCreatorId = creatorId;
    const prevId = effectivePrevId;
    const nextId = effectiveNextId;
    if (currentService && currentCreatorId) {
      if (prevId) {
        untrack(() => {
          void contentState.loadPost(currentService, currentCreatorId, prevId);
        });
      }
      if (nextId) {
        untrack(() => {
          void contentState.loadPost(currentService, currentCreatorId, nextId);
        });
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
        notify.success(i18n.t(targetState ? 'favorites.saved_locally' : 'favorites.removed_locally'));
      } else {
        notify.success(i18n.t(targetState ? 'post.added_to_favorites' : 'post.removed_from_favorites'));
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
      notify.error(i18n.t('post.favorite_failed'), error);
    } finally {
      favoritingPending = false;
    }
  }

  function remoteFileUrl(file: Attachment) {
    return attachmentMediaUrl(file, service);
  }

  function fileUrl(file: { path?: string; server?: string; name?: string; cloud_node_id?: string }) {
    // 1. Check if the file is already downloaded to local disk
    const localJob = attachmentDownload(file);
    const localPath = (localJob && localJob.status === 'completed' && localJob.final_path)
      ? localJob.final_path
      : '';

    if (localPath) {
      if (mediaPort) {
        const encoded = localPath.replace(/\\/g, '/').split('/').map((part) => encodeURIComponent(part)).join('/');
        return `http://127.0.0.1:${mediaPort}/media/${encoded}`;
      }
      return convertFileSrc(localPath);
    }

    // 2. Cloud streaming via local Axum proxy
    if (file.path?.startsWith('/cloud_stream/') && mediaPort) {
      return `http://127.0.0.1:${mediaPort}${file.path}`;
    }

    // 3. Absolute http / https URL
    if (file.path?.startsWith('http://') || file.path?.startsWith('https://')) {
      return file.path;
    }

    // 4. Remote Kemono / Coomer / Fanbox attachment URL
    return remoteFileUrl(file as Attachment);
  }

  let probeQueue: string[] = [];
  let activeProbes = 0;
  const MAX_CONCURRENT_PROBES = 3;

  function processProbeQueue() {
    while (activeProbes < MAX_CONCURRENT_PROBES && probeQueue.length > 0) {
      const path = probeQueue.shift()!;
      activeProbes++;
      const file = media.find((m) => m.path === path) || (post?.file?.path === path ? post.file : undefined);
      const url = path.startsWith('http://') || path.startsWith('https://')
        ? path
        : file ? remoteFileUrl(file) : '';

      if (!url || !url.startsWith('http')) {
        activeProbes--;
        probingMediaPaths.delete(path);
        continue;
      }

      apiProbeDownloadSize(url)
        .then((size) => {
          if (size && size > 0) {
            globalProbedSizes.set(path, size);
            globalProbedSizes.set(url, size);
            probedMediaSizes = { ...probedMediaSizes, [path]: size };
          } else {
            probingMediaPaths.delete(path);
          }
        })
        .catch(() => {
          probingMediaPaths.delete(path);
        })
        .finally(() => {
          activeProbes--;
          processProbeQueue();
        });
    }
  }

  function probeAttachmentSize(file?: Attachment) {
    const path = file?.path;
    if (!path) return;
    const currentSize = getEffectiveFileSize(file);
    if (currentSize > 0 || probedMediaSizes[path] || globalProbedSizes.has(path) || probingMediaPaths.has(path)) {
      if (!probedMediaSizes[path] && globalProbedSizes.has(path)) {
        probedMediaSizes = { ...probedMediaSizes, [path]: globalProbedSizes.get(path)! };
      }
      return;
    }

    probingMediaPaths.add(path);
    probeQueue.push(path);
    processProbeQueue();
  }

  $effect(() => {
    const currentMedia = media;
    const currentFile = post?.file;
    if (!post) return;
    untrack(() => {
      if (currentFile) probeAttachmentSize(currentFile);
      for (const file of currentMedia) {
        probeAttachmentSize(file);
      }
    });
  });


  async function deleteDownload(item: DownloadItem) {
    if (deletingDownloadId) return;
    deletingDownloadId = item.id;
    const filename = item.filename;
    try {
      await downloadState.remove(item.id);
      notify.success(i18n.t('post.download_deleted'), filename);
    } catch (error) {
      notify.error(i18n.t('post.download_delete_failed'), error);
    } finally {
      deletingDownloadId = null;
    }
  }

  async function download(file: { path?: string; server?: string; name?: string }, index: number) {
    if (!file.path) return;
    try {
      if (!post) return;
      const targetName = file.name || `${postId}_${index + 1}`;
      await downloadState.start(post, file.path, fileUrl(file), targetName);
      notify.success(i18n.t('feed.download_started'), targetName);
    } catch (error) {
      notify.error(i18n.t('feed.download_failed'), error);
    }
  }

  async function openFileInFolder(item: DownloadItem) {
    try {
      await apiShowInFolder(item.final_path);
    } catch (error) {
      notify.error(i18n.t('downloads.show_in_folder_failed') || 'Failed to reveal file', error);
    }
  }

  async function openPostFolder() {
    const completed = media.map(attachmentDownload).find((d) => d && d.status === 'completed');
    if (completed?.final_path) {
      const p = completed.final_path;
      const lastSlash = Math.max(p.lastIndexOf('/'), p.lastIndexOf('\\'));
      const folder = lastSlash > 0 ? p.slice(0, lastSlash) : p;
      try {
        await apiShowInFolder(folder);
      } catch (error) {
        notify.error(i18n.t('downloads.open_folder_failed') || 'Failed to open folder', error);
      }
    }
  }

  let allMediaDownloaded = $derived(
    media.length > 0 && media.every((file) => {
      const job = attachmentDownload(file);
      return job?.status === 'completed';
    })
  );

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

      if (started > 0) notify.success(i18n.t('post.download_all_started', { count: started }), post.title || undefined);
      if (failed > 0) notify.error(i18n.t('post.download_all_failed', { count: failed }));
    } finally {
      downloadingAll = false;
    }
  }

  async function toggleLibrary() {
    if (!post) return;
    const wasSaved = saved;
    try {
      await libraryState.toggle(post);
      notify.success(i18n.t(wasSaved ? 'library.removed' : 'library.saved'), post.title || undefined);
    } catch (error) {
      notify.error(i18n.t('library.save_error'), error);
    }
  }

  async function handleStashToggle(collectionId: string) {
    if (!post || !collectionId) return;
    const isCurrentlyIn = postStashes.includes(collectionId);
    const collection = libraryState.collections.find((c) => c.id === collectionId);
    try {
      if (collection?.kind === 'inbox') {
        if (isCurrentlyIn) {
          await libraryState.remove(post);
          notify.success(i18n.t('library.removed'), post.title || undefined);
        } else {
          await libraryState.save(post, collectionId);
          notify.success(i18n.t('library.saved'), post.title || undefined);
        }
      } else {
        if (isCurrentlyIn) {
          await libraryState.removeFromStash(collectionId, post);
          notify.success(i18n.t('library.removed_from_stash') || 'Removed from stash', post.title || undefined);
        } else {
          await libraryState.save(post, collectionId);
          notify.success(i18n.t('library.added_to_stash') || 'Added to stash', post.title || undefined);
        }
      }
    } catch (error) {
      notify.error(i18n.t('library.save_error') || 'Stash operation failed', error);
    }
  }

  async function handleCreateStash(name: string) {
    if (!post || !name.trim()) return;
    try {
      const newStash = await libraryState.createStash(name.trim());
      await libraryState.save(post, newStash.id);
      notify.success(i18n.t('library.added_to_stash') || 'Added to stash', newStash.name);
    } catch (error) {
      notify.error(i18n.t('library.save_error') || 'Failed to create stash', error);
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
  {@const declaredBytes = getEffectiveFileSize(file)}
  {@const declaredSize = declaredBytes > 0 ? formatBytes(declaredBytes) : ''}
  {@const hasProgress = Boolean(active && !verifying && knownTotal > 0)}
  {@const progress = job && knownTotal > 0 ? Math.min(100, Math.round(job.downloaded_bytes / knownTotal * 100)) : 0}
  <div class="media-download-group">
    {#if downloaded}
      <Button
        variant="ghost"
        class="media-download-btn is-downloaded"
        onclick={() => openMediaViewer(file, filteredMedia)}
        title={i18n.t('post.viewer_open')}
      >
        <span class="attachment-button-state downloaded-state">
          <IconCheck class="w-[16px] h-[16px]" />
          <span>{i18n.t('post.downloaded')} · {formatBytes(Math.max(downloaded.total_bytes, downloaded.downloaded_bytes))}</span>
        </span>
      </Button>
      <Button
        variant="ghost"
        class="media-action-icon-btn"
        onclick={() => void openFileInFolder(downloaded)}
        tooltip={i18n.t('downloads.show_in_folder')}
        aria-label={i18n.t('downloads.show_in_folder')}
      >
        <IconFolder class="w-[18px] h-[18px]" />
      </Button>
      <Button
        variant="ghost"
        disabled={deletingDownloadId === downloaded.id}
        class="media-action-icon-btn is-danger"
        onclick={() => void deleteDownload(downloaded)}
        tooltip={i18n.t('post.delete_download')}
        aria-label={i18n.t('post.delete_download')}
      >
        {#if deletingDownloadId === downloaded.id}
          <IconLoading class="w-[18px] h-[18px]" />
        {:else}
          <IconDelete class="w-[18px] h-[18px]" />
        {/if}
      </Button>
    {:else}
      <Button
        variant="ghost"
        disabled={Boolean(active)}
        onclick={() => void download(file, index)}
        class={`media-download-btn${active ? ' is-downloading' : ''}`}
        title={i18n.t(verifying ? 'downloads.status_verifying' : active ? 'post.downloading' : 'post.download')}
      >
        {#if active}
          {#if hasProgress}<span class="attachment-progress-fill" style:width={`${progress}%`}></span>{/if}
          <span class="attachment-button-state downloading-state">
            {#if hasProgress}<IconDownload />{:else}<IconLoading />{/if}
            <span>{verifying ? i18n.t('downloads.status_verifying') : i18n.t('post.downloading')}{hasProgress ? ` · ${progress}%` : ''}</span>
          </span>
        {:else}
          <span class="attachment-button-state"><IconDownload /><span>{i18n.t('post.download')}{declaredSize ? ` · ${declaredSize}` : ''}</span></span>
        {/if}
      </Button>
    {/if}
  </div>
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

          <div class="sticky-stash-select">
            <Select
              options={stashOptions}
              selectedValues={postStashes}
              placeholder={libraryButtonLabel}
              onchange={handleStashToggle}
              createLabel={i18n.t('library.new_stash')}
              onCreate={handleCreateStash}
              variant={saved || postStashes.length > 0 ? 'accent' : 'ghost'}
              multi={true}
              closeOnChange={false}
              iconOnly={layoutState.isMobile}
              icon={saved || postStashes.length > 0 ? IconSaved : IconSave}
              disabled={saving}
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

        <div class="stash-select-container">
          <Select
            options={stashOptions}
            selectedValues={postStashes}
            placeholder={libraryButtonLabel}
            onchange={handleStashToggle}
            createLabel={i18n.t('library.new_stash')}
            onCreate={handleCreateStash}
            variant={saved || postStashes.length > 0 ? 'accent' : 'ghost'}
            multi={true}
            closeOnChange={false}
            icon={saved || postStashes.length > 0 ? IconSaved : IconSave}
            disabled={saving}
            class="stash-select"
          />
        </div>
      {/if}
    </div>

    {#if post}
      <header class="detail-header">
        <div class="min-w-0 flex-1">
          <h1>{cleanPostTitle(post.title) || i18n.t('feed.untitled')}</h1>
          <div class="post-date post-meta-row flex items-center flex-wrap gap-2 mt-2 min-h-[38px] text-sm text-[var(--fg-muted)]">
            <div class="flex items-center gap-1.5 shrink-0">
              <span class="text-[var(--fg-subtle)]">{i18n.t('post.published_at')}:</span>
              <strong class="font-semibold text-[var(--fg-default)]">{publishedDateStr}</strong>
            </div>

            {#if showEdited}
              <span class="text-[var(--fg-subtle)]">·</span>
              <div class="flex items-center gap-1.5 shrink-0">
                <span class="text-[var(--fg-subtle)]">{i18n.t('post.edited_at') || 'Edited'}:</span>
                <strong class="font-medium text-[var(--fg-default)]">{editedDateStr}</strong>
              </div>
            {/if}

            {#if showImported}
              <span class="text-[var(--fg-subtle)]">·</span>
              <div class="flex items-center gap-1.5 shrink-0">
                <span class="text-[var(--fg-subtle)]">{i18n.t('post.imported_at')}:</span>
                <strong class="font-medium text-[var(--fg-default)]">{addedDateStr}</strong>
              </div>
            {/if}

            {#if candidateProviders.length > 0}
              <span class="text-[var(--fg-subtle)]">·</span>
              <div class="inline-flex items-center shrink-0">
                <Select
                  variant="ghost"
                  disabled={candidateProviders.length === 1}
                  options={providerSelectOptions}
                  value={activeProviderId}
                  onchange={(val) => providerState.setSelectedProvider(service, creatorId, postId, val)}
                />
              </div>
            {/if}

            {#if postRevisions.length > 0}
              <span class="text-[var(--fg-subtle)]">·</span>
              <div class="inline-flex items-center shrink-0">
                <Select
                  variant="ghost"
                  options={revisionSelectOptions}
                  value={selectedRevId !== null ? String(selectedRevId) : 'latest'}
                  onchange={onRevisionChange}
                />
              </div>
            {/if}
          </div>

          {#if postTags.length > 0}
            <div class="post-tags-row mt-2">
              <TagList
                tags={postTags}
                maxVisible={16}
                onclick={(_tag) => {
                  navigationState.openCreator(service, creatorId);
                }}
              />
            </div>
          {/if}
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
                  <CountBadge count={mediaCounts.all} showZero={true} />
                </Button>
                {#if mediaCounts.video > 0}
                  <Button variant={activeMediaTab === 'video' ? 'accent' : 'ghost'} onclick={() => activeMediaTab = 'video'}>
                    <IconVideo class="w-[16px] h-[16px]" />
                    <span>{i18n.t('post.tab_video')}</span>
                    <CountBadge count={mediaCounts.video} />
                  </Button>
                {/if}
                {#if mediaCounts.photo > 0}
                  <Button variant={activeMediaTab === 'photo' ? 'accent' : 'ghost'} onclick={() => activeMediaTab = 'photo'}>
                    <IconImage class="w-[16px] h-[16px]" />
                    <span>{i18n.t('post.tab_photo')}</span>
                    <CountBadge count={mediaCounts.photo} />
                  </Button>
                {/if}
                {#if mediaCounts.file > 0}
                  <Button variant={activeMediaTab === 'file' ? 'accent' : 'ghost'} onclick={() => activeMediaTab = 'file'}>
                    <IconDocument class="w-[16px] h-[16px]" />
                    <span>{i18n.t('post.tab_file')}</span>
                    <CountBadge count={mediaCounts.file} />
                  </Button>
                {/if}
                {#if mediaCounts.cloud > 0}
                  <Button variant={activeMediaTab === 'cloud' ? 'accent' : 'ghost'} onclick={() => activeMediaTab = 'cloud'}>
                    <IconCloud class="w-[16px] h-[16px]" />
                    <span>{i18n.t('post.tab_cloud') || 'Cloud Files'}</span>
                    <CountBadge count={mediaCounts.cloud} />
                  </Button>
                {/if}
              </nav>
            {/if}

            <div class="media-controls-actions">
              {#if media.length >= 20 || mediaSearchQuery}
                <div class="media-search-wrapper">
                  <SearchBar
                    bind:value={mediaSearchQuery}
                    placeholder={i18n.t('post.search_media') || 'Search media...'}
                    expandable={true}
                  />
                </div>
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
          </div>

          {#if filteredMedia.length > 0 || (hasEmbed && (activeMediaTab === 'all' || activeMediaTab === 'video') && embedMatchesSearch)}
            <div class="media-gallery-wrapper">
              <div class="media-gallery-container" class:is-collapsed={isGalleryOverflowing && !galleryExpanded}>
                <section class="media-gallery" bind:clientHeight={galleryHeight} aria-label={i18n.t('post.media')}>
                {#if postEmbed && !isEmbedResolvedToCloud && (activeMediaTab === 'all' || activeMediaTab === 'video') && embedMatchesSearch}
                  <div class="media-item is-embed-item">
                    <div class="media-header">
                      <span class="media-filename">{postEmbed.subject || postEmbed.description || post.title}</span>
                      <span class="cloud-source-badge">{postEmbed.provider || postEmbed.provider_url || 'Embed'}</span>
                    </div>

                    {#if postEmbed.html}
                      <div class="media-embed-player">
                        {@html postEmbed.html}
                      </div>
                    {:else if postEmbed.url && isVideoUrl(postEmbed.url)}
                      <!-- svelte-ignore a11y_media_has_caption -->
                      <video
                        src={postEmbed.url}
                        controls
                        playsinline
                        preload="metadata"
                        use:panicCapture
                        onkeydown={handleGlobalPanicKey}
                      ></video>
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
                        onclick={() => embedAttachment && openMediaViewer(embedAttachment, filteredMedia)}
                        use:tooltip={i18n.t('post.viewer_open')}
                        aria-label={i18n.t('post.viewer_open')}
                      ><IconEye /></button>
                    {/if}

                    {#if postEmbed.url}
                      <div class="media-download-group">
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
                      </div>
                    {/if}
                  </div>
                {/if}

                {#each filteredMedia as file, index}
                  {@const isCloudFolder = (file as any)?.is_cloud_folder === true}
                  {@const isDeferred = file?.deferred === true || (!file?.path && Boolean(file?.name))}
                  {@const url = file?.path ? fileUrl(file) : ''}
                  {@const isCloud = (file as any)?.is_cloud === true}
                  <div class="media-item" class:is-deferred={isDeferred} class:is-folder-item={isCloudFolder}>
                    {#if isCloudFolder}
                      {@const fRes = (file as any).cloud_folder_result}
                      {@const fNodeId = (file as any).cloud_node_id}
                      {@const fChildCount = (file as any).cloud_child_count || 0}
                      <div class="media-header">
                        <span class="media-filename">{file?.name || 'Folder'}</span>
                        <span class="cloud-source-badge">{(file as any).cloud_provider || 'Cloud'}</span>
                        {#if fChildCount > 0}
                          <span class="media-filesize">({fChildCount} items)</span>
                        {/if}
                      </div>
                      <button
                        type="button"
                        class="file-placeholder media-open-surface cursor-pointer hover:bg-[var(--bg-card-hover)] transition-colors"
                        onclick={() => openCloudFolderModal(fRes, fNodeId)}
                        aria-label="Open folder {file?.name}"
                      >
                        <IconFolder class="placeholder-icon text-[var(--accent)]" />
                        <p class="placeholder-text font-medium">{file?.name}</p>
                      </button>
                      <div class="media-download-group">
                        <Button
                          variant="ghost"
                          class="media-download-btn"
                          onclick={() => openCloudFolderModal(fRes, fNodeId)}
                          title={i18n.t('post.browse_folder') || 'Browse Folder'}
                        >
                          <span class="attachment-button-state">
                            <IconFolder class="w-[16px] h-[16px]" />
                            <span>{i18n.t('post.browse_folder') || 'Browse Folder'}</span>
                          </span>
                        </Button>
                        <Button
                          variant="ghost"
                          class="media-action-icon-btn"
                          onclick={() => downloadCloudSubfolder(fRes, fNodeId)}
                          tooltip={i18n.t('post.download') || 'Download'}
                          aria-label={i18n.t('post.download') || 'Download'}
                        >
                          <IconDownload class="w-[18px] h-[18px]" />
                        </Button>
                      </div>
                    {:else if isDeferred}
                      <div class="media-header">
                        <span class="media-filename">{file?.name || i18n.t('post.file')}</span>
                        {#if isCloud}
                          <span class="cloud-source-badge">{(file as any).cloud_provider || 'Cloud'}</span>
                        {/if}
                      </div>
                      <div class="file-placeholder media-open-surface is-deferred-placeholder" title={i18n.t('post.file_not_saved_desc')}>
                        <IconWarning class="placeholder-icon text-red-500" />
                        <p class="placeholder-text text-red-400">{i18n.t('post.file_not_saved')}</p>
                      </div>
                    {:else if isAttachmentVideo(file, url) || isAttachmentImage(file, url)}
                      {@const isVid = isAttachmentVideo(file, url)}
                      {@const effSize = getEffectiveFileSize(file)}
                      <div class="media-header">
                        <span class="media-filename">{file?.name || i18n.t('post.file')}</span>
                        {#if isCloud}
                          <span class="cloud-source-badge">{(file as any).cloud_provider || 'Cloud'}</span>
                        {/if}
                        {#if effSize > 0}
                          <span class="media-filesize">({formatBytes(effSize)})</span>
                        {/if}
                      </div>
                      {#if isVid && (videoFailures[index] || (isH265Video(file?.name) && !hevcSupported))}
                        <div class="video-placeholder">
                          <IconVideoOff class="placeholder-icon" />
                          <p class="placeholder-text">{isH265Video(file?.name) && !hevcSupported ? i18n.t('post.unsupported_codec_desc') : (i18n.t('post.video_load_failed') || 'Failed to play video')}</p>
                        </div>
                      {:else}
                        {#if isVid}
                          <!-- svelte-ignore a11y_media_has_caption -->
                          <video
                            src={url}
                            poster={attachmentThumbnailUrl(file, service)}
                            controls
                            playsinline
                            preload={index === 0 ? 'metadata' : 'none'}
                            use:panicCapture
                            onkeydown={handleGlobalPanicKey}
                            onerror={(e) => handleVideoError(e, file, index)}
                            onplay={handleVideoPlay}
                          ></video>
                          <button
                            class="media-viewer-open-btn"
                            type="button"
                            onclick={(e) => openMediaViewer(file!, filteredMedia, e.currentTarget.parentElement?.querySelector('video'))}
                            use:tooltip={i18n.t('post.viewer_open')}
                            aria-label={i18n.t('post.viewer_open')}
                          ><IconEye /></button>
                        {:else}
                          <button
                            class="media-open-surface"
                            type="button"
                            onclick={() => openMediaViewer(file!, filteredMedia)}
                            aria-label={`${i18n.t('post.viewer_open')}: ${file?.name || post.title}`}
                          >
                            <img
                              src={url}
                              alt={file?.name || post.title}
                              loading={index < 2 ? 'eager' : 'lazy'}
                              decoding="async"
                              onerror={(e) => {
                                const target = e.currentTarget as HTMLImageElement;
                                const fallback = attachmentThumbnailUrl(file!, service);
                                if (fallback && target.src !== fallback) {
                                  target.src = fallback;
                                }
                              }}
                            />
                          </button>
                        {/if}
                      {/if}
                      {@render mediaDownloadAction(file!, index)}
                    {:else}
                      {@const ext = getFileExtension(file?.name).toUpperCase()}
                      {@const effSize = getEffectiveFileSize(file)}
                      {@const sizeStr = effSize > 0 ? formatBytes(effSize) : ''}
                      <div class="media-header">
                        <span class="media-filename">{file?.name || i18n.t('post.file')}</span>
                        {#if isCloud}
                          <span class="cloud-source-badge">{(file as any).cloud_provider || 'Cloud'}</span>
                        {/if}
                        {#if effSize > 0}
                          <span class="media-filesize">({formatBytes(effSize)})</span>
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
          </div>
          {:else if mediaSearchQuery.trim()}
            <div class="media-search-empty">
              <IconSearch class="media-search-empty-icon" />
              <p class="media-search-empty-title">{i18n.t('post.no_media_found') || 'No media found'}</p>
              <p class="media-search-empty-desc">{i18n.t('post.no_media_found_desc') || 'No attachments match your search query.'}</p>
            </div>
          {/if}
        </div>
      {/if}

      {#if richContent && !isHtmlContentEmpty(richContent)}
        <section class="post-content">
          <div class="html-content-container" class:is-collapsed={isOverflowing && !contentExpanded}>
            <div class="html-content" bind:clientHeight={contentHeight}>
              <RichContent html={richContent} currentService={service} currentCreatorId={creatorId} onopencloud={handleOpenCloudFromText} />
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
          {#if allMediaDownloaded}
            <Button
              variant="ghost"
              onclick={() => void openPostFolder()}
              class="post-footer-action"
            >
              <IconFolder class="w-[18px] h-[18px]" />
              <span>{i18n.t('downloads.open_post_folder')}</span>
            </Button>
          {:else if media.length > 0}
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
              const url = postPageUrl(service, creatorId, postId);
              void apiOpenInBrowser(url).catch((err) => console.warn('Failed to open post URL in browser:', err));
            }}
            class="post-footer-action"
            title={postPageUrl(service, creatorId, postId)}
          >
            <IconOpen class="w-[18px] h-[18px]" />
            <span>{i18n.t('post.open_in_browser')}</span>
          </Button>
        </div>

        <div class="post-footer-toolbar">
          <div class="footer-nav-left">
            <Button
              variant="ghost"
              disabled={!effectivePrevId}
              onclick={() => effectivePrevId && navigationState.openPost(service, creatorId, effectivePrevId)}
              class="footer-nav-btn"
              title={i18n.t('post.next') || 'Next'}
            >
              <IconChevronLeft class="w-[18px] h-[18px]" />
              <span>{leftPostTitle}</span>
            </Button>
          </div>

          <div class="footer-nav-right">
            <Button
              variant="ghost"
              disabled={!effectiveNextId}
              onclick={() => effectiveNextId && navigationState.openPost(service, creatorId, effectiveNextId)}
              class="footer-nav-btn"
              title={i18n.t('post.previous') || 'Previous'}
            >
              <span>{rightPostTitle}</span>
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
            <CountBadge count={comments.length} variant="header" />
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

<CloudFolderModal
  folder={activeCloudModalFolder}
  initialFolderId={activeCloudModalInitialFolderId}
  post={post}
  open={isCloudModalOpen}
  onclose={() => isCloudModalOpen = false}
/>

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

  .stash-select-container {
    margin-left: auto;
    min-width: 170px;
    max-width: 260px;
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

  .media-sort-selector {
    width: 200px;
    flex-shrink: 0;
  }

  .media-controls-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: auto;
  }

  .media-search-wrapper {
    display: flex;
    align-items: center;
  }

  .media-search-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 56px 20px;
    text-align: center;
    gap: 6px;
    color: var(--text-secondary);
  }

  .media-search-empty :global(.media-search-empty-icon) {
    width: 44px;
    height: 44px;
    opacity: 0.35;
    margin-bottom: 4px;
  }

  .media-search-empty-title {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .media-search-empty-desc {
    font-size: 13px;
    color: var(--text-muted);
    margin: 0;
  }

  .media-gallery {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 340px), 1fr));
    gap: 20px;
    max-width: 1000px;
    margin: 0 auto;
    padding: 16px 0;
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
    border: var(--border-width) solid var(--border-color, rgba(255, 255, 255, 0.08));
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

  .cloud-source-badge {
    font-size: 10px;
    font-weight: 600;
    padding: 2px 7px;
    border-radius: var(--radius-full, 9999px);
    background: rgba(255, 255, 255, 0.08);
    color: var(--fg-default);
    letter-spacing: 0.04em;
    flex-shrink: 0;
    text-transform: uppercase;
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

  .media-download-group {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    margin-top: 12px;
    align-self: center;
    max-width: 100%;
  }

  :global(.media-download-btn) {
    position: relative;
    width: 220px;
    max-width: 100%;
    overflow: hidden;
    isolation: isolate;
  }

  :global(.media-download-btn.is-downloaded) {
    width: auto;
    min-width: 180px;
  }

  :global(.media-download-btn.is-downloading:disabled) {
    opacity: 1;
  }

  :global(.media-action-icon-btn) {
    width: 44px !important;
    height: 44px !important;
    padding: 0 !important;
    border-radius: var(--radius-full) !important;
    display: grid !important;
    place-items: center !important;
    flex-shrink: 0;
    transition: color var(--duration-fast), background var(--duration-fast);
  }

  :global(.media-action-icon-btn.is-danger:hover) {
    color: var(--danger, #ff626d) !important;
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

  .downloaded-state {
    color: var(--accent-primary);
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

  .media-gallery-wrapper {
    position: relative;
    width: 100%;
  }

  .media-gallery-container {
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
    flex: 0 0 46px !important;
    width: 46px !important;
    height: 46px !important;
    min-width: 46px !important;
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
    display: flex;
    align-items: center;
  }

  :global(.sticky-header-bar.is-mobile) .sticky-stash-select {
    display: flex;
    width: 46px;
    height: 46px;
    flex-shrink: 0;
  }

  :global(.sticky-header-bar.is-mobile) .sticky-stash-select :global(.select-trigger) {
    width: 46px !important;
    height: 46px !important;
    min-width: 46px !important;
    max-width: 46px !important;
    border-radius: 50% !important;
    padding: 0 !important;
    display: flex !important;
    align-items: center !important;
    justify-content: center !important;
    gap: 0 !important;
  }

  :global(.sticky-header-bar.is-mobile) .sticky-stash-select :global(.select-trigger svg) {
    width: 20px !important;
    height: 20px !important;
    flex-shrink: 0 !important;
  }

  :global(.sticky-header-bar.is-mobile) .sticky-post-actions :global(.btn) {
    width: 46px !important;
    height: 46px !important;
    min-width: 46px !important;
    flex: 0 0 46px !important;
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
    min-width: 170px;
    max-width: 240px;
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
    min-height: 46px;
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
