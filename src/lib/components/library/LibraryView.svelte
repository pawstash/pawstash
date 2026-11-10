<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { i18n } from '$lib/i18n';
  import { libraryState } from '$lib/state/libraryState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { downloadState } from '$lib/state/downloadState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { themeState } from '$lib/theme/themeState.svelte';
  import type { Post } from '$lib/types/content';
  import { parseDateTimestamp } from '$lib/utils/formatters';
  import PageShell from '$lib/components/layout/PageShell.svelte';
  import HeaderActions from '$lib/components/layout/HeaderActions.svelte';
  import PostGrid from '$lib/components/content/PostGrid.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import Toggle from '$lib/components/ui/Toggle.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import PopoverMenu from '$lib/components/ui/PopoverMenu.svelte';
  import BottomSheet from '$lib/components/ui/BottomSheet.svelte';
  import SortControl from '$lib/components/layout/SortControl.svelte';
  import { sortOption } from '$lib/utils/sort';
  import type { MenuItem } from '$lib/components/layout/OverflowMenu.svelte';
  import IconChevronDown from '~icons/fluent/chevron-down-20-regular';
  import ServiceIcon from '$lib/components/content/ServiceIcon.svelte';
  import { ripple, tooltip } from '$lib/motion';
  import { notify } from '$lib/utils/toast';
  import { notifyAddedToStash, notifyRemovedFromStash } from '$lib/utils/stashToast';
  import { selectionState } from '$lib/state/selectionState.svelte';
  import { attachmentMediaUrl } from '$lib/utils/media';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import IconCheckmark from '~icons/fluent/checkmark-20-regular';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';
  import IconDelete from '~icons/fluent/delete-24-regular';
  import IconHeart from '~icons/fluent/heart-24-regular';
  import { favoriteActions } from '$lib/state/favoriteActions.svelte';
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
  import IconCheckboxChecked from '~icons/fluent/checkbox-checked-24-regular';
  import IconFolder from '~icons/fluent/folder-24-regular';
  import IconFolderDismiss from '~icons/fluent/folder-dismiss-24-regular';
  import IconArrowDownload from '~icons/fluent/arrow-download-24-regular';
  import IconDraft from '~icons/fluent/drafts-24-regular';

  import type { FilterMap } from '$lib/types/filter';
  import { countActiveFilters, matchesTriStateFilter, toggleFilterKey } from '$lib/types/filter';
  import { matchesPostFormatFilter } from '$lib/utils/media';

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
  let isSelectionActive = $derived(selectionState.active && selectionState.scope === 'posts');
  let headingEl = $state<HTMLElement | null>(null);
  let selectedPosts = $derived(isSelectionActive ? selectionState.getItems<Post>() : []);
  const stashOptions = $derived(libraryState.stashOptions);

  const batchSelectedStashes = $derived(libraryState.stashesForPosts(selectedPosts));

  async function handleBatchToggleStash(collectionId: string) {
    const items = selectionState.getItems<Post>();
    if (items.length === 0 || !collectionId) return;
    const isAllIn = batchSelectedStashes.includes(collectionId);
    try {
      if (isAllIn) {
        for (const p of items) {
          await libraryState.removeFromStash(collectionId, p);
        }
        notifyRemovedFromStash(collectionId, items);
      } else {
        for (const p of items) {
          await libraryState.save(p, collectionId);
        }
        notifyAddedToStash(collectionId);
      }
    } catch (error) {
      notify.error(i18n.t('library.save_error'), error);
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
      notifyAddedToStash(newStash.id, newStash.name);
    } catch (error) {
      notify.error(i18n.t('library.save_error'), error);
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
        i18n.t('selection.remove_from_stash'),
        i18n.t('counts.posts', { count: items.length })
      );
      selectionState.exit();
    } catch (err) {
      notify.error(i18n.t('library.save_error'), err);
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
        i18n.t('selection.remove_from_library'),
        i18n.t('counts.posts', { count: items.length })
      );
      selectionState.exit();
    } catch (err) {
      notify.error(i18n.t('library.save_error'), err);
    }
  }

  async function batchDownloadPosts() {
    const items = selectionState.getItems<Post>();
    if (items.length === 0) return;
    try {
      const count = await downloadState.downloadPosts(items);
      notify.success(
        i18n.t('selection.download_all'),
        i18n.t('counts.files', { count: count })
      );
      selectionState.exit();
    } catch (err) {
      notify.error(i18n.t('downloads.action_error'), err);
    }
  }

  let manageOpen = $state(false);
  let editStashName = $state('');
  let renamingPending = $state(false);
  let clearingPending = $state(false);
  let clearConfirmOpen = $state(false);
  let deleteConfirmOpen = $state(false);
  let deletingPending = $state(false);

  let filtersOpen = $state(false);

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
    sortOption('added_desc', 'added', 'desc'),
    sortOption('added_asc', 'added', 'asc'),
    sortOption('published_desc', 'published', 'desc'),
    sortOption('published_asc', 'published', 'asc'),
    sortOption('title_asc', 'title', 'asc'),
    sortOption('title_desc', 'title', 'desc')
  ]);

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

  let editStashColor = $state<string | null>(null);
  const stashColorPalette = ['#ef4444', '#f97316', '#f59e0b', '#10b981', '#06b6d4', '#3b82f6', '#8b5cf6', '#ec4899'];

  $effect(() => {
    if (manageOpen || mobileManageOpen) {
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
      notify.success(i18n.t('settings.stash_color'));
    } catch (error) {
      notify.error(i18n.t('library.save_error'), error);
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
      mobileManageOpen = false;
    } catch (error) {
      notify.error(i18n.t('library.save_error'), error);
    } finally {
      renamingPending = false;
    }
  }

  async function handleClearStash() {
    const collectionId = libraryState.selectedCollectionId;
    if (!collectionId || clearingPending) return;
    const name = libraryState.selectedCollection?.name ?? '';

    clearingPending = true;
    try {
      await libraryState.clearStash(collectionId);
      notify.success(i18n.t('library.stash_cleared'), { description: name || undefined, glyph: 'cleared' });
      clearConfirmOpen = false;
      manageOpen = false;
      mobileManageOpen = false;
    } catch (error) {
      notify.error(i18n.t('library.save_error'), error);
    } finally {
      clearingPending = false;
    }
  }

  async function handleDeleteStash() {
    const collectionId = libraryState.selectedCollectionId;
    if (!collectionId || deletingPending) return;
    const name = libraryState.selectedCollection?.name ?? '';

    deletingPending = true;
    try {
      await libraryState.deleteStash(collectionId);
      notify.success(i18n.t('library.stash_deleted'), { description: name || undefined, glyph: 'deleted' });
      deleteConfirmOpen = false;
      manageOpen = false;
      mobileManageOpen = false;
    } catch (error) {
      notify.error(i18n.t('library.save_error'), error);
    } finally {
      deletingPending = false;
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
    { id: 'image', label: () => i18n.t('feed.format_photo'), icon: IconImage },
    { id: 'video', label: () => i18n.t('feed.format_video'), icon: IconVideo },
    { id: 'audio', label: () => i18n.t('feed.format_audio'), icon: IconMusic },
    { id: 'text', label: () => i18n.t('feed.format_text'), icon: IconText },
    { id: 'archive', label: () => i18n.t('feed.format_archive'), icon: IconDocument },
    { id: 'wip', label: () => i18n.t('feed.format_wip'), icon: IconDraft }
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
      list = list.filter(post => matchesPostFormatFilter(post, formatFilters));
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

  let pageMenu = $derived<MenuItem[]>([
    ...(isStashSelected
      ? [
          {
            group: 'stash',
            label: i18n.t('library.manage_stash'),
            icon: IconEdit,
            onclick: () => (layoutState.isMobile ? (mobileManageOpen = true) : (manageOpen = true))
          }
        ]
      : []),
    {
      group: 'page',
      label: i18n.t(isSelectionActive ? 'selection.exit' : 'selection.select_mode'),
      icon: IconCheckboxChecked,
      active: isSelectionActive,
      onclick: () => (isSelectionActive ? selectionState.exit() : selectionState.enter('posts'))
    },
    {
      group: 'page',
      label: i18n.t('common.refresh'),
      icon: IconArrowClockwise,
      disabled: libraryState.loading,
      onclick: () => void libraryState.refresh()
    }
  ]);
</script>

{#snippet manageStashContent()}
  {@const stash = libraryState.selectedCollection}
  {@const stashName = stash ? libraryState.getStashDisplayName(stash) : ''}
  <div class="manage-stash">
    <form class="stash-rename-row" onsubmit={handleRenameStash}>
      <Input
        bind:value={editStashName}
        label={i18n.t('library.stash_name')}
        disabled={renamingPending}
      >
        {#snippet right()}
          <button
            type="submit"
            class="field-action"
            use:ripple
            disabled={!editStashName.trim() || editStashName.trim() === stashName || renamingPending}
            use:tooltip={i18n.t('library.rename_stash')}
            aria-label={i18n.t('library.rename_stash')}
          >
            {#if renamingPending}<IconLoading />{:else}<IconCheckmark />{/if}
          </button>
        {/snippet}
      </Input>
    </form>

    <span class="floating-section-label">{i18n.t('settings.stash_color')}</span>
    <div class="stash-color-swatches" role="radiogroup" aria-label={i18n.t('settings.stash_color')}>
      {#each stashColorPalette as color (color)}
        <button
          type="button"
          class="color-swatch"
          role="radio"
          aria-checked={editStashColor === color}
          style:background={color}
          onclick={() => handleSetStashColor(color)}
          aria-label={color}
        ></button>
      {/each}
      <button
        type="button"
        class="color-swatch is-reset"
        role="radio"
        aria-checked={!editStashColor}
        onclick={() => handleSetStashColor(null)}
        use:tooltip={i18n.t('library.reset_color')}
        aria-label={i18n.t('library.reset_color')}
      >
        <IconDismiss />
      </button>
    </div>

    <div class="floating-divider" role="separator"></div>

    <button
      type="button"
      class="floating-item"
      use:ripple
      disabled={clearingPending || (stash?.item_count ?? 0) === 0}
      onclick={() => (clearConfirmOpen = true)}
    >
      <IconBroom />
      <span class="floating-item-label">{i18n.t('library.clear_stash')}</span>
      <span class="floating-item-detail">{i18n.number(stash?.item_count ?? 0)}</span>
    </button>
    {#if !stash?.is_system && stash?.kind !== 'inbox'}
      <button
        type="button"
        class="floating-item is-danger"
        use:ripple
        disabled={clearingPending}
        onclick={() => (deleteConfirmOpen = true)}
      >
        <IconDelete />
        <span class="floating-item-label">{i18n.t('library.delete_stash')}</span>
      </button>
    {/if}
  </div>
{/snippet}

{#snippet manageStashPopover()}
  {#if isStashSelected && !layoutState.isMobile}
    <PopoverMenu
      bind:open={manageOpen}
      anchorEl={headingEl}
      align="left"
      title={i18n.t('library.manage_stash')}
      width="320px"
      menuClass="manage-stash-popover"
    >
      {@render manageStashContent()}
    </PopoverMenu>
  {/if}
{/snippet}

{#snippet filterInnerContent()}
  <div class="filter-section-title">
    <div class="filter-section-icon"><IconGlobe class="w-3.5 h-3.5" /></div>
    <span>{i18n.t('feed.platform')}</span>
  </div>
  <div class="service-options">
    <Button
      variant="ghost"
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
          <IconSearch class="w-3.5 h-3.5 ml-auto text-[var(--status-success)] shrink-0" />
        {:else if state === 'exclude'}
          <IconDismiss class="w-3.5 h-3.5 ml-auto text-[var(--status-error)] shrink-0" />
        {/if}
      </Button>
    {/each}
  </div>

  <div class="filter-section-title">
    <div class="filter-section-icon"><IconDocument class="w-3.5 h-3.5" /></div>
    <span>{i18n.t('feed.format')}</span>
  </div>
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
          <IconSearch class="w-3.5 h-3.5 ml-auto text-[var(--status-success)] shrink-0" />
        {:else if state === 'exclude'}
          <IconDismiss class="w-3.5 h-3.5 ml-auto text-[var(--status-error)] shrink-0" />
        {/if}
      </Button>
    {/each}
  </div>

  <div class="filter-section-title">
    <div class="filter-section-icon"><IconOptions class="w-3.5 h-3.5" /></div>
    <span>{i18n.t('feed.filters')}</span>
  </div>
  <div class="settings-list">
    <div
      role="switch"
      aria-checked={onlyWithAttachments}
      tabindex="0"
      class="view-option"
      class:active={onlyWithAttachments}
      use:ripple
      onclick={() => (onlyWithAttachments = !onlyWithAttachments)}
      onkeydown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          onlyWithAttachments = !onlyWithAttachments;
        }
      }}
    >
      <span>
        <strong>{i18n.t('feed.with_attachments')}</strong>
        <small>{i18n.t('feed.with_attachments_desc')}</small>
      </span>
      <Toggle
        checked={onlyWithAttachments}
        tabindex={-1}
        class="pointer-events-none"
      />
    </div>

    <div
      role="switch"
      aria-checked={onlyDownloaded}
      tabindex="0"
      class="view-option"
      class:active={onlyDownloaded}
      use:ripple
      onclick={() => (onlyDownloaded = !onlyDownloaded)}
      onkeydown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          onlyDownloaded = !onlyDownloaded;
        }
      }}
    >
      <span>
        <strong>{i18n.t('library.only_downloaded')}</strong>
        <small>{i18n.t('library.only_downloaded_desc')}</small>
      </span>
      <Toggle
        checked={onlyDownloaded}
        tabindex={-1}
        class="pointer-events-none"
      />
    </div>
  </div>
{/snippet}

{#snippet collectionPicker()}
  <div class="title-picker-anchor" bind:this={headingEl}>
    <Select
      options={[
        {
          value: 'all',
          label: i18n.t('library.all'),
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
      createLabel={i18n.t('library.new_stash')}
      onCreate={async (name) => {
        if (!name.trim()) return;
        const newStash = await libraryState.createStash(name.trim());
        await selectCollection(newStash.id);
      }}
      align="left"
    >
      {#snippet trigger({ toggle, open, selectedLabel })}
        <button type="button" class="title-picker" aria-haspopup="listbox" aria-expanded={open} use:ripple onclick={toggle}>
          <h1>{selectedLabel}</h1>
          <IconChevronDown />
        </button>
      {/snippet}
    </Select>
  </div>
  <SortControl variant="standalone" options={sortOptions} value={currentSortValue} onchange={handleSortChange} />
  {@render manageStashPopover()}
{/snippet}

{#snippet libraryFilter()}
  <PopoverMenu
    bind:open={filtersOpen}
    title={i18n.t('feed.filters')}
    badge={activeFilterCount}
    active={activeFilterCount > 0}
    icon={IconOptions}
  >
    {@render filterInnerContent()}
  </PopoverMenu>
{/snippet}

{#snippet search()}
  <HeaderActions pinOpen bind:searchOpen bind:searchQuery searchPlaceholder={i18n.t('library.search_placeholder')} />
{/snippet}

<PageShell title={i18n.t('library.title')} menu={pageMenu} scrollKey={navigationState.entryKey} onrefresh={() => libraryState.refresh()}
  selectionTotal={filteredPosts.length}
  onSelectAll={handleSelectAllPosts}
  {selectionActions}
>
  {#snippet heading()}{@render collectionPicker()}{/snippet}
  {#snippet filter()}{@render libraryFilter()}{/snippet}
  {#snippet actions()}{@render search()}{/snippet}

  {#if libraryState.error && filteredPosts.length === 0}
      <div class="library-error">
        <strong class="text-sm font-semibold text-ink/85">{i18n.t('library.load_error')}</strong>
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

{#snippet selectionActions()}
  <Select
    options={stashOptions}
    selectedValues={batchSelectedStashes}
    placeholder={i18n.t('library.add_to_stash')}
    onchange={handleBatchToggleStash}
    createLabel={i18n.t('library.new_stash')}
    onCreate={handleBatchCreateAndAddToStash}
    variant="ghost"
    multi={true}
    closeOnChange={false}
    icon={IconFolder}
    iconOnly={true}
    ariaLabel={i18n.t('library.add_to_stash')}
    align="right"
  />

  {#if isStashSelected}
    <Button
      variant="ghost"
      class="btn-icon"
      onclick={batchRemoveFromThisStash}
      title={i18n.t('selection.remove_from_stash')}
    >
      <IconFolderDismiss />
    </Button>
  {/if}

  <Button
    variant="ghost"
    class="btn-icon"
    onclick={batchDownloadPosts}
    title={i18n.t('selection.download_all')}
  >
    <IconArrowDownload />
  </Button>

  <Button
    variant="ghost"
    class="btn-icon"
    onclick={() => void favoriteActions.favoritePosts(selectionState.getItems<Post>()).then((done) => done && selectionState.exit())}
    title={i18n.t('selection.favorite')}
  >
    <IconHeart />
  </Button>

  <Button
    variant="ghost"
    class="btn-icon"
    onclick={batchDeleteFromLibrary}
    title={i18n.t('selection.remove_from_library')}
  >
    <IconDelete />
  </Button>
{/snippet}

{#if layoutState.isMobile}
  <BottomSheet
    isOpen={mobileManageOpen}
    title={i18n.t('library.manage_stash')}
    onclose={() => (mobileManageOpen = false)}
  >
    {@render manageStashContent()}
  </BottomSheet>
{/if}

<ConfirmDialog
  isOpen={clearConfirmOpen}
  title={i18n.t('library.confirm_clear')}
  description={i18n.t('library.clear_confirm')}
  confirmLabel={i18n.t('library.clear_stash')}
  confirmVariant="danger"
  loading={clearingPending}
  onconfirm={handleClearStash}
  onclose={() => (clearConfirmOpen = false)}
/>

<ConfirmDialog
  isOpen={deleteConfirmOpen}
  title={i18n.t('library.confirm_delete')}
  description={i18n.t('library.delete_confirm')}
  confirmLabel={i18n.t('library.delete_stash')}
  confirmVariant="danger"
  loading={deletingPending}
  onconfirm={handleDeleteStash}
  onclose={() => (deleteConfirmOpen = false)}
/>

<style>
  .title-picker-anchor {
    display: flex;
    min-width: 0;
  }

  .title-picker-anchor :global(.select-root),
  .title-picker-anchor :global(.select-custom-trigger) {
    min-width: 0;
  }

  .title-picker {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
    margin-inline-start: -12px;
    padding: 4px 8px 4px 12px;
    border: 0;
    border-radius: var(--radius-full);
    background: transparent;
    color: var(--text-primary);
    transition: background-color var(--duration-fast) var(--ease-standard);
  }

  .title-picker h1 {
    margin: 0;
    min-width: 0;
    overflow: hidden;
    font-family: var(--font-display);
    font-size: var(--type-title-lg-size);
    line-height: var(--type-title-lg-line);
    font-weight: 400;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .title-picker :global(svg) {
    flex: none;
    width: 20px;
    height: 20px;
    color: var(--text-secondary);
  }

  @media (hover: hover) {
    .title-picker:hover {
      background: var(--state-layer-hover);
    }
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

  .manage-stash {
    display: flex;
    flex-direction: column;
  }

  .stash-rename-row {
    display: flex;
    padding-block-start: 4px;
    padding-inline: max(0px, calc(var(--menu-inset) - 8px));
  }

  .stash-rename-row :global(.input-box) {
    flex: 1;
  }

  .stash-color-swatches {
    --swatch-size: 24px;
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    padding: 4px var(--menu-inset) 8px;
  }

  :global([data-layout='mobile']) .stash-color-swatches {
    --swatch-size: 28px;
  }

</style>
