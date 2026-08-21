<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { i18n } from '$lib/i18n';
  import { libraryState } from '$lib/state/libraryState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { downloadState } from '$lib/state/downloadState.svelte';
  import { configState } from '$lib/state/configState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import type { PawchivePost } from '$lib/types/pawchive';
  import PageShell from '$lib/components/layout/PageShell.svelte';
  import PageHeader from '$lib/components/layout/PageHeader.svelte';
  import HeaderActions from '$lib/components/layout/HeaderActions.svelte';
  import PostGrid from '$lib/components/pawchive/PostGrid.svelte';
  import StickyHeader from '$lib/components/layout/StickyHeader.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import Checkbox from '$lib/components/ui/Checkbox.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import PopoverMenu from '$lib/components/ui/PopoverMenu.svelte';
  import CountBadge from '$lib/components/ui/CountBadge.svelte';
  import ServiceIcon from '$lib/components/pawchive/ServiceIcon.svelte';
  import { ripple } from '$lib/motion';
  import { notify } from '$lib/utils/toast';
  import { selectionState } from '$lib/state/selectionState.svelte';
  import { getPostDownloadTargets } from '$lib/utils/media';
  import SelectionActionBar from '$lib/components/ui/SelectionActionBar.svelte';
  import IconAdd from '~icons/fluent/add-24-regular';
  import IconCheckmark from '~icons/fluent/checkmark-20-regular';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';
  import IconDelete from '~icons/fluent/delete-24-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';
  import IconArrowClockwise from '~icons/fluent/arrow-clockwise-24-regular';
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
  import IconMoreHorizontal from '~icons/fluent/more-horizontal-24-regular';
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

  async function handleBatchToggleStash(collectionId: string) {
    const items = selectionState.getItems<PawchivePost>();
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
    const items = selectionState.getItems<PawchivePost>();
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

  let isCustomStash = $derived(
    libraryState.selectedCollectionId !== null &&
    libraryState.selectedCollection &&
    libraryState.selectedCollection.kind === 'stash'
  );

  function handleSelectAllPosts() {
    selectionState.selectAll(filteredPosts.map((p) => ({
      key: `${p.service}:${p.user}:${p.id}`,
      item: p
    })));
  }

  async function batchRemoveFromThisStash() {
    const items = selectionState.getItems<PawchivePost>();
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
    const items = selectionState.getItems<PawchivePost>();
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

  onDestroy(() => {
    downloadState.destroy();
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

  $effect(() => {
    if (manageOpen || stickyManageOpen) {
      editStashName = libraryState.selectedCollection?.name ?? '';
    }
  });

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

  function fileUrl(file: { path?: string; server?: string }) {
    const cdn = file.server || `https://${configState.settings.file_domain}`;
    return `${cdn}/data${file.path}`;
  }

  function isPostDownloaded(post: PawchivePost): boolean {
    const file = post.file as any;
    const attachments = (post.attachments as any[]) || [];
    
    const urls: string[] = [];
    if (file && file.path) {
      urls.push(fileUrl(file));
    }
    for (const a of attachments) {
      if (a && a.path) {
        urls.push(fileUrl(a));
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
        const valA = a.published || '';
        const valB = b.published || '';
        comparison = valA.localeCompare(valB);
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
    />
    <Button
      type="submit"
      variant="accent"
      disabled={!editStashName.trim() || editStashName.trim() === libraryState.selectedCollection?.name || renamingPending}
      title={i18n.t('library.rename_stash')}
      class="btn-icon rename-submit-btn"
    >
      {#if renamingPending}<IconLoading class="w-[18px] h-[18px]" />{:else}<IconCheckmark class="w-[18px] h-[18px]" />{/if}
    </Button>
  </form>

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
  </div>
{/snippet}

{#snippet manageStashTrigger(sticky = false)}
  {#if isCustomStash}
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
      <IconGlobe class="w-[14px] h-[14px]" />
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
        <ServiceIcon service={service} class="w-[14px] h-[14px]" />
        <span>{service}</span>
      </Button>
    {/each}
  </div>

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
        <IconComponent class="w-[14px] h-[14px]" />
        <span>{fmt.label()}</span>
      </Button>
    {/each}
  </div>

  <span class="filter-label section-label">{i18n.t('feed.filters')}</span>
  <div class="view-option" class:active={onlyWithAttachments}>
    <Checkbox
      checked={onlyWithAttachments}
      onchange={(v) => onlyWithAttachments = v}
    />
    <button type="button" onclick={() => onlyWithAttachments = !onlyWithAttachments}>
      <strong>{i18n.t('feed.with_attachments')}</strong>
      <small>{i18n.t('feed.with_attachments_desc')}</small>
    </button>
    <IconDocument class="view-option-icon w-[20px] h-[20px]" />
  </div>

  <div class="view-option" class:active={onlyDownloaded}>
    <Checkbox
      checked={onlyDownloaded}
      onchange={(v) => onlyDownloaded = v}
    />
    <button type="button" onclick={() => onlyDownloaded = !onlyDownloaded}>
      <strong>{i18n.t('library.only_downloaded')}</strong>
      <small>{i18n.t('library.only_downloaded_desc')}</small>
    </button>
    <IconArrowDownload class="view-option-icon w-[20px] h-[20px]" />
  </div>
{/snippet}

{#snippet collectionTabs()}
  {#if !layoutState.isMobile}
    <div class="flex items-center gap-2">
      <Button
        variant={libraryState.selectedCollectionId === null ? 'accent' : 'ghost'}
        onclick={() => selectCollection(null)}
        class="library-tab"
      >
        <span>{i18n.t('library.all') || 'Library'}</span>
        <CountBadge count={libraryState.collections.reduce((sum, c) => sum + c.item_count, 0)} />
      </Button>

      <div class="desktop-stash-picker">
        <Select
          variant={libraryState.selectedCollectionId !== null ? 'accent' : 'ghost'}
          options={libraryState.collections.map((c) => ({
            value: c.id,
            label: `${c.kind === 'inbox' ? (i18n.t('library.inbox') || 'Inbox') : c.name} (${c.item_count})`
          }))}
          value={libraryState.selectedCollectionId ?? undefined}
          placeholder={i18n.t('library.stashes') || 'Stashes'}
          onchange={(val) => {
            if (val) selectCollection(String(val));
          }}
          createLabel={i18n.t('library.new_stash') || 'New stash'}
          onCreate={async (name) => {
            if (!name.trim()) return;
            const newStash = await libraryState.createStash(name.trim());
            await selectCollection(newStash.id);
          }}
          class="desktop-stash-select"
          style="height: 44px;"
        />
      </div>
    </div>
  {:else}
    <div class="mobile-collection-picker">
      <Select
        variant="accent"
        options={[
          { value: 'all', label: `${i18n.t('library.all') || 'All'} (${libraryState.collections.reduce((sum, c) => sum + c.item_count, 0)})` },
          ...libraryState.collections.map((c) => ({
            value: c.id,
            label: `${c.kind === 'inbox' ? (i18n.t('library.inbox') || 'Inbox') : c.name} (${c.item_count})`
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
        class="mobile-stash-select"
        style="height: 44px;"
      />
    </div>
  {/if}
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
    {#if !layoutState.isMobile || !searchOpen}
      {@render manageStashTrigger(sticky)}
    {/if}

    <HeaderActions
      bind:searchOpen
      bind:searchQuery
      searchPlaceholder={i18n.t('library.search_placeholder') || 'Search library...'}
    >
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
    </HeaderActions>
  </div>
{/snippet}

<PageShell scrollable={true} scrollKey={navigationState.entryKey} onrefresh={() => libraryState.refresh()}>
  {#snippet overlay()}
    <StickyHeader
      threshold={120}
      title={libraryState.selectedCollection ? libraryState.selectedCollection.name : (i18n.t('library.title') || 'Library')}
    >
      {#snippet center()}
        <div class="flex items-center gap-2">
          {@render collectionTabs()}
          <Select
            variant="ghost"
            options={sortOptions}
            value={currentSortValue}
            onchange={handleSortChange}
            class="library-sort-select"
            style="height: 44px;"
          />
        </div>
      {/snippet}
      {#snippet trailing()}
        {@render actionsCluster(true)}
      {/snippet}
    </StickyHeader>
  {/snippet}

  <PageHeader>
    {#snippet tabs()}
      {@render collectionTabs()}
    {/snippet}
    {#snippet filters()}
      <Select
        variant="ghost"
        options={sortOptions}
        value={currentSortValue}
        onchange={handleSortChange}
        class="library-sort-select"
        style="height: 44px;"
      />
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

  {#if isCustomStash}
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

<style>
  :global(.btn-icon) {
    width: 44px !important;
    height: 44px !important;
    padding: 0 !important;
    border-radius: var(--radius-full) !important;
    flex-shrink: 0;
    display: inline-flex !important;
    align-items: center !important;
    justify-content: center !important;
  }

  :global(.btn-icon svg) {
    width: 20px !important;
    height: 20px !important;
    flex-shrink: 0 !important;
  }



  :global(.library-tab) {
    height: 44px !important;
    padding: 0 18px !important;
    font-size: 13.5px !important;
    border-radius: var(--radius-full) !important;
    flex-shrink: 0 !important;
    display: inline-flex !important;
    align-items: center !important;
    gap: 8px !important;
  }

  .desktop-stash-picker {
    display: flex;
    align-items: center;
    min-width: 0;
    flex-shrink: 0;
  }

  :global(.desktop-stash-select) {
    width: auto !important;
    min-width: 130px !important;
    max-width: 220px !important;
  }

  :global(.desktop-stash-select .select-trigger) {
    height: 44px !important;
    font-size: 13.5px !important;
    padding: 0 14px !important;
    border-radius: var(--radius-full) !important;
  }

  :global(.btn-create-stash) {
    height: 44px !important;
    padding: 0 18px !important;
    font-size: 13.5px !important;
    border-radius: var(--radius-full) !important;
    gap: 8px !important;
    flex-shrink: 0 !important;
  }

  .mobile-collection-picker {
    display: flex;
    align-items: center;
    min-width: 0;
    flex-shrink: 0;
  }

  :global(.mobile-stash-select) {
    width: auto !important;
    min-width: 140px !important;
    max-width: 100% !important;
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
</style>
