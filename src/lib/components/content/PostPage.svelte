<script module lang="ts">
  const globalCloudFolderCache = new Map<string, import('$lib/types/cloud').CloudFolderResult>();
  const globalPostCloudNodes = new Map<string, import('$lib/types/content').Attachment[]>();
  const globalProbedSizes = new Map<string, number>();
</script>

<script lang="ts">
  import { onMount, onDestroy, untrack, getContext } from 'svelte';
  import { SCROLLABLE_CONTEXT, type ScrollableContext } from '$lib/actions/scrollable';
  import { contentState, postCacheKey, creatorCacheKey, normalizePostId, type CachedPost } from '$lib/state/contentState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { configState } from '$lib/state/configState.svelte';
  import { libraryState } from '$lib/state/libraryState.svelte';
  import { downloadState } from '$lib/state/downloadState.svelte';
  import { accountState } from '$lib/state/accountState.svelte';
  import { themeState, getContrastColor } from '$lib/theme/themeState.svelte';
  import { creatorsState } from '$lib/state/creatorsState.svelte';
  import { apiFetchAccountFavorites, apiSetPostFavorite, apiFetchCreatorProfile, apiGetCreatorArtworkPath, apiOpenInBrowser, apiFetchPostComments, apiProbeDownloadSize, apiProbeDownloadSizes, apiShowInFolder, apiStartDownload, apiOpenDownloadFile, apiResolvePostUrl } from '$lib/utils/ipc';
  import type { Attachment, Comment, Post } from '$lib/types/content';
  import type { DownloadItem } from '$lib/types/download';
  import type { LibraryCollection } from '$lib/types/library';
  import { i18n } from '$lib/i18n';
  import { toast } from 'svelte-sonner';
  import { formatDate, formatBytes, parseTags, getPostTags, cleanPostTitle, parseDateTimestamp } from '$lib/utils/formatters';
  import { isImageUrl, isVideoUrl, attachmentMediaUrl, attachmentThumbnailUrl, isAttachmentVideo, isAttachmentAudio, isAttachmentImage, getPlatformPostUrl, formatProviderName, postThumbnailUrl, creatorAvatarSrc, resolveLocalMediaUrl, getFileExtension, getUnsupportedContainerFormat, isH265Video, diagnoseVideoFailure, diagnoseVideoFailureAsync, cleanMediaPath, isPostUnarchived, getAttachmentDeclaredSize, type MediaFailureState } from '$lib/utils/media';
  import { thumbHashToAverageColor } from '$lib/utils/thumbhash';
  import { serverPortState } from '$lib/state/serverPort.svelte';
  import { extractDirectMediaLinks } from './RichContent.svelte';
  import { apiResolveCloudLink } from '$lib/utils/ipc';
  import { logger, logMediaError } from '$lib/utils/logger';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { getVideoThumbnail } from '$lib/utils/videoThumbnail';
  import { handleGlobalPanicKey, panicCapture } from '$lib/utils/panic';
  import PageShell from '$lib/components/layout/PageShell.svelte';
  import StickyHeader from '$lib/components/layout/StickyHeader.svelte';
  import HeroBackdrop from '$lib/components/ui/HeroBackdrop.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { tooltip, ripple } from '$lib/motion';
  import Button from '$lib/components/ui/Button.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import HeaderActions from '$lib/components/layout/HeaderActions.svelte';
  import TagList from '$lib/components/ui/TagList.svelte';
  import CountBadge from '$lib/components/ui/CountBadge.svelte';
  import ChoiceGroup, { type ChoiceOption } from '$lib/components/ui/ChoiceGroup.svelte';
  import ServiceIcon from './ServiceIcon.svelte';
  import RichContent from './RichContent.svelte';

  const scrollContext = getContext<ScrollableContext | undefined>(SCROLLABLE_CONTEXT);
  import PostPoll from './PostPoll.svelte';
  import MediaViewer, { type MediaViewerItem, type MediaViewerKind } from './MediaViewer.svelte';
  import IconFullscreen from '~icons/fluent/full-screen-maximize-24-regular';
  import IconCloud from '~icons/fluent/cloud-24-regular';
  import IconSearch from '~icons/fluent/search-24-regular';
  import IconArrowLeft from '~icons/fluent/arrow-left-24-regular';
  import IconDownload from '~icons/fluent/arrow-download-24-regular';
  import IconArrowDownload from '~icons/fluent/arrow-download-24-regular';
  import IconCheck from '~icons/fluent/checkmark-20-regular';
  import IconDelete from '~icons/fluent/delete-24-regular';
  import IconPause from '~icons/fluent/pause-24-regular';
  import IconPlay from '~icons/fluent/play-24-regular';
  import IconPlayFilled from '~icons/fluent/play-24-filled';
  import IconMusicFilled from '~icons/fluent/music-note-2-24-filled';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';
  import IconArrowClockwise from '~icons/fluent/arrow-clockwise-24-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';
  import IconChevronLeft from '~icons/fluent/chevron-left-24-regular';
  import IconChevronRight from '~icons/fluent/chevron-right-24-regular';
  import IconSave from '~icons/fluent/bookmark-add-24-regular';
  import IconSaved from '~icons/fluent/bookmark-24-filled';
  import IconHeart from '~icons/fluent/heart-24-regular';
  import IconHeartFilled from '~icons/fluent/heart-24-filled';
  import IconDocument from '~icons/fluent/document-24-regular';
  import IconMusic from '~icons/fluent/music-note-2-24-regular';
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
  import BottomSheet from '$lib/components/ui/BottomSheet.svelte';
  import IconMoreVertical from '~icons/fluent/more-vertical-24-regular';
  import IconCopy from '~icons/fluent/copy-24-regular';
  import IconArrowSort from '~icons/fluent/arrow-sort-24-regular';
  import CloudFolderModal from '$lib/components/content/CloudFolderModal.svelte';
  import CodecGuideModal from '$lib/components/content/CodecGuideModal.svelte';
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
    linked_object_id?: number | string;
    linked_object_type?: string;
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

  const emptyEntry: CachedPost = { post: null, loading: false, loaded: false, error: null };
  let autoEntry = $derived(contentState.posts[postCacheKey(service, creatorId, postId)]);
  let entry = $derived.by(() => {
    const byActive = contentState.posts[postCacheKey(service, creatorId, postId, activeProviderId)];
    if (activeProviderId && activeProviderId !== 'auto') {
      return byActive ?? emptyEntry;
    }
    if (byActive?.post) return byActive;
    if (autoEntry?.post) return autoEntry;
    return byActive ?? autoEntry ?? emptyEntry;
  });
  let rawPost = $derived(entry.post);
  let availableProviders = $derived.by<string[]>(() => {
    const raw = (rawPost as any)?.extra?.available_providers;
    if (Array.isArray(raw)) return raw as string[];
    return [];
  });
  let providersWithPost = $derived.by(() => {
    if (availableProviders.length > 0) {
      return candidateProviders.filter((p) => availableProviders.includes(p.id));
    }
    return candidateProviders;
  });

  let filteredRevisions = $derived.by(() => {
    if (!activeProviderId || activeProviderId === 'auto') {
      return postRevisions;
    }
    return postRevisions.filter((rev) => {
      const revPost = (rev as any).post || rev;
      const revProvId = (rev as any).provider_id || revPost?.extra?.provider_id || (rev as any)?.extra?.provider_id;
      if (!revProvId) {
        return candidateProviders.length <= 1;
      }
      return revProvId.toLowerCase() === activeProviderId.toLowerCase();
    });
  });

  let post = $derived.by(() => {
    if (selectedRevId !== null) {
      const found = filteredRevisions.find((r) => r.revision_id === selectedRevId);
      if (found) return ((found as any).post || found) as Post;
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
    const currentId = normalizePostId(postId);
    return creatorPosts.findIndex((p) => normalizePostId(p.id) === currentId);
  });

  const currentNormId = $derived(normalizePostId(postId));

  let candidateNewerId = $derived.by(() => {
    const fromPost = normalizePostId(post?.prev);
    if (fromPost && fromPost !== currentNormId) return fromPost;
    if (currentPostIndexInCreator > 0) {
      const cand = normalizePostId(creatorPosts[currentPostIndexInCreator - 1]?.id);
      if (cand && cand !== currentNormId) return cand;
    }
    return '';
  });

  let candidateOlderId = $derived.by(() => {
    const fromPost = normalizePostId(post?.next);
    if (fromPost && fromPost !== currentNormId) return fromPost;
    if (currentPostIndexInCreator >= 0 && currentPostIndexInCreator < creatorPosts.length - 1) {
      const cand = normalizePostId(creatorPosts[currentPostIndexInCreator + 1]?.id);
      if (cand && cand !== currentNormId) return cand;
    }
    return '';
  });

  let effectiveNewerId = $derived.by(() => {
    if (candidateNewerId && candidateOlderId && candidateNewerId === candidateOlderId) {
      if (currentPostIndexInCreator === 0) {
        return '';
      }
      if (currentPostIndexInCreator === creatorPosts.length - 1) {
        return candidateNewerId;
      }
      return '';
    }
    return candidateNewerId;
  });

  let effectiveOlderId = $derived.by(() => {
    if (candidateNewerId && candidateOlderId && candidateNewerId === candidateOlderId) {
      if (currentPostIndexInCreator === 0) {
        return candidateOlderId;
      }
      if (currentPostIndexInCreator === creatorPosts.length - 1) {
        return '';
      }
      return candidateOlderId;
    }
    return candidateOlderId;
  });

  let newerPost = $derived.by(() => {
    if (!effectiveNewerId) return null;
    const key = postCacheKey(service, creatorId, effectiveNewerId);
    const cached = contentState.posts[key]?.post;
    return cached ?? creatorPosts.find((p) => normalizePostId(p.id) === effectiveNewerId) ?? null;
  });

  let olderPost = $derived.by(() => {
    if (!effectiveOlderId) return null;
    const key = postCacheKey(service, creatorId, effectiveOlderId);
    const cached = contentState.posts[key]?.post;
    return cached ?? creatorPosts.find((p) => normalizePostId(p.id) === effectiveOlderId) ?? null;
  });

  function extractAdjacentTitle(targetPost: Post | null, fallbackLabel: string): string {
    if (!targetPost) return fallbackLabel;
    const clean = cleanPostTitle(targetPost.title);
    if (clean) return clean;
    const rawText = targetPost.content || targetPost.substring || '';
    if (rawText) {
      const stripped = cleanPostTitle(rawText);
      if (stripped) {
        return stripped.length > 50 ? `${stripped.slice(0, 50)}...` : stripped;
      }
    }
    if (targetPost.published) {
      return formatDate(targetPost.published);
    }
    return fallbackLabel;
  }

  let leftPostTitle = $derived(
    extractAdjacentTitle(olderPost, i18n.t('post.previous') || 'Previous')
  );
  let rightPostTitle = $derived(
    extractAdjacentTitle(newerPost, i18n.t('post.next') || 'Next')
  );
  
  let richContent = $derived(post?.content || post?.substring || '');
  let postTags = $derived(getPostTags(post));

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
      ...filteredRevisions.map((rev, idx) => {
        const revPost = (rev as any).post || rev;
        const revDate = revPost.edited || revPost.added || revPost.published;
        const revProvId = (rev as any).provider_id || revPost?.extra?.provider_id || (rev as any)?.extra?.provider_id;
        const showProviderTag = (!activeProviderId || activeProviderId === 'auto') && revProvId;
        const providerName = showProviderTag ? ` • ${formatProviderName(revProvId)}` : '';
        return {
          value: String(rev.revision_id),
          label: `v${rev.revision_id || filteredRevisions.length - idx}${providerName} (${formatDate(revDate)})`
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
      const found = filteredRevisions.find((r) => r.revision_id === revId);
      if (found) {
        const revPost = (found as any).post || found;
        const provId = (found as any)?.provider_id || revPost?.extra?.provider_id || (found as any)?.extra?.provider_id;
        if (provId && candidateProviders.some((p) => p.id === provId)) {
          providerState.setSelectedProvider(service, creatorId, postId, provId);
        }
      }
    }
  }

  function onProviderChange(val: string) {
    providerState.setSelectedProvider(service, creatorId, postId, val);
    if (selectedRevId !== null) {
      if (val !== 'auto') {
        const found = postRevisions.find((r) => r.revision_id === selectedRevId);
        const revPost = (found as any)?.post || found;
        const revProvId = (found as any)?.provider_id || revPost?.extra?.provider_id || (found as any)?.extra?.provider_id;
        if (!found || (revProvId && revProvId.toLowerCase() !== val.toLowerCase())) {
          providerState.setSelectedRevision(service, creatorId, postId, null);
        }
      }
    }
  }

  $effect(() => {
    if (selectedRevId !== null) {
      if (filteredRevisions.length === 0) {
        providerState.setSelectedRevision(service, creatorId, postId, null);
      } else {
        const exists = filteredRevisions.some((r) => r.revision_id === selectedRevId);
        if (!exists) {
          providerState.setSelectedRevision(service, creatorId, postId, null);
        }
      }
    }
  });

  let mobileMoreOpen = $state(false);

  let currentProviderName = $derived.by(() => {
    if (activeProviderId && activeProviderId !== 'auto') {
      const p = providerState.getProviderById(activeProviderId);
      if (p) return formatProviderName(p.name);
    }
    const defaultProv = providerState.getProviderForService(service);
    if (defaultProv) return formatProviderName(defaultProv.name || defaultProv.id);
    return 'Provider';
  });

  let creatorProfile = $derived(contentState.creators[creatorCacheKey(service, creatorId)]?.profile);

  function openInProvider(targetProvId?: string) {
    const effectiveProvId = targetProvId || (activeProviderId && activeProviderId !== 'auto' ? activeProviderId : undefined);
    if (post?.page_url && (!effectiveProvId || effectiveProvId === post.provider_id)) {
      void apiOpenInBrowser(post.page_url).catch((err) => logger.warn('Failed to open post URL in provider', err));
      return;
    }
    apiResolvePostUrl(service, creatorId, postId, effectiveProvId)
      .then((url) => {
        if (url) void apiOpenInBrowser(url).catch((err) => logger.warn('Failed to open post URL in provider', err));
      })
      .catch((err) => logger.warn('Failed to resolve post URL', err));
  }

  function openOriginalPost() {
    const url = getPlatformPostUrl(service, creatorId, postId, creatorProfile?.public_id);
    if (url) void apiOpenInBrowser(url).catch((err) => logger.warn('Failed to open original post URL in browser', err));
  }

  function openInBrowser() {
    openInProvider();
  }

  let copiedPostLink = $state(false);
  let copyLinkTimeout: ReturnType<typeof setTimeout> | null = null;
  function copyPostLink(targetProvId?: string) {
    const effectiveProvId = targetProvId || (activeProviderId && activeProviderId !== 'auto' ? activeProviderId : undefined);
    const doCopy = (url: string) => {
      void navigator.clipboard.writeText(url);
      copiedPostLink = true;
      if (copyLinkTimeout) clearTimeout(copyLinkTimeout);
      copyLinkTimeout = setTimeout(() => {
        copiedPostLink = false;
      }, 2000);
    };

    if (post?.page_url && (!effectiveProvId || effectiveProvId === post.provider_id)) {
      doCopy(post.page_url);
      return;
    }

    apiResolvePostUrl(service, creatorId, postId, effectiveProvId)
      .then((url) => {
        if (url) doCopy(url);
      })
      .catch((err) => logger.warn('Failed to resolve post URL for copy', err));
  }

  let copiedOriginalLink = $state(false);
  let copyOriginalLinkTimeout: ReturnType<typeof setTimeout> | null = null;
  function copyOriginalLink() {
    const url = getPlatformPostUrl(service, creatorId, postId, creatorProfile?.public_id);
    if (!url) return;
    void navigator.clipboard.writeText(url);
    copiedOriginalLink = true;
    if (copyOriginalLinkTimeout) clearTimeout(copyOriginalLinkTimeout);
    copyOriginalLinkTimeout = setTimeout(() => {
      copiedOriginalLink = false;
    }, 2000);
  }

  let copiedPostId = $state(false);
  let copyIdTimeout: ReturnType<typeof setTimeout> | null = null;
  function copyPostId() {
    if (!postId) return;
    void navigator.clipboard.writeText(postId);
    copiedPostId = true;
    if (copyIdTimeout) clearTimeout(copyIdTimeout);
    copyIdTimeout = setTimeout(() => {
      copiedPostId = false;
    }, 2000);
  }

  onDestroy(() => {
    if (copyLinkTimeout) clearTimeout(copyLinkTimeout);
    if (copyOriginalLinkTimeout) clearTimeout(copyOriginalLinkTimeout);
    if (copyIdTimeout) clearTimeout(copyIdTimeout);
  });

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
      const port = serverPortState.port || 0;
      const raw = node.download_url || node.stream_url || '';
      const streamUrl = raw.startsWith('/cloud_stream/') && port > 0
        ? serverPortState.mediaUrl(raw)
        : raw;
      if (!streamUrl) continue;
      try {
        await apiStartDownload(targetPost, node.id, streamUrl, node.name);
        started++;
      } catch (err) {
        logger.error(`Failed to queue download for "${node.name}"`, err);
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
        logger.warn(`Failed to resolve cloud link from text: ${url}`, err);
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
    untrack(() => {
      const cachedNodes = globalPostCloudNodes.get(currentPostKey);
      if (cachedNodes && cachedNodes.length > 0) {
        resolvedCloudAttachments = cachedNodes;
        cloudResolving = false;
        return;
      }

      const cloudUrls = post?.cloud_urls || [];
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

            const fileNodes = res.nodes.filter((n) => !n.is_folder);
            const nodesToDisplay = fileNodes.length > 0 ? fileNodes : res.nodes;

            for (const node of nodesToDisplay) {
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
            logger.warn(`Failed to auto-resolve cloud link for post gallery: ${url}`, err);
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

  function isFileUnarchived(file: Attachment | null | undefined, isDownloaded?: boolean): boolean {
    if (!file) return false;
    if (isDownloaded) return false;
    if ((file as any)?.is_cloud === true) return false;
    const isDeferred = (file as any)?.deferred === true;
    const hasNoPath = !file.path || file.path.trim() === '' || file.path === 'null';
    if (isDeferred || hasNoPath) return true;
    if (post?.has_full === false) return true;
    return false;
  }

  function isSameAttachment(a: Attachment | null | undefined, b: Attachment | null | undefined): boolean {
    if (!a || !b) return false;
    if (a === b) return true;

    const aNode = (a as any).cloud_node_id;
    const bNode = (b as any).cloud_node_id;
    if (aNode && bNode) return aNode === bNode;
    if (aNode && b.path && (b.path === aNode || b.path.endsWith(aNode))) return true;
    if (bNode && a.path && (a.path === bNode || a.path.endsWith(bNode))) return true;

    if (a.path && b.path && a.path === b.path) return true;

    const aIsCloud = (a as any).is_cloud === true;
    const bIsCloud = (b as any).is_cloud === true;
    if (aIsCloud === bIsCloud && a.name && b.name && a.name.trim().toLowerCase() === b.name.trim().toLowerCase()) {
      return true;
    }

    return false;
  }

  let media = $derived.by(() => {
    const items: Attachment[] = [];
    if (post) {
      if (post.file && (post.file.path || post.file.name)) {
        items.push(post.file);
      }
      if (post.attachments && Array.isArray(post.attachments)) {
        for (const att of post.attachments) {
          if (!att || (!att.path && !att.name)) continue;
          const exists = items.some((existing) => isSameAttachment(existing, att));
          if (!exists) {
            items.push(att);
          }
        }
      }
      if (resolvedCloudAttachments.length > 0) {
        for (const att of resolvedCloudAttachments) {
          const stubIndex = items.findIndex(
            (existing) =>
              isFileUnarchived(existing) &&
              existing.name &&
              att.name &&
              existing.name.trim().toLowerCase() === att.name.trim().toLowerCase()
          );
          if (stubIndex >= 0) {
            items[stubIndex] = att;
          } else {
            const exists = items.some((existing) => isSameAttachment(existing, att));
            if (!exists) {
              items.push(att);
            }
          }
        }
      }
      const directMedia = extractDirectMediaLinks(post.content || post.substring || '');
      for (const d of directMedia) {
        const att: Attachment = {
          name: d.name,
          path: d.url,
          size: undefined,
          server: '',
          is_cloud: false,
          is_cloud_folder: false
        } as any;
        const exists = items.some((existing) => isSameAttachment(existing, att));
        if (!exists) {
          items.push(att);
        }
      }
    }

    const postDownloads = downloadState.downloads.filter((d) =>
      d.service === service &&
      d.creator_id === creatorId &&
      d.post_id === postId &&
      d.status === 'completed' &&
      d.final_path
    );
    for (const d of postDownloads) {
      const port = serverPortState.port || 0;
      const encoded = d.final_path!.replace(/\\/g, '/').split('/').map((part) => encodeURIComponent(part)).join('/');
      const streamUrl = port > 0 ? serverPortState.mediaUrl(`/media/${encoded}`) : convertFileSrc(d.final_path!);
      const isCloudNode = Boolean(d.media_id && (d.media_id.includes('cloud_') || d.media_id.startsWith('mega:') || d.media_id.startsWith('http')));
      const downloadAtt = {
        name: d.filename,
        path: streamUrl,
        size: d.total_bytes || d.downloaded_bytes,
        server: '',
        is_cloud: isCloudNode,
        is_cloud_folder: false,
        cloud_provider: isCloudNode ? 'download' : undefined,
        cloud_node_id: d.media_id
      } as Attachment;

      const existingIdx = items.findIndex((existing) => isSameAttachment(existing, downloadAtt));
      if (existingIdx >= 0) {
        items[existingIdx] = {
          ...items[existingIdx],
          path: streamUrl,
          cloud_node_id: d.media_id
        } as any;
      } else {
        items.push(downloadAtt);
      }
    }

    return items;
  });

  let activeMediaTab = $state<'all' | 'video' | 'photo' | 'file' | 'cloud' | 'downloaded'>('all');
  let mediaSort = $state<'default' | 'name_asc' | 'name_desc' | 'size_desc' | 'size_asc'>('default');
  let mediaSearchQuery = $state('');
  let mediaSearchOpen = $state(false);

  function closeMediaSearch() {
    mediaSearchOpen = false;
    mediaSearchQuery = '';
  }
  let viewerIndex = $state<number | null>(null);
  let viewerFiles = $state<Attachment[]>([]);
  let contentExpanded = $state(false);
  let contentHeight = $state(0);
  const MAX_CONTENT_HEIGHT = 480;
  let isOverflowing = $derived(contentHeight > MAX_CONTENT_HEIGHT);

  let hevcSupported = $state(true);
  let videoFailures = $state<Record<number, MediaFailureState>>({});
  let isCodecModalOpen = $state(false);

  function handleVideoLoadedMetadata(e: Event, file?: Attachment | null, index?: number) {
    if (typeof index === 'number' && file) {
      const video = e.currentTarget as HTMLVideoElement;
      // Chromium reports zero dimensions when it can play audio but not the video codec.
      if (video.videoWidth === 0 && video.videoHeight === 0 && video.duration > 0) {
        logger.warn(`Video "${file.name}" has audio but unsupported video codec (videoWidth=0)`);
        videoFailures[index] = { preset: 'unsupported_codec', format: 'H.265 / HEVC' };
      }
    }
  }

  async function handleVideoError(e: Event, file?: Attachment | null, index?: number) {
    if (typeof index === 'number' && file) {
      const video = e.currentTarget as HTMLVideoElement;
      logMediaError('video', video.src, file.name, video.error);

      const job = attachmentDownload(file);
      const isDownloaded = Boolean(job?.final_path && job.status === 'completed');

      if (!isDownloaded) {
        const remote = remoteFileUrl(file);
        if (remote && video.src !== remote && !video.src.includes('cloud_stream')) {
          logger.warn(`Local video playback failed, falling back to remote stream for "${file.name}": ${remote}`);
          video.src = remote;
          return;
        }
      }

      const syncDiag = diagnoseVideoFailure(file, video, {
        isLocal: isDownloaded,
        isUnarchived: isFileUnarchived(file, isDownloaded)
      });
      videoFailures = { ...videoFailures, [index]: syncDiag };

      if (!isDownloaded && (syncDiag.preset === 'unavailable' || syncDiag.preset === 'network')) {
        const asyncDiag = await diagnoseVideoFailureAsync(file, video, {
          isLocal: isDownloaded,
          isUnarchived: isFileUnarchived(file, isDownloaded)
        });
        if (asyncDiag && asyncDiag.preset !== syncDiag.preset) {
          videoFailures = { ...videoFailures, [index]: asyncDiag };
        }
      }
    }
  }

  function openFileExternally(item?: DownloadItem | null) {
    if (!item?.final_path) return;
    void apiOpenDownloadFile(item.final_path);
  }

  function isHtmlContentEmpty(html?: string) {
    if (!html) return true;
    if (/<(iframe|img|video|audio|embed|object|picture|svg)\b/i.test(html)) return false;
    const text = html.replace(/<[^>]*>/g, '').replace(/&nbsp;/g, '').trim();
    return text.length === 0;
  }

  let galleryExpanded = $state(false);
  let galleryHeight = $state(0);
  const MAX_GALLERY_HEIGHT = 960;
  let isGalleryOverflowing = $derived(galleryHeight > MAX_GALLERY_HEIGHT);

  let galleryWrapperEl = $state<HTMLElement | null>(null);
  let contentWrapperEl = $state<HTMLElement | null>(null);
  let commentsWrapperEl = $state<HTMLElement | null>(null);

  let galleryScrollBeforeExpand = $state<number | null>(null);
  let contentScrollBeforeExpand = $state<number | null>(null);
  let commentsScrollBeforeExpand = $state<number | null>(null);

  function smartCollapseSection(
    setCollapsed: () => void,
    el: HTMLElement | null,
    collapsedMaxHeight: number,
    scrollBeforeExpand: number | null
  ) {
    const vp = scrollContext?.viewport;
    if (!vp || !el) {
      setCollapsed();
      return;
    }

    const rect = el.getBoundingClientRect();
    const vpRect = vp.getBoundingClientRect();
    const isAtEnd = rect.bottom <= vpRect.bottom + 80;

    if (isAtEnd) {
      const currentHeight = rect.height;
      const collapsedHeight = Math.min(currentHeight, collapsedMaxHeight);
      const heightDiff = currentHeight - collapsedHeight;

      setCollapsed();
      if (heightDiff > 0) {
        vp.scrollTop = Math.max(0, vp.scrollTop - heightDiff);
      }
    } else {
      setCollapsed();
      if (typeof scrollBeforeExpand === 'number') {
        vp.scrollTo({ top: scrollBeforeExpand, behavior: 'smooth' });
      } else {
        el.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
      }
    }
  }

  $effect(() => {
    if (activeMediaTab) {
      galleryExpanded = false;
    }
  });

  let isEmbedResolvedToCloud = $derived(Boolean(
    postEmbed?.url && resolvedCloudAttachments.length > 0
  ));

  let isEmbedLinkedPost = $derived(Boolean(
    postEmbed && (
      postEmbed.linked_object_type === 'post' ||
      Boolean(postEmbed.linked_object_id) ||
      (postEmbed.url && /patreon\.com\/.*\/posts\/[a-zA-Z0-9_-]*?(\d+)/i.test(postEmbed.url))
    )
  ));

  let linkedPostId = $derived.by(() => {
    if (!postEmbed) return null;
    if (postEmbed.linked_object_id) return String(postEmbed.linked_object_id);
    if (postEmbed.url) {
      const match = postEmbed.url.match(/posts\/[a-zA-Z0-9_-]*?(\d+)/i);
      if (match) return match[1];
    }
    return null;
  });

  let isEmbedVideo = $derived(Boolean(
    postEmbed && !isEmbedLinkedPost && (
      Boolean(postEmbed.html && /<iframe|<video/i.test(postEmbed.html)) ||
      Boolean(postEmbed.url && (isVideoUrl(postEmbed.url) || /youtube|youtu\.be|vimeo|redgifs|streamable|sproutvideo|bilibili|vids\.io/i.test(postEmbed.url)))
    )
  ));

  let hasEmbed = $derived(Boolean(!isEmbedResolvedToCloud && postEmbed && (postEmbed.url || postEmbed.subject || postEmbed.html)));

  let isEmbedVisibleInTab = $derived.by(() => {
    if (!hasEmbed) return false;
    if (activeMediaTab === 'all') return true;
    if (activeMediaTab === 'video') return isEmbedVideo;
    if (activeMediaTab === 'file') return isEmbedLinkedPost || !isEmbedVideo;
    return false;
  });

  let embedMatchesSearch = $derived.by(() => {
    const q = mediaSearchQuery.trim().toLowerCase();
    if (!q) return true;
    const title = (postEmbed?.subject || postEmbed?.description || post?.title || '').toLowerCase();
    return title.includes(q);
  });

  function isFileDownloaded(file?: Attachment | null): boolean {
    if (!file) return false;
    const job = attachmentDownload(file);
    if (job?.final_path && job.status === 'completed') return true;
    const p = file.path || '';
    return (p.startsWith('http://127.0.0.1') && p.includes('/media/')) ||
      p.startsWith('asset://') ||
      p.startsWith('http://asset.localhost') ||
      p.startsWith('https://asset.localhost');
  }

  let mediaCounts = $derived.by(() => {
    let videos = 0;
    let photos = 0;
    let files = 0;
    let cloud = 0;
    let downloaded = 0;
    for (const file of media) {
      if ((file as any).is_cloud) {
        cloud++;
      }
      if (isFileDownloaded(file)) {
        downloaded++;
      }
      const url = file.path ? fileUrl(file) : (file.name || '');
      if (isAttachmentVideo(file, url)) videos++;
      else if (isAttachmentImage(file, url)) photos++;
      else files++;
    }
    if (hasEmbed) {
      if (isEmbedVideo) {
        videos++;
      } else {
        files++;
      }
    }
    return {
      all: media.length + (hasEmbed ? 1 : 0),
      video: videos,
      photo: photos,
      file: files,
      cloud,
      downloaded
    };
  });

  let activeCategoriesCount = $derived.by(() => {
    let count = 0;
    if (mediaCounts.video > 0) count++;
    if (mediaCounts.photo > 0) count++;
    if (mediaCounts.file > 0) count++;
    if (mediaCounts.cloud > 0) count++;
    if (mediaCounts.downloaded > 0) count++;
    return count;
  });

  type MediaTab = 'all' | 'video' | 'photo' | 'file' | 'cloud' | 'downloaded';

  let mediaTabOptions = $derived.by(() => {
    const opts: ChoiceOption<MediaTab>[] = [
      {
        value: 'all',
        label: i18n.t('post.tab_all'),
        icon: IconGrid,
        count: mediaCounts.all,
        showZero: true
      }
    ];

    if (mediaCounts.video > 0) {
      opts.push({
        value: 'video',
        label: i18n.t('post.tab_video'),
        icon: IconVideo,
        count: mediaCounts.video
      });
    }
    if (mediaCounts.photo > 0) {
      opts.push({
        value: 'photo',
        label: i18n.t('post.tab_photo'),
        icon: IconImage,
        count: mediaCounts.photo
      });
    }
    if (mediaCounts.file > 0) {
      opts.push({
        value: 'file',
        label: i18n.t('post.tab_file'),
        icon: IconDocument,
        count: mediaCounts.file
      });
    }
    if (mediaCounts.cloud > 0) {
      opts.push({
        value: 'cloud',
        label: i18n.t('post.tab_cloud') || 'Cloud Files',
        icon: IconCloud,
        count: mediaCounts.cloud
      });
    }
    if (mediaCounts.downloaded > 0) {
      opts.push({
        value: 'downloaded',
        label: i18n.t('post.tab_downloaded') || 'Downloaded',
        icon: IconArrowDownload,
        count: mediaCounts.downloaded
      });
    }

    return opts;
  });

  $effect(() => {
    if (!mediaTabOptions.some((opt) => opt.value === activeMediaTab)) {
      activeMediaTab = 'all';
    }
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
    const declared = getAttachmentDeclaredSize(file);
    if (declared > 0) return declared;
    if (file.path) {
      if (probedMediaSizes[file.path] && probedMediaSizes[file.path] > 0) return probedMediaSizes[file.path];
      const clean = cleanMediaPath(file.path);
      if (clean && probedMediaSizes[clean] && probedMediaSizes[clean] > 0) return probedMediaSizes[clean];
      const cached = globalProbedSizes.get(file.path) || (clean ? globalProbedSizes.get(clean) : undefined);
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
    } else if (activeMediaTab === 'downloaded') {
      list = list.filter((file) => isFileDownloaded(file));
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

  let embedAttachment = $derived(postEmbed && !isEmbedLinkedPost ? ({
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
    const isUnarchived = isFileUnarchived(file, Boolean(job?.final_path && job.status === 'completed'));
    const mediaIndex = media.findIndex((m) => isSameAttachment(m, file));
    const cachedVideoThumb = mediaIndex >= 0 ? videoThumbnails[mediaIndex] : undefined;
    const key = job?.media_id || job?.id || job?.filename || file.path || file.name || `vid_${itemIndex}`;
    return {
      id: key,
      url,
      poster: cachedVideoThumb || attachmentThumbnailUrl(file, service),
      name: file.name || i18n.t('post.file'),
      kind: isEmbed ? 'video' : mediaViewerKind(file, url),
      size: getEffectiveFileSize(file) || file.size,
      width,
      height,
      html: (file as any)?.html,
      downloadStatus: job?.status,
      downloadedBytes: job?.downloaded_bytes,
      totalBytes: job?.total_bytes,
      downloadedPath: job?.final_path,
      isUnarchived
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

  let activeGalleryItems = $derived.by((): Attachment[] => {
    const items: Attachment[] = [];
    if (hasEmbed && isEmbedVisibleInTab && embedMatchesSearch && embedAttachment) {
      items.push(embedAttachment);
    }
    items.push(...filteredMedia);
    return items;
  });

  function openMediaViewer(file: Attachment, source: Attachment[] = activeGalleryItems, originVideoEl?: HTMLVideoElement | null) {
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

    const sourceList = source && source.length > 0 ? source : [file];
    const sourceItems = sourceList
      .filter((item): item is Attachment => Boolean(item?.path || (item as any)?.html))
      .filter((item, itemIndex, list) => list.findIndex((candidate) => isSameAttachment(candidate, item)) === itemIndex);

    let nextIndex = sourceItems.findIndex((item) => isSameAttachment(item, file));
    if (nextIndex < 0) {
      sourceItems.unshift(file);
      nextIndex = 0;
    }

    viewerFiles = sourceItems;
    viewerIndex = nextIndex;
  }

  let initialViewerHandled = $state(false);
  let lastHandledViewerKey = $state('');

  function findTargetAttachment(needleStr: string): Attachment | undefined {
    if (!needleStr || !needleStr.trim()) return undefined;
    const raw = decodeURIComponent(needleStr).trim();
    const normNeedle = raw.toLowerCase().split('?')[0].split('#')[0].replace(/\\/g, '/');
    const needleBase = normNeedle.split('/').pop() || normNeedle;

    const probeAtt: Attachment = {
      name: needleBase,
      path: normNeedle,
      cloud_node_id: raw
    } as any;

    for (const f of media) {
      if (isSameAttachment(f, probeAtt)) {
        return f;
      }
    }

    const downloadedMatch = downloadState.downloads.find((d) => {
      if (d.service !== service || d.creator_id !== creatorId || d.post_id !== postId) return false;
      const dAtt: Attachment = {
        name: d.filename,
        path: d.final_path || d.url || d.media_id,
        cloud_node_id: d.media_id
      } as any;
      return isSameAttachment(dAtt, probeAtt);
    });

    if (downloadedMatch && downloadedMatch.status === 'completed' && downloadedMatch.final_path) {
      const port = serverPortState.port || 0;
      const encoded = downloadedMatch.final_path.replace(/\\/g, '/').split('/').map((part) => encodeURIComponent(part)).join('/');
      const streamUrl = port > 0 ? serverPortState.mediaUrl(`/media/${encoded}`) : convertFileSrc(downloadedMatch.final_path);
      return {
        name: downloadedMatch.filename,
        path: streamUrl,
        size: downloadedMatch.total_bytes || downloadedMatch.downloaded_bytes,
        server: '',
        is_cloud: true,
        is_cloud_folder: false,
        cloud_provider: 'download',
        cloud_node_id: downloadedMatch.media_id
      } as any;
    }

    for (const [_, res] of cloudFolderResults) {
      const matchingNode = res.nodes.find((n) => {
        if (n.is_folder) return false;
        const nAtt: Attachment = {
          name: n.name,
          path: n.stream_url || n.download_url || `cloud:${res.provider}:${n.id}`,
          cloud_node_id: n.id
        } as any;
        return isSameAttachment(nAtt, probeAtt);
      });

      if (matchingNode) {
        return {
          name: matchingNode.name,
          path: matchingNode.stream_url || matchingNode.download_url || `cloud:${res.provider}:${matchingNode.id}`,
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

    return undefined;
  }

  $effect(() => {
    const currentKey = `${postId}:${initialMedia || ''}:${openViewer ? '1' : '0'}`;
    if (currentKey !== lastHandledViewerKey) {
      lastHandledViewerKey = currentKey;
      initialViewerHandled = false;
    }
  });

  $effect(() => {
    const _ver = cloudResolvedVersion;
    const isPostFullyLoaded = entry.loaded && Boolean(post?.detail_fetched);
    const isPostLoading = entry.loading || (!isPostFullyLoaded && !entry.loaded && !entry.error);
    const isCloudResolving = cloudResolving;
    const currentMedia = media;

    if (!initialViewerHandled && (openViewer || initialMedia)) {
      let targetFile: Attachment | undefined;
      if (initialMedia) {
        targetFile = findTargetAttachment(initialMedia);
      }

      if (targetFile) {
        initialViewerHandled = true;
        openMediaViewer(targetFile, currentMedia);
        return;
      }

      if ((isPostLoading || isCloudResolving) && !entry.error) {
        return;
      }

      if (currentMedia.length > 0) {
        targetFile = post?.file || (post?.attachments && post.attachments.length > 0 ? post.attachments[0] : null) || currentMedia[0];
        if (targetFile) {
          initialViewerHandled = true;
          openMediaViewer(targetFile, currentMedia);
        }
      } else if (!isPostLoading && !isCloudResolving) {
        initialViewerHandled = true;
      }
    }
  });

  $effect(() => {
    if (viewerIndex !== null && activeGalleryItems.length > viewerFiles.length) {
      viewerFiles = activeGalleryItems;
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
    if (!post) return;
    const targetFile = (post.file && (isAttachmentImage(post.file, post.file.path) || isAttachmentVideo(post.file, post.file.path)))
      ? post.file
      : (post.attachments?.find((att) => isAttachmentImage(att, att.path) || isAttachmentVideo(att, att.path)) || post.file || media[0]);
    if (!targetFile) return;
    const source = [targetFile, ...media.filter((file) => file.path !== targetFile?.path)];
    openMediaViewer(targetFile, source);
  }

  async function downloadViewerItem(item: MediaViewerItem) {
    const fileIndex = viewerFiles.findIndex((file) => (file.path || file.name) === item.id || file.name === item.name);
    const file = viewerFiles[fileIndex];
    if (!file) return;
    const job = attachmentDownload(file);
    if (job && !['failed', 'cancelled', 'missing'].includes(job.status)) return;
    await download(file, Math.max(0, fileIndex));
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
  let creatorAvatar = $state('');
  let creatorAvatarFailed = $state(false);
  let deletingDownloadId = $state<string | null>(null);
  let downloadingAll = $state(false);
  let activeVideoIndexes = $state<Set<number>>(new Set());
  let activeAudioIndexes = $state<Set<number>>(new Set());
  let videoThumbnails = $state<Record<number, string>>({});

  $effect(() => {
    const currentMedia = media;
    const downloads = downloadState.downloads;
    const port = serverPortState.port;

    for (let i = 0; i < currentMedia.length; i++) {
      const file = currentMedia[i];
      if (!file) continue;
      const url = fileUrl(file);
      if (!isAttachmentVideo(file, url)) continue;
      if (videoThumbnails[i]) continue;

      const localJob = attachmentDownload(file);
      if (url) {
        const key = localJob?.media_id || localJob?.id || localJob?.filename || file.path || file.name || `vid_${i}`;
        getVideoThumbnail(key, url).then((thumb) => {
          if (thumb) {
            videoThumbnails = { ...videoThumbnails, [i]: thumb };
          }
        });
      }
    }
  });
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
    const currentProvider = activeProviderId;
    const currentKey = `${currentService}:${currentCreatorId}:${currentPostId}:${currentProvider || 'auto'}`;
    if (currentService && currentCreatorId && currentPostId) {
      if (lastLoadedPostKey !== currentKey) {
        lastLoadedPostKey = currentKey;
        probingMediaPaths.clear();
        closeMediaSearch();
      }
      untrack(() => {
        void contentState.loadPost(currentService, currentCreatorId, currentPostId, true, currentProvider).then(() => {
          void checkFavoriteStatus();
        });
        void providerState.loadPostRevisions(currentService, currentCreatorId, currentPostId);
      });
    }
  });

  $effect(() => {
    const currentService = service;
    const currentCreatorId = creatorId;
    const oldId = effectiveOlderId;
    const newId = effectiveNewerId;
    if (currentService && currentCreatorId) {
      if (oldId) {
        untrack(() => {
          void contentState.loadPost(currentService, currentCreatorId, oldId);
        });
      }
      if (newId) {
        untrack(() => {
          void contentState.loadPost(currentService, currentCreatorId, newId);
        });
      }
    }
  });

  $effect(() => {
    if (service && creatorId) {
      const key = creatorCacheKey(service, creatorId);
      if (!contentState.creators[key]?.loaded && !contentState.creators[key]?.loading) {
        void contentState.loadCreator(service, creatorId);
      }
    }
  });

  $effect(() => {
    if (service && creatorId) {
      const cacheKey = `${service.toLowerCase()}:${creatorId.toLowerCase()}`;
      const existingCreator = contentState.creators[cacheKey];
      if (existingCreator?.profile) {
        if (typeof existingCreator.profile.name === 'string' && existingCreator.profile.name) {
          creatorName = existingCreator.profile.name;
        }
        creatorAvatar = creatorAvatarSrc(existingCreator.profile);
        creatorAvatarFailed = false;
      } else {
        const cachedName = creatorsState.creatorsMap.get(cacheKey);
        if (typeof cachedName === 'string' && cachedName) {
          creatorName = cachedName;
        }
        void apiGetCreatorArtworkPath(service, creatorId, 'avatar')
          .then((path) => {
            if (path) {
              creatorAvatar = resolveLocalMediaUrl(path) || '';
              creatorAvatarFailed = false;
            }
          })
          .catch(() => {});

        void apiFetchCreatorProfile(service, creatorId)
          .then((profile) => {
            if (profile) {
              if (typeof profile.name === 'string' && profile.name) {
                creatorName = profile.name;
              }
              const cachedCreator = contentState.getCreator(service, creatorId);
              cachedCreator.profile = profile;
              creatorAvatar = creatorAvatarSrc(profile);
              creatorAvatarFailed = false;
            }
          })
          .catch((err) => {
            logger.warn(`Failed to load creator profile for ${service}:${creatorId}`, err);
          });
      }
    }
  });

  onMount(() => {
    void downloadState.init();
    void serverPortState.ensurePort();

    try {
      const v = document.createElement('video');
      const canPlay = v.canPlayType('video/mp4; codecs="hevc"') || v.canPlayType('video/mp4; codecs="hvc1"');
      hevcSupported = canPlay === 'probably' || canPlay === 'maybe';
    } catch (e) {
      hevcSupported = false;
    }
  });

  let heroImageUrl = $derived.by(() => {
    if (!post) return '';
    if (post.file && isAttachmentImage(post.file, post.file.path)) {
      return fileUrl(post.file);
    }
    const firstImg = post.attachments?.find((att) => isAttachmentImage(att, att.path));
    if (firstImg) {
      return fileUrl(firstImg);
    }
    const thumb = postThumbnailUrl(post);
    if (thumb) return thumb;
    if (post.file) {
      return attachmentThumbnailUrl(post.file, service);
    }
    return '';
  });

  function getAverageColor(url: string): Promise<string> {
    return new Promise((resolve) => {
      const img = new Image();
      let effectiveUrl = url;
      const port = serverPortState.port || 0;
      if (port > 0 && (url.startsWith('http://') || url.startsWith('https://')) && !url.includes('127.0.0.1')) {
        effectiveUrl = serverPortState.mediaUrl(`/cloud_stream/proxy?url=${encodeURIComponent(url)}`);
      }
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
          logger.warn('Canvas color extraction failed', e);
          resolve('');
        }
      };
      img.onerror = () => resolve('');
      img.src = effectiveUrl;
    });
  }

  $effect(() => {
    if (!post || !configState.settings.dynamic_accent) return;

    let cancelled = false;
    const cachedAccent = contentState.getPostAccent(service, creatorId, postId);
    const postThumbhash = (post as any)?.preview_thumbhash || (post.extra as any)?.preview_thumbhash || (post.file as any)?.extra?.thumbhash;
    const thumbColor = cachedAccent || thumbHashToAverageColor(postThumbhash);

    if (thumbColor) {
      themeState.setOverrideAccent(thumbColor);
    }

    if (!cachedAccent && !thumbColor && heroImageUrl) {
      void getAverageColor(heroImageUrl).then((color) => {
        if (!color || cancelled) return;
        contentState.setPostAccent(service, creatorId, postId, color);
        themeState.setOverrideAccent(color);
      });
    }

    return () => {
      cancelled = true;
      themeState.clearOverrideAccent();
    };
  });

  async function checkFavoriteStatus() {
    try {
      const favorites = await accountState.fetchFavorites('post');
      isFavorited = favorites.some((fav) => String(fav.id) === String(postId) && fav.service === service);
    } catch (error) {
      logger.error(`Failed to check post favorite status for ${service}:${postId}`, error);
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
      if (targetState) {
        accountState.addPostFavoriteOptimistic(post);
      } else {
        accountState.removePostFavoriteOptimistic(service, creatorId, postId);
      }
    } catch (error) {
      logger.error(`Failed to toggle post favorite for ${service}:${postId}`, error);
      notify.error(i18n.t('post.favorite_failed'), error);
    } finally {
      favoritingPending = false;
    }
  }

  function remoteFileUrl(file: Attachment) {
    return attachmentMediaUrl(file, service, post);
  }

  function fileUrl(file: Attachment) {
    const port = serverPortState.port || 0;

    const localJob = attachmentDownload(file);
    const localPath = (localJob && localJob.status === 'completed' && localJob.final_path)
      ? localJob.final_path
      : '';

    if (localPath) {
      if (port > 0) {
        const encoded = localPath.replace(/\\/g, '/').split('/').map((part) => encodeURIComponent(part)).join('/');
        return serverPortState.mediaUrl(`/media/${encoded}`);
      }
      return convertFileSrc(localPath);
    }

    const backendUrl = file.url || file.path || '';

    if (backendUrl.startsWith('/cloud_stream/')) {
      if (port > 0) {
        return serverPortState.mediaUrl(backendUrl);
      }
      return '';
    }

    if (backendUrl.startsWith('http://') || backendUrl.startsWith('https://')) {
      return backendUrl;
    }

    const remoteUrl = remoteFileUrl(file as Attachment);
    if (!remoteUrl) return '';

    return remoteUrl;
  }

  let isProbingBatch = false;

  async function probeMediaBatch(files: Attachment[]) {
    if (isProbingBatch || !files || files.length === 0) return;

    const probeTargets: { key: string; url: string; file: Attachment }[] = [];

    for (const file of files) {
      if (!file) continue;
      const path = file.path;
      const clean = path ? cleanMediaPath(path) : undefined;
      const currentSize = getEffectiveFileSize(file);
      const cached = (path ? globalProbedSizes.get(path) : undefined) || (clean ? globalProbedSizes.get(clean) : undefined);

      if (currentSize > 0 || (path && probedMediaSizes[path]) || (clean && probedMediaSizes[clean]) || cached) {
        if (cached && path && !probedMediaSizes[path]) {
          probedMediaSizes = { ...probedMediaSizes, [path]: cached, ...(clean ? { [clean]: cached } : {}) };
        }
        continue;
      }

      const url = path?.startsWith('http://') || path?.startsWith('https://')
        ? path
        : remoteFileUrl(file);

      if (url && url.startsWith('http')) {
        const key = path || url;
        if (!probingMediaPaths.has(key)) {
          probingMediaPaths.add(key);
          probeTargets.push({ key, url, file });
        }
      }
    }

    if (probeTargets.length === 0) return;

    isProbingBatch = true;
    try {
      const urls = probeTargets.map((t) => t.url);
      const sizes = await apiProbeDownloadSizes(urls);
      const updates: Record<string, number> = {};
      for (const target of probeTargets) {
        const size = sizes && typeof sizes === 'object' ? (sizes[target.url] || sizes[target.key]) : undefined;
        if (typeof size === 'number' && size > 0) {
          globalProbedSizes.set(target.key, size);
          globalProbedSizes.set(target.url, size);
          const clean = cleanMediaPath(target.key);
          if (clean) globalProbedSizes.set(clean, size);
          updates[target.key] = size;
          updates[target.url] = size;
          if (clean) updates[clean] = size;
        }
        probingMediaPaths.delete(target.key);
      }

      if (Object.keys(updates).length > 0) {
        probedMediaSizes = { ...probedMediaSizes, ...updates };
      }
    } catch (err) {
      logger.warn('Failed to batch probe media sizes', err);
      for (const target of probeTargets) {
        probingMediaPaths.delete(target.key);
      }
    } finally {
      isProbingBatch = false;
    }
  }

  $effect(() => {
    const currentMedia = media;
    const currentFile = post?.file;
    if (!post) return;
    untrack(() => {
      const allFiles = currentFile ? [currentFile, ...currentMedia] : currentMedia;
      void probeMediaBatch(allFiles);
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

  function resolveAttachmentDownloadUrl(file: { path?: string; server?: string; name?: string; is_cloud?: boolean; cloud_folder_result?: any; cloud_node_id?: string }): string {
    const port = serverPortState.port || 0;
    let targetUrl = file.path || '';

    if (file.is_cloud && file.cloud_folder_result) {
      const matchingNode = file.cloud_folder_result.nodes?.find((n: any) => n.id === file.cloud_node_id || n.name === file.name);
      if (matchingNode) {
        if (matchingNode.stream_url?.startsWith('/cloud_stream/') && port > 0) {
          return serverPortState.mediaUrl(matchingNode.stream_url);
        }
        if (matchingNode.download_url) {
          targetUrl = matchingNode.download_url;
        } else if (matchingNode.stream_url) {
          targetUrl = matchingNode.stream_url;
        }
      }
    }

    if (targetUrl.startsWith('/cloud_stream/') && port > 0) {
      return serverPortState.mediaUrl(targetUrl);
    }

    if (targetUrl.startsWith('http://') || targetUrl.startsWith('https://')) {
      return targetUrl;
    }

    return remoteFileUrl(file as Attachment);
  }

  async function download(file: { path?: string; server?: string; name?: string; is_cloud?: boolean; cloud_folder_result?: any; cloud_node_id?: string }, index: number) {
    if (!file.path) return;
    try {
      if (!post) return;
      const targetName = file.name || `${postId}_${index + 1}`;
      const targetUrl = resolveAttachmentDownloadUrl(file);
      await downloadState.start(post, file.path, targetUrl, targetName);
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
        pendingMedia.map((file, index) => {
          const targetUrl = resolveAttachmentDownloadUrl(file);
          return downloadState.start(post!, file.path!, targetUrl, file.name || `${postId}_${index + 1}`);
        })
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
        logger.error(`Failed to load comments for ${service}:${postId}`, err);
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
  {@const isDownloading = Boolean(job && ['queued', 'resolving', 'downloading', 'verifying'].includes(job.status))}
  {@const isPaused = job?.status === 'paused'}
  {@const isFailedOrCancelled = Boolean(job && ['failed', 'cancelled', 'missing'].includes(job.status))}
  {@const verifying = job?.status === 'verifying'}
  {@const queued = job?.status === 'queued'}
  {@const resolving = job?.status === 'resolving'}
  {@const knownTotal = job?.total_bytes || 0}
  {@const declaredBytes = getEffectiveFileSize(file)}
  {@const declaredSize = declaredBytes > 0 ? formatBytes(declaredBytes) : ''}
  {@const hasProgress = Boolean(job && !verifying && !queued && knownTotal > 0)}
  {@const progress = job && knownTotal > 0 ? Math.min(100, Math.round(job.downloaded_bytes / knownTotal * 100)) : 0}
  {@const isUnarchived = isFileUnarchived(file, Boolean(downloaded))}
  <div class="media-download-group">
    {#if downloaded}
      <Button
        variant="ghost"
        class="media-download-btn is-downloaded"
        onclick={() => openMediaViewer(file, activeGalleryItems)}
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
    {:else if isDownloading && job}
      <Button
        variant="ghost"
        class="media-download-btn is-downloading"
        disabled={true}
        title={i18n.t(verifying ? 'downloads.status_verifying' : queued ? 'downloads.status_queued' : resolving ? 'downloads.status_resolving' : 'post.downloading')}
      >
        {#if hasProgress}<span class="attachment-progress-fill" style:width={`${progress}%`}></span>{/if}
        <span class="attachment-button-state downloading-state">
          {#if hasProgress}<IconDownload />{:else}<IconLoading />{/if}
          <span>
            {verifying ? i18n.t('downloads.status_verifying') : queued ? i18n.t('downloads.status_queued') : resolving ? i18n.t('downloads.status_resolving') : i18n.t('post.downloading')}
            {hasProgress ? ` · ${progress}%` : ''}
          </span>
        </span>
      </Button>
      <Button
        variant="ghost"
        class="media-action-icon-btn"
        onclick={() => void downloadState.pause(job.id)}
        tooltip={i18n.t('downloads.pause')}
        aria-label={i18n.t('downloads.pause')}
      >
        <IconPause class="w-[18px] h-[18px]" />
      </Button>
      <Button
        variant="ghost"
        class="media-action-icon-btn is-danger"
        onclick={() => void downloadState.remove(job.id)}
        tooltip={i18n.t('downloads.cancel')}
        aria-label={i18n.t('downloads.cancel')}
      >
        <IconDismiss class="w-[18px] h-[18px]" />
      </Button>
    {:else if isPaused && job}
      <Button
        variant="ghost"
        class="media-download-btn is-downloading is-paused"
        onclick={() => void downloadState.resume(job.id)}
        title={i18n.t('downloads.resume')}
      >
        {#if hasProgress}<span class="attachment-progress-fill opacity-50" style:width={`${progress}%`}></span>{/if}
        <span class="attachment-button-state downloading-state text-[var(--warning,#fbbf24)]">
          <IconPause />
          <span>{i18n.t('downloads.status_paused')}{hasProgress ? ` · ${progress}%` : ''}</span>
        </span>
      </Button>
      <Button
        variant="ghost"
        class="media-action-icon-btn"
        onclick={() => void downloadState.resume(job.id)}
        tooltip={i18n.t('downloads.resume')}
        aria-label={i18n.t('downloads.resume')}
      >
        <IconPlay class="w-[18px] h-[18px]" />
      </Button>
      <Button
        variant="ghost"
        class="media-action-icon-btn is-danger"
        onclick={() => void downloadState.remove(job.id)}
        tooltip={i18n.t('downloads.cancel')}
        aria-label={i18n.t('downloads.cancel')}
      >
        <IconDismiss class="w-[18px] h-[18px]" />
      </Button>
    {:else if isFailedOrCancelled && job}
      <Button
        variant="ghost"
        class="media-download-btn is-failed"
        onclick={() => void downloadState.retry(job.id)}
        title={i18n.t('downloads.retry')}
      >
        <span class="attachment-button-state text-[var(--danger,#ff626d)]">
          <IconArrowClockwise />
          <span>{i18n.t('downloads.retry')}{declaredSize ? ` · ${declaredSize}` : ''}</span>
        </span>
      </Button>
      <Button
        variant="ghost"
        class="media-action-icon-btn"
        onclick={() => void downloadState.retry(job.id)}
        tooltip={i18n.t('downloads.retry')}
        aria-label={i18n.t('downloads.retry')}
      >
        <IconArrowClockwise class="w-[18px] h-[18px]" />
      </Button>
      <Button
        variant="ghost"
        class="media-action-icon-btn is-danger"
        onclick={() => void downloadState.remove(job.id)}
        tooltip={i18n.t('downloads.remove')}
        aria-label={i18n.t('downloads.remove')}
      >
        <IconDismiss class="w-[18px] h-[18px]" />
      </Button>
    {:else if isUnarchived}
      <Button
        variant="ghost"
        class="media-download-btn opacity-60 cursor-not-allowed"
        disabled={true}
        title={i18n.t('post.file_not_archived')}
      >
        <span class="attachment-button-state text-[var(--fg-muted)]">
          <IconVideoOff class="w-[16px] h-[16px]" />
          <span>{i18n.t('post.unarchived')}</span>
        </span>
      </Button>
    {:else}
      <Button
        variant="ghost"
        onclick={() => void download(file, index)}
        class="media-download-btn"
        title={i18n.t('post.download')}
      >
        <span class="attachment-button-state">
          <IconDownload />
          <span>{i18n.t('post.download')}{declaredSize ? ` · ${declaredSize}` : ''}</span>
        </span>
      </Button>
    {/if}
  </div>
{/snippet}

<PageShell scrollable={true} scrollKey={navigationState.entryKey}>
  {#snippet overlay()}
    <StickyHeader threshold={120}>
      {#snippet leading()}
        <Button variant="ghost" onclick={() => navigationState.back()} class="sticky-back-btn btn-icon" title={i18n.t('nav.back')}>
          <IconArrowLeft class="w-[20px] h-[20px]" />
        </Button>
        <span class="sticky-post-title">{cleanPostTitle(post?.title) || ''}</span>
      {/snippet}

      {#snippet trailing()}
        {#if post}
          <Button
            variant="ghost"
            onclick={() => navigationState.openCreator(service, creatorId)}
            class="sticky-creator-btn action-btn"
            title={`${i18n.t('feed.open_creator') || 'Creator'}: ${creatorName}`}
            aria-label="Creator"
          >
            {#if creatorAvatar && !creatorAvatarFailed}
              <span class="post-creator-avatar"><img src={creatorAvatar} alt="" onerror={() => creatorAvatarFailed = true} /></span>
            {:else}
              <ServiceIcon {service} />
            {/if}
            <span class="sticky-creator-name">{creatorName}</span>
          </Button>

          {#if !layoutState.isMobile}
            <Button
              variant={isFavorited ? 'accent' : 'ghost'}
              disabled={favoritingPending}
              onclick={toggleFavorite}
              class="sticky-action-btn action-btn"
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
                icon={saved || postStashes.length > 0 ? IconSaved : IconSave}
                disabled={saving}
              />
            </div>
          {:else}
            <Button
              variant={isFavorited ? 'accent' : 'ghost'}
              disabled={favoritingPending}
              onclick={toggleFavorite}
              class="sticky-action-btn btn-icon sticky-fav-btn"
              title={i18n.t(isFavorited ? 'post.unfavorite' : 'post.favorite')}
              aria-label={i18n.t(isFavorited ? 'post.unfavorite' : 'post.favorite')}
            >
              {#if isFavorited}
                <IconHeartFilled class="w-[18px] h-[18px] fav-active-heart" />
              {:else}
                <IconHeart class="w-[18px] h-[18px]" />
              {/if}
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
                iconOnly={true}
                icon={saved || postStashes.length > 0 ? IconSaved : IconSave}
                disabled={saving}
                ariaLabel={libraryButtonLabel}
              />
            </div>

            <Button
              variant="ghost"
              class="btn-icon action-btn"
              onclick={() => (mobileMoreOpen = true)}
              title={i18n.t('common.more') || 'More'}
              aria-label="More actions"
            >
              <IconMoreVertical class="w-5 h-5" />
            </Button>
          {/if}
        {/if}
      {/snippet}
    </StickyHeader>
  {/snippet}

  {#if heroImageUrl}
    <HeroBackdrop src={heroImageUrl} />
  {/if}

  <div class="post-content-wrapper">
    <div class="post-actions-bar">
      {#if !layoutState.isMobile}
        <div class="left-actions flex items-center gap-2">
          <Button variant="ghost" onclick={() => navigationState.back()} class="action-btn">
            <IconArrowLeft class="w-[18px] h-[18px]" />
            <span>{i18n.t('nav.back')}</span>
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
              <span class="creator-name-text">{creatorName}</span>
            </Button>

            <Button
              variant={isFavorited ? 'accent' : 'ghost'}
              disabled={favoritingPending}
              onclick={toggleFavorite}
              class="action-btn"
              title={i18n.t(isFavorited ? 'post.unfavorite' : 'post.favorite')}
              aria-label={i18n.t(isFavorited ? 'post.unfavorite' : 'post.favorite')}
            >
              {#if isFavorited}
                <IconHeartFilled class="w-[18px] h-[18px] fav-active-heart" />
              {:else}
                <IconHeart class="w-[18px] h-[18px]" />
              {/if}
              <span>{i18n.t(isFavorited ? 'post.unfavorite' : 'post.favorite')}</span>
            </Button>
          {/if}
        </div>

        <div class="right-actions flex items-center gap-2 ml-auto">
          {#if post}
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
      {:else}
        <div class="left-actions flex items-center gap-2 min-w-0">
          <Button
            variant="ghost"
            onclick={() => navigationState.back()}
            class="action-btn btn-icon"
            title={i18n.t('nav.back')}
            aria-label={i18n.t('nav.back')}
          >
            <IconArrowLeft class="w-5 h-5" />
          </Button>

          {#if post}
            <Button
              variant="ghost"
              onclick={() => navigationState.openCreator(service, creatorId)}
              class="action-btn creator-btn"
              title={`${i18n.t('feed.open_creator') || 'Creator'}: ${creatorName}`}
              aria-label="Creator"
            >
              {#if creatorAvatar && !creatorAvatarFailed}
                <span class="post-creator-avatar"><img src={creatorAvatar} alt="" onerror={() => creatorAvatarFailed = true} /></span>
              {:else}
                <ServiceIcon {service} />
              {/if}
              <span class="creator-name-text">{creatorName}</span>
            </Button>

            <Button
              variant={isFavorited ? 'accent' : 'ghost'}
              disabled={favoritingPending}
              onclick={toggleFavorite}
              class="action-btn btn-icon"
              title={i18n.t(isFavorited ? 'post.unfavorite' : 'post.favorite')}
              aria-label={i18n.t(isFavorited ? 'post.unfavorite' : 'post.favorite')}
            >
              {#if isFavorited}
                <IconHeartFilled class="w-5 h-5 fav-active-heart" />
              {:else}
                <IconHeart class="w-5 h-5" />
              {/if}
            </Button>
          {/if}
        </div>

        <div class="right-actions flex items-center gap-2 ml-auto shrink-0">
          {#if post}
            <div class="stash-select-container mobile-stash-select">
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
                iconOnly={true}
                icon={saved || postStashes.length > 0 ? IconSaved : IconSave}
                disabled={saving}
                class="stash-select"
                ariaLabel={libraryButtonLabel}
              />
            </div>

            <Button
              variant="ghost"
              class="action-btn btn-icon"
              onclick={() => (mobileMoreOpen = true)}
              title={i18n.t('common.more') || 'More'}
              aria-label="More options"
            >
              <IconMoreVertical class="w-5 h-5" />
            </Button>
          {/if}
        </div>
      {/if}
    </div>

    {#if post || (entry.loaded && !entry.loading) || (activeProviderId && activeProviderId !== 'auto') || candidateProviders.length > 0}
      <header class="detail-header">
        <div class="min-w-0 flex-1">
          {#if post}
            <h1>{cleanPostTitle(post.title) || i18n.t('feed.untitled')}</h1>
          {:else if autoEntry?.post?.title}
            <h1 class="opacity-75">{cleanPostTitle(autoEntry.post.title)}</h1>
          {/if}
          {#if publishedDateStr}
            <div class="post-date post-dates-row flex items-center flex-wrap gap-2 mt-2 text-sm text-[var(--fg-muted)]">
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
            </div>
          {/if}

          <div class="post-meta-actions-row flex items-center flex-wrap gap-2 mt-1 min-h-[38px] text-sm text-[var(--fg-muted)]">
            <Button
              variant="ghost"
              onclick={openOriginalPost}
              tooltip={`${i18n.t('post.open_original_post') || 'Open original post'}: ${service}`}
              aria-label={`Open on ${service}`}
            >
              <ServiceIcon {service} class="w-4 h-4" />
              <span class="capitalize">{service}</span>
            </Button>

            {#if post}
              {#if activeProviderId === 'auto' && providersWithPost.length > 1}
                {#each providersWithPost as prov}
                  <span class="text-[var(--fg-subtle)]">·</span>
                  <Button
                    variant="ghost"
                    onclick={() => openInProvider(prov.id)}
                    tooltip={`${i18n.t('post.open_in_provider') || 'Open in provider'}: ${formatProviderName(prov.name || prov.id)}`}
                    aria-label={`Open in ${formatProviderName(prov.name || prov.id)}`}
                  >
                    <IconOpen class="w-4 h-4" />
                    <span>{formatProviderName(prov.name || prov.id)}</span>
                  </Button>
                {/each}
              {:else}
                <span class="text-[var(--fg-subtle)]">·</span>
                <Button
                  variant="ghost"
                  onclick={() => openInProvider(activeProviderId && activeProviderId !== 'auto' ? activeProviderId : undefined)}
                  tooltip={`${i18n.t('post.open_in_provider') || 'Open in provider'}: ${currentProviderName}`}
                  aria-label={`Open in ${currentProviderName}`}
                >
                  <IconOpen class="w-4 h-4" />
                  <span>{currentProviderName}</span>
                </Button>
              {/if}
            {/if}

            <span class="text-[var(--fg-subtle)]">·</span>
            <Button
              variant="ghost"
              onclick={copyPostId}
              tooltip={i18n.t('post.copy_id') || 'Copy ID'}
            >
              <span class="font-mono text-[var(--fg-subtle)]">#{postId}</span>
              {#if copiedPostId}
                <IconCheck class="w-[16px] h-[16px] text-accent" />
              {:else}
                <IconCopy class="w-[16px] h-[16px] opacity-60" />
              {/if}
            </Button>

            {#if candidateProviders.length > 1}
              <span class="text-[var(--fg-subtle)]">·</span>
              <div class="inline-flex items-center shrink-0">
                <Select
                  variant="ghost"
                  options={providerSelectOptions}
                  value={activeProviderId}
                  onchange={onProviderChange}
                />
              </div>
            {/if}

            {#if filteredRevisions.length > 0}
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
                size="sm"
                onclick={(tag) => {
                  navigationState.openCreator(service, creatorId, tag);
                }}
              />
            </div>
          {/if}
        </div>
      </header>

      {#if post}
        {#if post.poll}
          <PostPoll poll={post.poll} />
        {/if}

      {#if entry.loading && !entry.loaded}
        <div class="detail-loading">{i18n.t('feed.loading')}</div>
      {/if}

      {#if media.length > 0 || hasEmbed}
        <div class="media-section">
          {#if ((media.length > 1 || (hasEmbed && media.length > 0)) && activeCategoriesCount > 1) || media.length >= 20 || mediaSearchQuery || media.length > 1}
            <div class="media-controls-row" class:search-active={mediaSearchOpen}>
              {#if (media.length > 1 || (hasEmbed && media.length > 0)) && activeCategoriesCount > 1}
                <div class="post-tabs-scroll">
                  <ChoiceGroup
                    options={mediaTabOptions}
                    value={activeMediaTab}
                    onchange={(val) => activeMediaTab = val as MediaTab}
                    ariaLabel="Media categories"
                    align="left"
                    class="post-media-choice"
                  />
                </div>
              {/if}

              <div class="media-controls-actions" class:search-active={mediaSearchOpen}>
                <HeaderActions
                  bind:searchOpen={mediaSearchOpen}
                  bind:searchQuery={mediaSearchQuery}
                  showSearchButton={media.length >= 20 || Boolean(mediaSearchQuery)}
                  searchPlaceholder={i18n.t('post.search_media') || 'Search media...'}
                  onsearchtoggle={(open) => {
                    if (!open) closeMediaSearch();
                  }}
                >
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
                        icon={IconArrowSort}
                        iconOnly={layoutState.isMobile}
                        ariaLabel={i18n.t('favorites.sort_by') || 'Sort'}
                      />
                    </div>
                  {/if}
                </HeaderActions>
              </div>
            </div>
          {/if}

          {#if filteredMedia.length > 0 || (hasEmbed && isEmbedVisibleInTab && embedMatchesSearch)}
            <div class="media-gallery-wrapper" bind:this={galleryWrapperEl}>
              <div class="media-gallery-container" class:is-collapsed={isGalleryOverflowing && !galleryExpanded}>
                <section class="media-gallery" bind:clientHeight={galleryHeight} aria-label={i18n.t('post.media')}>
                {#if postEmbed && !isEmbedResolvedToCloud && isEmbedVisibleInTab && embedMatchesSearch}
                  {#if isEmbedLinkedPost}
                    <div class="media-item is-embed-item is-linked-post-item">
                      <div class="media-header">
                        <span class="media-filename">{postEmbed.subject || postEmbed.description || (linkedPostId ? `Post #${linkedPostId}` : post.title)}</span>
                        <span class="cloud-source-badge">Post</span>
                      </div>

                      <button
                        class="file-placeholder media-open-surface cursor-pointer hover:bg-[var(--bg-card-hover)] transition-colors"
                        type="button"
                        onclick={() => {
                          if (linkedPostId) {
                            navigationState.openPost(service, creatorId, linkedPostId);
                          } else if (postEmbed?.url) {
                            void apiOpenInBrowser(postEmbed.url);
                          }
                        }}
                        aria-label={postEmbed.subject || (linkedPostId ? `Post #${linkedPostId}` : 'Linked Post')}
                      >
                        <IconDocument class="placeholder-icon text-[var(--accent)]" />
                        <p class="placeholder-text font-medium">{postEmbed.subject || postEmbed.description || (linkedPostId ? `Open Post #${linkedPostId}` : i18n.t('post.linked_post') || 'Linked Post')}</p>
                      </button>

                      <div class="media-download-group">
                        <Button
                          variant="ghost"
                          onclick={() => {
                            if (linkedPostId) {
                              navigationState.openPost(service, creatorId, linkedPostId);
                            } else if (postEmbed?.url) {
                              void apiOpenInBrowser(postEmbed.url);
                            }
                          }}
                          class="media-download-btn is-embed-btn"
                          title={i18n.t('post.open_linked_post') || 'Open Post'}
                        >
                          <span class="attachment-button-state">
                            <IconDocument class="w-[16px] h-[16px]" />
                            <span>{i18n.t('post.open_linked_post') || (linkedPostId ? `Post #${linkedPostId}` : 'Open Post')}</span>
                          </span>
                        </Button>
                      </div>
                    </div>
                  {:else}
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
                          src={embedAttachment ? fileUrl(embedAttachment) : postEmbed.url}
                          controls
                          playsinline
                          preload="none"
                          use:panicCapture
                          onkeydown={handleGlobalPanicKey}
                        ></video>
                      {:else}
                        <button
                          class="file-placeholder media-open-surface"
                          type="button"
                          onclick={() => {
                            if (postEmbed?.url) {
                              void apiOpenInBrowser(postEmbed.url);
                            }
                          }}
                          aria-label={postEmbed.subject || 'External Link'}
                        >
                          <IconGlobe class="placeholder-icon text-[var(--text-secondary)]" />
                          <p class="placeholder-text">{postEmbed.subject || postEmbed.description || 'External Link'}</p>
                        </button>
                      {/if}

                      {#if embedAttachment && isEmbedVideo}
                        <button
                          class="media-viewer-open-btn"
                          type="button"
                          onclick={() => embedAttachment && openMediaViewer(embedAttachment, filteredMedia)}
                          use:tooltip={i18n.t('post.viewer_open')}
                          aria-label={i18n.t('post.viewer_open')}
                        ><IconFullscreen /></button>
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
                {/if}

                {#each filteredMedia as file, index}
                  {@const isCloudFolder = (file as any)?.is_cloud_folder === true}
                  {@const isDeferred = file?.deferred === true || (!file?.path && Boolean(file?.name))}
                  {@const url = file?.path ? fileUrl(file) : ''}
                  {@const isCloud = (file as any)?.is_cloud === true}
                  {@const downloaded = attachmentDownload(file)}
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
                        {#if isCloud && (file as any).cloud_provider}
                          <span class="cloud-source-badge">{(file as any).cloud_provider}</span>
                        {/if}
                      </div>
                      <div class="file-placeholder media-open-surface is-deferred-placeholder" title={i18n.t('post.file_not_saved_desc')}>
                        <IconWarning class="placeholder-icon text-red-500" />
                        <p class="placeholder-text text-red-400">{i18n.t('post.file_not_saved')}</p>
                      </div>
                    {:else if isAttachmentVideo(file, url) || isAttachmentImage(file, url)}
                      {@const isVid = isAttachmentVideo(file, url)}
                      {@const isUnarchived = isFileUnarchived(file, Boolean(downloaded?.final_path))}
                      <div class="media-header">
                        <span class="media-filename">{file?.name || i18n.t('post.file')}</span>
                        {#if isCloud && (file as any).cloud_provider}
                          <span class="cloud-source-badge">{(file as any).cloud_provider}</span>
                        {/if}
                      </div>
                      {#if isVid && (isUnarchived || videoFailures[index] || (isH265Video(file?.name, url) && !hevcSupported))}
                        {@const failure = (isUnarchived ? { preset: 'unarchived' as const } : null) || videoFailures[index] || (isH265Video(file?.name, url) && !hevcSupported ? { preset: 'unsupported_codec' as const, format: 'H.265 / HEVC' } : null)}
                        <div class="file-placeholder is-error-placeholder">
                          <IconVideoOff class="placeholder-icon" />
                          {#if failure?.preset === 'unsupported_codec'}
                            <p class="placeholder-text">{i18n.t('post.unsupported_codec_desc')}</p>
                            <Button
                              variant="ghost"
                              class="placeholder-fix-btn"
                              onclick={() => isCodecModalOpen = true}
                            >
                              <IconSparkle class="w-[15px] h-[15px] text-[var(--accent-primary)]" />
                              <span>{i18n.t('post.codec_how_to_fix')}</span>
                            </Button>
                          {:else if failure?.preset === 'unsupported_format'}
                            <p class="placeholder-text">
                              {i18n.t('post.unsupported_format_desc', { format: failure.format || getFileExtension(file?.name) })}
                            </p>
                            {#if downloaded?.final_path}
                              <Button
                                variant="ghost"
                                class="placeholder-fix-btn"
                                onclick={() => void openFileExternally(downloaded)}
                              >
                                <IconPlay class="w-[15px] h-[15px] text-[var(--accent-primary)]" />
                                <span>{i18n.t('post.open_in_player')}</span>
                              </Button>
                            {:else}
                              <p class="placeholder-subtext">{i18n.t('post.unsupported_format_hint')}</p>
                            {/if}
                          {:else if failure?.preset === 'forbidden' || failure?.httpStatus === 403}
                            <p class="placeholder-text text-[var(--warning,#fbbf24)]">{i18n.t('post.error_forbidden') || 'HTTP 403 Forbidden'}</p>
                            <p class="placeholder-subtext">{i18n.t('post.error_forbidden_hint')}</p>
                            <Button
                              variant="ghost"
                              class="placeholder-fix-btn"
                              onclick={() => {
                                delete videoFailures[index];
                                activeVideoIndexes = new Set(activeVideoIndexes).add(index);
                              }}
                            >
                              <IconArrowClockwise class="w-[15px] h-[15px] text-[var(--accent-primary)]" />
                              <span>{i18n.t('downloads.retry') || 'Retry'}</span>
                            </Button>
                          {:else if failure?.preset === 'not_found' || failure?.httpStatus === 404}
                            <p class="placeholder-text text-red-400">{i18n.t('post.error_not_found') || 'HTTP 404 Not Found'}</p>
                            <p class="placeholder-subtext">{i18n.t('post.error_not_found_hint')}</p>
                          {:else if failure?.preset === 'rate_limited' || failure?.httpStatus === 429}
                            <p class="placeholder-text text-[var(--warning,#fbbf24)]">{i18n.t('post.error_rate_limited') || 'HTTP 429 Rate Limited'}</p>
                            <p class="placeholder-subtext">{i18n.t('post.error_rate_limited_hint')}</p>
                            <Button
                              variant="ghost"
                              class="placeholder-fix-btn"
                              onclick={() => {
                                delete videoFailures[index];
                                activeVideoIndexes = new Set(activeVideoIndexes).add(index);
                              }}
                            >
                              <IconArrowClockwise class="w-[15px] h-[15px] text-[var(--accent-primary)]" />
                              <span>{i18n.t('downloads.retry') || 'Retry'}</span>
                            </Button>
                          {:else if failure?.preset === 'server_error' || (failure?.httpStatus && failure.httpStatus >= 500)}
                            <p class="placeholder-text text-red-400">{failure.message || i18n.t('post.error_server')}</p>
                            <p class="placeholder-subtext">{i18n.t('post.error_server_hint')}</p>
                            <Button
                              variant="ghost"
                              class="placeholder-fix-btn"
                              onclick={() => {
                                delete videoFailures[index];
                                activeVideoIndexes = new Set(activeVideoIndexes).add(index);
                              }}
                            >
                              <IconArrowClockwise class="w-[15px] h-[15px] text-[var(--accent-primary)]" />
                              <span>{i18n.t('downloads.retry') || 'Retry'}</span>
                            </Button>
                          {:else if failure?.preset === 'network'}
                            <p class="placeholder-text">{i18n.t('post.network_stream_error')}</p>
                            <Button
                              variant="ghost"
                              class="placeholder-fix-btn"
                              onclick={() => {
                                delete videoFailures[index];
                                activeVideoIndexes = new Set(activeVideoIndexes).add(index);
                              }}
                            >
                              <IconArrowClockwise class="w-[15px] h-[15px] text-[var(--accent-primary)]" />
                              <span>{i18n.t('downloads.retry') || 'Retry'}</span>
                            </Button>
                          {:else if failure?.preset === 'decode'}
                            <p class="placeholder-text">{i18n.t('post.decode_error')}</p>
                            {#if failure.message}
                              <p class="placeholder-subtext font-mono text-[11px] opacity-75">{failure.message}</p>
                            {/if}
                            {#if downloaded?.final_path}
                              <Button
                                variant="ghost"
                                class="placeholder-fix-btn"
                                onclick={() => void openFileExternally(downloaded)}
                              >
                                <IconPlay class="w-[15px] h-[15px] text-[var(--accent-primary)]" />
                                <span>{i18n.t('post.open_in_player')}</span>
                              </Button>
                            {/if}
                          {:else if failure?.preset === 'unarchived'}
                            <p class="placeholder-text">{i18n.t('post.file_not_archived')}</p>
                          {:else if failure?.preset === 'unavailable'}
                            <p class="placeholder-text">{i18n.t('post.cloud_file_unavailable')}</p>
                          {:else}
                            <p class="placeholder-text">{failure?.message || i18n.t('post.video_load_failed') || 'Failed to play video'}</p>
                          {/if}
                        </div>
                      {:else}
                        {#if isVid}
                          {@const thumbUrl = videoThumbnails[index] || attachmentThumbnailUrl(file, service)}
                          {#if activeVideoIndexes.has(index)}
                            <!-- svelte-ignore a11y_media_has_caption -->
                            <video
                              src={url}
                              poster={thumbUrl}
                              controls
                              autoplay
                              playsinline
                              preload="auto"
                              use:panicCapture
                              onkeydown={handleGlobalPanicKey}
                              onloadedmetadata={(e) => handleVideoLoadedMetadata(e, file, index)}
                              onplaying={(e) => handleVideoLoadedMetadata(e, file, index)}
                              onerror={(e) => handleVideoError(e, file, index)}
                              onplay={handleVideoPlay}
                            ></video>
                          {:else if thumbUrl}
                            <button
                              class="media-open-surface video-thumbnail-surface"
                              type="button"
                              onclick={() => {
                                activeVideoIndexes = new Set(activeVideoIndexes).add(index);
                              }}
                              aria-label="Play video"
                            >
                              <img
                                src={thumbUrl}
                                alt={file?.name || post.title}
                                loading="lazy"
                                decoding="async"
                              />
                              <div class="video-play-overlay">
                                <div class="play-btn-circle">
                                  <IconPlayFilled class="w-6 h-6 ml-0.5 text-white" />
                                </div>
                              </div>
                            </button>
                          {:else}
                            <button
                              class="file-placeholder media-open-surface"
                              type="button"
                              onclick={() => {
                                activeVideoIndexes = new Set(activeVideoIndexes).add(index);
                              }}
                              aria-label="Play video"
                            >
                              <div class="play-btn-circle">
                                <IconPlayFilled class="w-6 h-6 ml-0.5 text-white" />
                              </div>
                              <p class="placeholder-text">{getFileExtension(file?.name).toUpperCase() || 'VIDEO'}</p>
                            </button>
                          {/if}
                          <button
                            class="media-viewer-open-btn"
                            type="button"
                            onclick={(e) => openMediaViewer(file!, activeGalleryItems, e.currentTarget.parentElement?.querySelector('video'))}
                            use:tooltip={i18n.t('post.viewer_open')}
                            aria-label={i18n.t('post.viewer_open')}
                          ><IconFullscreen /></button>
                        {:else}
                          <button
                            class="media-open-surface"
                            type="button"
                            onclick={() => openMediaViewer(file!, activeGalleryItems)}
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
                    {:else if isAttachmentAudio(file, url)}
                      {@const ext = getFileExtension(file?.name).toUpperCase() || 'AUDIO'}
                      {@const isUnarchived = isFileUnarchived(file, Boolean(downloaded?.final_path))}
                      <div class="media-header">
                        <span class="media-filename">{file?.name || i18n.t('post.file')}</span>
                        {#if isCloud && (file as any).cloud_provider}
                          <span class="cloud-source-badge">{(file as any).cloud_provider}</span>
                        {/if}
                      </div>
                      {#if isUnarchived}
                        <div class="file-placeholder is-error-placeholder">
                          <IconVideoOff class="placeholder-icon" />
                          <p class="placeholder-text">{i18n.t('post.file_not_archived')}</p>
                        </div>
                      {:else if activeAudioIndexes.has(index)}
                        <div class="audio-player-wrapper">
                          <IconMusicFilled class="placeholder-icon video-play-accent mb-2" />
                          <audio
                            src={url}
                            controls
                            autoplay
                            preload="auto"
                            use:panicCapture
                            onplay={handleVideoPlay}
                          ></audio>
                        </div>
                      {:else}
                        <button
                          class="file-placeholder media-open-surface"
                          type="button"
                          onclick={() => {
                            activeAudioIndexes = new Set(activeAudioIndexes).add(index);
                          }}
                          aria-label="Play audio"
                        >
                          <div class="play-btn-circle">
                            <IconMusicFilled class="w-6 h-6 text-white" />
                          </div>
                          <p class="placeholder-text">{ext}</p>
                        </button>
                      {/if}
                      {#if !isUnarchived}
                        <button
                          class="media-viewer-open-btn"
                          type="button"
                          onclick={() => openMediaViewer(file!, activeGalleryItems)}
                          use:tooltip={i18n.t('post.viewer_open')}
                          aria-label={i18n.t('post.viewer_open')}
                        ><IconFullscreen /></button>
                      {/if}
                      {@render mediaDownloadAction(file!, index)}
                    {:else}
                      {@const ext = getFileExtension(file?.name).toUpperCase() || 'FILE'}
                      {@const isUnarchived = isFileUnarchived(file, Boolean(downloaded?.final_path))}
                      <div class="media-header">
                        <span class="media-filename">{file?.name || i18n.t('post.file')}</span>
                        {#if isCloud && (file as any).cloud_provider}
                          <span class="cloud-source-badge">{(file as any).cloud_provider}</span>
                        {/if}
                      </div>
                      {#if isUnarchived}
                        <div class="file-placeholder is-error-placeholder">
                          <IconVideoOff class="placeholder-icon" />
                          <p class="placeholder-text">{i18n.t('post.file_not_archived')}</p>
                        </div>
                      {:else}
                        <button class="file-placeholder media-open-surface" type="button" onclick={() => openMediaViewer(file!, activeGalleryItems)} aria-label={`${i18n.t('post.viewer_open')}: ${file?.name || i18n.t('post.file')}`}>
                          <IconDocument class="placeholder-icon" />
                          <p class="placeholder-text">{ext}</p>
                        </button>
                      {/if}
                      {@render mediaDownloadAction(file!, index)}
                    {/if}
                  </div>
                {/each}
              </section>
            </div>

            {#if isGalleryOverflowing && !galleryExpanded}
              <div class="gallery-fade-overlay">
                <Button
                  variant="ghost"
                  onclick={() => {
                    galleryScrollBeforeExpand = scrollContext?.viewport?.scrollTop ?? null;
                    galleryExpanded = true;
                  }}
                  class="gallery-expand-btn"
                >
                  <IconChevronDown class="w-[16px] h-[16px]" />
                  <span>{i18n.t('post.expand_gallery')}</span>
                </Button>
              </div>
            {:else if isGalleryOverflowing && galleryExpanded}
              <div class="gallery-collapse-action">
                <Button
                  variant="ghost"
                  onclick={() => smartCollapseSection(() => galleryExpanded = false, galleryWrapperEl, MAX_GALLERY_HEIGHT, galleryScrollBeforeExpand)}
                  class="gallery-collapse-btn"
                >
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
        <section class="post-content" bind:this={contentWrapperEl}>
          <div class="html-content-container" class:is-collapsed={isOverflowing && !contentExpanded}>
            <div class="html-content" bind:clientHeight={contentHeight}>
              <RichContent
                html={richContent}
                currentService={service}
                currentCreatorId={creatorId}
                cloudUrls={post?.cloud_urls || []}
                onopencloud={handleOpenCloudFromText}
                onopenmedia={(src, alt) => {
                  const existing = media.find((f) => f.path && (f.path === src || src.includes(f.path)));
                  if (existing) {
                    openMediaViewer(existing, activeGalleryItems);
                  } else {
                    const inlineAtt: Attachment = {
                      name: alt || src.split('/').pop()?.split('?')[0] || 'Image',
                      path: src
                    } as any;
                    openMediaViewer(inlineAtt, [inlineAtt, ...activeGalleryItems]);
                  }
                }}
              />
            </div>
          </div>

          {#if isOverflowing && !contentExpanded}
            <div class="content-fade-overlay">
              <Button
                variant="ghost"
                onclick={() => {
                  contentScrollBeforeExpand = scrollContext?.viewport?.scrollTop ?? null;
                  contentExpanded = true;
                }}
                class="expand-btn"
              >
                <IconChevronDown class="w-[16px] h-[16px]" />
                <span>{i18n.t('post.read_more')}</span>
              </Button>
            </div>
          {:else if isOverflowing && contentExpanded}
            <div class="content-collapse-action">
              <Button
                variant="ghost"
                onclick={() => smartCollapseSection(() => contentExpanded = false, contentWrapperEl, MAX_CONTENT_HEIGHT, contentScrollBeforeExpand)}
                class="collapse-btn"
              >
                <IconChevronUp class="w-[16px] h-[16px]" />
                <span>{i18n.t('post.read_less')}</span>
              </Button>
            </div>
          {/if}
        </section>

        {#if allMediaDownloaded || media.length > 0 || post.file}
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

            {#if post.file || media.length > 0}
              <Button variant="ghost" onclick={openPreviewViewer} class="post-footer-action">
                <IconEye class="w-[18px] h-[18px]" />
                <span>{i18n.t('post.view_preview')}</span>
              </Button>
            {/if}
          </div>
        {/if}

        <div class="post-footer-toolbar">
          <div class="footer-nav-left">
            <Button
              variant="ghost"
              disabled={!effectiveOlderId}
              onclick={() => effectiveOlderId && navigationState.openPost(service, creatorId, effectiveOlderId)}
              class="footer-nav-btn"
              title={i18n.t('post.previous') || 'Previous'}
            >
              <IconChevronLeft class="w-[18px] h-[18px]" />
              <span>{leftPostTitle}</span>
            </Button>
          </div>

          <div class="footer-nav-right">
            <Button
              variant="ghost"
              disabled={!effectiveNewerId}
              onclick={() => effectiveNewerId && navigationState.openPost(service, creatorId, effectiveNewerId)}
              class="footer-nav-btn"
              title={i18n.t('post.next') || 'Next'}
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
          <div class="comments-wrapper" bind:this={commentsWrapperEl}>
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
                <Button
                  variant="ghost"
                  onclick={() => {
                    commentsScrollBeforeExpand = scrollContext?.viewport?.scrollTop ?? null;
                    commentsExpanded = true;
                  }}
                >
                  <IconChevronDown class="w-[16px] h-[16px]" />
                  <span>{i18n.t('post.expand_comments') || 'Show All Comments'}</span>
                </Button>
              </div>
            {:else if isCommentsOverflowing && commentsExpanded}
              <div class="comments-collapse-action">
                <Button
                  variant="ghost"
                  onclick={() => smartCollapseSection(() => commentsExpanded = false, commentsWrapperEl, MAX_COMMENTS_HEIGHT, commentsScrollBeforeExpand)}
                  class="comments-collapse-btn"
                >
                  <IconChevronUp class="w-[16px] h-[16px]" />
                  <span>{i18n.t('post.collapse_comments') || 'Collapse Comments'}</span>
                </Button>
              </div>
            {/if}
          </div>
        {/if}
      </section>
      {:else if entry.loading}
        <div class="detail-loading py-16 flex flex-col items-center justify-center gap-3">
          <IconLoading class="w-8 h-8 text-accent animate-spin" />
          <span class="text-sm text-[var(--fg-muted)]">{i18n.t('feed.loading') || 'Loading post...'}</span>
        </div>
      {:else}
        <div class="detail-empty py-16 px-4 flex flex-col items-center justify-center text-center">
          <IconWarning class="w-10 h-10 text-[var(--fg-subtle)] mb-3 opacity-60" />
          <h2 class="text-lg font-semibold text-[var(--fg-default)] mb-1">
            {i18n.t('post.not_found_on_provider', { provider: currentProviderName }) || `Post not found on ${currentProviderName}`}
          </h2>
          <p class="text-sm text-[var(--fg-muted)] max-w-md mb-6">
            {i18n.t('post.not_found_on_provider_desc', { provider: currentProviderName }) || `This post does not exist on ${currentProviderName}. You can switch to Merged view or select another provider.`}
          </p>
          {#if candidateProviders.length > 1}
            <Button
              variant="accent"
              onclick={() => onProviderChange('auto')}
            >
              <IconSparkle class="w-4 h-4 mr-1.5" />
              <span>{i18n.t('post.switch_to_merged') || 'Switch to Merged'}</span>
            </Button>
          {/if}
        </div>
      {/if}
    {:else if entry.loading}
      <div class="detail-loading">{i18n.t('feed.loading')}</div>
    {:else if entry.error}
      <div class="detail-loading">{entry.error}</div>
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

<CodecGuideModal
  open={isCodecModalOpen}
  onclose={() => isCodecModalOpen = false}
/>

{#if layoutState.isMobile && post}
  <BottomSheet
    open={mobileMoreOpen}
    title={cleanPostTitle(post.title) || i18n.t('common.more') || 'More'}
    onclose={() => (mobileMoreOpen = false)}
  >
    <div class="flex flex-col gap-1 py-1">
      <button
        type="button"
        class="sheet-action-item"
        use:ripple
        onclick={() => {
          mobileMoreOpen = false;
          openOriginalPost();
        }}
      >
        <ServiceIcon {service} />
        <div class="flex flex-col min-w-0">
          <span class="text-sm font-semibold text-primary">{i18n.t('post.open_original_post') || 'Open original post'}</span>
          <span class="text-xs text-muted capitalize">{service}</span>
        </div>
      </button>

      {#if activeProviderId === 'auto' && providersWithPost.length > 1}
        {#each providersWithPost as prov}
          <button
            type="button"
            class="sheet-action-item"
            use:ripple
            onclick={() => {
              mobileMoreOpen = false;
              openInProvider(prov.id);
            }}
          >
            <IconOpen class="text-secondary" />
            <div class="flex flex-col min-w-0">
              <span class="text-sm font-semibold text-primary">{i18n.t('post.open_in_provider') || 'Open in provider'}</span>
              <span class="text-xs text-muted">{formatProviderName(prov.name || prov.id)}</span>
            </div>
          </button>
        {/each}
      {:else}
        <button
          type="button"
          class="sheet-action-item"
          use:ripple
          onclick={() => {
            mobileMoreOpen = false;
            openInProvider(activeProviderId && activeProviderId !== 'auto' ? activeProviderId : undefined);
          }}
        >
          <IconOpen class="text-secondary" />
          <div class="flex flex-col min-w-0">
            <span class="text-sm font-semibold text-primary">{i18n.t('post.open_in_provider') || 'Open in provider'}</span>
            <span class="text-xs text-muted">{currentProviderName}</span>
          </div>
        </button>
      {/if}

      <button
        type="button"
        class="sheet-action-item"
        use:ripple
        onclick={() => {
          copyPostLink();
          toast.success(i18n.t('post.link_copied') || 'Link copied to clipboard');
          mobileMoreOpen = false;
        }}
      >
        <IconCopy class="text-secondary" />
        <div class="flex flex-col min-w-0">
          <span class="text-sm font-semibold text-primary">{i18n.t('post.copy_link')}</span>
        </div>
      </button>

      <button
        type="button"
        class="sheet-action-item"
        use:ripple
        onclick={() => {
          copyPostId();
          toast.success(i18n.t('common.copied') || 'Copied to clipboard');
          mobileMoreOpen = false;
        }}
      >
        <IconDocument class="text-secondary" />
        <div class="flex flex-col min-w-0">
          <span class="text-sm font-semibold text-primary">{i18n.t('post.copy_id') || 'Copy post ID'}</span>
          <span class="text-xs font-mono text-muted">{postId}</span>
        </div>
      </button>

      <button
        type="button"
        class="sheet-action-item"
        use:ripple
        onclick={() => {
          mobileMoreOpen = false;
          navigationState.openCreator(service, creatorId);
        }}
      >
        <div class="sheet-action-avatar">
          {#if creatorAvatar && !creatorAvatarFailed}
            <img src={creatorAvatar} alt="" />
          {:else}
            <ServiceIcon {service} />
          {/if}
        </div>
        <div class="flex flex-col min-w-0">
          <span class="text-sm font-semibold text-primary">{creatorName}</span>
          <span class="text-xs text-muted">{i18n.t('feed.open_creator')}</span>
        </div>
      </button>

      {#if post.file || media.length > 0}
        <button
          type="button"
          class="sheet-action-item"
          use:ripple
          onclick={() => {
            mobileMoreOpen = false;
            openPreviewViewer();
          }}
        >
          <IconEye class="text-secondary" />
          <div class="flex flex-col min-w-0">
            <span class="text-sm font-semibold text-primary">{i18n.t('post.view_preview')}</span>
          </div>
        </button>
      {/if}

      {#if media.length > 0}
        {#if allMediaDownloaded}
          <button
            type="button"
            class="sheet-action-item"
            use:ripple
            onclick={() => {
              mobileMoreOpen = false;
              void openPostFolder();
            }}
          >
            <IconFolder class="text-secondary" />
            <div class="flex flex-col min-w-0">
              <span class="text-sm font-semibold text-primary">{i18n.t('downloads.open_post_folder')}</span>
            </div>
          </button>
        {:else}
          <button
            type="button"
            class="sheet-action-item"
            disabled={downloadingAll || media.every((file) => {
              const job = attachmentDownload(file);
              return Boolean(job && !['failed', 'cancelled', 'missing'].includes(job.status));
            })}
            use:ripple
            onclick={() => {
              mobileMoreOpen = false;
              void downloadAllMedia();
            }}
          >
            {#if downloadingAll}
              <IconLoading class="text-accent" />
            {:else}
              <IconDownload class="text-secondary" />
            {/if}
            <div class="flex flex-col min-w-0">
              <span class="text-sm font-semibold text-primary">
                {i18n.t(downloadingAll ? 'post.downloading_all' : 'post.download_all')}{totalMediaBytes > 0 ? ` · ${formatBytes(totalMediaBytes)}` : ''}
              </span>
            </div>
          </button>
        {/if}
      {/if}
    </div>
  </BottomSheet>
{/if}

<style>
  .post-content-wrapper {
    position: relative;
    z-index: 2;
    min-width: 0;
    max-width: 100%;
  }

  .post-actions-bar {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 20px;
    padding-bottom: 14px;
    z-index: 10;
    min-width: 0;
    max-width: 100%;
    --control-height: var(--control-height-md, 46px);
    --control-font-size: var(--control-font-md, 14px);
    --control-icon-size: var(--control-icon-md, 20px);
    --control-padding-x: var(--control-padding-md, 20px);
  }

  .post-actions-bar :global(.btn.btn-icon),
  :global(.sticky-header-bar) :global(.btn.btn-icon) {
    flex: 0 0 calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    height: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    min-width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    padding: 0 !important;
    border-radius: 50% !important;
    display: flex !important;
    align-items: center !important;
    justify-content: center !important;
  }

  .post-actions-bar :global(.mobile-stash-select) {
    min-width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    max-width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    height: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    margin-left: 0 !important;
  }

  .post-actions-bar :global(.mobile-stash-select .select-trigger),
  .post-actions-bar :global(.mobile-stash-select .select-trigger.icon-only) {
    width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    min-width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    height: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    border-radius: 50% !important;
    padding: 0 !important;
    display: flex !important;
    align-items: center !important;
    justify-content: center !important;
  }

  :global(.page-shell.mobile) .post-actions-bar,
  :global(.page-shell.is-mobile) .post-actions-bar {
    margin-bottom: 8px;
    padding-bottom: 6px;
    gap: 6px;
  }

  @media (max-width: 640px) {
    .post-actions-bar {
      margin-bottom: 8px;
      padding-bottom: 6px;
      gap: 6px;
    }
  }

  .post-actions-bar :global(.btn),
  .media-controls-row :global(.btn),
  .comments-header-row :global(.btn) {
    height: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    padding: 0 18px !important;
    font-size: calc(var(--control-font-size, 14px) * var(--ui-scale, 1)) !important;
    border-radius: var(--radius-full) !important;
    gap: 8px !important;
  }

  .post-actions-bar :global(.btn svg),
  .media-controls-row :global(.btn svg),
  .comments-header-row :global(.btn svg) {
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

  .post-actions-bar :global(.creator-btn) {
    padding-left: 8px !important;
    padding-right: 14px !important;
    gap: 8px !important;
    min-width: 0 !important;
    max-width: min(220px, 45vw) !important;
    flex-shrink: 1 !important;
  }

  .creator-name-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
    font-size: calc(var(--control-font-size, 14px) * var(--ui-scale, 1));
    font-weight: 500;
  }

  @media (max-width: 640px) {
    .post-actions-bar :global(.creator-btn) {
      padding-left: 6px !important;
      padding-right: 12px !important;
      max-width: min(150px, 35vw) !important;
    }

    .creator-name-text {
      font-size: 13px;
      max-width: clamp(55px, 20vw, 100px);
    }
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
    min-width: 0;
    max-width: 100%;
  }

  h1 {
    margin: 0;
    color: white;
    font-family: var(--font-sans);
    font-size: clamp(28px, 4.5vw, 42px);
    font-weight: var(--font-weight-normal);
    line-height: 1.12;
    word-break: break-word;
    overflow-wrap: break-word;
    min-width: 0;
  }

  :global(.page-shell.mobile) .detail-header,
  :global(.page-shell.is-mobile) .detail-header {
    padding-bottom: 6px;
  }

  :global(.page-shell.mobile) .detail-header h1,
  :global(.page-shell.is-mobile) .detail-header h1 {
    font-size: clamp(22px, 6vw, 30px);
  }

  @media (max-width: 640px) {
    .detail-header {
      padding-bottom: 6px;
    }
    .detail-header h1 {
      font-size: clamp(22px, 6vw, 30px);
    }
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
    gap: 16px;
    margin-bottom: 20px;
    min-width: 0;
    max-width: 100%;
    --control-height: var(--control-height-md, 46px);
    --control-font-size: var(--control-font-md, 14px);
    --control-icon-size: var(--control-icon-md, 20px);
    --control-padding-x: var(--control-padding-md, 20px);
  }

  .post-tabs-scroll {
    display: flex;
    align-items: center;
    flex: 1 1 auto;
    min-width: 0;
    max-width: 100%;
    overflow-x: auto;
    scrollbar-width: none;
    -webkit-overflow-scrolling: touch;
    -webkit-mask-image: linear-gradient(to right, black calc(100% - 24px), transparent 100%);
    mask-image: linear-gradient(to right, black calc(100% - 24px), transparent 100%);
    padding-right: 20px;
  }

  .post-tabs-scroll::-webkit-scrollbar {
    display: none;
  }

  .post-tabs-scroll :global(.choice-group) {
    flex-wrap: nowrap !important;
    max-width: none !important;
    flex-shrink: 0 !important;
  }

  :global(.page-shell.mobile) .media-sort-selector,
  :global(.page-shell.is-mobile) .media-sort-selector {
    width: auto !important;
    max-width: none !important;
  }

  @media (max-width: 640px) {
    .media-sort-selector {
      width: auto !important;
      max-width: none !important;
    }
  }

  :global(.page-shell.mobile) .media-sort-selector :global(.select-trigger),
  :global(.page-shell.is-mobile) .media-sort-selector :global(.select-trigger) {
    width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    min-width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    height: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    padding: 0 !important;
    border-radius: 50% !important;
    display: flex !important;
    align-items: center !important;
    justify-content: center !important;
  }

  @media (max-width: 640px) {
    .media-sort-selector :global(.select-trigger) {
      width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
      min-width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
      height: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
      padding: 0 !important;
      border-radius: 50% !important;
      display: flex !important;
      align-items: center !important;
      justify-content: center !important;
    }
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
    flex-shrink: 0;
  }

  :global(.page-shell.mobile) .media-controls-row:has(:global(.search-active)) .post-tabs-scroll,
  :global(.page-shell.is-mobile) .media-controls-row:has(:global(.search-active)) .post-tabs-scroll,
  :global(.page-shell.mobile) .media-controls-row.search-active .post-tabs-scroll,
  :global(.page-shell.is-mobile) .media-controls-row.search-active .post-tabs-scroll {
    display: none !important;
  }

  @media (max-width: 768px) {
    .media-controls-row:has(:global(.search-active)) .post-tabs-scroll,
    .media-controls-row.search-active .post-tabs-scroll {
      display: none !important;
    }
  }

  :global(.page-shell.mobile) .media-controls-row:has(:global(.search-active)) .media-controls-actions,
  :global(.page-shell.is-mobile) .media-controls-row:has(:global(.search-active)) .media-controls-actions,
  :global(.page-shell.mobile) .media-controls-row.search-active .media-controls-actions,
  :global(.page-shell.is-mobile) .media-controls-row.search-active .media-controls-actions {
    width: 100% !important;
    flex: 1 !important;
    margin-left: 0 !important;
  }

  @media (max-width: 768px) {
    .media-controls-row:has(:global(.search-active)) .media-controls-actions,
    .media-controls-row.search-active .media-controls-actions {
      width: 100% !important;
      flex: 1 !important;
      margin-left: 0 !important;
    }
  }

  :global(.page-shell.mobile .media-controls-actions .header-actions-root.search-active),
  :global(.page-shell.is-mobile .media-controls-actions .header-actions-root.search-active) {
    width: 100% !important;
    flex: 1 !important;
  }

  @media (max-width: 768px) {
    .media-controls-actions :global(.header-actions-root.search-active) {
      width: 100% !important;
      flex: 1 !important;
    }
    .media-controls-actions :global(.search-bar-expanded) {
      width: 100% !important;
      flex: 1 !important;
    }
    .media-controls-actions.search-active :global(.action-buttons-cluster),
    .media-controls-actions :global(.search-active .action-buttons-cluster),
    .media-controls-row.search-active .media-sort-selector {
      display: none !important;
    }
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
    border: none;
    border-radius: var(--radius-full, 9999px);
    background: rgba(22, 22, 28, 0.82);
    backdrop-filter: blur(16px) saturate(180%);
    -webkit-backdrop-filter: blur(16px) saturate(180%);
    color: rgba(255, 255, 255, 0.88);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    box-shadow: 0 4px 16px -2px rgba(0, 0, 0, 0.5);
    transition: background var(--duration-fast, 150ms) var(--ease-expo, ease-out),
                color var(--duration-fast, 150ms) var(--ease-expo, ease-out),
                transform var(--duration-fast, 150ms) var(--ease-expo, ease-out),
                box-shadow var(--duration-fast, 150ms) var(--ease-expo, ease-out);
  }

  .media-viewer-open-btn:hover {
    background: rgba(38, 38, 46, 0.94);
    color: #ffffff;
    transform: scale(1.08);
    box-shadow: 0 6px 20px -2px rgba(0, 0, 0, 0.65);
  }

  .media-viewer-open-btn:active {
    transform: scale(0.95);
  }

  .media-viewer-open-btn :global(svg) {
    width: 19px;
    height: 19px;
    transition: transform var(--duration-fast, 150ms) var(--ease-expo, ease-out);
  }

  .media-viewer-open-btn:hover :global(svg) {
    transform: scale(1.05);
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
    border-radius: var(--radius-md, 10px);
    background: transparent;
    align-self: center;
  }

  .audio-player-wrapper {
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
  }

  .audio-player-wrapper audio {
    width: 100%;
    max-width: 320px;
  }

  .file-placeholder :global(.placeholder-icon.video-play-accent) {
    color: var(--accent-primary, #6366f1);
    opacity: 0.85;
  }

  .file-placeholder:hover :global(.placeholder-icon.video-play-accent) {
    color: var(--accent-primary, #6366f1);
    opacity: 1;
    transform: scale(1.12);
  }

  .video-thumbnail-surface {
    position: relative;
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: 0;
    padding: 0;
    cursor: pointer;
    border-radius: var(--radius-md, 10px);
    overflow: hidden;
  }

  .video-play-overlay {
    position: absolute;
    inset: 0;
    z-index: 10;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    pointer-events: none;
  }

  .play-btn-circle {
    width: 54px;
    height: 54px;
    border-radius: var(--radius-full, 9999px);
    background: rgba(0, 0, 0, 0.32);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    border: none;
    outline: none;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #ffffff;
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.25);
  }

  :global(.placeholder-fix-btn) {
    height: 32px !important;
    padding: 0 12px !important;
    font-size: 12.5px !important;
    border-radius: var(--radius-full, 9999px) !important;
    gap: 6px !important;
    margin-top: 4px;
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

  .media-gallery-wrapper,
  .post-content,
  .comments-wrapper {
    scroll-margin-top: 92px;
  }

  :global(.page-shell.mobile) .media-gallery-wrapper,
  :global(.page-shell.mobile) .post-content,
  :global(.page-shell.mobile) .comments-wrapper {
    scroll-margin-top: calc(56px + var(--mobile-status-bar-height, 0px) + 12px);
  }

  .gallery-collapse-action,
  .content-collapse-action,
  .comments-collapse-action {
    position: sticky;
    bottom: 24px;
    z-index: 25;
    display: flex;
    justify-content: center;
    margin-top: 14px;
    pointer-events: none;
  }

  .gallery-collapse-action :global(.btn),
  .content-collapse-action :global(.btn),
  .comments-collapse-action :global(.btn) {
    pointer-events: auto;
  }

  :global(.page-shell.mobile) .gallery-collapse-action,
  :global(.page-shell.mobile) .content-collapse-action,
  :global(.page-shell.mobile) .comments-collapse-action {
    bottom: calc(var(--mobile-nav-height, 64px) + max(16px, env(safe-area-inset-bottom, 16px)));
  }

  @media (max-width: 640px) {
    .gallery-collapse-action,
    .content-collapse-action,
    .comments-collapse-action {
      bottom: calc(var(--mobile-nav-height, 64px) + max(16px, env(safe-area-inset-bottom, 16px)));
    }
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

  :global(.sticky-header-bar) :global(.sticky-back-btn) {
    flex: 0 0 calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    height: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    min-width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    border-radius: 50% !important;
    padding: 0 !important;
    display: flex !important;
    align-items: center !important;
    justify-content: center !important;
    flex-shrink: 0 !important;
  }

  :global(.sticky-header-bar) :global(.sticky-back-btn svg) {
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

  :global(.sticky-header-bar:not(.is-mobile)) .sticky-post-title {
    font-size: 17px !important;
    font-weight: 600 !important;
  }

  :global(.sticky-header-bar) :global(.sticky-creator-btn) {
    height: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    padding: 0 14px 0 8px !important;
    border-radius: var(--radius-full) !important;
    display: flex !important;
    align-items: center !important;
    gap: 8px !important;
    flex-shrink: 1 !important;
    min-width: 0 !important;
  }

  .sticky-creator-name {
    font-family: var(--font-sans);
    font-size: 13.5px;
    font-weight: 500;
    max-width: 140px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  :global(.sticky-header-bar.is-mobile) :global(.sticky-creator-btn) {
    height: 38px !important;
    padding: 0 10px 0 6px !important;
    gap: 6px !important;
    max-width: min(130px, 32vw) !important;
  }

  :global(.sticky-header-bar.is-mobile) :global(.sticky-creator-btn) .post-creator-avatar {
    width: 24px !important;
    height: 24px !important;
  }

  :global(.sticky-header-bar.is-mobile) .sticky-creator-name {
    font-size: 12.5px;
    max-width: clamp(50px, 18vw, 85px);
  }

  :global(.sticky-header-bar) :global(.sticky-fav-btn) {
    width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    height: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    min-width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    padding: 0 !important;
    border-radius: 50% !important;
    flex-shrink: 0 !important;
    display: flex !important;
    align-items: center !important;
    justify-content: center !important;
  }

  .sticky-stash-select {
    display: flex;
    align-items: center;
  }

  :global(.sticky-header-bar.is-mobile) .sticky-stash-select {
    display: flex;
    width: calc(var(--control-height, 46px) * var(--ui-scale, 1));
    height: calc(var(--control-height, 46px) * var(--ui-scale, 1));
    flex-shrink: 0;
  }

  :global(.sticky-header-bar.is-mobile) .sticky-stash-select :global(.select-trigger) {
    width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    height: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    min-width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    max-width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
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

  :global(.sticky-header-bar:not(.is-mobile)) .sticky-stash-select {
    display: block;
    min-width: 170px;
    max-width: 240px;
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
