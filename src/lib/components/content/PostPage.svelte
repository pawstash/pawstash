<script module lang="ts">
  const globalProbedSizes = new Map<string, number>();
</script>

{#snippet sourceBadge(file: Attachment)}
  {@const name = cloudProviderName(file.cloud_provider) ?? String(file.cloud_provider)}
  {#if mirrorsOf(file).length > 0}
    <button
      type="button"
      class="cloud-source-badge is-switch"
      onclick={(event) => contextMenuState.show(event, i18n.t('post.source'), sources.menu(file))}
      use:tooltip={i18n.t('post.source')}
      aria-label={`${i18n.t('post.source')}: ${name}`}
    >
      {name}<IconChevronDown />
    </button>
  {:else}
    <span class="cloud-source-badge">{name}</span>
  {/if}
{/snippet}

<script lang="ts">
  import { contextMenuState } from '$lib/state/contextMenuState.svelte';
  import { MirrorSources, mirrorsOf } from '$lib/state/mirrorSources.svelte';
  import { PostCloudLinks } from '$lib/state/postCloudLinks.svelte';
  import { edgeFade } from '$lib/actions/edgeFade';
  import { onMount, untrack, getContext, tick } from 'svelte';
  import EmptyState from '$lib/components/ui/EmptyState.svelte';
  import PostDownloadPanel from './PostDownloadPanel.svelte';
  import ArchivePreview from './ArchivePreview.svelte';
  import { cloudNodePath } from '$lib/utils/cloudNodes';
  import { SCROLLABLE_CONTEXT, type ScrollableContext } from '$lib/actions/scrollable';
  import { contentState, postCacheKey, creatorCacheKey, normalizePostId, type CachedPost } from '$lib/state/contentState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { configState } from '$lib/state/configState.svelte';
  import { libraryState } from '$lib/state/libraryState.svelte';
  import { downloadState } from '$lib/state/downloadState.svelte';
  import { favoriteActions } from '$lib/state/favoriteActions.svelte';
  import { accountState } from '$lib/state/accountState.svelte';
  import { themeState } from '$lib/theme/themeState.svelte';
  import { creatorsState } from '$lib/state/creatorsState.svelte';
  import { apiFetchCreatorProfile, apiGetCreatorArtworkPath, openExternalUrl, apiProbeDownloadSizes, apiShowInFolder, apiStartDownload, openDownloadedFile, apiResolvePostUrl } from '$lib/utils/ipc';
  import type { Attachment, Post } from '$lib/types/content';
  import { isFailedDownload, type DownloadItem } from '$lib/types/download';
  import type { LibraryCollection } from '$lib/types/library';
  import { i18n } from '$lib/i18n';
  import { formatDate, formatBytes, getPostTags, cleanPostTitle } from '$lib/utils/formatters';
  import { isVideoUrl, attachmentMediaUrl, attachmentThumbnailUrl, isSameAttachment, isCloudMirror, cloudProviderName, isListableArchive, isAttachmentVideo, isAttachmentAudio, isAttachmentImage, getPlatformPostUrl, formatProviderName, postThumbnailUrl, creatorAvatarSrc, resolveLocalMediaUrl, getFileExtension, isH265Video, diagnoseVideoFailure, diagnoseVideoFailureAsync, cleanMediaPath, getAttachmentDeclaredSize, isOpenableArchive, thumbnailProxySrc, type MediaFailureState, formatServiceName, videoPoster } from '$lib/utils/media';
  import { seedFromImageUrl, seedFromThumbHash } from '$lib/theme/seedColor';
  import { serverPortState } from '$lib/state/serverPort.svelte';
  import { extractDirectMediaLinks, sanitizeEmbedHtml } from './RichContent.svelte';
  import { apiOpenArchive } from '$lib/utils/ipc';
  import { cloudLinkContext, cloudLockKind, postSecretText } from '$lib/utils/cloudLink';
  import { logger, logMediaError } from '$lib/utils/logger';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { getVideoThumbnail } from '$lib/utils/mediaThumbnail';
  import { thumbnailKey } from '$lib/utils/cacheKey';
  import { extraField } from '$lib/utils/fields';
  import { handleGlobalPanicKey, panicCapture } from '$lib/utils/panic';
  import PageShell from '$lib/components/layout/PageShell.svelte';
  import HeroBackdrop from '$lib/components/ui/HeroBackdrop.svelte';
  import { tooltip, ripple } from '$lib/motion';
  import Button from '$lib/components/ui/Button.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import HeaderActions from '$lib/components/layout/HeaderActions.svelte';
  import TagList from '$lib/components/ui/TagList.svelte';
  import ChoiceGroup, { type ChoiceOption } from '$lib/components/ui/ChoiceGroup.svelte';
  import ServiceIcon from './ServiceIcon.svelte';
  import RichContent from './RichContent.svelte';
  import PostComments from './PostComments.svelte';
  import { collapseSection } from '$lib/utils/collapseSection';

  const scrollContext = getContext<ScrollableContext | undefined>(SCROLLABLE_CONTEXT);
  import PostPoll from './PostPoll.svelte';
  import MediaViewer, { type MediaViewerItem, type MediaViewerKind } from './MediaViewer.svelte';
  import MediaSurface from '$lib/components/ui/MediaSurface.svelte';
  import { mediaRegistry } from '$lib/media/mediaRegistry';
  import IconFullscreen from '~icons/fluent/full-screen-maximize-24-regular';
  import IconCloud from '~icons/fluent/cloud-24-regular';
  import IconSearch from '~icons/fluent/search-24-regular';
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
  import IconEye from '~icons/fluent/eye-24-regular';
  import IconSource from '~icons/fluent/database-24-regular';
  import IconLink from '~icons/fluent/link-24-regular';
  import IconLinkSquare from '~icons/fluent/link-square-24-regular';
  import IconTextCopy from '~icons/fluent/document-copy-24-regular';
  import IconPersonOpen from '~icons/fluent/person-arrow-right-24-regular';
  import IconHistory from '~icons/fluent/history-24-regular';
  import IconFolderOpen from '~icons/fluent/folder-open-24-regular';
  import IconNumberSymbol from '~icons/fluent/number-symbol-24-regular';
  import ChipSelect from '$lib/components/layout/ChipSelect.svelte';
  import type { MenuItem } from '$lib/components/layout/OverflowMenu.svelte';
  import IconChevronDown from '~icons/fluent/chevron-down-24-regular';
  import IconChevronUp from '~icons/fluent/chevron-up-24-regular';
  import IconVideoOff from '~icons/fluent/video-off-24-regular';
  import IconWarning from '~icons/fluent/warning-24-regular';
  import IconGrid from '~icons/fluent/grid-24-regular';
  import IconVideo from '~icons/fluent/video-24-regular';
  import IconImage from '~icons/fluent/image-24-regular';
  import IconFolder from '~icons/fluent/folder-24-regular';
  import IconFolderZip from '~icons/fluent/folder-zip-24-regular';
  import IconGlobe from '~icons/fluent/globe-24-regular';
  import IconOpen from '~icons/fluent/open-24-regular';
  import IconSparkle from '~icons/fluent/sparkle-24-regular';
  import SortControl from '$lib/components/layout/SortControl.svelte';
  import { defaultSortOption, sortOption } from '$lib/utils/sort';
  import CloudFolderModal from '$lib/components/content/CloudFolderModal.svelte';
  import CloudLinkCard from '$lib/components/content/CloudLinkCard.svelte';
  import CodecGuideModal from '$lib/components/content/CodecGuideModal.svelte';
  import type { ArchiveAttachment, ArchiveOpenResult, CloudFolderResult, CloudNode } from '$lib/types/cloud';
  import { providerState } from '$lib/state/providerState.svelte';
  import { notify } from '$lib/utils/toast';
  import { copyText } from '$lib/utils/clipboard';
  import { notifyAddedToStash, notifyRemovedFromStash } from '$lib/utils/stashToast';

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
      { value: 'auto', label: i18n.t('post.source_auto') },
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
    const raw = extraField(rawPost, 'available_providers');
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
  let embedHtml = $derived(postEmbed?.html ? sanitizeEmbedHtml(postEmbed.html) : '');
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
    extractAdjacentTitle(olderPost, i18n.t('post.previous'))
  );
  let rightPostTitle = $derived(
    extractAdjacentTitle(newerPost, i18n.t('post.next'))
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
      { value: 'latest', label: i18n.t('post.revision_current') },
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

  let postTitleEl = $state<HTMLElement | null>(null);

  let postMenu = $derived.by<MenuItem[]>(() => {
    if (!post) return [];
    const openTargets =
      activeProviderId === 'auto' && providersWithPost.length > 1
        ? providersWithPost.map((p) => ({ id: p.id as string | undefined, name: formatProviderName(p.name || p.id) }))
        : [{ id: activeProviderId && activeProviderId !== 'auto' ? activeProviderId : undefined, name: currentProviderName }];
    return [
      ...(downloadableMedia.length > 0
        ? [
            allMediaDownloaded
              ? { group: 'get', label: i18n.t('downloads.open_post_folder'), icon: IconFolderOpen, onclick: () => void openPostFolder() }
              : {
                  group: 'get',
                  label: i18n.t(downloadingAll ? 'post.downloading_all' : 'post.download_all'),
                  detail: totalMediaBytes > 0 ? formatBytes(totalMediaBytes) : undefined,
                  icon: IconDownload,
                  disabled: downloadingAll || allMediaQueued,
                  onclick: () => void downloadAllMedia()
                }
          ]
        : []),
      ...(post.file || media.length > 0
        ? [{ group: 'get', label: i18n.t('post.view_preview'), icon: IconEye, onclick: openPreviewViewer }]
        : []),
      {
        group: 'open',
        label: i18n.t('feed.open_creator'),
        detail: creatorName || undefined,
        icon: IconPersonOpen,
        onclick: () => navigationState.openCreator(service, creatorId)
      },
      { group: 'open', label: i18n.t('post.open_original_post'), detail: formatServiceName(service), icon: IconOpen, onclick: openOriginalPost },
      ...openTargets.map((target) => ({
        group: 'open',
        label: i18n.t('post.open_in_provider'),
        detail: target.name,
        icon: IconGlobe,
        onclick: () => openInProvider(target.id)
      })),
      {
        group: 'copy',
        label: i18n.t('post.copy_link'),
        detail: currentProviderName,
        icon: IconLink,
        onclick: copyPostLink
      },
      ...(getPlatformPostUrl(service, creatorId, postId, creatorProfile?.public_id)
        ? [{ group: 'copy', label: i18n.t('post.copy_link'), detail: formatServiceName(service), icon: IconLinkSquare, onclick: copyOriginalLink }]
        : []),
      ...(postPlainText ? [{ group: 'copy', label: i18n.t('post.copy_text'), icon: IconTextCopy, onclick: copyPostText }] : []),
      { group: 'copy', label: i18n.t('post.copy_id'), detail: `#${postId}`, detailMono: true, icon: IconNumberSymbol, onclick: copyPostId },
      { group: 'page', label: i18n.t('common.refresh'), icon: IconArrowClockwise, onclick: () => void refreshPost() }
    ];
  });

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
      void openExternalUrl(post.page_url);
      return;
    }
    apiResolvePostUrl(service, creatorId, postId, effectiveProvId)
      .then((url) => {
        if (url) void openExternalUrl(url);
      })
      .catch((err) => {
        logger.warn('Failed to resolve post URL', err);
        notify.error(i18n.t('common.open_link_failed'), err);
      });
  }

  function openOriginalPost() {
    const url = getPlatformPostUrl(service, creatorId, postId, creatorProfile?.public_id);
    if (url) void openExternalUrl(url);
  }

  function copyPostLink() {
    const providerId = activeProviderId && activeProviderId !== 'auto' ? activeProviderId : undefined;
    if (post?.page_url && (!providerId || providerId === post.provider_id)) {
      copyText(post.page_url, i18n.t('post.link_copied'));
      return;
    }
    apiResolvePostUrl(service, creatorId, postId, providerId)
      .then((url) => {
        if (url) copyText(url, i18n.t('post.link_copied'));
        else notify.error(i18n.t('common.copy_failed'));
      })
      .catch((error) => notify.error(i18n.t('common.copy_failed'), error));
  }

  function copyOriginalLink() {
    const url = getPlatformPostUrl(service, creatorId, postId, creatorProfile?.public_id);
    if (url) copyText(url, i18n.t('post.link_copied'));
  }

  let postPlainText = $derived.by(() => {
    if (!richContent || typeof DOMParser === 'undefined') return '';
    const html = richContent.replace(/<br\s*\/?>/gi, '\n').replace(/<\/(p|div|li|h[1-6])>/gi, '$&\n');
    const text = new DOMParser().parseFromString(html, 'text/html').body.textContent ?? '';
    return text.replace(/\n{3,}/g, '\n\n').trim();
  });

  function copyPostText() {
    copyText(postPlainText, i18n.t('post.text_copied'));
  }

  async function refreshPost() {
    const providerId = activeProviderId && activeProviderId !== 'auto' ? activeProviderId : undefined;
    try {
      await contentState.loadPost(service, creatorId, postId, true, providerId);
      commentsSection?.reload();
    } catch (error) {
      notify.error(i18n.t('feed.refresh_failed'), error);
    }
  }

  function copyPostId() {
    if (postId) copyText(postId);
  }

  const cloud = new PostCloudLinks();
  let activeCloudModalFolder = $state<CloudFolderResult | null>(null);
  let activeCloudModalInitialFolderId = $state<string | null>(null);
  let isCloudModalOpen = $state(false);

  let activeArchive = $state<ArchiveOpenResult | null>(null);
  let activeArchiveFile = $state<ArchiveAttachment | null>(null);
  let openingArchivePath = $state<string | null>(null);

  function openCloudFolderModal(folderResult?: CloudFolderResult, folderId?: string | null) {
    if (!folderResult) return;
    activeArchive = null;
    activeArchiveFile = null;
    activeCloudModalFolder = folderResult;
    activeCloudModalInitialFolderId = folderId || null;
    isCloudModalOpen = true;
  }

  function archiveRequest(file: Attachment, target: Post) {
    const job = attachmentDownload(file);
    return {
      url: resolveAttachmentDownloadUrl(file) || null,
      local_path: job?.status === 'completed' && job.final_path ? job.final_path : null,
      name: file.name ?? '',
      service: target.service,
      creator_id: target.user,
      post_id: target.id,
      post_text: postSecretText(target) || null
    };
  }

  const archivePreviews = new Map<string, Promise<ArchiveOpenResult>>();

  function previewArchive(file: Attachment): Promise<ArchiveOpenResult> {
    if (!post) return Promise.reject(new Error('No post'));
    const request = archiveRequest(file, post);
    const key = request.local_path || request.url || file.path || request.name;
    let pending = archivePreviews.get(key);
    if (!pending) {
      pending = apiOpenArchive(request);
      archivePreviews.set(key, pending);
      pending.catch(() => archivePreviews.delete(key));
    }
    return pending;
  }

  async function openArchive(file: Attachment) {
    if (!post || !file.path || !file.name || openingArchivePath) return;
    const name = file.name;
    const url = resolveAttachmentDownloadUrl(file);
    const requestedFor = postKey;
    openingArchivePath = file.path;
    try {
      const result = await apiOpenArchive(archiveRequest(file, post));
      if (postKey !== requestedFor) return;
      activeArchive = result;
      activeArchiveFile = { name, url, media_id: file.path, size: getEffectiveFileSize(file) || undefined };
      activeCloudModalFolder = result.folder ?? null;
      activeCloudModalInitialFolderId = null;
      isCloudModalOpen = true;
    } catch (error) {
      notify.error(i18n.t('cloud.archive_open_failed'), error);
    } finally {
      openingArchivePath = null;
    }
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

    const byId = new Map(folderResult.nodes.map((node) => [node.id, node]));
    let started = 0;
    for (const node of targetNodes) {
      const port = serverPortState.port || 0;
      const raw = node.download_url || node.stream_url || '';
      const streamUrl = raw.startsWith('/cloud_stream/') && port > 0
        ? serverPortState.mediaUrl(raw)
        : raw;
      if (!streamUrl) continue;
      try {
        const path = cloudNodePath(byId, node);
        await apiStartDownload(targetPost, node.id, streamUrl, path.includes('/') ? path : `${folderResult.title}/${path}`);
        started++;
      } catch (err) {
        logger.error(`Failed to queue download for "${node.name}"`, err);
      }
    }
    notify.success(
      i18n.t('feed.download_started'),
      { description: i18n.t('downloads.files_queued', { count: started }), glyph: 'download' }
    );
  }

  async function revealLockedCloudLink(url: string) {
    await tick();
    document
      .querySelector(`[data-cloud-url="${CSS.escape(url)}"]`)
      ?.scrollIntoView({ behavior: 'smooth', block: 'center' });
  }

  async function handleOpenCloudFromText(url: string) {
    let existing = cloud.cached(url);
    if (!existing && cloud.isLocked(url)) {
      void revealLockedCloudLink(url);
      return;
    }
    if (!existing && configState.settings.cloud_scraping_enabled !== false) {
      const requestedFor = postKey;
      try {
        existing = await cloud.fetch(url, post ? cloudLinkContext(post) : undefined);
      } catch (err) {
        const kind = cloudLockKind(err);
        if (kind) {
          if (postKey === requestedFor) {
            cloud.markLocked(url, kind);
            void revealLockedCloudLink(url);
          }
          return;
        }
        logger.warn(`Failed to resolve cloud link from text: ${url}`, err);
      }
      if (postKey !== requestedFor) return;
    }
    if (existing) {
      openCloudFolderModal(existing, null);
    } else {
      void openExternalUrl(url);
    }
  }

  $effect(() => {
    const currentPostKey = postKey;
    const cloudUrls = configState.settings.cloud_scraping_enabled === false ? [] : post?.cloud_urls || [];
    untrack(() => cloud.sync(currentPostKey, cloudUrls, post ? cloudLinkContext(post) : undefined));
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
      if (cloud.resolved.length > 0) {
        for (const att of cloud.resolved) {
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
            const mirrorOf = items.findIndex((existing) => isCloudMirror(existing, att));
            if (mirrorOf >= 0) {
              items[mirrorOf] = { ...items[mirrorOf], mirrors: [...mirrorsOf(items[mirrorOf]), att] };
            } else if (!items.some((existing) => isSameAttachment(existing, att))) {
              items.push(att);
            }
          }
        }
        for (let i = 0; i < items.length; i++) {
          if (mirrorsOf(items[i]).length > 0) items[i] = sources.pick(items[i]);
        }
      }
      const directMedia = extractDirectMediaLinks(post.content || post.substring || '');
      const knownCloudLinks = new Set(
        (post.cloud_urls || []).map((url) => url.replace(/&amp;/g, '&').trim())
      );
      for (const d of directMedia) {
        const link = d.url.replace(/&amp;/g, '&').trim();
        if (knownCloudLinks.has(link)) continue;
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
  let mediaSortOptions = $derived([
    defaultSortOption('default'),
    sortOption('name_asc', 'name', 'asc'),
    sortOption('name_desc', 'name', 'desc'),
    sortOption('size_desc', 'size', 'desc'),
    sortOption('size_asc', 'size', 'asc')
  ]);
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
  let hevcAttempts = $state<Set<number>>(new Set());
  let videoFailures = $state<Record<number, MediaFailureState>>({});
  let videoAttempts = $state<Record<number, number>>({});
  let isCodecModalOpen = $state(false);

  function retryVideo(index: number) {
    const file = media[index];
    if (file) mediaRegistry.reset(mediaElementKey(file));
    videoAttempts = { ...videoAttempts, [index]: (videoAttempts[index] ?? 0) + 1 };
    const next = { ...videoFailures };
    delete next[index];
    videoFailures = next;
    activeVideoIndexes = new Set(activeVideoIndexes).add(index);
  }

  function clearVideoFailure(index: number) {
    if (!videoFailures[index]) return;
    const next = { ...videoFailures };
    delete next[index];
    videoFailures = next;
  }

  function tryHevc(index: number) {
    hevcAttempts = new Set(hevcAttempts).add(index);
    clearVideoFailure(index);
    activeVideoIndexes = new Set(activeVideoIndexes).add(index);
  }

  function checkVideoElement(video: HTMLVideoElement, file: Attachment, index: number) {
    if (video.error) {
      video.pause();
      void handleVideoElementError(video, file, index);
      return;
    }
    if (video.videoWidth > 0 && video.videoHeight > 0) {
      const known = videoRatios[index];
      if (!known || known.width !== video.videoWidth || known.height !== video.videoHeight) {
        videoRatios = { ...videoRatios, [index]: { width: video.videoWidth, height: video.videoHeight } };
      }
      clearVideoFailure(index);
      return;
    }
    if (video.readyState < HTMLMediaElement.HAVE_CURRENT_DATA || !(video.duration > 0)) return;
    video.pause();
    video.muted = true;
    logger.warn(`Video "${file.name}" has audio but unsupported video codec (videoWidth=0)`);
    videoFailures = { ...videoFailures, [index]: { preset: 'unsupported_codec', format: 'H.265 / HEVC' } };
  }

  function handleVideoLoadedMetadata(e: Event, file?: Attachment | null, index?: number) {
    if (typeof index !== 'number' || !file) return;
    checkVideoElement(e.currentTarget as HTMLVideoElement, file, index);
  }

  async function handleVideoError(e: Event, file?: Attachment | null, index?: number) {
    if (typeof index !== 'number' || !file) return;
    await handleVideoElementError(e.currentTarget as HTMLVideoElement, file, index);
  }

  async function handleVideoElementError(video: HTMLVideoElement, file: Attachment, index: number) {
    if (!video.error) {
      logger.debug(`Media element reported an error without a media error for "${file.name}"`);
      return;
    }
    video.pause();
    video.muted = true;
    logMediaError('video', video.src, file.name, video.error);

    const job = attachmentDownload(file);
    const isDownloaded = Boolean(job?.final_path && job.status === 'completed');

    if (!isDownloaded) {
      const remote = remoteFileUrl(file);
      if (remote && video.src !== remote && !video.src.includes('cloud_stream')) {
        logger.warn(`Local video playback failed, falling back to remote stream for "${file.name}": ${remote}`);
        video.src = remote;
        mediaRegistry.overrideElementSource(video, remote);
        return;
      }
    }

    if (!isDownloaded && sources.fail(file)) {
      logger.warn(`Playback failed on ${file.cloud_provider} for "${file.name}", switching to a mirror`);
      return;
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

    if (videoFailures[index]) {
      const next = new Set(activeVideoIndexes);
      next.delete(index);
      activeVideoIndexes = next;
    }
  }

  function openFileExternally(item?: DownloadItem | null) {
    if (!item?.final_path) return;
    void openDownloadedFile(item.final_path);
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

  let galleryScrollBeforeExpand = $state<number | null>(null);
  let commentsSection = $state<{ reload(): void } | undefined>();
  let contentScrollBeforeExpand = $state<number | null>(null);


  $effect(() => {
    if (activeMediaTab) {
      galleryExpanded = false;
    }
  });

  let isEmbedResolvedToCloud = $derived(Boolean(
    postEmbed?.url && cloud.resolved.length > 0
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
  let showMediaTabs = $derived((media.length > 1 || (hasEmbed && media.length > 0)) && activeCategoriesCount > 1);

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
        label: i18n.t('post.tab_cloud'),
        icon: IconCloud,
        count: mediaCounts.cloud
      });
    }
    if (mediaCounts.downloaded > 0) {
      opts.push({
        value: 'downloaded',
        label: i18n.t('post.tab_downloaded'),
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

  function mediaKeyFor(file?: { path?: string; name?: string } | null): string {
    if (!file) return '';
    const job = attachmentDownload(file);
    return String(job?.media_id || job?.id || job?.filename || file.path || file.name || '');
  }

  function mediaElementKey(file?: Attachment | null): string {
    if (!file) return '';
    return mediaKeyFor(file) || (file.path ? fileUrl(file) : '');
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
    html: embedHtml || undefined
  } as Attachment & { html?: string }) : null);

  let viewerItems = $derived.by((): MediaViewerItem[] => viewerFiles.map((file, itemIndex) => {
    const isEmbed = file === embedAttachment || Boolean((file as any)?.html);
    const url = file.path ? fileUrl(file) : '';
    const job = attachmentDownload(file);
    const mediaIndex = media.findIndex((m) => isSameAttachment(m, file));
    const kind = isEmbed ? 'video' : mediaViewerKind(file, url);
    const measured = kind === 'video' && mediaIndex >= 0 ? videoRatios[mediaIndex] : undefined;
    const width = typeof file.width === 'number' && file.width > 0 ? file.width : measured?.width;
    const height = typeof file.height === 'number' && file.height > 0 ? file.height : measured?.height;
    const isUnarchived = isFileUnarchived(file, Boolean(job?.final_path && job.status === 'completed'));
    const cachedVideoThumb = mediaIndex >= 0 ? videoThumbnails[mediaIndex] : undefined;
    const key = mediaElementKey(file) || `vid_${itemIndex}`;
    return {
      id: key,
      thumbnailKey: thumbnailKey(key, post),
      url,
      poster: cachedVideoThumb || attachmentThumbnailUrl(file, service, post),
      name: file.name || i18n.t('post.file'),
      kind,
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
  let viewerOriginRect = $state<DOMRect | null>(null);

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
    const targetKey = mediaElementKey(file);
    const shared = targetKey ? mediaRegistry.element(targetKey) : undefined;

    if (originVideoEl && originVideoEl !== shared) originVideoEl.pause();

    document.querySelectorAll<HTMLMediaElement>('video, audio').forEach((el) => {
      if (el !== shared && !el.paused) el.pause();
    });

    viewerInitialTime = shared ? 0 : (originVideoEl?.currentTime || 0);

    const originElement = (document.activeElement as HTMLElement | null)?.closest('.video-frame, .media-open-surface');
    viewerOriginRect = originElement ? originElement.getBoundingClientRect() : null;

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

    for (const [_, res] of cloud.folders) {
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
    cloud.version;
    const isPostFullyLoaded = entry.loaded && Boolean(post?.detail_fetched);
    const isPostLoading = entry.loading || (!isPostFullyLoaded && !entry.loaded && !entry.error);
    const isCloudResolving = cloud.resolving;
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

  function handleCloseViewer() {
    viewerIndex = null;
    viewerInitialTime = 0;
    viewerOriginRect = null;
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
  const stashOptions = $derived(libraryState.stashOptions);
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
      return i18n.t('library.save');
    }
    if (customStashNames.length === 1) {
      return customStashNames[0];
    }
    if (customStashNames.length > 1) {
      return i18n.t('library.in_stashes_count', { count: customStashes.length });
    }
    return i18n.t('library.saved');
  });

  let isFavorited = $derived(accountState.isPostFavorite(service, creatorId, postId));
  let creatorAvatar = $state('');
  let creatorAvatarFailed = $state(false);
  let deletingDownloadId = $state<string | null>(null);
  let downloadingAll = $state(false);
  let activeVideoIndexes = $state<Set<number>>(new Set());
  const sources = new MirrorSources(() => postKey);

  async function retryOnMirror(failedJobId: string, file: Attachment) {
    if (!post || !sources.fail(file)) return;
    const next = downloadableMedia.find((item) => sources.key(item) === sources.key(file));
    if (!next?.path) return;
    try {
      await downloadState.remove(failedJobId);
      await downloadState.start(post, next.path, resolveAttachmentDownloadUrl(next), next.name || postId);
    } catch (error) {
      notify.error(i18n.t('feed.download_failed'), error);
    }
  }

  $effect(() => {
    for (const file of downloadableMedia) {
      if (mirrorsOf(file).length === 0) continue;
      const job = downloadState.downloads.find(
        (item) => item.service === service && item.creator_id === creatorId && item.post_id === postId && item.media_id === file.path
      );
      if (job && isFailedDownload(job)) untrack(() => void retryOnMirror(job.id, file));
    }
  });
  let videoFrames = $state<Record<number, { width: number; height: number }>>({});
  let videoRatios = $state<Record<number, { width: number; height: number }>>({});

  function startVideo(index: number, surface: HTMLElement) {
    const { width, height } = surface.getBoundingClientRect();
    videoFrames[index] = { width, height };
    activeVideoIndexes = new Set(activeVideoIndexes).add(index);
    const file = media[index];
    if (file) playMediaAfterMount(file);
  }

  function playMediaAfterMount(file: Attachment) {
    const key = mediaElementKey(file);
    if (!key) return;
    void tick().then(() => mediaRegistry.element(key)?.play().catch(() => {}));
  }
  let activeAudioIndexes = $state<Set<number>>(new Set());
  let videoThumbnails = $state<Record<number, string>>({});

  $effect(() => {
    const currentMedia = media;
    downloadState.downloads;
    serverPortState.port;

    for (let i = 0; i < currentMedia.length; i++) {
      const file = currentMedia[i];
      if (!file) continue;
      const url = fileUrl(file);
      if (!isAttachmentVideo(file, url)) continue;
      if (videoThumbnails[i]) continue;

      const localJob = attachmentDownload(file);
      if (url) {
        const key = thumbnailKey(localJob?.media_id || localJob?.id || localJob?.filename || file.path || file.name || `vid_${i}`, post);
        getVideoThumbnail(key, url).then((thumb) => {
          if (thumb) {
            videoThumbnails = { ...videoThumbnails, [i]: thumb };
          }
        });
      }
    }
  });
  let downloadableMedia = $derived(media.filter((file) => file.is_cloud_folder !== true));

  let totalMediaBytes = $derived.by(() => {
    if (downloadableMedia.length === 0) return 0;

    let total = 0;
    for (const file of downloadableMedia) {
      const size = getEffectiveFileSize(file);
      if (size <= 0) return 0;
      total += size;
    }
    return total;
  });

  let creatorName = $state('');

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
      return attachmentThumbnailUrl(post.file, service, post);
    }
    return '';
  });

  function proxiedForSeed(url: string): string {
    const port = serverPortState.port || 0;
    if (port > 0 && (url.startsWith('http://') || url.startsWith('https://')) && !url.includes('127.0.0.1')) {
      return serverPortState.mediaUrl(`/cloud_stream/proxy?url=${encodeURIComponent(url)}`);
    }
    return url;
  }

  $effect(() => {
    if (!post || !configState.settings.dynamic_accent) return;

    let cancelled = false;
    const cachedAccent = contentState.getPostAccent(service, creatorId, postId);
    const postThumbhash = (post as any)?.preview_thumbhash || (post.extra as any)?.preview_thumbhash || (post.file as any)?.extra?.thumbhash;
    const thumbColor = cachedAccent || seedFromThumbHash(postThumbhash);

    if (thumbColor) {
      themeState.setOverrideAccent(thumbColor);
    }

    if (!cachedAccent && heroImageUrl) {
      void seedFromImageUrl(proxiedForSeed(heroImageUrl)).then((color) => {
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
      await accountState.fetchFavorites('post');
    } catch (error) {
      logger.error(`Failed to check post favorite status for ${service}:${postId}`, error);
    }
  }

  function toggleFavorite() {
    if (post) void favoriteActions.togglePost(post);
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
      notify.success(i18n.t('post.download_deleted'), { description: filename, glyph: 'deleted' });
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
      notify.success(i18n.t('feed.download_started'), { description: targetName, glyph: 'download' });
    } catch (error) {
      notify.error(i18n.t('feed.download_failed'), error);
    }
  }

  async function openFileInFolder(item: DownloadItem) {
    try {
      await apiShowInFolder(item.final_path);
    } catch (error) {
      notify.error(i18n.t('downloads.show_in_folder_failed'), error);
    }
  }

  async function openPostFolder() {
    const completed = downloadableMedia.map(attachmentDownload).find((d) => d && d.status === 'completed');
    if (completed?.final_path) {
      const p = completed.final_path;
      const lastSlash = Math.max(p.lastIndexOf('/'), p.lastIndexOf('\\'));
      const folder = lastSlash > 0 ? p.slice(0, lastSlash) : p;
      try {
        await apiShowInFolder(folder);
      } catch (error) {
        notify.error(i18n.t('downloads.open_folder_failed'), error);
      }
    }
  }

  let allMediaDownloaded = $derived(
    downloadableMedia.length > 0 && downloadableMedia.every((file) => {
      const job = attachmentDownload(file);
      return job?.status === 'completed';
    })
  );

  let allMediaQueued = $derived(
    downloadableMedia.every((file) => {
      const job = attachmentDownload(file);
      return Boolean(job && !['failed', 'cancelled', 'missing'].includes(job.status));
    })
  );

  async function downloadAllMedia() {
    if (!post || downloadingAll) return;

    const pendingMedia = downloadableMedia.filter((file) => {
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

  async function handleStashToggle(collectionId: string) {
    if (!post || !collectionId) return;
    const isCurrentlyIn = postStashes.includes(collectionId);
    const collection = libraryState.collections.find((c) => c.id === collectionId);
    try {
      if (collection?.kind === 'inbox') {
        if (isCurrentlyIn) {
          await libraryState.remove(post);
          notify.success(i18n.t('library.removed'), { description: post.title || undefined, glyph: 'removed' });
        } else {
          await libraryState.save(post, collectionId);
          notify.success(i18n.t('library.saved'), post.title || undefined);
        }
      } else {
        if (isCurrentlyIn) {
          await libraryState.removeFromStash(collectionId, post);
          notifyRemovedFromStash(collectionId, [post]);
        } else {
          await libraryState.save(post, collectionId);
          notifyAddedToStash(collectionId, post.title || undefined);
        }
      }
    } catch (error) {
      notify.error(i18n.t('library.save_error'), error);
    }
  }

  async function handleCreateStash(name: string) {
    if (!post || !name.trim()) return;
    try {
      const newStash = await libraryState.createStash(name.trim());
      await libraryState.save(post, newStash.id);
      notifyAddedToStash(newStash.id, newStash.name);
    } catch (error) {
      notify.error(i18n.t('library.save_error'), error);
    }
  }

  $effect(() => {
    if (post) {
      void libraryState.loadPostStashes(post);
    }
  });


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
          <span><span class="state-word">{i18n.t('post.downloaded')} · </span>{formatBytes(Math.max(downloaded.total_bytes, downloaded.downloaded_bytes))}</span>
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
            <span class:state-word={hasProgress}>{verifying ? i18n.t('downloads.status_verifying') : queued ? i18n.t('downloads.status_queued') : resolving ? i18n.t('downloads.status_resolving') : i18n.t('post.downloading')}{hasProgress ? ' · ' : ''}</span>{hasProgress ? `${progress}%` : ''}
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
        <span class="attachment-button-state downloading-state text-[var(--status-warning)]">
          <IconPause />
          <span><span class:state-word={hasProgress}>{i18n.t('downloads.status_paused')}{hasProgress ? ' · ' : ''}</span>{hasProgress ? `${progress}%` : ''}</span>
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
        <span class="attachment-button-state text-[var(--status-error)]">
          <IconArrowClockwise />
          <span><span class:state-word={Boolean(declaredSize)}>{i18n.t('downloads.retry')}{declaredSize ? ' · ' : ''}</span>{declaredSize}</span>
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
        <span class="attachment-button-state text-[var(--text-secondary)]">
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
          <span><span class:state-word={Boolean(declaredSize)}>{i18n.t('post.download')}{declaredSize ? ' · ' : ''}</span>{declaredSize}</span>
        </span>
      </Button>
    {/if}
    {#if !isUnarchived && isOpenableArchive(file.name)}
      <Button
        variant="ghost"
        class="media-action-icon-btn"
        disabled={openingArchivePath !== null}
        onclick={() => void openArchive(file)}
        tooltip={i18n.t('cloud.archive_open')}
        aria-label={i18n.t('cloud.archive_open')}
      >
        {#if openingArchivePath === file.path}
          <IconLoading class="w-[18px] h-[18px]" />
        {:else}
          <IconFolderZip class="w-[18px] h-[18px]" />
        {/if}
      </Button>
    {/if}
  </div>
{/snippet}

{#snippet postBarCreator(condensed: boolean)}
  {#if post}
    <Button
      variant="ghost"
      class="post-bar-creator {condensed ? 'is-condensed' : ''}"
      onclick={() => navigationState.openCreator(service, creatorId)}
      title={`${i18n.t('feed.open_creator')}: ${creatorName}`}
      aria-label={`${i18n.t('feed.open_creator')}: ${creatorName}`}
    >
      {#if creatorAvatar && !creatorAvatarFailed}
        <span class="post-creator-avatar"><img src={creatorAvatar} alt="" onerror={() => (creatorAvatarFailed = true)} /></span>
      {:else}
        <ServiceIcon {service} />
      {/if}
      <span class="post-bar-creator-name" aria-hidden={condensed}>{creatorName}</span>
    </Button>
  {/if}
{/snippet}

{#snippet postBarActions()}
  {#if post}
    <Button
      variant="ghost"
      class="btn-icon"
      disabled={Boolean(post && favoriteActions.isPostPending(post))}
      onclick={toggleFavorite}
      title={i18n.t(isFavorited ? 'post.unfavorite' : 'post.favorite')}
      aria-label={i18n.t(isFavorited ? 'post.unfavorite' : 'post.favorite')}
      aria-pressed={isFavorited}
    >
      {#if isFavorited}<IconHeartFilled />{:else}<IconHeart />{/if}
    </Button>

    <Select
      options={stashOptions}
      selectedValues={postStashes}
      placeholder={libraryButtonLabel}
      onchange={handleStashToggle}
      createLabel={i18n.t('library.new_stash')}
      onCreate={handleCreateStash}
      variant="ghost"
      multi={true}
      closeOnChange={false}
      iconOnly={true}
      icon={saved || postStashes.length > 0 ? IconSaved : IconSave}
      disabled={saving}
      ariaLabel={libraryButtonLabel}
      align="right"
    />
  {/if}
{/snippet}

<PageShell
  back
  title={cleanPostTitle(post?.title) || i18n.t('feed.untitled')}
  titleTrigger={postTitleEl}
  menu={postMenu}
  scrollKey={navigationState.entryKey}
>
  {#snippet heading(condensed)}{@render postBarCreator(condensed)}{/snippet}
  {#snippet actions()}
    {@render postBarActions()}
  {/snippet}

  {#if heroImageUrl}
    <HeroBackdrop src={heroImageUrl} />
  {/if}

  <div class="post-content-wrapper">
    {#if post || (entry.loaded && !entry.loading) || (activeProviderId && activeProviderId !== 'auto') || candidateProviders.length > 0}
      <header class="detail-header">
        <div class="min-w-0 flex-1">
          {#if post}
            <h1 bind:this={postTitleEl}>{cleanPostTitle(post.title) || i18n.t('feed.untitled')}</h1>
          {:else if autoEntry?.post?.title}
            <h1>{cleanPostTitle(autoEntry.post.title)}</h1>
          {/if}
          {#if publishedDateStr}
            <div class="post-date post-dates-row flex items-center flex-wrap gap-2 mt-2 text-sm text-[var(--text-secondary)]">
              <div class="flex items-center gap-1.5 shrink-0">
                <span class="text-[var(--text-muted)]">{i18n.t('post.published_at')}:</span>
                <strong class="font-semibold text-[var(--text-primary)]">{publishedDateStr}</strong>
              </div>

              {#if showEdited}
                <span class="text-[var(--text-muted)]">·</span>
                <div class="flex items-center gap-1.5 shrink-0">
                  <span class="text-[var(--text-muted)]">{i18n.t('post.edited_at')}:</span>
                  <strong class="font-medium text-[var(--text-primary)]">{editedDateStr}</strong>
                </div>
              {/if}

              {#if showImported}
                <span class="text-[var(--text-muted)]">·</span>
                <div class="flex items-center gap-1.5 shrink-0">
                  <span class="text-[var(--text-muted)]">{i18n.t('post.imported_at')}:</span>
                  <strong class="font-medium text-[var(--text-primary)]">{addedDateStr}</strong>
                </div>
              {/if}
            </div>
          {/if}

          {#if candidateProviders.length > 1 || filteredRevisions.length > 0}
            <div class="post-meta-actions-row flex items-center flex-wrap gap-2 mt-2 -ml-[10px] min-h-[38px]">
              {#if candidateProviders.length > 1}
                <ChipSelect
                  options={providerSelectOptions}
                  value={activeProviderId ?? 'auto'}
                  onchange={onProviderChange}
                  icon={IconSource}
                  label={i18n.t('post.source')}
                />
              {/if}
              {#if filteredRevisions.length > 0}
                <ChipSelect
                  options={revisionSelectOptions}
                  value={selectedRevId !== null ? String(selectedRevId) : 'latest'}
                  onchange={onRevisionChange}
                  icon={IconHistory}
                  label={i18n.t('post.revision_current')}
                />
              {/if}
            </div>
          {/if}

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

      {#if entry.loading && !entry.loaded && media.length === 0 && !hasEmbed}
        <div class="media-gallery media-gallery-skeleton" role="status" aria-label={i18n.t('feed.loading')}>
          {#each [0, 1] as index (index)}
            <div class="media-skeleton">
              <div class="media-skeleton__picture"></div>
              <div class="media-skeleton__caption"></div>
            </div>
          {/each}
        </div>
      {/if}

      {#if cloud.locked.length > 0}
        <div class="locked-cloud-links">
          {#each cloud.locked as link (link.url)}
            <CloudLinkCard
              url={link.url}
              {post}
              lock={link.kind}
              onresolved={(res) => cloud.unlock(link.url, res)}
            />
          {/each}
        </div>
      {/if}

      {#if media.length > 0 || hasEmbed}
        <div class="media-section">
          {#if showMediaTabs || media.length >= 20 || mediaSearchQuery || media.length > 1}
            <div class="media-controls-row post-section-controls" class:search-active={mediaSearchOpen}>
              {#if showMediaTabs}
                <div class="post-tabs-scroll" use:edgeFade>
                  <ChoiceGroup
                    options={mediaTabOptions}
                    value={activeMediaTab}
                    onchange={(val) => activeMediaTab = val as MediaTab}
                    ariaLabel="Media categories"
                    align="left"
                    class="post-media-choice"
                    hasAddon={() => media.length > 1}
                  >
                    {#snippet addon()}
                      <SortControl options={mediaSortOptions} value={mediaSort} onchange={(val) => (mediaSort = val as typeof mediaSort)} />
                    {/snippet}
                  </ChoiceGroup>
                </div>
              {/if}

              <div class="media-controls-actions" class:search-active={mediaSearchOpen}>
                <HeaderActions
                  bind:searchOpen={mediaSearchOpen}
                  bind:searchQuery={mediaSearchQuery}
                  showSearchButton={media.length >= 20 || Boolean(mediaSearchQuery)}
                  searchPlaceholder={i18n.t('post.search_media')}
                  onsearchtoggle={(open) => {
                    if (!open) closeMediaSearch();
                  }}
                >
                  {#if media.length > 1 && !showMediaTabs}
                    <SortControl variant="standalone" options={mediaSortOptions} value={mediaSort} onchange={(val) => (mediaSort = val as typeof mediaSort)} />
                  {/if}
                </HeaderActions>
              </div>
            </div>
          {/if}

          {#if filteredMedia.length > 0 || (hasEmbed && isEmbedVisibleInTab && embedMatchesSearch)}
            <div class="media-gallery-wrapper section-scroll-anchor" bind:this={galleryWrapperEl}>
              <div class="media-gallery-container" class:is-collapsed={isGalleryOverflowing && !galleryExpanded}>
                <section class="media-gallery" bind:clientHeight={galleryHeight} aria-label={i18n.t('post.media')}>
                {#if postEmbed && !isEmbedResolvedToCloud && isEmbedVisibleInTab && embedMatchesSearch}
                  {#if isEmbedLinkedPost}
                    <div class="media-item is-embed-item is-linked-post-item">
                      <div class="media-header">
                        <span class="media-filename">{postEmbed.subject || postEmbed.description || (linkedPostId ? i18n.t('post.linked_post_title', { id: linkedPostId }) : post.title)}</span>
                        <span class="cloud-source-badge">{i18n.t('post.linked_post_badge')}</span>
                      </div>

                      <button
                        class="file-placeholder media-open-surface"
                        type="button"
                        onclick={() => {
                          if (linkedPostId) {
                            navigationState.openPost(service, creatorId, linkedPostId);
                          } else if (postEmbed?.url) {
                            void openExternalUrl(postEmbed.url);
                          }
                        }}
                        aria-label={postEmbed.subject || (linkedPostId ? `Post #${linkedPostId}` : 'Linked Post')}
                      >
                        <IconDocument class="placeholder-icon" />
                        <p class="placeholder-text font-medium">{postEmbed.subject || postEmbed.description || (linkedPostId ? `Open Post #${linkedPostId}` : i18n.t('post.linked_post'))}</p>
                      </button>

                      <div class="media-download-group">
                        <Button
                          variant="ghost"
                          onclick={() => {
                            if (linkedPostId) {
                              navigationState.openPost(service, creatorId, linkedPostId);
                            } else if (postEmbed?.url) {
                              void openExternalUrl(postEmbed.url);
                            }
                          }}
                          class="media-download-btn is-embed-btn"
                          title={i18n.t('post.open_linked_post')}
                        >
                          <span class="attachment-button-state">
                            <IconDocument class="w-[16px] h-[16px]" />
                            <span>{i18n.t('post.open_linked_post')}</span>
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

                      {#if embedHtml}
                        <div class="media-embed-player">
                          {@html embedHtml}
                        </div>
                      {:else if postEmbed.url && isVideoUrl(postEmbed.url)}
                        <!-- svelte-ignore a11y_media_has_caption -->
                        <video
                          src={embedAttachment ? fileUrl(embedAttachment) : postEmbed.url}
                          poster={videoPoster()}
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
                              void openExternalUrl(postEmbed.url);
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
                            onclick={() => postEmbed?.url && void openExternalUrl(postEmbed.url)}
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
                        <span class="media-filename">{file?.name || i18n.t('post.cloud_folder')}</span>
                        <span class="cloud-source-badge">{(file as any).cloud_provider || i18n.t('post.cloud_provider_fallback')}</span>
                        {#if fChildCount > 0}
                          <span class="media-filesize">({i18n.t('counts.items', { count: fChildCount })})</span>
                        {/if}
                      </div>
                      <button
                        type="button"
                        class="file-placeholder media-open-surface"
                        onclick={() => openCloudFolderModal(fRes, fNodeId)}
                        aria-label={i18n.t('post.open_cloud_folder', { name: file?.name ?? '' })}
                      >
                        <IconFolder class="placeholder-icon" />
                        <p class="placeholder-text font-medium">{file?.name}</p>
                      </button>
                      <div class="media-download-group">
                        <Button
                          variant="ghost"
                          class="media-download-btn"
                          onclick={() => openCloudFolderModal(fRes, fNodeId)}
                          title={i18n.t('post.browse_folder')}
                        >
                          <span class="attachment-button-state">
                            <IconFolder class="w-[16px] h-[16px]" />
                            <span>{i18n.t('post.browse_folder')}</span>
                          </span>
                        </Button>
                        <Button
                          variant="ghost"
                          class="media-action-icon-btn"
                          onclick={() => downloadCloudSubfolder(fRes, fNodeId)}
                          tooltip={i18n.t('post.download')}
                          aria-label={i18n.t('post.download')}
                        >
                          <IconDownload class="w-[18px] h-[18px]" />
                        </Button>
                      </div>
                    {:else if isDeferred}
                      <div class="media-header">
                        <span class="media-filename">{file?.name || i18n.t('post.file')}</span>
                        {#if isCloud && file?.cloud_provider}
                          {@render sourceBadge(file)}
                        {/if}
                      </div>
                      <div class="file-placeholder media-open-surface is-deferred-placeholder" title={i18n.t('post.file_not_saved_desc')}>
                        <IconWarning class="placeholder-icon" />
                        <p class="placeholder-text">{i18n.t('post.file_not_saved')}</p>
                      </div>
                    {:else if isAttachmentVideo(file, url) || isAttachmentImage(file, url)}
                      {@const isVid = isAttachmentVideo(file, url)}
                      {@const isUnarchived = isFileUnarchived(file, Boolean(downloaded?.final_path))}
                      <div class="media-header">
                        <span class="media-filename">{file?.name || i18n.t('post.file')}</span>
                        {#if isCloud && file?.cloud_provider}
                          {@render sourceBadge(file)}
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
                            <p class="placeholder-text text-[var(--status-warning)]">{i18n.t('post.error_forbidden')}</p>
                            <p class="placeholder-subtext">{i18n.t('post.error_forbidden_hint')}</p>
                            <Button
                              variant="ghost"
                              class="placeholder-fix-btn"
                              onclick={() => {
                                retryVideo(index);
                              }}
                            >
                              <IconArrowClockwise class="w-[15px] h-[15px] text-[var(--accent-primary)]" />
                              <span>{i18n.t('downloads.retry')}</span>
                            </Button>
                          {:else if failure?.preset === 'not_found' || failure?.httpStatus === 404}
                            <p class="placeholder-text text-[var(--status-error)]">{i18n.t('post.error_not_found')}</p>
                            <p class="placeholder-subtext">{i18n.t('post.error_not_found_hint')}</p>
                          {:else if failure?.preset === 'rate_limited' || failure?.httpStatus === 429}
                            <p class="placeholder-text text-[var(--status-warning)]">{i18n.t('post.error_rate_limited')}</p>
                            <p class="placeholder-subtext">{i18n.t('post.error_rate_limited_hint')}</p>
                            <Button
                              variant="ghost"
                              class="placeholder-fix-btn"
                              onclick={() => {
                                retryVideo(index);
                              }}
                            >
                              <IconArrowClockwise class="w-[15px] h-[15px] text-[var(--accent-primary)]" />
                              <span>{i18n.t('downloads.retry')}</span>
                            </Button>
                          {:else if failure?.preset === 'server_error' || (failure?.httpStatus && failure.httpStatus >= 500)}
                            <p class="placeholder-text text-[var(--status-error)]">{failure.message || i18n.t('post.error_server')}</p>
                            <p class="placeholder-subtext">{i18n.t('post.error_server_hint')}</p>
                            <Button
                              variant="ghost"
                              class="placeholder-fix-btn"
                              onclick={() => {
                                retryVideo(index);
                              }}
                            >
                              <IconArrowClockwise class="w-[15px] h-[15px] text-[var(--accent-primary)]" />
                              <span>{i18n.t('downloads.retry')}</span>
                            </Button>
                          {:else if failure?.preset === 'network'}
                            <p class="placeholder-text">{i18n.t('post.network_stream_error')}</p>
                            <Button
                              variant="ghost"
                              class="placeholder-fix-btn"
                              onclick={() => {
                                retryVideo(index);
                              }}
                            >
                              <IconArrowClockwise class="w-[15px] h-[15px] text-[var(--accent-primary)]" />
                              <span>{i18n.t('downloads.retry')}</span>
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
                            <p class="placeholder-text">{failure?.message || i18n.t('post.video_load_failed')}</p>
                          {/if}
                        </div>
                      {:else}
                        {#if isVid}
                          {@const thumbUrl = videoThumbnails[index] || attachmentThumbnailUrl(file, service, post)}
                          {@const frame = videoFrames[index]}
                          <div
                            class="video-frame"
                            class:is-sized={Boolean(frame)}
                            style:width={frame ? `${frame.width}px` : undefined}
                            style:height={frame ? `${frame.height}px` : undefined}
                          >
                            {#if activeVideoIndexes.has(index)}
                              {#key videoAttempts[index] ?? 0}
                                <MediaSurface
                                  mediaKey={mediaElementKey(file)}
                                  kind="video"
                                  src={url}
                                  poster={videoPoster(thumbUrl)}
                                  autoplay
                                  resume
                                  onmedia={(el) => {
                                    if (el instanceof HTMLVideoElement) checkVideoElement(el, file!, index);
                                  }}
                                  onloadedmetadata={(e) => handleVideoLoadedMetadata(e, file, index)}
                                  onplaying={(e) => handleVideoLoadedMetadata(e, file, index)}
                                  onerror={(e) => handleVideoError(e, file, index)}
                                  onplay={handleVideoPlay}
                                />
                              {/key}
                            {:else if thumbUrl}
                              <button
                                class="media-open-surface video-thumbnail-surface"
                                type="button"
                                onclick={(e) => startVideo(index, e.currentTarget)}
                                aria-label={i18n.t('post.play_video')}
                              >
                                <img
                                  src={thumbnailProxySrc(thumbUrl, 600) ?? thumbUrl}
                                  alt={file?.name || post.title}
                                  loading="lazy"
                                  decoding="async"
                                />
                                <span class="play-btn-circle is-overlay"><IconPlayFilled /></span>
                              </button>
                            {:else}
                              <button
                                class="file-placeholder media-open-surface"
                                type="button"
                                onclick={(e) => startVideo(index, e.currentTarget)}
                                aria-label={i18n.t('post.play_video')}
                              >
                                <span class="play-btn-circle"><IconPlayFilled /></span>
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
                          </div>
                        {:else}
                          {@const thumbUrl = attachmentThumbnailUrl(file!, service, post)}
                          {@const displaySrc = thumbnailProxySrc(downloaded?.final_path ? url : (thumbUrl || url), 800) ?? url}
                          <button
                            class="media-open-surface"
                            type="button"
                            onclick={() => openMediaViewer(file!, activeGalleryItems)}
                            aria-label={`${i18n.t('post.viewer_open')}: ${file?.name || post.title}`}
                          >
                            <img
                              src={displaySrc}
                              alt={file?.name || post.title}
                              loading={index < 2 ? 'eager' : 'lazy'}
                              decoding="async"
                              onerror={(e) => {
                                const target = e.currentTarget as HTMLImageElement;
                                const fallback = target.src === thumbUrl ? url : thumbUrl;
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
                        {#if isCloud && file?.cloud_provider}
                          {@render sourceBadge(file)}
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
                          <MediaSurface
                            mediaKey={mediaElementKey(file)}
                            kind="audio"
                            src={url}
                            autoplay
                            resume
                            onplay={handleVideoPlay}
                          />
                        </div>
                      {:else}
                        <button
                          class="file-placeholder media-open-surface"
                          type="button"
                          onclick={() => {
                            activeAudioIndexes = new Set(activeAudioIndexes).add(index);
                            if (file) playMediaAfterMount(file);
                          }}
                          aria-label={i18n.t('post.play_audio')}
                        >
                          <span class="play-btn-circle"><IconMusicFilled /></span>
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
                      {@const isArchiveFile = isOpenableArchive(file?.name)}
                      <div class="media-header">
                        <span class="media-filename">{file?.name || i18n.t('post.file')}</span>
                        {#if isCloud && file?.cloud_provider}
                          {@render sourceBadge(file)}
                        {/if}
                      </div>
                      {#if isUnarchived}
                        <div class="file-placeholder is-error-placeholder">
                          <IconVideoOff class="placeholder-icon" />
                          <p class="placeholder-text">{i18n.t('post.file_not_archived')}</p>
                        </div>
                      {:else if isListableArchive(file?.name)}
                        <ArchivePreview name={file!.name!} label={ext} load={() => previewArchive(file!)} onopen={() => void openArchive(file!)} />
                      {:else}
                        <button class="file-placeholder media-open-surface" type="button" onclick={() => (isArchiveFile ? void openArchive(file!) : openMediaViewer(file!, activeGalleryItems))} aria-label={`${i18n.t(isArchiveFile ? 'cloud.archive_open' : 'post.viewer_open')}: ${file?.name || i18n.t('post.file')}`}>
                          {#if isArchiveFile}<IconFolderZip class="placeholder-icon" />{:else}<IconDocument class="placeholder-icon" />{/if}
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
              <div class="section-collapse-action">
                <Button
                  variant="ghost"
                  onclick={() => collapseSection(scrollContext?.viewport, galleryWrapperEl, MAX_GALLERY_HEIGHT, galleryScrollBeforeExpand, () => (galleryExpanded = false))}
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
              <p class="media-search-empty-title">{i18n.t('post.no_media_found')}</p>
              <p class="media-search-empty-desc">{i18n.t('post.no_media_found_desc')}</p>
            </div>
          {/if}
        </div>
      {/if}

      {#if richContent && !isHtmlContentEmpty(richContent)}
        <section class="post-content section-scroll-anchor" bind:this={contentWrapperEl}>
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
            <div class="section-collapse-action">
              <Button
                variant="ghost"
                onclick={() => collapseSection(scrollContext?.viewport, contentWrapperEl, MAX_CONTENT_HEIGHT, contentScrollBeforeExpand, () => (contentExpanded = false))}
                class="collapse-btn"
              >
                <IconChevronUp class="w-[16px] h-[16px]" />
                <span>{i18n.t('post.read_less')}</span>
              </Button>
            </div>
          {/if}
        </section>

        {#snippet previewAction()}
          <Button variant="primary" onclick={openPreviewViewer} class="post-footer-action">
            <IconEye class="w-[18px] h-[18px]" />
            <span>{i18n.t('post.view_preview')}</span>
          </Button>
        {/snippet}

        {#if downloadableMedia.length > 0}
          <div class="post-footer-actions-row">
            <PostDownloadPanel
              jobs={downloadableMedia.map(attachmentDownload)}
              totalBytes={totalMediaBytes}
              starting={downloadingAll}
              ondownload={() => void downloadAllMedia()}
              onopenfolder={() => void openPostFolder()}
              trailing={previewAction}
            />
          </div>
        {:else if post.file || media.length > 0}
          <div class="post-footer-actions-row">{@render previewAction()}</div>
        {/if}

        <nav class="post-footer-toolbar" aria-label={i18n.t('post.adjacent_posts')}>
          <button
            type="button"
            class="post-nav-card"
            disabled={!effectiveOlderId}
            use:ripple
            onclick={() => effectiveOlderId && navigationState.openPost(service, creatorId, effectiveOlderId)}
          >
            <IconChevronLeft />
            <span class="post-nav-text">
              <span class="post-nav-label">{i18n.t('post.previous')}</span>
              <span class="post-nav-title">{leftPostTitle}</span>
            </span>
          </button>
          <button
            type="button"
            class="post-nav-card is-next"
            disabled={!effectiveNewerId}
            use:ripple
            onclick={() => effectiveNewerId && navigationState.openPost(service, creatorId, effectiveNewerId)}
          >
            <span class="post-nav-text">
              <span class="post-nav-label">{i18n.t('post.next')}</span>
              <span class="post-nav-title">{rightPostTitle}</span>
            </span>
            <IconChevronRight />
          </button>
        </nav>
      {/if}

      {#key postKey}
        <PostComments bind:this={commentsSection} {service} {creatorId} {postId} />
      {/key}
      {:else if entry.loading || !entry.loaded}
        <div class="detail-loading" role="status" aria-label={i18n.t('feed.loading')}><IconLoading /></div>
      {:else}
        <EmptyState
          icon={IconSource}
          title={i18n.t('post.not_found_on_provider', { provider: currentProviderName })}
          description={i18n.t('post.not_found_on_provider_desc', { provider: currentProviderName })}
        >
          {#if candidateProviders.length > 1}
            <Button variant="tonal" onclick={() => onProviderChange('auto')}>{i18n.t('post.switch_to_merged')}</Button>
          {/if}
        </EmptyState>
      {/if}
    {:else if entry.loading}
      <div class="detail-loading" role="status" aria-label={i18n.t('feed.loading')}><IconLoading /></div>
    {:else if entry.error}
      <EmptyState icon={IconWarning} tone="error" title={i18n.t('post.load_error')} description={entry.error}>
        <Button variant="tonal" onclick={() => void refreshPost()}>
          <IconArrowClockwise />
          <span>{i18n.t('common.retry')}</span>
        </Button>
      </EmptyState>
    {/if}
  </div>
</PageShell>

{#if viewerIndex !== null && viewerItems.length > 0}
  <MediaViewer
    items={viewerItems}
    initialIndex={viewerIndex}
    initialTime={viewerInitialTime}
    originRect={viewerOriginRect}
    onclose={handleCloseViewer}
    ondownload={downloadViewerItem}
  />
{/if}

<CloudFolderModal
  folder={activeCloudModalFolder}
  initialFolderId={activeCloudModalInitialFolderId}
  post={post}
  open={isCloudModalOpen}
  archive={activeArchive}
  archiveFile={activeArchiveFile}
  onclose={() => isCloudModalOpen = false}
/>

<CodecGuideModal
  open={isCodecModalOpen}
  onclose={() => isCodecModalOpen = false}
/>

<style>
  .post-content-wrapper {
    position: relative;
    z-index: 2;
    min-width: 0;
    max-width: 100%;
  }

  .post-creator-avatar {
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    flex: none;
    overflow: hidden;
    border-radius: 50%;
    background: rgba(var(--surface-tint-rgb), 0.1);
  }

  .post-creator-avatar img {
    width: 100%;
    height: 100%;
    display: block;
    object-fit: cover;
  }

  :global(.post-bar-creator) {
    min-width: 0;
    max-width: 240px;
    flex-shrink: 1;
  }

  .post-bar-creator-name {
    min-width: 0;
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    transition:
      max-width var(--duration-fast) var(--ease-standard),
      margin var(--duration-fast) var(--ease-standard),
      opacity var(--duration-fast) var(--ease-standard);
  }

  :global(.post-bar-creator.is-condensed) .post-bar-creator-name {
    max-width: 0;
    margin-inline: calc(-1 * var(--control-gap, 8px)) -12px;
    opacity: 0;
  }

  @media (prefers-reduced-motion: reduce) {
    .post-bar-creator-name {
      transition: none;
    }
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
    color: var(--text-primary);
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
    color: var(--text-muted);
    font-size: 12px;
  }

  .detail-loading { min-height: 300px; display: grid; place-items: center; color: var(--text-secondary); }
  .detail-loading :global(svg) { width: 32px; height: 32px; }

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
  }

  .post-tabs-scroll::-webkit-scrollbar {
    display: none;
  }

  .post-tabs-scroll :global(.choice-group) {
    flex-wrap: nowrap !important;
    max-width: none !important;
    flex-shrink: 0 !important;
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
    .media-controls-row.search-active :global(.sort-control) {
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

  .media-skeleton {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .media-skeleton__picture,
  .media-skeleton__caption {
    background: rgba(var(--surface-tint-rgb), 0.06);
    animation: media-skeleton-pulse 1.6s var(--ease-standard) infinite;
  }

  .media-skeleton__picture {
    aspect-ratio: 16 / 9;
    border-radius: var(--radius-md);
  }

  .media-skeleton__caption {
    width: 45%;
    height: 12px;
    border-radius: var(--radius-full);
  }

  @keyframes media-skeleton-pulse {
    50% { opacity: 0.5; }
  }

  @media (prefers-reduced-motion: reduce) {
    .media-skeleton__picture,
    .media-skeleton__caption {
      animation: none;
    }
  }

  .media-gallery {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 340px), 1fr));
    gap: 24px 16px;
    padding-block: 8px 16px;
  }
  
  .media-embed-player {
    position: relative;
    width: 100%;
    border-radius: var(--radius-md);
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
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    grid-template-areas:
      "media media"
      "name actions";
    align-items: center;
    column-gap: 8px;
    row-gap: 4px;
    text-align: start;
    content-visibility: auto;
    contain-intrinsic-size: 320px 420px;
  }

  .media-item > :global(*) {
    grid-column: 1 / -1;
  }

  .media-item > .media-header {
    grid-area: name;
  }

  .media-item > .media-download-group {
    grid-area: actions;
  }

  .media-viewer-open-btn {
    position: absolute;
    top: 8px;
    right: 8px;
    z-index: 15;
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-full);
    background: var(--on-media-scrim-strong);
    color: var(--on-media);
    transition: background-color var(--duration-fast) var(--ease-standard);
  }

  @media (hover: hover) {
    .media-viewer-open-btn:hover {
      background: var(--on-media-scrim-strong) linear-gradient(var(--state-layer-hover), var(--state-layer-hover));
    }
  }

  .media-viewer-open-btn :global(svg) {
    width: 20px;
    height: 20px;
  }

  .media-header {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    padding-inline: 4px;
  }

  .media-filename {
    min-width: 0;
    overflow: hidden;
    color: var(--text-secondary);
    font-family: var(--font-sans);
    font-size: var(--type-body-sm-size);
    line-height: var(--type-body-sm-line);
    text-overflow: ellipsis;
    white-space: nowrap;
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
    display: inline-flex;
    align-items: center;
    gap: 2px;
    flex-shrink: 0;
    padding: 2px 8px;
    border: 0;
    border-radius: var(--radius-full);
    background: var(--surface-container-high);
    color: var(--text-secondary);
    font: inherit;
    font-size: var(--type-label-sm-size);
    line-height: var(--type-label-sm-line);
    font-weight: 500;
  }

  .cloud-source-badge.is-switch {
    padding-inline-end: 4px;
    cursor: pointer;
  }

  .cloud-source-badge.is-switch :global(svg) {
    width: 14px;
    height: 14px;
  }

  .cloud-source-badge.is-switch:active {
    background: var(--surface-container-high) linear-gradient(var(--state-layer-pressed), var(--state-layer-pressed));
  }

  @media (hover: hover) {
    .cloud-source-badge.is-switch:hover {
      background: var(--surface-container-high) linear-gradient(var(--state-layer-hover), var(--state-layer-hover));
    }
  }

  .media-item img {
    position: relative;
    z-index: 2;
    display: block;
    width: 100%;
    height: auto;
    max-height: 520px;
    object-fit: contain;
    border-radius: var(--radius-md);
    justify-self: center;
  }

  .media-item > video,
  .media-item :global(.media-surface video) {
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

  .audio-player-wrapper :global(.media-surface audio) {
    width: 100%;
    max-width: 320px;
  }

  .file-placeholder :global(.placeholder-icon.video-play-accent) {
    color: var(--accent-primary);
    opacity: 0.85;
  }

  .file-placeholder:hover :global(.placeholder-icon.video-play-accent) {
    color: var(--accent-primary);
    opacity: 1;
    transform: scale(1.12);
  }

  .video-frame {
    position: relative;
    width: fit-content;
    max-width: 100%;
    margin-inline: auto;
  }

  .video-frame:not(:has(.video-thumbnail-surface)) {
    width: 100%;
  }

  .video-frame :global(.media-viewer-open-btn) {
    top: var(--tile-chip-inset, 8px);
    right: var(--tile-chip-inset, 8px);
  }

  .video-frame.is-sized :global(.media-surface video) {
    width: 100%;
    height: 100%;
    min-height: 0;
    max-height: none;
    background: var(--surface-container);
  }

  .video-thumbnail-surface {
    position: relative;
    display: grid;
    place-items: center;
    padding: 0;
    border: 0;
    border-radius: var(--radius-md, 10px);
    background: transparent;
    overflow: hidden;
    cursor: pointer;
  }

  .video-thumbnail-surface img {
    grid-area: 1 / 1;
    width: auto;
    max-width: 100%;
  }

  .play-btn-circle {
    display: grid;
    place-items: center;
    width: 64px;
    height: 64px;
    border-radius: var(--radius-full);
    background: var(--on-media-scrim-strong);
    color: var(--on-media);
    transition: background-color var(--duration-fast) var(--ease-standard);
  }

  .play-btn-circle :global(svg) {
    width: 28px;
    height: 28px;
  }

  .play-btn-circle.is-overlay {
    position: relative;
    z-index: 3;
    grid-area: 1 / 1;
    pointer-events: none;
  }

  .play-btn-circle.is-overlay :global(svg) {
    margin-inline-start: 3px;
  }

  @media (hover: hover) {
    .video-thumbnail-surface:hover .play-btn-circle {
      background: var(--on-media-scrim-strong) linear-gradient(var(--state-layer-hover), var(--state-layer-hover));
    }
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
    aspect-ratio: 16 / 9;
    width: 100%;
    box-sizing: border-box;
    padding: 16px;
    border: 0;
    border-radius: var(--radius-md);
    background: var(--surface-container);
    cursor: pointer;
    text-align: center;
    transition: background-color var(--duration-fast) var(--ease-standard);
  }

  .file-placeholder.is-error-placeholder,
  .file-placeholder.is-deferred-placeholder {
    cursor: default;
  }

  .file-placeholder.is-error-placeholder :global(.placeholder-icon),
  .file-placeholder.is-deferred-placeholder :global(.placeholder-icon) {
    color: var(--status-error);
    opacity: 1;
  }

  @media (hover: hover) {
    button.file-placeholder:hover {
      background: var(--surface-container-high);
    }
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
    justify-content: flex-end;
    gap: 2px;
    min-width: 0;
    --control-height: var(--control-height-sm);
    --control-font-size: var(--type-label-lg-size);
    --control-icon-size: var(--control-icon-sm);
    --control-padding-x: 12px;
  }

  :global(.media-download-btn) {
    position: relative;
    max-width: 100%;
    overflow: hidden;
    isolation: isolate;
  }

  .media-download-group .state-word {
    display: none;
  }

  .media-download-group :global(.media-download-btn) {
    font-variant-numeric: tabular-nums;
  }

  :global(.media-download-btn.is-downloading:disabled) {
    opacity: 1;
  }

  :global(.media-action-icon-btn) {
    width: var(--control-height) !important;
    height: var(--control-height) !important;
    padding: 0 !important;
    border-radius: var(--radius-full) !important;
    display: grid !important;
    place-items: center !important;
    flex-shrink: 0;
    transition: color var(--duration-fast), background var(--duration-fast);
  }

  :global(.media-action-icon-btn.is-danger:hover) {
    color: var(--status-error) !important;
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

  .locked-cloud-links {
    display: flex;
    flex-direction: column;
    gap: calc(var(--space-2) * var(--ui-scale, 1));
    margin-bottom: calc(var(--space-4) * var(--ui-scale, 1));
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
    color: var(--text-primary);
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
    color: var(--accent-primary, var(--text-primary));
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

  .post-footer-toolbar {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
    margin-bottom: 24px;
  }

  .post-nav-card {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    min-height: 64px;
    padding: 12px 16px;
    border: 0;
    border-radius: var(--radius-lg);
    background: var(--surface-container);
    color: var(--text-primary);
    text-align: start;
    transition: background-color var(--duration-fast) var(--ease-standard);
  }

  .post-nav-card.is-next {
    justify-content: flex-end;
    text-align: end;
  }

  @media (hover: hover) {
    .post-nav-card:not(:disabled):hover {
      background: var(--surface-container-high);
    }
  }

  .post-nav-card:focus-visible {
    outline: 2px solid var(--accent-primary);
    outline-offset: 2px;
  }

  .post-nav-card:disabled {
    opacity: var(--opacity-disabled);
  }

  .post-nav-card :global(svg) {
    flex: none;
    width: 20px;
    height: 20px;
    color: var(--text-secondary);
  }

  .post-nav-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .post-nav-label {
    color: var(--text-secondary);
    font-size: var(--type-label-md-size);
    line-height: var(--type-label-md-line);
  }

  .post-nav-title {
    overflow: hidden;
    font-size: var(--type-body-md-size);
    line-height: var(--type-body-md-line);
    font-weight: 500;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .detail-header h1,
  .post-date,
  .media-filename {
    user-select: text;
    cursor: text;
  }

  .post-footer-actions-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    margin-block: 16px;
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
