<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { contentState, creatorCacheKey, type CachedCreator } from '$lib/state/contentState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { configState } from '$lib/state/configState.svelte';
  import { i18n } from '$lib/i18n';
  import { accountState } from '$lib/state/accountState.svelte';
  import { libraryState } from '$lib/state/libraryState.svelte';
  import { subscriptionState } from '$lib/state/subscriptionState.svelte';
  import { themeState, getContrastColor } from '$lib/theme/themeState.svelte';
  import {
    apiFetchCreatorArtworkDataUrl,
    apiFetchCreatorPosts,
    apiOpenInBrowser,
    apiSetCreatorFavorite
  } from '$lib/utils/ipc';
  import { creatorAvatarUrl, creatorBannerUrl, creatorPageUrl, matchesPostFormat } from '$lib/utils/media';
  import type { DownloadScope, InitialImport } from '$lib/types/subscription';
  import type { PawchivePost } from '$lib/types/pawchive';
  import PageShell from '$lib/components/layout/PageShell.svelte';
  import StickyHeader from '$lib/components/layout/StickyHeader.svelte';
  import HeroBackdrop from '$lib/components/ui/HeroBackdrop.svelte';
  import SearchBar from '$lib/components/ui/SearchBar.svelte';
  import PostGrid from './PostGrid.svelte';
  import ServiceIcon from './ServiceIcon.svelte';
  import pawchiveLogo from './pawchive-favicon.png';
  import Select from '$lib/components/ui/Select.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Checkbox from '$lib/components/ui/Checkbox.svelte';
  import PopoverMenu from '$lib/components/ui/PopoverMenu.svelte';
  import { selectionState } from '$lib/state/selectionState.svelte';
  import { downloadState } from '$lib/state/downloadState.svelte';
  import { getPostDownloadTargets } from '$lib/utils/media';
  import { apiSetPostFavorite } from '$lib/utils/ipc';
  import SelectionActionBar from '$lib/components/ui/SelectionActionBar.svelte';
  import IconArrowLeft from '~icons/fluent/arrow-left-24-regular';
  import IconAdd from '~icons/fluent/add-24-regular';
  import IconSettings from '~icons/fluent/settings-24-regular';
  import IconGrid from '~icons/fluent/grid-24-regular';
  import IconDelete from '~icons/fluent/delete-24-regular';
  import IconHeart from '~icons/fluent/heart-24-regular';
  import IconHeartFilled from '~icons/fluent/heart-24-filled';
  import IconSearch from '~icons/fluent/search-24-regular';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';
  import IconCheckboxChecked from '~icons/fluent/checkbox-checked-24-regular';
  import IconFolder from '~icons/fluent/folder-24-regular';
  import IconArrowDownload from '~icons/fluent/arrow-download-24-regular';
  import IconBookmarkAdd from '~icons/fluent/bookmark-add-24-regular';
  import IconOptions from '~icons/fluent/options-24-regular';
  import IconImage from '~icons/fluent/image-24-regular';
  import IconVideo from '~icons/fluent/video-24-regular';
  import IconMusic from '~icons/fluent/music-note-2-24-regular';
  import IconText from '~icons/fluent/document-text-24-regular';
  import IconDocument from '~icons/fluent/document-24-regular';
  import { toast } from 'svelte-sonner';

  interface Props {
    service: string;
    creatorId: string;
  }

  let { service, creatorId }: Props = $props();

  const savedState = navigationState.getViewState<{
    postSearchQuery?: string;
    postSearchOpen?: boolean;
    postSearchResults?: PawchivePost[];
    selectedFormats?: string[];
    onlyWithAttachments?: boolean;
  }>(navigationState.entryKey);

  const emptyEntry: CachedCreator = {
    profile: null,
    posts: [],
    loading: false,
    loadingMore: false,
    loaded: false,
    error: null,
    offset: 0,
    hasMore: true
  };

  let entry = $derived.by(() => contentState.creators[creatorCacheKey(service, creatorId)] ?? emptyEntry);
  let subscription = $derived(subscriptionState.forCreator(service, creatorId));
  let subscriptionMenuOpen = $state(false);
  let saving = $state(false);
  let destination = $state('00000000-0000-0000-0000-000000000001');
  let initialImport = $state<InitialImport>('none');
  let autoDownload = $state(false);
  let downloadScope = $state<DownloadScope>('primary');
  let interval = $state(30);
  let authenticated = $derived(accountState.session.authenticated);
  let isFavorited = $state(false);
  let favoritingPending = $state(false);

  let postSearchOpen = $state(savedState?.postSearchOpen ?? Boolean(savedState?.postSearchQuery));
  let postSearchQuery = $state(savedState?.postSearchQuery ?? '');
  let postSearchResults = $state<PawchivePost[]>(savedState?.postSearchResults ?? []);
  let selectedFormats = $state<string[]>(savedState?.selectedFormats ?? []);
  let onlyWithAttachments = $state<boolean>(savedState?.onlyWithAttachments ?? false);
  let filtersOpen = $state(false);
  let stickyFiltersOpen = $state(false);
  let activeFilterCount = $derived(selectedFormats.length + (onlyWithAttachments ? 1 : 0));

  let postSearchLoading = $state(false);
  let postSearchError = $state<string | null>(null);
  let postSearchOffset = $state(0);
  let postSearchHasMore = $state(false);
  let postSearchRequest = 0;
  const CREATOR_POST_PAGE_SIZE = 50;

  function toggleFormat(fmt: string) {
    if (selectedFormats.includes(fmt)) {
      selectedFormats = selectedFormats.filter((f) => f !== fmt);
    } else {
      selectedFormats = [...selectedFormats, fmt];
    }
  }

  function clearAllFilters() {
    selectedFormats = [];
    onlyWithAttachments = false;
  }

  $effect(() => {
    navigationState.saveViewState(navigationState.entryKey, {
      postSearchQuery,
      postSearchOpen,
      postSearchResults,
      selectedFormats,
      onlyWithAttachments
    });
  });

  let creatorName = $derived(typeof entry.profile?.name === 'string' ? entry.profile.name : creatorId);
  let cachedAvatarUrl = $state<string | null>(null);
  let cachedBannerUrl = $state<string | null>(null);
  let avatarUrl = $derived(cachedAvatarUrl || creatorAvatarUrl(service, creatorId));
  let bannerUrl = $derived(cachedBannerUrl || creatorBannerUrl(service, creatorId));
  let avatarFailed = $state(false);
  let bannerFailed = $state(false);
  let effectiveAvatar = $derived(avatarFailed ? null : avatarUrl);
  let effectiveBanner = $derived(bannerFailed ? null : bannerUrl);

  $effect(() => {
    if (bannerUrl) {
      bannerFailed = false;
      const img = new Image();
      img.src = bannerUrl;
      img.onerror = () => {
        bannerFailed = true;
      };
    }
  });
  let initialLetter = $derived(creatorName ? creatorName.charAt(0).toUpperCase() : '?');
  let normalizedPostSearch = $derived(postSearchQuery.trim().toLocaleLowerCase());
  let visibleCreatorPosts = $derived.by(() => {
    let posts = entry.posts;
    if (normalizedPostSearch) {
      if (normalizedPostSearch.length >= 2) {
        posts = postSearchResults;
      } else {
        posts = entry.posts.filter((post) =>
          [post.title, post.id, post.content, post.substring]
            .some((value) => String(value ?? '').toLocaleLowerCase().includes(normalizedPostSearch))
        );
      }
    }

    if (onlyWithAttachments) {
      posts = posts.filter((post) => (post.attachment_count ?? post.attachments?.length ?? 0) > 0 || Boolean(post.file?.path));
    }

    if (selectedFormats.length > 0) {
      posts = posts.filter((post) => selectedFormats.some((fmt) => matchesPostFormat(post, fmt)));
    }

    return posts;
  });

  async function openPostSearch() {
    postSearchOpen = true;
    await tick();
    document.querySelector<HTMLInputElement>('.creator-post-search-input')?.focus();
  }

  function closePostSearch() {
    postSearchRequest += 1;
    postSearchOpen = false;
    postSearchQuery = '';
    postSearchResults = [];
    postSearchLoading = false;
    postSearchError = null;
    postSearchOffset = 0;
    postSearchHasMore = false;
  }

  async function searchCreatorPosts(reset = false) {
    const query = postSearchQuery.trim();
    if (query.length < 2 || (!reset && postSearchLoading)) return;

    const request = ++postSearchRequest;
    const offset = reset ? 0 : postSearchOffset;
    postSearchLoading = true;
    postSearchError = null;

    try {
      const posts = await apiFetchCreatorPosts(service, creatorId, query, offset);
      if (request !== postSearchRequest || query !== postSearchQuery.trim()) return;

      postSearchResults = reset ? posts : [...postSearchResults, ...posts];
      postSearchOffset = offset + posts.length;
      postSearchHasMore = posts.length === CREATOR_POST_PAGE_SIZE;
    } catch (error) {
      if (request === postSearchRequest) {
        postSearchError = error instanceof Error ? error.message : String(error);
      }
    } finally {
      if (request === postSearchRequest) postSearchLoading = false;
    }
  }

  $effect(() => {
    const query = normalizedPostSearch;
    postSearchRequest += 1;
    postSearchError = null;

    if (query.length < 2) {
      postSearchResults = [];
      postSearchOffset = 0;
      postSearchHasMore = false;
      postSearchLoading = false;
      return;
    }

    const timeout = window.setTimeout(() => void searchCreatorPosts(true), 250);
    return () => window.clearTimeout(timeout);
  });

  onMount(() => {
    void contentState.loadCreator(service, creatorId);
    void apiFetchCreatorArtworkDataUrl(service, creatorId, 'avatar').then((url) => cachedAvatarUrl = url).catch(() => {});
    void apiFetchCreatorArtworkDataUrl(service, creatorId, 'banner').then((url) => cachedBannerUrl = url).catch(() => {});
  });

  $effect(() => {
    if (service && creatorId) {
      void checkFavoriteStatus();
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
        } catch (error) {
          console.warn('Creator artwork color extraction failed:', error);
          resolve('');
        }
      };
      img.onerror = () => resolve('');
      img.src = url;
    });
  }

  async function getCreatorAccentColor(hasBanner: boolean, hasAvatar: boolean) {
    const artworkKinds: Array<'banner' | 'avatar'> = [];
    if (hasBanner) artworkKinds.push('banner');
    if (hasAvatar) artworkKinds.push('avatar');

    for (const artworkKind of artworkKinds) {
      try {
        const dataUrl = await apiFetchCreatorArtworkDataUrl(service, creatorId, artworkKind);
        const color = await getAverageColor(dataUrl);
        if (color) return color;
      } catch (error) {
        console.warn(`Failed to extract creator ${artworkKind} accent:`, error);
      }
    }
    return '';
  }

  $effect(() => {
    const hasBanner = Boolean(effectiveBanner);
    const hasAvatar = Boolean(effectiveAvatar);
    const dynamicAccent = configState.settings.dynamic_accent;
    let cancelled = false;

    if (dynamicAccent && (hasBanner || hasAvatar)) {
      void getCreatorAccentColor(hasBanner, hasAvatar).then((color) => {
        if (!color || cancelled) return;
        const root = document.documentElement;
        root.style.setProperty('--accent-primary', color);
        root.style.setProperty('--accent-primary-hover', color);
        root.style.setProperty('--accent-glow', color.replace('rgb', 'rgba').replace(')', ', 0.35)'));
        root.style.setProperty('--text-on-accent', getContrastColor(color));
      });
    }

    return () => {
      cancelled = true;
      themeState.applyCssTokens();
    };
  });

  async function checkFavoriteStatus() {
    try {
      const favorites = await accountState.fetchFavorites('creator');
      isFavorited = favorites.some((favorite) =>
        String(favorite.id).toLowerCase() === creatorId.toLowerCase() &&
        String(favorite.service ?? '').toLowerCase() === service.toLowerCase()
      );
    } catch (error) {
      console.error('Failed to check creator favorite status:', error);
    }
  }

  async function toggleFavorite() {
    if (favoritingPending) return;

    favoritingPending = true;
    const targetState = !isFavorited;
    try {
      await apiSetCreatorFavorite(service, creatorId, targetState);
      isFavorited = targetState;

      if (targetState) {
        accountState.addCreatorFavoriteOptimistic({ id: creatorId, service, name: creatorName });
      } else {
        accountState.removeCreatorFavoriteOptimistic(service, creatorId);
      }

      if (!authenticated) {
        toast.success(i18n.t(targetState ? 'favorites.saved_locally' : 'favorites.removed_locally'));
      } else {
        toast.success(i18n.t(targetState ? 'post.added_to_favorites' : 'post.removed_from_favorites'));
      }
    } catch (error) {
      toast.error(i18n.t('post.favorite_failed'), { description: String(error) });
    } finally {
      favoritingPending = false;
    }
  }

  function initEditorFields() {
    if (subscription) {
      destination = subscription.destination_collection_id;
      initialImport = subscription.initial_import;
      autoDownload = subscription.auto_download;
      downloadScope = subscription.download_scope;
      interval = subscription.poll_interval_minutes;
    }
  }

  async function subscribeDefault() {
    saving = true;
    try {
      await subscriptionState.save({
        service,
        creator_id: creatorId,
        creator_name: creatorName,
        destination_collection_id: '00000000-0000-0000-0000-000000000001',
        initial_import: 'none',
        auto_download: false,
        download_scope: 'primary',
        poll_interval_minutes: 30
      });
      toast.success(i18n.t('subscriptions.saved'));
    } catch (error) {
      toast.error(i18n.t('subscriptions.action_error'), { description: String(error) });
    } finally {
      saving = false;
    }
  }

  async function saveSubscription() {
    saving = true;
    try {
      await subscriptionState.save({
        service,
        creator_id: creatorId,
        creator_name: creatorName,
        destination_collection_id: destination,
        initial_import: initialImport,
        auto_download: autoDownload,
        download_scope: downloadScope,
        poll_interval_minutes: interval
      });
      subscriptionMenuOpen = false;
      toast.success(i18n.t('subscriptions.saved'));
    } catch (error) {
      toast.error(i18n.t('subscriptions.action_error'), { description: String(error) });
    } finally {
      saving = false;
    }
  }

  async function unsubscribe() {
    if (!subscription) return;
    saving = true;
    try {
      await subscriptionState.remove(subscription.id);
      subscriptionMenuOpen = false;
      toast.success(i18n.t('library.removed') || 'Unsubscribed successfully');
    } catch (error) {
      toast.error(i18n.t('subscriptions.action_error'), { description: String(error) });
    } finally {
      saving = false;
    }
  }

  function openInBrowser() {
    const url = creatorPageUrl(service, creatorId);
    void apiOpenInBrowser(url).catch((err) => console.warn('Failed to open creator URL in browser:', err));
  }

  let isSelectionActive = $derived(selectionState.active && selectionState.scope === 'posts');
  let selectedPosts = $derived(isSelectionActive ? selectionState.getItems<PawchivePost>() : []);
  let stashes = $derived(libraryState.collections.filter((c) => c.kind === 'stash'));
  let stashOptions = $derived(stashes.map((s) => ({ value: s.id, label: s.name })));

  let batchSelectedStashes = $derived.by(() => {
    if (selectedPosts.length === 0) return [];
    const stashCounts = new Map<string, number>();
    for (const post of selectedPosts) {
      const ids = libraryState.getCustomPostStashes(post);
      for (const id of ids) {
        stashCounts.set(id, (stashCounts.get(id) || 0) + 1);
      }
    }
    const result: string[] = [];
    for (const [id, count] of stashCounts.entries()) {
      if (count === selectedPosts.length) {
        result.push(id);
      }
    }
    return result;
  });

  function handleSelectAllPosts() {
    selectionState.selectAll(visibleCreatorPosts.map((p) => ({
      key: `${(p.service || '').toLowerCase()}:${p.user}:${p.id}`,
      item: p
    })));
  }

  async function handleBatchToggleStash(collectionId: string) {
    const items = selectionState.getItems<PawchivePost>();
    if (items.length === 0 || !collectionId) return;
    const isAllIn = batchSelectedStashes.includes(collectionId);
    try {
      if (isAllIn) {
        for (const p of items) {
          await libraryState.removeFromStash(collectionId, p);
        }
        toast.success(i18n.t('library.removed_from_stash') || 'Removed from stash');
      } else {
        for (const p of items) {
          await libraryState.save(p, collectionId);
        }
        toast.success(i18n.t('library.added_to_stash') || 'Added to stash');
      }
    } catch (error) {
      toast.error(error instanceof Error ? error.message : String(error));
    }
  }

  async function handleBatchCreateAndAddToStash(name: string) {
    const items = selectionState.getItems<PawchivePost>();
    if (items.length === 0 || !name.trim()) return;
    try {
      const newStash = await libraryState.createStash(name.trim());
      for (const p of items) {
        await libraryState.save(p, newStash.id);
      }
      toast.success(i18n.t('library.added_to_stash') || 'Added to stash');
    } catch (error) {
      toast.error(error instanceof Error ? error.message : String(error));
    }
  }

  async function batchSaveToLibrary() {
    const items = selectionState.getItems<PawchivePost>();
    if (items.length === 0) return;
    try {
      for (const post of items) {
        await libraryState.save(post);
      }
      toast.success(i18n.t('selection.save_to_library') || `Saved ${items.length} posts to library`);
      selectionState.exit();
    } catch (err) {
      toast.error(String(err));
    }
  }

  async function batchDownloadPosts() {
    const items = selectionState.getItems<PawchivePost>();
    if (items.length === 0) return;
    let count = 0;
    try {
      for (const post of items) {
        const targets = getPostDownloadTargets(post);
        for (const target of targets) {
          await downloadState.start(post, target.mediaId, target.url, target.filename);
          count++;
        }
      }
      toast.success(i18n.t('selection.download_all') || `Queued ${count} files for download`);
      selectionState.exit();
    } catch (err) {
      toast.error(String(err));
    }
  }

  async function batchFavoritePosts(isFav: boolean) {
    const items = selectionState.getItems<PawchivePost>();
    if (items.length === 0) return;
    try {
      for (const post of items) {
        await apiSetPostFavorite(post.service, post.user, post.id, isFav);
      }
      toast.success(i18n.t(isFav ? 'selection.favorite' : 'selection.unfavorite') || `Updated favorites for ${items.length} posts`);
      await accountState.fetchFavorites('post');
      selectionState.exit();
    } catch (err) {
      toast.error(String(err));
    }
  }
