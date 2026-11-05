<script lang="ts">
  import { haptic } from '$lib/utils/haptics';
  import { onDestroy } from 'svelte';
  import type { Post } from '$lib/types/content';
  import type { LibraryCollection } from '$lib/types/library';
  import { configState } from '$lib/state/configState.svelte';
  import { contentState, postCacheKey } from '$lib/state/contentState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { libraryState } from '$lib/state/libraryState.svelte';
  import { creatorsState } from '$lib/state/creatorsState.svelte';
  import { selectionState } from '$lib/state/selectionState.svelte';
  import { accountState } from '$lib/state/accountState.svelte';
  import { favoriteActions } from '$lib/state/favoriteActions.svelte';
  import { contextMenuState } from '$lib/state/contextMenuState.svelte';
  import { postMenuItems } from '$lib/utils/contextMenus';
  import { subscriptionState } from '$lib/state/subscriptionState.svelte';
  import { playbackState } from '$lib/state/playbackState.svelte';
  import { i18n } from '$lib/i18n';
  import { tooltip, ripple } from '$lib/motion';
  import { notify } from '$lib/utils/toast';
  import { notifyAddedToStash, notifyRemovedFromStash } from '$lib/utils/stashToast';
  import { formatDate, cleanPostTitle, decodeHtmlEntities } from '$lib/utils/formatters';
  import { isVideoUrl, isAttachmentVideo, postMediaUrl, postThumbnailSrc, postPlaceholderUrl, getPostFileCounts, isPostUnarchived, detailIsOutdated, videoPoster, thumbnailProxySrc } from '$lib/utils/media';
  import { claimHoverVideo, releaseHoverVideo } from '$lib/utils/hoverVideo';
  import { getMediaThumbnail, cancelMediaThumbnail } from '$lib/utils/mediaThumbnail';
  import { thumbnailKey } from '$lib/utils/cacheKey';
  import { extraField } from '$lib/utils/fields';
  import ServiceIcon from './ServiceIcon.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import IconWarning from '~icons/fluent/warning-24-regular';
  import IconImage from '~icons/fluent/image-24-regular';
  import IconVideo from '~icons/fluent/video-24-regular';
  import IconMusic from '~icons/fluent/music-note-2-24-regular';
  import IconFolderZip from '~icons/fluent/folder-zip-24-regular';
  import IconDocument from '~icons/fluent/document-24-regular';
  import IconCloud from '~icons/fluent/cloud-24-regular';
  import IconAttach from '~icons/fluent/attach-24-regular';
  import IconHeart from '~icons/fluent/heart-24-filled';
  import IconHeartOutline from '~icons/fluent/heart-24-regular';
  import IconLock from '~icons/fluent/lock-closed-24-regular';
  import IconDocumentText from '~icons/fluent/document-text-24-regular';
  import IconSave from '~icons/fluent/bookmark-add-24-regular';
  import IconSaved from '~icons/fluent/bookmark-24-filled';
  import IconBookmarkMultiple from '~icons/fluent/bookmark-multiple-24-regular';
  import IconPersonSubscribed from '~icons/fluent/person-available-24-filled';
  import IconFolder from '~icons/fluent/folder-24-regular';
  import IconDelete from '~icons/fluent/delete-24-regular';
  import IconFolderDismiss from '~icons/fluent/folder-dismiss-24-regular';
  import IconCheckmark from '~icons/fluent/checkmark-20-filled';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';

  interface Props {
    post: Post;
    showCreator?: boolean;
    orderedKeys?: string[];
    itemsMap?: Map<string, Post>;
    enterDelay?: number | null;
  }

  let { post, showCreator = true, orderedKeys, itemsMap, enterDelay = null }: Props = $props();

  const ratios = {
    square: '1 / 1',
    portrait: '4 / 5',
    landscape: '3 / 2',
    widescreen: '16 / 9'
  } as const;

  let postKey = $derived(`${(post.service || '').toLowerCase()}:${post.user}:${post.id}`);
  let isSelectionActive = $derived(selectionState.active && selectionState.scope === 'posts');
  let selected = $derived(isSelectionActive && selectionState.isSelected(postKey));

  let cachedDetail = $derived.by(() => {
    if (!post?.service || !post?.user || !post?.id) return undefined;
    const cached = contentState.posts[postCacheKey(post.service, post.user, post.id)]?.post;
    return cached?.detail_fetched ? cached : undefined;
  });
  let detailIsStale = $derived(Boolean(cachedDetail && detailIsOutdated(post, cachedDetail)));
  let effectivePost = $derived(cachedDetail && !detailIsStale ? { ...post, ...cachedDetail } : post);

  let mediaUrl = $derived(postMediaUrl(effectivePost));
  let thumbnailUrl = $derived(postThumbnailSrc(effectivePost));
  let placeholderUrl = $derived(postPlaceholderUrl(effectivePost));
  let isVideo = $derived(
    isVideoUrl(mediaUrl) ||
    isAttachmentVideo(effectivePost?.file, mediaUrl) ||
    Boolean(effectivePost?.attachments?.some((a) => isAttachmentVideo(a, a.url)))
  );
  let isAnimated = $derived(
    Boolean(
      isVideo ||
      (thumbnailUrl && (thumbnailUrl.includes('preview.webp') || /\.gif(?:$|\?)/i.test(thumbnailUrl)))
    )
  );

  let generatedThumbnail = $state<string | null>(null);
  let frozenThumbnail = $state<string | null>(null);

  function freezeFrame(target: HTMLImageElement) {
    if (frozenThumbnail) return;
    try {
      const w = target.naturalWidth || target.width;
      const h = target.naturalHeight || target.height;
      if (!w || !h) return;
      const canvas = document.createElement('canvas');
      canvas.width = Math.min(360, w);
      canvas.height = Math.max(1, Math.round(canvas.width * (h / w)));
      const ctx = canvas.getContext('2d');
      if (!ctx) return;
      ctx.drawImage(target, 0, 0, canvas.width, canvas.height);
      const data = canvas.toDataURL('image/jpeg', 0.85);
      if (data && data.length > 50) {
        frozenThumbnail = data;
      }
    } catch {}
  }

  $effect(() => {
    if (thumbnailUrl && !isAnimated) {
      generatedThumbnail = null;
      return;
    }
    if (!effectivePost?.service || !effectivePost?.user || !effectivePost?.id) return;
    const key = thumbnailKey(null, effectivePost);
    const url = effectivePost.thumbnail_url || effectivePost.file?.thumbnail_url || mediaUrl || effectivePost.file?.path;
    if (!url) return;

    let cancelled = false;
    getMediaThumbnail(key, url, isVideo ? 'video' : 'image', 360).then((thumb) => {
      if (!cancelled && thumb) {
        generatedThumbnail = thumb;
      }
    });

    return () => {
      cancelled = true;
      cancelMediaThumbnail(key);
    };
  });

  let activeThumbnail = $derived(thumbnailUrl || generatedThumbnail);
  let idleThumbnail = $derived(
    frozenThumbnail ||
    (isAnimated && generatedThumbnail ? generatedThumbnail : null) ||
    activeThumbnail
  );
  let tileThumbnail = $derived(thumbnailProxySrc(idleThumbnail, 400));

  let imageAttempt = $state(0);
  let tileSources = $derived.by(() =>
    [...new Set([tileThumbnail, idleThumbnail, thumbnailProxySrc(mediaUrl, 400), mediaUrl]
      .filter((value): value is string => Boolean(value)))]
  );

  $effect(() => {
    tileSources;
    imageAttempt = 0;
  });

  let isHovered = $state(false);
  let showVideo = $state(false);
  let hoverVideoFailed = $state(false);
  let videoEl = $state<HTMLVideoElement | null>(null);
  let playTimeout: ReturnType<typeof setTimeout> | undefined;

  let hoverVideoUrl = $derived.by(() => {
    if (isVideo && mediaUrl) return mediaUrl;
    if (thumbnailUrl && (thumbnailUrl.includes('preview.webp') || /\.gif(?:$|\?)/i.test(thumbnailUrl))) {
      return thumbnailUrl;
    }
    return undefined;
  });

  function handleCardMouseEnter() {
    isHovered = true;
    handleCardHover();
    if (hoverVideoUrl && !hoverVideoFailed) {
      if (playTimeout) clearTimeout(playTimeout);
      playTimeout = setTimeout(() => {
        if (isHovered) {
          showVideo = true;
        }
      }, 180);
    }
  }

  function handleCardMouseLeave() {
    isHovered = false;
    if (playTimeout) clearTimeout(playTimeout);
    showVideo = false;
    if (videoEl) {
      videoEl.pause();
    }
  }

  $effect(() => {
    const element = videoEl;
    if (!showVideo || !element) return;
    claimHoverVideo(element);
    return () => releaseHoverVideo(element);
  });

  onDestroy(() => {
    if (playTimeout) clearTimeout(playTimeout);
  });

  let isLite = $derived(configState.settings.card_view_mode === 'lite');
  let fileCounts = $derived(getPostFileCounts(effectivePost));

  interface FileBadge {
    key: string;
    icon: any;
    count: number;
    tooltip: string;
    cloud?: boolean;
  }

  let fileBadges = $derived.by<FileBadge[]>(() => {
    if (fileCounts.attachments > 0) {
      return [{
        key: 'attachments',
        icon: IconAttach,
        count: fileCounts.attachments,
        tooltip: i18n.t('feed.attachments_count', { count: fileCounts.attachments })
      }];
    }

    const byKind: FileBadge[] = [
      { key: 'images', icon: IconImage, count: fileCounts.images, tooltip: i18n.t('counts.photos', { count: fileCounts.images }) },
      { key: 'videos', icon: IconVideo, count: fileCounts.videos, tooltip: i18n.t('counts.videos', { count: fileCounts.videos }) },
      { key: 'audios', icon: IconMusic, count: fileCounts.audios, tooltip: i18n.t('counts.audio', { count: fileCounts.audios }) },
      { key: 'archives', icon: IconFolderZip, count: fileCounts.archives, tooltip: i18n.t('counts.archives', { count: fileCounts.archives }) },
      { key: 'documents', icon: IconDocument, count: fileCounts.documents, tooltip: i18n.t('counts.documents', { count: fileCounts.documents }) },
      { key: 'clouds', icon: IconCloud, count: fileCounts.clouds, tooltip: i18n.t('counts.cloud_links', { count: fileCounts.clouds }), cloud: true }
    ].filter((badge) => badge.count > 0);

    if (byKind.length <= 2) return byKind;

    return [{
      key: 'mixed',
      icon: IconAttach,
      count: byKind.reduce((sum, badge) => sum + badge.count, 0),
      tooltip: byKind.map((badge) => badge.tooltip).join(' · ')
    }];
  });
  let isUnarchived = $derived(isPostUnarchived(effectivePost));
  let isFavorited = $derived(accountState.isPostFavorite(post.service, post.user, post.id));
  let creatorFavorited = $derived(accountState.isCreatorFavorite(post.service, post.user));
  let creatorSubscribed = $derived(Boolean(subscriptionState.forCreator(post.service, post.user)));
  let stashMenuOpen = $state(false);

  function handleToggleFavorite(event: MouseEvent) {
    event.stopPropagation();
    event.preventDefault();
    void favoriteActions.togglePost(post);
  }

  let lockedCount = $derived(Number(extraField(post, 'locked_attachments_count')) || 0);
  let isLocked = $derived(Boolean(extraField(post, 'is_locked')) || lockedCount > 0);
  let textContent = $derived.by(() => {
    return (post.content || post.substring || '').trim();
  });
  let isTextOnly = $derived(!thumbnailUrl && !mediaUrl && !isVideo && fileCounts.total === 0 && textContent.length > 0);

  let imageLoaded = $state(false);
  let imageError = $state(false);
  let showBlurPlaceholder = $derived(
    Boolean(placeholderUrl) &&
    !configState.settings.disable_blur_placeholders &&
    !imageLoaded &&
    !imageError &&
    !isTextOnly
  );
  let textExcerpt = $derived.by(() => {
    if (!textContent) return '';
    const stripped = decodeHtmlEntities(textContent.replace(/<[^>]*>/g, ' ')).replace(/\s+/g, ' ').trim();
    if (stripped.length > 180) {
      return stripped.slice(0, 177) + '...';
    }
    return stripped;
  });
  let ratio = $derived(ratios[configState.settings.grid_aspect_ratio]);
  let saved = $derived(libraryState.isSaved(post));
  let saving = $derived(libraryState.isPending(post));
  const stashOptions = $derived(libraryState.stashOptions);
  let postStashes = $derived(libraryState.getPostStashes(post));
  let customStashes = $derived(libraryState.getCustomPostStashes(post));
  let customStashObjects = $derived(
    customStashes
      .map((id) => libraryState.collections.find((c) => c.id === id))
      .filter((c): c is LibraryCollection => Boolean(c))
  );
  let customStashNames = $derived(
    customStashObjects.map((c) => libraryState.getStashDisplayName(c))
  );
  let isInsideLibrary = $derived(navigationState.route.name === 'library');
  let stashCount = $derived(customStashObjects.length);
  let stashColor = $derived(customStashObjects[0]?.color || null);
  let stashLabel = $derived.by(() => {
    if (isInsideLibrary) return '';
    if (customStashNames.length > 0) return customStashNames[0];
    if (!saved) return '';
    const inbox = libraryState.inbox;
    return inbox ? libraryState.getStashDisplayName(inbox) : i18n.t('library.saved');
  });
  let isInsideSpecificLibraryCategory = $derived(
    isInsideLibrary && libraryState.selectedCollectionId !== null
  );

  let cardActionTooltip = $derived.by(() => {
    if (isInsideLibrary) {
      return i18n.t('library.manage_stashes');
    }
    if (!saved) {
      return i18n.t('library.add_to_stash');
    }
    if (customStashNames.length > 0) {
      return `${i18n.t('library.saved')} · ${customStashNames.join(', ')}`;
    }
    return i18n.t('library.saved');
  });

  let creatorName = $derived.by(() => {
    for (const key of ['creator_name', 'creatorName', 'username', 'user_name', 'author', 'name']) {
      const value = extraField<string>(post, key);
      if (value) return value;
    }

    const serviceLower = (post.service || '').toLowerCase();
    const userIdLower = (post.user || '').toLowerCase();
    const cacheKey = `${serviceLower}:${userIdLower}`;

    const name = creatorsState.creatorsMap.get(cacheKey) || creatorsState.creatorsMap.get(userIdLower);
    if (name) return name;

    const cached = contentState.creators[cacheKey];
    if (cached?.profile?.name) return cached.profile.name;

    return post.user || 'Unknown';
  });

  let lastContextTime = 0;

  function handleCardClick(event: MouseEvent) {
    if (Date.now() - lastContextTime < 500) return;

    if (event.ctrlKey || event.metaKey) {
      event.preventDefault();
      event.stopPropagation();
      selectionState.toggle('posts', postKey, post, orderedKeys, false, itemsMap);
      return;
    }

    if (isSelectionActive) {
      event.preventDefault();
      event.stopPropagation();
      selectionState.toggle('posts', postKey, post, orderedKeys, event.shiftKey, itemsMap);
      return;
    }

    openPost();
  }

  function handleContextMenu(event: MouseEvent) {
    event.preventDefault();
    lastContextTime = Date.now();
    haptic();
    const select = () => selectionState.toggle('posts', postKey, post, orderedKeys, false, itemsMap);
    if (isSelectionActive) select();
    else contextMenuState.show(event, post.title || creatorName || post.id, postMenuItems(post, select));
  }

  function handleSelectCheckbox(event: MouseEvent) {
    event.stopPropagation();
    selectionState.toggle('posts', postKey, post, orderedKeys, event.shiftKey, itemsMap);
  }

  $effect(() => {
    const service = post?.service;
    const user = post?.user;
    const id = post?.id;
    if (service && user && id && !effectivePost.detail_fetched) {
      contentState.enqueueDetailPrefetch(service, user, id, false, detailIsStale);
    }
    return () => {
      if (service && user && id) {
        contentState.cancelPrefetch(service, user, id);
      }
    };
  });

  function handleCardHover() {
    if (post?.service && post?.user && post?.id && !effectivePost.detail_fetched) {
      contentState.enqueueDetailPrefetch(post.service, post.user, post.id, true, detailIsStale);
    }
  }

  function openPost() {
    contentState.seedPost(effectivePost);
    navigationState.openPost(post.service, post.user, post.id);
  }

  function openCreator(event: MouseEvent) {
    event.stopPropagation();
    navigationState.openCreator(post.service, post.user);
  }

  async function handleStashToggle(collectionId: string) {
    if (!post || !collectionId) return;
    const isCurrentlyIn = postStashes.includes(collectionId);
    try {
      if (isCurrentlyIn) {
        await libraryState.removeFromStash(collectionId, post);
        notifyRemovedFromStash(collectionId, [post]);
      } else {
        await libraryState.save(post, collectionId);
        notifyAddedToStash(collectionId, post.title || undefined);
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

  async function handleRemoveFromCurrentCategory(event: MouseEvent) {
    event.stopPropagation();
    event.preventDefault();
    const collectionId = libraryState.selectedCollectionId;
    if (!collectionId) return;

    try {
      if (libraryState.selectedCollection?.kind === 'stash') {
        await libraryState.removeFromStash(collectionId, post);
        notifyRemovedFromStash(collectionId, [post]);
      } else {
        await libraryState.remove(post);
        notify.success(i18n.t('library.removed'), { description: post.title || undefined, glyph: 'removed' });
      }
    } catch (error) {
      notify.error(i18n.t('library.save_error'), error);
    }
  }
</script>

<article
  class="grid-tile"
  class:selected={selected}
  class:is-entering={enterDelay !== null}
  style:aspect-ratio={ratio}
  style:--tile-enter-delay={enterDelay !== null ? `${enterDelay}ms` : null}
  data-post-key={postKey}
  data-select-key={postKey}
  onmouseenter={handleCardMouseEnter}
  onmouseleave={handleCardMouseLeave}
>
  <button
    class="grid-tile-open"
    type="button"
    onclick={handleCardClick}
    oncontextmenu={handleContextMenu}
    aria-label={cleanPostTitle(effectivePost.title)}
  ></button>

  {#if isSelectionActive}
    <button
      type="button"
      class="grid-tile-select-checkbox"
      class:checked={selected}
      onclick={handleSelectCheckbox}
      use:ripple
      aria-label={i18n.t('selection.select_post')}
    >
      {#if selected}
        <IconCheckmark />
      {/if}
    </button>
  {:else if isLite && isUnarchived}
    <div class="grid-tile-top-files">
      <span
        class="grid-tile-file-item grid-tile-unarchived-item"
        use:tooltip={i18n.t('feed.unarchived_badge')}
      >
        <IconWarning />
      </span>
    </div>
  {:else if !isLite}
    {#if isUnarchived || fileBadges.length > 0}
      <div class="grid-tile-top-files">
        {#if isUnarchived}
          <span
            class="grid-tile-file-item grid-tile-unarchived-item"
            use:tooltip={i18n.t('feed.unarchived_badge')}
          >
            <IconWarning />
          </span>
        {/if}
        {#each fileBadges as badge (badge.key)}
          {@const BadgeIcon = badge.icon}
          <span
            class="grid-tile-file-item"
            class:grid-tile-cloud-item={badge.cloud}
            use:tooltip={badge.tooltip}
          >
            <BadgeIcon />
            {#if badge.count > 1}<span>{badge.count}</span>{/if}
          </span>
        {/each}
      </div>
    {/if}

    <div class="grid-tile-top-actions tile-toolbar">
      <div class="grid-tile-action-item" class:is-active={isFavorited}>
        <button
          type="button"
          class="grid-tile-action grid-tile-action-fav"
          class:favorited={isFavorited}
          disabled={favoriteActions.isPostPending(post)}
          onclick={handleToggleFavorite}
          use:ripple
          use:tooltip={i18n.t(isFavorited ? 'post.unfavorite' : 'post.favorite')}
          aria-label={i18n.t(isFavorited ? 'post.unfavorite' : 'post.favorite')}
        >
          {#if isFavorited}
            <IconHeart />
          {:else}
            <IconHeartOutline />
          {/if}
        </button>
      </div>

      {#if isInsideSpecificLibraryCategory}
        <div class="grid-tile-action-item is-active">
          <button
            type="button"
            class="grid-tile-action grid-tile-action-danger"
            disabled={saving}
            onclick={handleRemoveFromCurrentCategory}
            use:ripple
            use:tooltip={i18n.t(libraryState.selectedCollection?.kind === 'stash' ? 'library.remove_from_stash' : 'library.remove')}
            aria-label={i18n.t(libraryState.selectedCollection?.kind === 'stash' ? 'library.remove_from_stash' : 'library.remove')}
          >
            {#if saving}
              <IconLoading />
            {:else if libraryState.selectedCollection?.kind === 'stash'}
              <IconFolderDismiss />
            {:else}
              <IconDelete />
            {/if}
          </button>
        </div>
      {:else}
        <div class="grid-tile-action-item" class:is-active={saved || stashMenuOpen}>
          <Select
            bind:open={stashMenuOpen}
            options={stashOptions}
            selectedValues={postStashes}
            placeholder={i18n.t('library.add_to_stash')}
            onchange={handleStashToggle}
            createLabel={i18n.t('library.new_stash')}
            onCreate={handleCreateStash}
            variant={isInsideLibrary ? 'ghost' : (saved ? 'accent' : 'ghost')}
            multi={true}
            closeOnChange={false}
            icon={IconFolder}
            align="right"
            class="card-stash-select"
          >
            {#snippet trigger({ toggle, open })}
              <button
                type="button"
                class="grid-tile-action grid-tile-action-stash"
                class:is-selected={open || (!isInsideLibrary && saved)}
                class:has-label={Boolean(stashLabel)}
                class:has-count={stashCount > 1}
                class:has-custom-color={Boolean(stashColor)}
                style={stashColor ? `--stash-custom-color: ${stashColor};` : undefined}
                disabled={saving}
                onclick={(e) => {
                  e.stopPropagation();
                  e.preventDefault();
                  toggle();
                }}
                use:ripple
                use:tooltip={cardActionTooltip}
                aria-label={cardActionTooltip}
                aria-expanded={open}
              >
                {#if saving}
                  <IconLoading />
                {:else if isInsideLibrary}
                  <IconBookmarkMultiple />
                {:else if stashCount > 1}
                  <IconBookmarkMultiple />
                {:else if saved}
                  <IconSaved />
                {:else}
                  <IconSave />
                {/if}
                {#if stashLabel}
                  <span class="tile-action-label">{stashLabel}</span>
                {/if}
                {#if stashCount > 1}
                  <span class="tile-action-count">{stashCount}</span>
                {/if}
              </button>
            {/snippet}
          </Select>
        </div>
      {/if}
    </div>
  {/if}

  {#if showBlurPlaceholder}
    <img
      class="grid-tile-media grid-tile-blur-placeholder"
      src={placeholderUrl}
      alt=""
      aria-hidden="true"
    />
  {/if}

  {#if idleThumbnail}
    <img
      class="grid-tile-media"
      src={tileSources[imageAttempt]}
      alt=""
      loading="lazy"
      decoding="async"
      onload={(e) => {
        imageLoaded = true;
        if (isAnimated) {
          freezeFrame(e.currentTarget as HTMLImageElement);
        }
      }}
      onerror={() => {
        if (imageAttempt < tileSources.length - 1) {
          imageAttempt++;
        } else {
          imageError = true;
          imageLoaded = false;
        }
      }}
    />
  {:else if isVideo}
    <div class="grid-tile-placeholder"><IconVideo /></div>
  {:else if mediaUrl}
    <img
      class="grid-tile-media"
      src={tileSources[imageAttempt]}
      alt=""
      loading="lazy"
      decoding="async"
      onerror={() => {
        if (imageAttempt < tileSources.length - 1) imageAttempt++;
      }}
    />
  {:else if isLocked}
    <div class="grid-tile-placeholder grid-tile-placeholder-locked" use:tooltip={i18n.t('feed.locked_content')}>
      <IconLock class="w-7 h-7 text-accent opacity-70" />
      <span class="text-xs text-secondary font-medium">{i18n.t('feed.locked')}</span>
    </div>
  {:else if isTextOnly}
    <div class="grid-tile-placeholder grid-tile-placeholder-text">
      <div class="grid-tile-text-snippet">
        <IconDocumentText class="w-5 h-5 text-accent opacity-50 mb-1.5 shrink-0" />
        <p>{textExcerpt}</p>
      </div>
    </div>
  {:else}
    <div class="grid-tile-placeholder"><IconImage /></div>
  {/if}

  {#if showVideo && hoverVideoUrl && !hoverVideoFailed}
    {#if isVideoUrl(hoverVideoUrl)}
      <video
        bind:this={videoEl}
        class="grid-tile-media grid-tile-hover-video"
        src={hoverVideoUrl}
        poster={videoPoster()}
        muted
        loop
        playsinline
        disablepictureinpicture
        disableremoteplayback
        autoplay
        onloadedmetadata={(e) => {
          const vid = e.currentTarget as HTMLVideoElement;
          if (vid.duration && isFinite(vid.duration) && vid.duration > 0) {
            const key = effectivePost?.id || '';
            if (key) playbackState.saveDuration(key, vid.duration);
          }
        }}
        onerror={() => { hoverVideoFailed = true; showVideo = false; }}
      ></video>
    {:else}
      <img
        class="grid-tile-media grid-tile-hover-video"
        src={hoverVideoUrl}
        alt=""
        aria-hidden="true"
      />
    {/if}
  {/if}

  <div class="grid-tile-shade"></div>

  <div class="grid-tile-footer">
    <h2 class="grid-tile-title">{cleanPostTitle(effectivePost.title) || i18n.t('feed.untitled')}</h2>

    {#if showCreator}
      <div class="grid-tile-author">
        <button
          type="button"
          class="grid-tile-logo inline-logo"
          onclick={openCreator}
          use:tooltip={i18n.t('feed.open_creator')}
          aria-label={`${i18n.t('feed.open_creator')}: ${post.service}`}
        >
          <ServiceIcon service={post.service} />
        </button>

        <span
          role="link"
          tabindex="0"
          class="grid-tile-author-name"
          onclick={openCreator}
          onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && openCreator(e as unknown as MouseEvent)}
        >
          {creatorName}
        </span>

        {#if creatorFavorited || creatorSubscribed}
          <span class="grid-tile-author-marks">
            {#if creatorFavorited}
              <span class="grid-tile-author-mark is-favorite" use:tooltip={i18n.t('feed.creator_favorited')}>
                <IconHeart />
              </span>
            {/if}
            {#if creatorSubscribed}
              <span class="grid-tile-author-mark is-subscribed" use:tooltip={i18n.t('feed.creator_subscribed')}>
                <IconPersonSubscribed />
              </span>
            {/if}
          </span>
        {/if}
      </div>
    {/if}

    <div class="grid-tile-meta">
      <span>{formatDate(post.published || post.added)}</span>
      {#if !isLite}
        <div class="grid-tile-meta-stats">
          {#if isLocked}
            <span class="grid-tile-meta-row is-locked" use:tooltip={i18n.t('feed.locked_content')}>
              <IconLock /> {lockedCount || 1}
            </span>
          {/if}
          {#if post.favorite_count !== undefined && post.favorite_count > 0}
            <span class="grid-tile-meta-row">
              <IconHeart class={isFavorited ? 'is-favorited' : ''} /> {post.favorite_count}
            </span>
          {/if}
        </div>
      {/if}
    </div>
  </div>
</article>

<style>
  :global(.card-stash-select) {
    width: auto !important;
    max-width: none !important;
    min-width: 0 !important;
    flex-shrink: 1 !important;
  }

  :global(.card-stash-select .select-custom-trigger) {
    min-width: 0;
  }

  .grid-tile-placeholder-text {
    align-items: flex-start !important;
    justify-content: flex-start !important;
    padding: 1rem 1rem 3.5rem 1rem !important;
    overflow: hidden;
  }

  .grid-tile-text-snippet {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    overflow: hidden;
  }

  .grid-tile-text-snippet p {
    font-size: 0.76rem;
    line-height: 1.35;
    color: var(--text-secondary);
    display: -webkit-box;
    -webkit-line-clamp: 6;
    line-clamp: 6;
    -webkit-box-orient: vertical;
    overflow: hidden;
    word-break: break-word;
    margin: 0;
  }

  .grid-tile-placeholder-locked {
    flex-direction: column;
    gap: 0.35rem;
  }

  :global(.grid-tile-hover-video) {
    pointer-events: none !important;
  }
</style>
