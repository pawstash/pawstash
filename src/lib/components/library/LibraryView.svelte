<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { i18n } from '$lib/i18n';
  import { libraryState } from '$lib/state/libraryState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { downloadState } from '$lib/state/downloadState.svelte';
  import { configState } from '$lib/state/configState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { themeState } from '$lib/theme/themeState.svelte';
  import type { Post } from '$lib/types/content';
  import { parseDateTimestamp, cleanPostTitle } from '$lib/utils/formatters';
  import PageShell from '$lib/components/layout/PageShell.svelte';
  import PageHeader from '$lib/components/layout/PageHeader.svelte';
  import HeaderActions from '$lib/components/layout/HeaderActions.svelte';
  import PostGrid from '$lib/components/content/PostGrid.svelte';
  import StickyHeader from '$lib/components/layout/StickyHeader.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import Checkbox from '$lib/components/ui/Checkbox.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import PopoverMenu from '$lib/components/ui/PopoverMenu.svelte';
  import CountBadge from '$lib/components/ui/CountBadge.svelte';
  import BottomSheet from '$lib/components/ui/BottomSheet.svelte';
  import ServiceIcon from '$lib/components/content/ServiceIcon.svelte';
  import { ripple } from '$lib/motion';
  import { notify } from '$lib/utils/toast';
  import { selectionState } from '$lib/state/selectionState.svelte';
  import { getPostDownloadTargets, attachmentMediaUrl } from '$lib/utils/media';
  import SelectionActionBar from '$lib/components/ui/SelectionActionBar.svelte';
  import IconCheckmark from '~icons/fluent/checkmark-20-regular';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';
  import IconDelete from '~icons/fluent/delete-24-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';
  import IconArrowClockwise from '~icons/fluent/arrow-clockwise-24-regular';
  import IconArrowSort from '~icons/fluent/arrow-sort-24-regular';
  import IconOptions from '~icons/fluent/options-24-regular';
  import IconGlobe from '~icons/fluent/globe-24-regular';
  import IconImage from '~icons/fluent/image-24-regular';
  import IconVideo from '~icons/fluent/video-24-regular';
  import IconMusic from '~icons/fluent/music-note-2-24-regular';
  import IconText from '~icons/fluent/document-text-24-regular';
  import IconDocument from '~icons/fluent/document-24-regular';
  import IconSearch from '~icons/fluent/search-24-regular';
  import IconEdit from '~icons/fluent/edit-24-regular';
  import IconBroom from '~icons/fluent/broom-24-regular';
  import IconMoreVertical from '~icons/fluent/more-vertical-24-regular';
  import IconCheckboxChecked from '~icons/fluent/checkbox-checked-24-regular';
  import IconFolder from '~icons/fluent/folder-24-regular';
  import IconFolderDismiss from '~icons/fluent/folder-dismiss-24-regular';
  import IconArrowDownload from '~icons/fluent/arrow-download-24-regular';
  import IconDraft from '~icons/fluent/drafts-24-regular';

  import type { FilterMap } from '$lib/types/filter';
  import { countActiveFilters, matchesTriStateFilter, toggleFilterKey } from '$lib/types/filter';
  import { getPostFormats } from '$lib/utils/media';

  const savedState = navigationState.getViewState<{
    searchQuery?: string;
    searchOpen?: boolean;
    serviceFilters?: FilterMap;
    formatFilters?: FilterMap;
    selectedCollectionId?: string | null;
    onlyWithAttachments?: boolean;
    onlyDownloaded?: boolean;
    sortBy?: 'added' | 'published' | 'title';
    sortOrder?: 'asc' | 'desc';
  }>(navigationState.entryKey);

  if (savedState?.selectedCollectionId !== undefined) {
    libraryState.selectedCollectionId = savedState.selectedCollectionId;
  }

  let searchOpen = $state(savedState?.searchOpen ?? Boolean(savedState?.searchQuery));
  let searchQuery = $state(savedState?.searchQuery ?? '');
  let input = $state<HTMLInputElement>();
  let isSelectionActive = $derived(selectionState.active && selectionState.scope === 'posts');
  let selectedPosts = $derived(isSelectionActive ? selectionState.getItems<Post>() : []);
  let stashes = $derived(libraryState.allStashes);
  let stashOptions = $derived(stashes.map((s) => ({ value: s.id, label: libraryState.getStashDisplayName(s) })));

  let batchSelectedStashes = $derived.by(() => {
    if (selectedPosts.length === 0) return [];
    const stashCounts = new Map<string, number>();
    for (const post of selectedPosts) {
      const ids = libraryState.getPostStashes(post);
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

  async function handleBatchToggleStash(collectionId: string) {
    const items = selectionState.getItems<Post>();
    if (items.length === 0 || !collectionId) return;
    const isAllIn = batchSelectedStashes.includes(collectionId);
    try {
      if (isAllIn) {
        for (const p of items) {
          await libraryState.removeFromStash(collectionId, p);
        }
        notify.success(i18n.t('library.removed_from_stash') || 'Removed from stash');
      } else {
        for (const p of items) {
          await libraryState.save(p, collectionId);
        }
        notify.success(i18n.t('library.added_to_stash') || 'Added to stash');
      }
    } catch (error) {
      notify.error(i18n.t('library.save_error') || 'Stash operation failed', error);
    }
  }

  async function handleBatchCreateAndAddToStash(name: string) {
    const items = selectionState.getItems<Post>();
    if (items.length === 0 || !name.trim()) return;
    try {
      const newStash = await libraryState.createStash(name.trim());
      for (const p of items) {
        await libraryState.save(p, newStash.id);
      }
      notify.success(i18n.t('library.added_to_stash') || 'Added to stash', newStash.name);
    } catch (error) {
      notify.error(i18n.t('library.save_error') || 'Failed to create stash', error);
    }
  }

  let isStashSelected = $derived(
    libraryState.selectedCollectionId !== null &&
    libraryState.selectedCollection !== null
  );

  function handleSelectAllPosts() {
    selectionState.selectAll(filteredPosts.map((p) => ({
      key: `${p.service}:${p.user}:${p.id}`,
      item: p
    })));
  }

  async function batchRemoveFromThisStash() {
    const items = selectionState.getItems<Post>();
    const stashId = libraryState.selectedCollectionId;
    if (items.length === 0 || !stashId) return;
    try {
      for (const post of items) {
        await libraryState.removeFromStash(stashId, post);
      }
      notify.success(
        i18n.t('selection.remove_from_stash') || 'Removed from stash',
        `${items.length} ${items.length === 1 ? 'post' : 'posts'}`
      );
      selectionState.exit();
    } catch (err) {
      notify.error(i18n.t('library.save_error') || 'Failed to remove from stash', err);
    }
  }

  async function batchDeleteFromLibrary() {
    const items = selectionState.getItems<Post>();
    if (items.length === 0) return;
    try {
      for (const post of items) {
        await libraryState.remove(post);
      }
      notify.success(
        i18n.t('selection.remove_from_library') || 'Deleted from library',
        `${items.length} ${items.length === 1 ? 'post' : 'posts'}`
      );
      selectionState.exit();
    } catch (err) {
      notify.error(i18n.t('library.save_error') || 'Failed to delete from library', err);
    }
  }

  async function batchDownloadPosts() {
    const items = selectionState.getItems<Post>();
    if (items.length === 0) return;
    try {
      const count = await downloadState.downloadPosts(items);
      notify.success(
        i18n.t('selection.download_all') || 'Queued downloads',
        `${count} ${count === 1 ? 'file' : 'files'}`
      );
      selectionState.exit();
    } catch (err) {
      notify.error(i18n.t('downloads.action_error') || 'Download failed', err);
    }
  }

  let manageOpen = $state(false);
  let stickyManageOpen = $state(false);
  let editStashName = $state('');
  let renamingPending = $state(false);
  let clearingPending = $state(false);

  let filtersOpen = $state(false);
  let stickyFiltersOpen = $state(false);

  let serviceFilters = $state<FilterMap>(savedState?.serviceFilters ?? {});
  let formatFilters = $state<FilterMap>(savedState?.formatFilters ?? {});
  let onlyWithAttachments = $state<boolean>(savedState?.onlyWithAttachments ?? false);
  let onlyDownloaded = $state<boolean>(savedState?.onlyDownloaded ?? false);

  let sortBy = $state<'added' | 'published' | 'title'>(savedState?.sortBy ?? 'added');
  let sortOrder = $state<'asc' | 'desc'>(savedState?.sortOrder ?? 'desc');
  let currentSortValue = $derived(`${sortBy}_${sortOrder}`);

  $effect(() => {
    navigationState.saveViewState(navigationState.entryKey, {
      searchQuery,
      searchOpen,
      serviceFilters: $state.snapshot(serviceFilters),
      formatFilters: $state.snapshot(formatFilters),
      onlyWithAttachments,
      onlyDownloaded,
      sortBy,
      sortOrder,
      selectedCollectionId: libraryState.selectedCollectionId
    });
  });

  let sortOptions = $derived([
    { value: 'added_desc', label: i18n.t('library.sort_added_desc') },
    { value: 'added_asc', label: i18n.t('library.sort_added_asc') },
    { value: 'published_desc', label: i18n.t('library.sort_published_desc') },
    { value: 'published_asc', label: i18n.t('library.sort_published_asc') },
    { value: 'title_asc', label: i18n.t('library.sort_title_asc') },
    { value: 'title_desc', label: i18n.t('library.sort_title_desc') }
  ]);

  let currentSortLabel = $derived(
    sortOptions.find((o) => o.value === currentSortValue)?.label ?? (i18n.t('favorites.sort_by') || 'Sort')
  );

  let mobileMoreOpen = $state(false);
  let mobileManageOpen = $state(false);

  function handleSortChange(val: string) {
    const parts = val.split('_');
    sortBy = parts[0] as any;
    sortOrder = parts[1] as any;
  }

  let basePosts = $derived(libraryState.posts);
  let services = $derived([...new Set(basePosts.map((post) => post.service))].sort());
  let activeFilterCount = $derived(
    countActiveFilters([serviceFilters, formatFilters]) + (onlyWithAttachments ? 1 : 0) + (onlyDownloaded ? 1 : 0)
  );

  onMount(() => {
    void downloadState.init();
    void libraryState.init()
      .then(async () => {
        if (libraryState.posts.length === 0) await libraryState.refresh();
      })
      .catch((error) => libraryState.error = error instanceof Error ? error.message : String(error));
  });

  $effect(() => {
    const stashColor = libraryState.selectedCollection?.color;
    if (stashColor) {
      themeState.setOverrideAccent(stashColor);
    } else {
      themeState.clearOverrideAccent();
    }

    return () => {
      themeState.clearOverrideAccent();
    };
  });

  onDestroy(() => {
    downloadState.destroy();
    themeState.clearOverrideAccent();
  });

  function selectCollection(id: string | null) {
    resetFilters();
    if (selectionState.active) selectionState.clear();
    void libraryState.selectCollection(id);
  }

  function closeSearch() {
    searchQuery = '';
    searchOpen = false;
  }

  let editStashColor = $state<string | null>(null);
  const stashColorPalette = ['#ef4444', '#f97316', '#f59e0b', '#10b981', '#06b6d4', '#3b82f6', '#8b5cf6', '#ec4899'];

  $effect(() => {
    if (manageOpen || stickyManageOpen) {
      editStashName = libraryState.selectedCollection ? libraryState.getStashDisplayName(libraryState.selectedCollection) : '';
      editStashColor = libraryState.selectedCollection?.color || null;
    }
  });

  async function handleSetStashColor(color: string | null) {
    const collectionId = libraryState.selectedCollectionId;
    if (!collectionId) return;
    try {
      editStashColor = color;
      await libraryState.updateStash(collectionId, { color });
      notify.success(i18n.t('settings.stash_color') || 'Stash color updated');
    } catch (error) {
      notify.error(i18n.t('library.save_error') || 'Failed to update color', error);
    }
  }

  async function handleRenameStash(event: SubmitEvent) {
    event.preventDefault();
    const collectionId = libraryState.selectedCollectionId;
    if (!collectionId || !editStashName.trim() || renamingPending) return;
    renamingPending = true;
    try {
      await libraryState.renameStash(collectionId, editStashName.trim());
      notify.success(i18n.t('library.stash_renamed'), editStashName.trim());
      manageOpen = false;
      stickyManageOpen = false;
      mobileManageOpen = false;
    } catch (error) {
      notify.error(i18n.t('library.save_error') || 'Failed to rename stash', error);
    } finally {
      renamingPending = false;
    }
  }

  async function handleClearStash() {
    const collectionId = libraryState.selectedCollectionId;
    if (!collectionId || clearingPending) return;
    const name = libraryState.selectedCollection?.name ?? '';
    const message = i18n.t('library.clear_confirm') || `Are you sure you want to clear "${name}"?`;
    if (!confirm(message)) return;

    clearingPending = true;
    try {
      await libraryState.clearStash(collectionId);
      notify.success(i18n.t('library.stash_cleared'), name || undefined);
      manageOpen = false;
      stickyManageOpen = false;
      mobileManageOpen = false;
    } catch (error) {
      notify.error(i18n.t('library.save_error') || 'Failed to clear stash', error);
    } finally {
      clearingPending = false;
    }
  }

  async function handleDeleteStash() {
    const collectionId = libraryState.selectedCollectionId;
    if (!collectionId) return;
    const name = libraryState.selectedCollection?.name ?? '';
    const message = i18n.t('library.delete_confirm') || `Are you sure you want to delete "${name}"?`;
    
    const confirmed = confirm(message);
    if (!confirmed) return;
    
    try {
      manageOpen = false;
      stickyManageOpen = false;
      mobileManageOpen = false;
      await libraryState.deleteStash(collectionId);
    } catch (error) {
      libraryState.error = error instanceof Error ? error.message : String(error);
    }
  }

  function toggleService(service: string) {
    serviceFilters = toggleFilterKey(serviceFilters, service);
  }

  function toggleFormat(fmt: string) {
    formatFilters = toggleFilterKey(formatFilters, fmt);
  }

  function resetFilters() {
    serviceFilters = {};
    formatFilters = {};
    onlyWithAttachments = false;
    onlyDownloaded = false;
  }

  const formatList = [
    { id: 'image', label: () => i18n.t('feed.format_photo') || 'Photo', icon: IconImage },
    { id: 'video', label: () => i18n.t('feed.format_video') || 'Video', icon: IconVideo },
    { id: 'audio', label: () => i18n.t('feed.format_audio') || 'Audio', icon: IconMusic },
    { id: 'text', label: () => i18n.t('feed.format_text') || 'Text', icon: IconText },
    { id: 'archive', label: () => i18n.t('feed.format_archive') || 'Files', icon: IconDocument },
    { id: 'wip', label: () => i18n.t('feed.format_wip') || 'WIP / Sketch', icon: IconDraft }
  ];

  function isPostDownloaded(post: Post): boolean {
    const file = post.file as any;
    const attachments = (post.attachments as any[]) || [];
    
    const urls: string[] = [];
    if (file && file.path) {
      urls.push(attachmentMediaUrl(file, post.service));
    }
    for (const a of attachments) {
      if (a && a.path) {
        urls.push(attachmentMediaUrl(a, post.service));
      }
    }
    
    return urls.some(url => 
      downloadState.downloads.some(d => d.url === url && d.status === 'completed')
    );
  }

  let filteredPosts = $derived.by(() => {
    let list = basePosts;
    const query = searchQuery.trim().toLocaleLowerCase();

    if (query) {
      list = list.filter((post) =>
        [post.title, post.user, post.service, post.id, post.content]
          .some((value) => String(value ?? '').toLocaleLowerCase().includes(query))
      );
    }

    if (Object.keys(serviceFilters).length > 0) {
      list = list.filter(post => matchesTriStateFilter([post.service], serviceFilters));
    }

    if (Object.keys(formatFilters).length > 0) {
      list = list.filter(post => matchesTriStateFilter(getPostFormats(post), formatFilters));
    }

    if (onlyWithAttachments) {
      list = list.filter(post => {
        const hasAtt = (post.attachment_count ?? post.attachments?.length ?? 0) > 0 || Boolean(post.file?.path);
        return hasAtt;
      });
    }

    if (onlyDownloaded) {
      list = list.filter(post => isPostDownloaded(post));
    }

    list = [...list].sort((a, b) => {
      let comparison = 0;
      if (sortBy === 'title') {
        comparison = (a.title || '').localeCompare(b.title || '', undefined, { sensitivity: 'base' });
      } else if (sortBy === 'published') {
        const valA = parseDateTimestamp(a.published);
        const valB = parseDateTimestamp(b.published);
        comparison = valA - valB;
      } else {
        const valA = a.library_added_at || '';
        const valB = b.library_added_at || '';
        comparison = valA.localeCompare(valB);
      }
      return sortOrder === 'asc' ? comparison : -comparison;
    });
    
    return list;
  });

  $effect(() => {
    if (!searchOpen) return;
    const inputs = [...document.querySelectorAll('.library-search-input')] as HTMLInputElement[];
    inputs.find((element) => element.getClientRects().length > 0)?.focus();
  });
</script>

{#snippet manageStashContent()}
  <div class="filter-heading">
    <strong>{i18n.t('library.manage_stash')}</strong>
    <CountBadge count={libraryState.selectedCollection?.item_count ?? 0} showZero={true} />
  </div>

  <span class="filter-label">{i18n.t('library.stash_name')}</span>
  <form class="stash-rename-row" onsubmit={handleRenameStash}>
    <Input
      bind:value={editStashName}
      placeholder={i18n.t('library.stash_name')}
      disabled={renamingPending}
      class="stash-rename-input"
    >
      {#snippet right()}
        <button
          type="submit"
          class="icon-btn"
          use:ripple
          disabled={!editStashName.trim() || editStashName.trim() === (libraryState.selectedCollection ? libraryState.getStashDisplayName(libraryState.selectedCollection) : '') || renamingPending}
          title={i18n.t('library.rename_stash')}
          aria-label="Rename stash"
        >
          {#if renamingPending}
            <IconLoading style="width: 18px; height: 18px;" />
          {:else}
            <IconCheckmark style="width: 18px; height: 18px;" />
          {/if}
        </button>
      {/snippet}
    </Input>
  </form>

  <span class="filter-label" style="margin-top: 10px;">{i18n.t('settings.stash_color') || 'Stash Color'}</span>
  <div class="stash-color-swatches">
    {#each stashColorPalette as color}
      <button
        type="button"
        class="stash-color-swatch"
        class:is-active={editStashColor === color}
        style:background={color}
        onclick={() => handleSetStashColor(color)}
        aria-label={color}
      ></button>
    {/each}
    <button
      type="button"
      class="stash-color-swatch stash-color-reset"
      class:is-active={!editStashColor}
      onclick={() => handleSetStashColor(null)}
      title="Reset color"
      aria-label="Reset color"
    >
      <IconDismiss class="w-3.5 h-3.5" />
    </button>
  </div>

  <div class="manage-stash-divider"></div>

  <div class="manage-stash-actions">
    <Button
      variant="ghost"
      size="sm"
      disabled={clearingPending || (libraryState.selectedCollection?.item_count ?? 0) === 0}
      onclick={handleClearStash}
      class="manage-stash-btn"
    >
      <IconBroom class="w-[16px] h-[16px]" />
      <span>{i18n.t('library.clear_stash')}</span>
    </Button>

    {#if !libraryState.selectedCollection?.is_system && libraryState.selectedCollection?.kind !== 'inbox'}
      <Button
        variant="danger"
        size="sm"
        disabled={clearingPending}
        onclick={handleDeleteStash}
        class="manage-stash-btn"
      >
        <IconDelete class="w-[16px] h-[16px]" />
        <span>{i18n.t('library.delete_stash')}</span>
      </Button>
    {/if}
  </div>
{/snippet}

{#snippet manageStashTrigger(sticky = false)}
  {#if isStashSelected}
    {#if sticky}
      <PopoverMenu
        bind:open={stickyManageOpen}
        title={i18n.t('library.manage_stash')}
        icon={IconEdit}
        width="340px"
      >
        {@render manageStashContent()}
      </PopoverMenu>
    {:else}
      <PopoverMenu
        bind:open={manageOpen}
        title={i18n.t('library.manage_stash')}
        icon={IconEdit}
        width="340px"
      >
        {@render manageStashContent()}
      </PopoverMenu>
    {/if}
  {/if}
{/snippet}

{#snippet filterInnerContent()}
  <span class="filter-label">{i18n.t('feed.platform')}</span>
  <div class="service-options">
    <Button
      variant={Object.keys(serviceFilters).length === 0 ? 'accent' : 'ghost'}
      size="sm"
      onclick={() => serviceFilters = {}}
      class="filter-chip chip-all {Object.keys(serviceFilters).length === 0 ? 'state-include' : ''}"
    >
      <IconGlobe class="w-5 h-5" />
      <span>{i18n.t('feed.all_platforms')}</span>
    </Button>
    {#each services as service}
      {@const state = serviceFilters[service] ?? 'neutral'}
      <Button
        variant="ghost"
        size="sm"
        onclick={() => toggleService(service)}
        class="filter-chip {state === 'include' ? 'state-include' : state === 'exclude' ? 'state-exclude' : ''}"
      >
        <ServiceIcon service={service} class="w-5 h-5" />
        <span>{service}</span>
        {#if state === 'include'}
          <IconSearch class="w-3.5 h-3.5 ml-auto text-[#4ade80] shrink-0" />
        {:else if state === 'exclude'}
          <IconDismiss class="w-3.5 h-3.5 ml-auto text-[#f87171] shrink-0" />
        {/if}
      </Button>
    {/each}
  </div>

  <div class="floating-divider"></div>

  <span class="filter-label">{i18n.t('feed.format') || 'Format'}</span>
  <div class="service-options">
    {#each formatList as fmt}
      {@const state = formatFilters[fmt.id] ?? 'neutral'}
      {@const IconComponent = fmt.icon}
      <Button
        variant="ghost"
        size="sm"
        onclick={() => toggleFormat(fmt.id)}
        class="filter-chip {state === 'include' ? 'state-include' : state === 'exclude' ? 'state-exclude' : ''}"
      >
        <IconComponent class="w-5 h-5" />
        <span>{fmt.label()}</span>
        {#if state === 'include'}
          <IconSearch class="w-3.5 h-3.5 ml-auto text-[#4ade80] shrink-0" />
        {:else if state === 'exclude'}
          <IconDismiss class="w-3.5 h-3.5 ml-auto text-[#f87171] shrink-0" />
        {/if}
      </Button>
    {/each}
  </div>

  <div class="floating-divider"></div>

  <span class="filter-label section-label">{i18n.t('feed.filters')}</span>
  <button
    type="button"
    class="view-option"
    class:active={onlyWithAttachments}
    use:ripple
    onclick={() => onlyWithAttachments = !onlyWithAttachments}
  >
    <Checkbox
      checked={onlyWithAttachments}
      onchange={(v) => onlyWithAttachments = v}
    />
    <span>
      <strong>{i18n.t('feed.with_attachments')}</strong>
      <small>{i18n.t('feed.with_attachments_desc')}</small>
    </span>
    <IconDocument class="view-option-icon" />
  </button>

  <button
    type="button"
    class="view-option"
    class:active={onlyDownloaded}
    use:ripple
    onclick={() => onlyDownloaded = !onlyDownloaded}
  >
    <Checkbox
      checked={onlyDownloaded}
      onchange={(v) => onlyDownloaded = v}
    />
    <span>
      <strong>{i18n.t('library.only_downloaded')}</strong>
      <small>{i18n.t('library.only_downloaded_desc')}</small>
    </span>
    <IconArrowDownload class="view-option-icon" />
  </button>
{/snippet}

{#snippet libraryTabs()}
  <div class="library-segmented-group">
    <Select
      variant="accent"
      options={[
        {
          value: 'all',
          label: i18n.t('library.all') || 'All',
          count: libraryState.collections.reduce((sum, c) => sum + c.item_count, 0)
        },
        ...libraryState.collections.map((c) => ({
          value: c.id,
          label: libraryState.getStashDisplayName(c),
          count: c.item_count,
          color: c.color || undefined
        }))
      ]}
      value={libraryState.selectedCollectionId ?? 'all'}
      onchange={(val) => selectCollection(val === 'all' ? null : String(val))}
      createLabel={i18n.t('library.new_stash') || 'New stash'}
      onCreate={async (name) => {
        if (!name.trim()) return;
        const newStash = await libraryState.createStash(name.trim());
        await selectCollection(newStash.id);
      }}
      class="library-collection-select"
    />

    <Select
      variant="accent"
      options={sortOptions}
      value={currentSortValue}
      onchange={handleSortChange}
      class="library-sort-select"
      icon={IconArrowSort}
      iconOnly={true}
      ariaLabel={`${i18n.t('favorites.sort_by') || 'Sort'}: ${currentSortLabel}`}
    />
  </div>
{/snippet}

{#snippet libraryFilter(sticky = false)}
  {#if sticky}
    <PopoverMenu
      bind:open={stickyFiltersOpen}
      title={i18n.t('feed.filters')}
      badge={activeFilterCount}
      active={activeFilterCount > 0}
      icon={IconOptions}
    >
      {@render filterInnerContent()}
    </PopoverMenu>
  {:else}
    <PopoverMenu
      bind:open={filtersOpen}
      title={i18n.t('feed.filters')}
      badge={activeFilterCount}
      active={activeFilterCount > 0}
      icon={IconOptions}
    >
      {@render filterInnerContent()}
    </PopoverMenu>
  {/if}
{/snippet}

{#snippet actionsCluster(sticky = false)}
  <div class="library-actions-cluster" class:search-active={searchOpen}>
    <HeaderActions
      bind:searchOpen
      bind:searchQuery
      searchPlaceholder={i18n.t('library.search_placeholder') || 'Search library...'}
    >
      {#if !layoutState.isMobile}
        {#if !searchOpen}
          {@render manageStashTrigger(sticky)}
        {/if}
        <Button
          variant={isSelectionActive ? 'accent' : 'ghost'}
          class="btn-icon"
          onclick={() => (isSelectionActive ? selectionState.exit() : selectionState.enter('posts'))}
          title={i18n.t('selection.select_mode') || 'Select mode'}
          aria-label="Select mode"
        >
          <IconCheckboxChecked class="w-5 h-5" />
        </Button>

        {@render libraryFilter(sticky)}
      {:else}
        {@render libraryFilter(sticky)}

        {#if isSelectionActive}
          <Button
            variant="accent"
            size="sm"
            class="px-2.5 h-[38px] text-xs font-semibold gap-1 rounded-full"
            onclick={() => selectionState.exit()}
            title={i18n.t('common.done') || 'Done'}
            aria-label="Exit selection mode"
          >
            <IconCheckmark class="w-4 h-4" />
            <span>{i18n.t('common.done') || 'Done'}</span>
          </Button>
        {:else}
          <Button
            variant="ghost"
            class="btn-icon"
            onclick={() => (mobileMoreOpen = true)}
            title={i18n.t('common.more') || 'More'}
            aria-label="More actions"
          >
            <IconMoreVertical class="w-5 h-5" />
          </Button>
        {/if}
      {/if}
    </HeaderActions>
  </div>
{/snippet}

<PageShell scrollable={true} scrollKey={navigationState.entryKey} onrefresh={() => libraryState.refresh()}>
  {#snippet overlay()}
    <StickyHeader threshold={120}>
      {#snippet center()}
        {@render libraryTabs()}
      {/snippet}
      {#snippet trailing()}
        {@render actionsCluster(true)}
      {/snippet}
    </StickyHeader>
  {/snippet}

  <PageHeader>
    {#snippet tabs()}
      {@render libraryTabs()}
    {/snippet}
    {#snippet actions()}
      {@render actionsCluster(false)}
    {/snippet}
  </PageHeader>

  {#if libraryState.error && filteredPosts.length === 0}
      <div class="library-error">
        <strong class="text-sm font-semibold text-white/85">{i18n.t('library.load_error')}</strong>
        <span class="library-error-desc">{libraryState.error}</span>
        <Button variant="accent" size="sm" onclick={() => void libraryState.refresh()}>
          <IconArrowClockwise class="h-4 w-4" /> {i18n.t('feed.retry')}
        </Button>
      </div>
  {:else}
      <PostGrid
        posts={filteredPosts}
        loading={libraryState.loading}
        hasMore={libraryState.hasMore}
        onLoadMore={() => libraryState.loadMore()}
        stateKey={`library:${libraryState.selectedCollectionId ?? 'all'}:services=${JSON.stringify(serviceFilters)}:formats=${JSON.stringify(formatFilters)}:attachments=${onlyWithAttachments}:downloaded=${onlyDownloaded}:sort=${sortBy}_${sortOrder}`}
        paginationKey={`${libraryState.selectedCollectionId ?? 'all'}:${filteredPosts.length}`}
        ariaLabel={i18n.t('library.title')}
        emptyTitle={i18n.t('library.empty')}
        emptyDescription={i18n.t('library.empty_desc')}
      />
      {#if libraryState.error}
        <p class="tail-error">{libraryState.error}</p>
      {/if}
  {/if}
</PageShell>

<SelectionActionBar
  totalCount={filteredPosts.length}
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

  {#if isStashSelected}
    <Button
      variant="ghost"
      size="sm"
      class="selection-btn"
      onclick={batchRemoveFromThisStash}
      title={i18n.t('selection.remove_from_stash')}
    >
      <IconFolderDismiss class="w-[16px] h-[16px]" />
      <span>{i18n.t('selection.remove_from_stash')}</span>
    </Button>
  {/if}

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
    variant="danger"
    size="sm"
    class="selection-btn"
    onclick={batchDeleteFromLibrary}
    title={i18n.t('selection.remove_from_library')}
  >
    <IconDelete class="w-[16px] h-[16px]" />
    <span>{i18n.t('selection.remove_from_library')}</span>
  </Button>
</SelectionActionBar>

{#if layoutState.isMobile}
  <BottomSheet
    open={mobileMoreOpen}
    title={i18n.t('common.more') || 'More'}
    onclose={() => (mobileMoreOpen = false)}
  >
    <div class="flex flex-col gap-1 py-1">
      {#if isStashSelected}
        <button
          type="button"
          class="sheet-action-item"
          use:ripple
          onclick={() => {
            mobileMoreOpen = false;
            mobileManageOpen = true;
          }}
        >
          <IconEdit class="text-secondary" />
          <div class="flex flex-col min-w-0">
            <span class="text-sm font-semibold text-primary">{i18n.t('library.manage_stash') || 'Manage stash'}</span>
          </div>
        </button>
      {/if}

      <button
        type="button"
        class="sheet-action-item"
        use:ripple
        onclick={() => {
          mobileMoreOpen = false;
          selectionState.enter('posts');
        }}
      >
        <IconCheckboxChecked class="text-secondary" />
        <div class="flex flex-col min-w-0">
          <span class="text-sm font-semibold text-primary">{i18n.t('selection.select_mode') || 'Select mode'}</span>
        </div>
      </button>

      <button
        type="button"
        class="sheet-action-item"
        disabled={libraryState.loading}
        use:ripple
        onclick={() => {
          mobileMoreOpen = false;
          void libraryState.refresh();
        }}
      >
        {#if libraryState.loading}
          <IconLoading class="text-accent" />
        {:else}
          <IconArrowClockwise class="text-secondary" />
        {/if}
        <div class="flex flex-col min-w-0">
          <span class="text-sm font-semibold text-primary">{i18n.t('feed.refresh') || 'Refresh'}</span>
        </div>
      </button>
    </div>
  </BottomSheet>

  <BottomSheet
    open={mobileManageOpen}
    title={i18n.t('library.manage_stash') || 'Manage stash'}
    onclose={() => (mobileManageOpen = false)}
  >
    {@render manageStashContent()}
  </BottomSheet>
{/if}

<style>
  .library-segmented-group {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    min-width: 0;
    flex-shrink: 0;
  }

  :global(.library-collection-select) {
    width: auto !important;
    min-width: 140px !important;
    max-width: 260px !important;
    flex-shrink: 0 !important;
  }

  :global(.library-collection-select .select-trigger) {
    height: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    background: var(--choice-active-bg, var(--accent-primary)) !important;
    color: var(--choice-active-text, var(--text-on-accent, #ffffff)) !important;
    font-weight: var(--font-weight-semibold) !important;
    border-top-left-radius: min(calc(var(--radius-full) * var(--ui-scale, 1)), calc(var(--control-height, 46px) / 2)) !important;
    border-bottom-left-radius: min(calc(var(--radius-full) * var(--ui-scale, 1)), calc(var(--control-height, 46px) / 2)) !important;
    border-top-right-radius: calc(var(--radius-sm, 6px) * var(--ui-scale, 1)) !important;
    border-bottom-right-radius: calc(var(--radius-sm, 6px) * var(--ui-scale, 1)) !important;
    border: none !important;
    box-shadow: none !important;
    padding-left: 18px !important;
    padding-right: 12px !important;
    transition:
      background var(--duration-fast) var(--ease-expo),
      color var(--duration-fast) var(--ease-expo),
      opacity var(--duration-fast) var(--ease-expo) !important;
  }

  :global(.library-collection-select .select-trigger:hover) {
    background: color-mix(in srgb, var(--choice-active-bg, var(--accent-primary)) 88%, white) !important;
    color: var(--choice-active-text, var(--text-on-accent, #ffffff)) !important;
  }

  :global(.library-collection-select .select-trigger:active) {
    opacity: 0.85 !important;
  }

  :global(.library-collection-select .select-trigger .trigger-label) {
    font-weight: var(--font-weight-semibold) !important;
    color: var(--choice-active-text, var(--text-on-accent, #ffffff)) !important;
  }

  :global(.library-collection-select .select-trigger .count-badge) {
    background: color-mix(in srgb, currentColor 22%, transparent) !important;
    color: inherit !important;
    font-size: calc(12px * var(--ui-scale, 1)) !important;
  }

  :global(.library-collection-select .select-trigger .trigger-chevron),
  :global(.library-collection-select .select-trigger .trigger-chevron svg) {
    color: var(--choice-active-text, currentColor) !important;
    opacity: 0.85 !important;
    transition:
      transform var(--duration-normal) var(--ease-expo),
      opacity var(--duration-fast) var(--ease-expo) !important;
  }

  :global(.library-collection-select .select-trigger:hover .trigger-chevron),
  :global(.library-collection-select .select-trigger:hover .trigger-chevron svg) {
    opacity: 1 !important;
  }

  :global(.library-sort-select) {
    width: auto !important;
    max-width: none !important;
    flex-shrink: 0 !important;
  }

  :global(.library-sort-select .select-trigger),
  :global(.library-sort-select .select-trigger.icon-only) {
    width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    min-width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    height: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    padding: 0 calc(3px * var(--ui-scale, 1)) 0 0 !important;
    background: var(--accent-container) !important;
    color: var(--choice-active-bg, var(--accent-on-container)) !important;
    border-top-left-radius: calc(var(--radius-sm, 6px) * var(--ui-scale, 1)) !important;
    border-bottom-left-radius: calc(var(--radius-sm, 6px) * var(--ui-scale, 1)) !important;
    border-top-right-radius: min(calc(var(--radius-full) * var(--ui-scale, 1)), calc(var(--control-height, 46px) / 2)) !important;
    border-bottom-right-radius: min(calc(var(--radius-full) * var(--ui-scale, 1)), calc(var(--control-height, 46px) / 2)) !important;
    border: none !important;
    box-shadow: none !important;
    transition:
      background var(--duration-fast) var(--ease-expo),
      color var(--duration-fast) var(--ease-expo),
      opacity var(--duration-fast) var(--ease-expo) !important;
  }

  :global(.library-sort-select .select-trigger:hover),
  :global(.library-sort-select .select-trigger.icon-only:hover) {
    background: color-mix(in srgb, var(--accent-container) 70%, var(--accent-primary)) !important;
    color: var(--choice-active-bg, var(--accent-on-container)) !important;
  }

  :global(.library-sort-select .select-trigger:active),
  :global(.library-sort-select .select-trigger.icon-only:active) {
    opacity: 0.85 !important;
  }

  :global(.library-sort-select .select-trigger svg),
  :global(.library-sort-select .select-trigger.icon-only svg) {
    width: 20px !important;
    height: 20px !important;
    color: var(--choice-active-bg, var(--accent-on-container)) !important;
    opacity: 1 !important;
  }

  .library-actions-cluster {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .library-actions-cluster.search-active {
    width: 100%;
    flex: 1;
  }



  .library-error {
    min-height: 310px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 16px;
    text-align: center;
  }

  .library-error-desc {
    max-width: 480px;
    color: var(--text-muted);
    font-size: 12px;
    overflow-wrap: anywhere;
  }

  .tail-error {
    margin: 18px 0 0;
    color: rgba(224, 60, 60, 0.8);
    font-size: 12px;
    text-align: center;
  }

  :global(.manage-stash-popover) {
    width: 320px !important;
  }

  .filter-heading {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 2px 4px 8px;
  }

  .stash-rename-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 4px;
  }

  :global(.stash-rename-input) {
    flex: 1;
    height: 40px !important;
    font-size: 13px !important;
  }

  :global(.rename-submit-btn) {
    width: 40px !important;
    height: 40px !important;
    flex-shrink: 0;
  }

  .manage-stash-divider {
    height: 1px;
    background: var(--border-color);
    margin: 14px 0 10px;
  }

  .manage-stash-actions {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  :global(.manage-stash-btn) {
    width: 100%;
    height: 38px !important;
    justify-content: flex-start !important;
    gap: 10px !important;
    padding: 0 12px !important;
    font-size: 13px !important;
  }

  .stash-color-swatches {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 8px 0 10px 0;
    flex-wrap: wrap;
  }

  .stash-color-swatch {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    border: 2px solid transparent;
    cursor: pointer;
    transition: transform var(--duration-fast) var(--ease-expo),
                border-color var(--duration-fast) var(--ease-expo);
    display: grid;
    place-items: center;
    padding: 0;
  }

  .stash-color-swatch:hover {
    transform: scale(1.15);
  }

  .stash-color-swatch.is-active {
    border-color: #ffffff;
    transform: scale(1.2);
    box-shadow: 0 0 8px rgba(255, 255, 255, 0.4);
  }

  .stash-color-reset {
    background: rgba(255, 255, 255, 0.1);
    color: var(--text-secondary);
  }
</style>