</script>

{#snippet subscriptionEditorFields()}
  <div class="editor-grid">
    <label class="editor-field">
      <span class="field-label">{i18n.t('subscriptions.destination')}</span>
      <Select
        options={libraryState.collections.map(c => ({ value: c.id, label: c.name }))}
        value={destination}
        onchange={(val) => destination = val}
      />
    </label>

    <label class="editor-field">
      <span class="field-label">{i18n.t('subscriptions.initial_import')}</span>
      <Select
        options={[
          { value: 'none', label: i18n.t('subscriptions.import_none') },
          { value: 'latest', label: i18n.t('subscriptions.import_latest') },
          { value: 'all', label: i18n.t('subscriptions.import_all') }
        ]}
        value={initialImport}
        onchange={(val) => initialImport = val as InitialImport}
      />
    </label>

    <label class="editor-field">
      <span class="field-label">{i18n.t('subscriptions.interval')}</span>
      <Select
        options={[
          { value: 15, label: '15 min' },
          { value: 30, label: '30 min' },
          { value: 60, label: '1 h' },
          { value: 360, label: '6 h' }
        ]}
        value={interval}
        onchange={(val) => interval = Number(val)}
      />
    </label>

    <span class="filter-label section-label">{i18n.t('feed.filters')}</span>
    <div class="view-option" class:active={autoDownload}>
      <Checkbox checked={autoDownload} onchange={(checked) => autoDownload = checked} />
      <button type="button" onclick={() => autoDownload = !autoDownload}>
        <strong>{i18n.t('subscriptions.auto_download')}</strong>
        <small>{i18n.t('subscriptions.auto_download_desc')}</small>
      </button>
      <IconArrowDownload class="view-option-icon w-[20px] h-[20px]" />
    </div>

    {#if autoDownload}
      <label class="editor-field">
        <span class="field-label">{i18n.t('subscriptions.download_scope')}</span>
        <Select
          options={[
            { value: 'primary', label: i18n.t('subscriptions.primary_file') },
            { value: 'all', label: i18n.t('subscriptions.all_files') }
          ]}
          value={downloadScope}
          onchange={(val) => downloadScope = val as DownloadScope}
        />
      </label>
    {/if}
  </div>

  <div class="editor-footer">
    <div class="footer-buttons">
      <Button
        variant="danger"
        onclick={() => { subscriptionMenuOpen = false; void unsubscribe(); }}
        class="unsubscribe-btn"
        disabled={saving}
        title={i18n.t('selection.unsubscribe') || 'Unsubscribe'}
      >
        <IconDelete class="w-[16px] h-[16px]" />
      </Button>
      <Button
        variant="accent"
        onclick={() => { subscriptionMenuOpen = false; void saveSubscription(); }}
        class="save-sub-btn"
        disabled={saving}
      >
        {i18n.t('subscriptions.save')}
      </Button>
    </div>
  </div>
{/snippet}

{#snippet subscriptionControl()}
  {#if !subscription?.enabled}
    <Button
      variant="primary"
      onclick={subscribeDefault}
      class="action-btn"
      disabled={saving}
      title={i18n.t('subscriptions.subscribe')}
    >
      <IconAdd class="w-[18px] h-[18px]" />
      <span class="btn-text">{i18n.t('subscriptions.subscribe')}</span>
    </Button>
  {:else}
    <PopoverMenu
      bind:open={subscriptionMenuOpen}
      title={i18n.t('subscriptions.settings')}
      icon={IconSettings}
      active={true}
      width="320px"
    >
      {#snippet trigger()}
        <Button
          variant="accent"
          onclick={() => { initEditorFields(); subscriptionMenuOpen = !subscriptionMenuOpen; }}
          class="action-btn"
          title={i18n.t('subscriptions.settings')}
          aria-label={i18n.t('subscriptions.settings')}
        >
          <IconSettings class="w-[18px] h-[18px]" />
          <span class="btn-text">{i18n.t('subscriptions.settings')}</span>
        </Button>
      {/snippet}

      {@render subscriptionEditorFields()}
    </PopoverMenu>
  {/if}
{/snippet}

{#snippet filterInnerContent()}
  <div class="filter-header">
    <span class="filter-title">{i18n.t('feed.filters')}</span>
    {#if activeFilterCount > 0}
      <button class="reset-btn" onclick={clearAllFilters}>
        <IconDismiss class="w-[14px] h-[14px]" />
        <span>{i18n.t('feed.reset_filters')}</span>
      </button>
    {/if}
  </div>

  <span class="filter-label">{i18n.t('feed.format') || 'Format'}</span>
  <div class="service-options">
    <Button
      variant={selectedFormats.includes('image') ? 'accent' : 'ghost'}
      size="sm"
      onclick={() => toggleFormat('image')}
      class="filter-chip"
    >
      <IconImage class="w-[14px] h-[14px]" />
      <span>{i18n.t('feed.format_photo') || 'Photo'}</span>
    </Button>

    <Button
      variant={selectedFormats.includes('video') ? 'accent' : 'ghost'}
      size="sm"
      onclick={() => toggleFormat('video')}
      class="filter-chip"
    >
      <IconVideo class="w-[14px] h-[14px]" />
      <span>{i18n.t('feed.format_video') || 'Video'}</span>
    </Button>

    <Button
      variant={selectedFormats.includes('audio') ? 'accent' : 'ghost'}
      size="sm"
      onclick={() => toggleFormat('audio')}
      class="filter-chip"
    >
      <IconMusic class="w-[14px] h-[14px]" />
      <span>{i18n.t('feed.format_audio') || 'Audio'}</span>
    </Button>

    <Button
      variant={selectedFormats.includes('text') ? 'accent' : 'ghost'}
      size="sm"
      onclick={() => toggleFormat('text')}
      class="filter-chip"
    >
      <IconText class="w-[14px] h-[14px]" />
      <span>{i18n.t('feed.format_text') || 'Text'}</span>
    </Button>

    <Button
      variant={selectedFormats.includes('archive') ? 'accent' : 'ghost'}
      size="sm"
      onclick={() => toggleFormat('archive')}
      class="filter-chip"
    >
      <IconDocument class="w-[14px] h-[14px]" />
      <span>{i18n.t('feed.format_archive') || 'Files'}</span>
    </Button>
  </div>

  <span class="filter-label section-label">{i18n.t('feed.filters')}</span>
  <div class="view-option" class:active={onlyWithAttachments}>
    <Checkbox
      checked={onlyWithAttachments}
      onchange={(v) => onlyWithAttachments = v}
    />
    <button onclick={() => onlyWithAttachments = !onlyWithAttachments}>
      <strong>{i18n.t('feed.with_attachments')}</strong>
      <small>{i18n.t('feed.with_attachments_desc')}</small>
    </button>
    <IconDocument class="view-option-icon w-[20px] h-[20px]" />
  </div>
{/snippet}

<PageShell scrollable={true} scrollKey={navigationState.entryKey}>
  {#snippet overlay()}
    <StickyHeader threshold={120} back={true}>
      {#snippet leading()}
        <div class="flex items-center gap-2 min-w-0">
          <div class="sticky-avatar">
            {#if effectiveAvatar}
              <img src={effectiveAvatar} alt={creatorName} onerror={() => avatarFailed = true} />
            {:else}
              <span class="sticky-initial">{initialLetter}</span>
            {/if}
          </div>
          <span class="sticky-creator-title truncate">{creatorName}</span>
        </div>
      {/snippet}

      {#snippet trailing()}
        <div class="flex items-center gap-2">
          <Button
            variant={isFavorited ? 'accent' : 'ghost'}
            disabled={favoritingPending}
            onclick={toggleFavorite}
            class="sticky-action-btn"
            title={i18n.t(isFavorited ? 'post.unfavorite' : 'post.favorite')}
          >
            {#if isFavorited}
              <IconHeartFilled class="w-5 h-5 fav-active-heart" />
            {:else}
              <IconHeart class="w-5 h-5" />
            {/if}
            <span class="btn-text">{i18n.t(isFavorited ? 'post.unfavorite' : 'post.favorite')}</span>
          </Button>

          <SearchBar
            bind:value={postSearchQuery}
            bind:open={postSearchOpen}
            placeholder={i18n.t('feed.search_placeholder')}
            onclose={closePostSearch}
          />

          <PopoverMenu
            bind:open={stickyFiltersOpen}
            title={i18n.t('feed.filters')}
            icon={IconOptions}
            badge={activeFilterCount}
            active={activeFilterCount > 0}
          >
            {@render filterInnerContent()}
          </PopoverMenu>

          <Button
            variant={isSelectionActive ? 'accent' : 'ghost'}
            class="btn-icon"
            onclick={() => (isSelectionActive ? selectionState.exit() : selectionState.enter('posts'))}
            title={i18n.t('selection.select_mode') || 'Select mode'}
            aria-label="Select mode"
          >
            <IconCheckboxChecked class="w-5 h-5" />
          </Button>
        </div>
      {/snippet}
    </StickyHeader>
  {/snippet}

  <HeroBackdrop src={effectiveBanner} />

  <div class="creator-content-wrapper">
    <div class="creator-actions-bar">
      <div class="left-actions">
        <Button variant="ghost" onclick={() => navigationState.back()} class="action-btn">
          <IconArrowLeft class="w-[18px] h-[18px]" />
          <span>{i18n.t('nav.back')}</span>
        </Button>

        <Button
          variant={isFavorited ? 'accent' : 'ghost'}
          disabled={favoritingPending}
          onclick={toggleFavorite}
          class="action-btn"
          title={i18n.t(isFavorited ? 'post.unfavorite' : 'post.favorite')}
        >
          {#if isFavorited}
            <IconHeartFilled class="w-[18px] h-[18px] fav-active-heart" />
          {:else}
            <IconHeart class="w-[18px] h-[18px]" />
          {/if}
          <span>{i18n.t(isFavorited ? 'post.unfavorite' : 'post.favorite')}</span>
        </Button>
      </div>

      <div class="right-actions">
        <SearchBar
          bind:value={postSearchQuery}
          bind:open={postSearchOpen}
          placeholder={i18n.t('feed.search_placeholder')}
          onclose={closePostSearch}
        />

        <PopoverMenu
          bind:open={filtersOpen}
          title={i18n.t('feed.filters')}
          icon={IconOptions}
          badge={activeFilterCount}
          active={activeFilterCount > 0}
        >
          {@render filterInnerContent()}
        </PopoverMenu>

        <Button
          variant={isSelectionActive ? 'accent' : 'ghost'}
          class="btn-icon"
          onclick={() => (isSelectionActive ? selectionState.exit() : selectionState.enter('posts'))}
          title={i18n.t('selection.select_mode') || 'Select mode'}
          aria-label="Select mode"
        >
          <IconCheckboxChecked class="w-5 h-5" />
        </Button>

        {@render subscriptionControl()}

        <Button
          variant="ghost"
          onclick={openInBrowser}
          class="action-btn"
          title={i18n.t('post.open_in_browser')}
        >
          <img src={pawchiveLogo} alt="" class="pawchive-action-icon" />
          <span>{i18n.t('post.open_in_browser')}</span>
        </Button>
      </div>
    </div>

    <header class="detail-header">
      <div class="creator-avatar-container">
        {#if effectiveAvatar}
          <img
            src={effectiveAvatar}
            alt={creatorName}
            class="creator-avatar-img"
            onerror={() => avatarFailed = true}
          />
        {:else}
          <div class="creator-avatar-initial">
            {initialLetter}
          </div>
        {/if}
      </div>

      <div class="creator-info-block">
        <div class="creator-title-row">
          <h1>{creatorName}</h1>
        </div>

        <p class="creator-subtitle">
          <span class="platform-name">
            <ServiceIcon {service} class="platform-icon" />
            <span>{service}</span>
          </span>
          <span class="dot-sep">•</span>
          <span class="creator-id-tag">{creatorId}</span>
          {#if entry.posts.length > 0}
            <span class="dot-sep">•</span>
            <span class="post-count-tag">
              <IconGrid class="w-[14px] h-[14px] inline-block" />
              {entry.posts.length}{entry.hasMore ? '+' : ''} {i18n.t('favorites.posts')}
            </span>
          {/if}
        </p>
      </div>
    </header>

    <div class="creator-posts-section">
      {#if postSearchError && visibleCreatorPosts.length === 0}
        <div class="creator-error">{postSearchError}</div>
      {:else if entry.error && entry.posts.length === 0}
        <div class="creator-error">{entry.error}</div>
      {:else}
        <PostGrid
          posts={visibleCreatorPosts}
          loading={normalizedPostSearch.length >= 2 ? postSearchLoading : (entry.loading || entry.loadingMore)}
          hasMore={normalizedPostSearch.length >= 2 ? postSearchHasMore : (!normalizedPostSearch && entry.hasMore)}
          onLoadMore={() => normalizedPostSearch.length >= 2
            ? searchCreatorPosts()
            : (!normalizedPostSearch ? contentState.loadMoreCreatorPosts(service, creatorId) : undefined)}
          stateKey={`creator:${service}:${creatorId}:${normalizedPostSearch}`}
          paginationKey={visibleCreatorPosts.length}
          showCreator={false}
        />
      {/if}
    </div>
  </div>
</PageShell>

<SelectionActionBar
  totalCount={visibleCreatorPosts.length}
  onSelectAll={handleSelectAllPosts}
>
  <Select
    options={stashOptions}
    selectedValues={batchSelectedStashes}
    placeholder={i18n.t('library.add_to_stash')}
    onchange={handleBatchToggleStash}
    createLabel={i18n.t('library.new_stash')}
    onCreate={handleBatchCreateAndAddToStash}
    variant={batchSelectedStashes.length > 0 ? 'accent' : 'ghost'}
    multi={true}
    closeOnChange={false}
    icon={IconFolder}
    class="selection-stash-select"
  />

  <Button
    variant="ghost"
    size="sm"
    class="selection-btn"
    onclick={batchSaveToLibrary}
    title={i18n.t('selection.save_to_library')}
  >
    <IconBookmarkAdd class="w-[16px] h-[16px]" />
    <span>{i18n.t('selection.save_to_library')}</span>
  </Button>

  <Button
    variant="ghost"
    size="sm"
    class="selection-btn"
    onclick={batchDownloadPosts}
    title={i18n.t('selection.download_all')}
  >
    <IconArrowDownload class="w-[16px] h-[16px]" />
    <span>{i18n.t('selection.download_all')}</span>
  </Button>

  <Button
    variant="ghost"
    size="sm"
    class="selection-btn"
    onclick={() => void batchFavoritePosts(true)}
    title={i18n.t('selection.favorite')}
  >
    <IconHeartFilled class="w-[16px] h-[16px] text-accent" />
    <span>{i18n.t('selection.favorite')}</span>
  </Button>
</SelectionActionBar>

<style>
  .creator-content-wrapper {
    position: relative;
    z-index: 2;
    display: flex;
    flex-direction: column;
    width: 100%;
  }

  .creator-actions-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 12px;
    margin-bottom: 24px;
    width: 100%;
  }

  .left-actions,
  .right-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .right-actions {
    margin-left: auto;
  }

  .creator-actions-bar :global(.action-btn) {
    height: 44px !important;
    padding: 0 18px !important;
    font-size: 13.5px !important;
    border-radius: var(--radius-full) !important;
    gap: 8px !important;
  }

  @media (max-width: 640px) {
    .creator-actions-bar :global(.action-btn span) {
      display: none !important;
    }
    .creator-actions-bar :global(.action-btn) {
      width: 44px !important;
      height: 44px !important;
      min-width: 44px !important;
      padding: 0 !important;
      border-radius: 50% !important;
      display: inline-flex !important;
      align-items: center !important;
      justify-content: center !important;
    }
  }



  .editor-grid {
    display: flex;
    flex-direction: column;
    gap: 14px;
    width: 100%;
  }

  .editor-field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: 100%;
  }

  .field-label {
    color: var(--text-secondary, rgba(255, 255, 255, 0.55));
    font-size: 12px;
    font-weight: 550;
  }

  .editor-footer {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding-top: 14px;
    border-top: 1px solid rgba(255, 255, 255, 0.08);
    width: 100%;
  }

  .footer-buttons {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    width: 100%;
  }

  .editor-footer :global(.save-sub-btn) {
    height: 40px !important;
    padding: 0 20px !important;
    border-radius: var(--radius-full) !important;
    font-weight: 600 !important;
    flex: 1;
  }

  .editor-footer :global(.unsubscribe-btn) {
    height: 40px !important;
    width: 40px !important;
    min-width: 40px !important;
    padding: 0 !important;
    border-radius: 50% !important;
    display: flex !important;
    align-items: center !important;
    justify-content: center !important;
  }

  .detail-header {
    display: flex;
    align-items: center;
    gap: 20px;
    margin-bottom: 32px;
    padding-bottom: 24px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .creator-avatar-container {
    position: relative;
    width: 72px;
    height: 72px;
    flex-shrink: 0;
  }

  .creator-avatar-img {
    width: 100%;
    height: 100%;
    border-radius: 0 !important;
    object-fit: cover;
    border: none !important;
    box-shadow: none !important;
  }

  .creator-avatar-initial {
    width: 100%;
    height: 100%;
    border-radius: 0 !important;
    display: grid;
    place-items: center;
    font-family: var(--font-sans);
    font-size: 28px;
    font-weight: 700;
    color: rgba(255, 255, 255, 0.9);
    background: rgba(255, 255, 255, 0.08);
    border: none !important;
    box-shadow: none !important;
  }

  .creator-info-block {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
    flex: 1;
  }

  .creator-title-row {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }

  .creator-title-row h1,
  .creator-id-tag {
    user-select: text;
    cursor: text;
  }

  h1 {
    margin: 0;
    color: var(--text-primary, #ffffff);
    font-family: var(--font-sans);
    font-size: clamp(26px, 4vw, 38px);
    font-weight: 700;
    line-height: 1.15;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .creator-subtitle {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    color: var(--text-secondary, rgba(255, 255, 255, 0.6));
    font-size: 13.5px;
    flex-wrap: wrap;
  }

  .platform-name {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    text-transform: capitalize;
    font-weight: 600;
    color: var(--text-primary, #ffffff);
  }

  :global(.platform-icon) {
    width: 15px !important;
    height: 15px !important;
    flex-shrink: 0;
    color: rgba(255, 255, 255, 0.85);
  }

  .dot-sep {
    opacity: 0.4;
  }

  .post-count-tag {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }

  .sticky-avatar {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    overflow: hidden;
    flex-shrink: 0;
    display: grid;
    place-items: center;
    background: rgba(255, 255, 255, 0.1);
  }

  .sticky-avatar img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .sticky-initial {
    font-size: 14px;
    font-weight: 700;
    color: #ffffff;
  }

  .sticky-creator-title {
    font-family: var(--font-sans);
    color: var(--text-primary, #ffffff);
    font-size: 15px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .creator-posts-section {
    width: 100%;
    margin-top: 8px;
  }

  :global(.sticky-action-btn) {
    height: 44px !important;
    padding: 0 18px !important;
    font-size: 13.5px !important;
    border-radius: var(--radius-full) !important;
    gap: 8px !important;
  }

  :global(.sticky-header-bar.is-mobile) :global(.sticky-action-btn .btn-text) {
    display: none !important;
  }

  @media (max-width: 640px) {
    :global(.sticky-action-btn .btn-text) {
      display: none !important;
    }
    :global(.sticky-action-btn) {
      width: 44px !important;
      height: 44px !important;
      min-width: 44px !important;
      padding: 0 !important;
      border-radius: 50% !important;
      display: inline-flex !important;
      align-items: center !important;
      justify-content: center !important;
    }
  }

  .pawchive-action-icon {
    width: 18px;
    height: 18px;
    flex: 0 0 18px;
    object-fit: contain;
    filter: brightness(0) invert(1);
  }

  .creator-error {
    min-height: 300px;
    display: grid;
    place-items: center;
    color: #fca5a5;
    font-size: 13.5px;
  }
</style>
