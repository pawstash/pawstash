<script lang="ts">
  import type { PawchivePost } from '$lib/types/pawchive';
  import type { LibraryCollection } from '$lib/types/library';
  import { configState } from '$lib/state/configState.svelte';
  import { contentState } from '$lib/state/contentState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { libraryState } from '$lib/state/libraryState.svelte';
  import { creatorsState } from '$lib/state/creatorsState.svelte';
  import { selectionState } from '$lib/state/selectionState.svelte';
  import { i18n } from '$lib/i18n';
  import { tooltip } from '$lib/motion';
  import { notify } from '$lib/utils/toast';
  import { formatDate, cleanPostTitle } from '$lib/utils/formatters';
  import { isVideoUrl, postAttachmentCount, postMediaUrl, postThumbnailUrl } from '$lib/utils/media';
  import ServiceIcon from './ServiceIcon.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import IconImage from '~icons/fluent/image-24-regular';
  import IconVideo from '~icons/fluent/video-24-regular';
  import IconAttach from '~icons/fluent/attach-24-regular';
  import IconHeart from '~icons/fluent/heart-24-filled';
  import IconSave from '~icons/fluent/bookmark-add-24-regular';
  import IconSaved from '~icons/fluent/bookmark-24-filled';
  import IconBookmarkMultiple from '~icons/fluent/bookmark-multiple-24-regular';
  import IconFolder from '~icons/fluent/folder-24-regular';
  import IconDelete from '~icons/fluent/delete-24-regular';
  import IconFolderDismiss from '~icons/fluent/folder-dismiss-24-regular';
  import IconCheckmark from '~icons/fluent/checkmark-20-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';

  interface Props {
    post: PawchivePost;
    showCreator?: boolean;
    orderedKeys?: string[];
    itemsMap?: Map<string, PawchivePost>;
  }

  let { post, showCreator = true, orderedKeys, itemsMap }: Props = $props();

  const ratios = {
    square: '1 / 1',
    portrait: '4 / 5',
    landscape: '3 / 2',
    widescreen: '16 / 9'
  } as const;

  let postKey = $derived(`${(post.service || '').toLowerCase()}:${post.user}:${post.id}`);
  let isSelectionActive = $derived(selectionState.active && selectionState.scope === 'posts');
  let selected = $derived(isSelectionActive && selectionState.isSelected(postKey));

  let mediaUrl = $derived(postMediaUrl(post));
  let thumbnailUrl = $derived(postThumbnailUrl(post));
  let video = $derived(isVideoUrl(mediaUrl));
  let attachments = $derived(postAttachmentCount(post));
  let ratio = $derived(ratios[configState.settings.grid_aspect_ratio]);
  let saved = $derived(libraryState.isSaved(post));
  let saving = $derived(libraryState.isPending(post));
  let stashes = $derived(libraryState.allStashes);
  let stashOptions = $derived(stashes.map((s) => ({ value: s.id, label: libraryState.getStashDisplayName(s) })));
  let postStashes = $derived(libraryState.getPostStashes(post));
  let customStashes = $derived(libraryState.getCustomPostStashes(post));
  let customStashNames = $derived(
    customStashes
      .map((id) => libraryState.collections.find((c) => c.id === id))
      .filter((c): c is LibraryCollection => Boolean(c))
      .map((c) => libraryState.getStashDisplayName(c))
  );
  let isInsideLibrary = $derived(navigationState.route.name === 'library');
  let isInsideSpecificLibraryCategory = $derived(
    isInsideLibrary && libraryState.selectedCollectionId !== null
  );

  let cardActionTooltip = $derived.by(() => {
    if (isInsideLibrary) {
      return i18n.t('library.manage_stashes') || 'Manage stashes';
    }
    if (!saved) {
      return i18n.t('library.add_to_stash') || 'Add to stash';
    }
    if (customStashNames.length > 0) {
      return `${i18n.t('library.saved')} · ${customStashNames.join(', ')}`;
    }
    return i18n.t('library.saved') || 'Saved in library';
  });

  let creatorName = $derived.by(() => {
    const extra = post.extra as any;
    if (extra?.creator_name) return extra.creator_name;
    if (extra?.creatorName) return extra.creatorName;
    if (extra?.username) return extra.username;
    if (extra?.user_name) return extra.user_name;
    if (extra?.author) return extra.author;
    if (extra?.name) return extra.name;

    const serviceLower = (post.service || '').toLowerCase();
    const userIdLower = (post.user || '').toLowerCase();
    const cacheKey = `${serviceLower}:${userIdLower}`;

    const name = creatorsState.creatorsMap.get(cacheKey) || creatorsState.creatorsMap.get(userIdLower);
    if (name) return name;

    const cached = contentState.creators[cacheKey];
    if (cached?.profile?.name) return cached.profile.name;

    return post.user || 'Unknown';
  });

  function handleCardClick(event: MouseEvent) {
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

  function handleSelectCheckbox(event: MouseEvent) {
    event.stopPropagation();
    selectionState.toggle('posts', postKey, post, orderedKeys, event.shiftKey, itemsMap);
  }

  function openPost() {
    contentState.seedPost(post);
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
        notify.success(i18n.t('library.removed_from_stash') || 'Removed from stash', post.title || undefined);
      } else {
        await libraryState.save(post, collectionId);
        notify.success(i18n.t('library.added_to_stash') || 'Added to stash', post.title || undefined);
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

  async function handleRemoveFromCurrentCategory(event: MouseEvent) {
    event.stopPropagation();
    event.preventDefault();
    const collectionId = libraryState.selectedCollectionId;
    if (!collectionId) return;

    try {
      if (libraryState.selectedCollection?.kind === 'stash') {
        await libraryState.removeFromStash(collectionId, post);
        notify.success(i18n.t('library.removed_from_stash') || 'Removed from stash', post.title || undefined);
      } else {
        await libraryState.remove(post);
        notify.success(i18n.t('library.removed') || 'Removed from library', post.title || undefined);
      }
    } catch (error) {
      notify.error(i18n.t('library.save_error') || 'Action failed', error);
    }
  }

  function openStashInLibrary(event: MouseEvent, stashId?: string) {
    event.stopPropagation();
    event.preventDefault();
    if (stashId) {
      void libraryState.selectCollection(stashId);
    } else {
      void libraryState.selectCollection(null);
    }
    navigationState.navigateRoot('library');
  }
</script>

<article
  class="grid-tile"
  class:selected={selected}
  style:aspect-ratio={ratio}
  data-post-key={postKey}
>
  <button class="grid-tile-open" type="button" onclick={handleCardClick} aria-label={cleanPostTitle(post.title)}></button>

  {#if isSelectionActive}
    <button
      type="button"
      class="grid-tile-select-checkbox"
      class:checked={selected}
      onclick={handleSelectCheckbox}
      aria-label="Select post"
    >
      {#if selected}
        <IconCheckmark class="w-[14px] h-[14px]" />
      {/if}
    </button>
  {:else}
    {#if saved && customStashNames.length > 0}
      <div class="grid-tile-stash-pills">
        {#each customStashes as stashId, i}
          {@const name = customStashNames[i] || stashId}
          <button
            type="button"
            class="grid-tile-stash-pill"
            onclick={(e) => openStashInLibrary(e, stashId)}
            use:tooltip={i18n.t('library.open_stash', { name }) || `Open stash: ${name}`}
            aria-label={name}
          >
            <span class="stash-pill-text">{name}</span>
          </button>
        {/each}
      </div>
    {/if}

    <div class="grid-tile-top-actions">
      {#if isInsideSpecificLibraryCategory}
        <button
          type="button"
          class="grid-tile-action grid-tile-action-danger"
          disabled={saving}
          onclick={handleRemoveFromCurrentCategory}
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
      {:else}
        <Select
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
              class="grid-tile-action"
              class:saved={!isInsideLibrary && saved}
              class:in-library={isInsideLibrary}
              disabled={saving}
              onclick={(e) => {
                e.stopPropagation();
                e.preventDefault();
                toggle();
              }}
              use:tooltip={cardActionTooltip}
              aria-label={cardActionTooltip}
              aria-expanded={open}
            >
              {#if saving}
                <IconLoading />
              {:else if isInsideLibrary}
                <IconBookmarkMultiple />
              {:else if saved}
                <IconSaved />
              {:else}
                <IconSave />
              {/if}
            </button>
          {/snippet}
        </Select>
      {/if}
    </div>
  {/if}

  {#if thumbnailUrl}
    <img
      class="grid-tile-media"
      src={thumbnailUrl}
      alt=""
      loading="lazy"
      decoding="async"
      onerror={(e) => {
        const target = e.currentTarget as HTMLImageElement;
        if (mediaUrl && target.src !== mediaUrl) {
          target.src = mediaUrl;
        } else {
          target.style.display = 'none';
        }
      }}
    />
  {:else if video}
    <div class="grid-tile-placeholder"><IconVideo /></div>
  {:else if mediaUrl}
    <img class="grid-tile-media" src={mediaUrl} alt="" loading="lazy" decoding="async" />
  {:else}
    <div class="grid-tile-placeholder"><IconImage /></div>
  {/if}

  <div class="grid-tile-shade"></div>
  <h2 class="grid-tile-title">{cleanPostTitle(post.title) || i18n.t('feed.untitled')}</h2>

  <div class="grid-tile-footer">
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

      {#if showCreator}
        <span
          role="link"
          tabindex="0"
          class="grid-tile-author-name"
          onclick={openCreator}
          onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && openCreator(e as unknown as MouseEvent)}
        >
          {creatorName}
        </span>
      {/if}
    </div>

    <div class="grid-tile-meta">
      <span>{formatDate(post.published || post.added)}</span>
      <div class="grid-tile-meta-stats">
        <span class="grid-tile-meta-row"><IconAttach /> {attachments}</span>
        {#if post.favorite_count !== undefined && post.favorite_count > 0}
          <span class="grid-tile-meta-row"><IconHeart /> {post.favorite_count}</span>
        {/if}
      </div>
    </div>
  </div>
</article>

<style>
  :global(.card-stash-select) {
    width: auto !important;
    max-width: none !important;
  }
</style>
