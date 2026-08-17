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
  import ServiceIcon from '$lib/components/pawchive/ServiceIcon.svelte';
  import { ripple } from '$lib/motion';
  import { toast } from 'svelte-sonner';
  import { selectionState } from '$lib/state/selectionState.svelte';
  import { getPostDownloadTargets } from '$lib/utils/media';
  import SelectionActionBar from '$lib/components/ui/SelectionActionBar.svelte';
  import IconAdd from '~icons/fluent/add-24-regular';
  import IconCheckmark from '~icons/fluent/checkmark-20-regular';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';
  import IconDelete from '~icons/fluent/delete-24-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';
  import IconArrowClockwise from '~icons/fluent/arrow-clockwise-24-regular';
  import IconFilter from '~icons/fluent/filter-24-regular';
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

  const savedState = navigationState.getViewState<{
    searchQuery?: string;
    searchOpen?: boolean;
    selectedServices?: string[];
    selectedFormats?: string[];
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
      toast.success(i18n.t('selection.remove_from_stash') || `Removed ${items.length} posts from stash`);
      selectionState.exit();
    } catch (err) {
      toast.error(String(err));
    }
  }

  async function batchDeleteFromLibrary() {
    const items = selectionState.getItems<PawchivePost>();
    if (items.length === 0) return;
    try {
      for (const post of items) {
        await libraryState.remove(post);
      }
      toast.success(i18n.t('selection.remove_from_library') || `Deleted ${items.length} posts from library`);
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

  let manageOpen = $state(false);
  let stickyManageOpen = $state(false);
  let editStashName = $state('');
  let renamingPending = $state(false);
  let clearingPending = $state(false);

  let filtersOpen = $state(false);
  let stickyFiltersOpen = $state(false);

  let selectedServices = $state<string[]>(savedState?.selectedServices ?? []);
  let selectedFormats = $state<string[]>(savedState?.selectedFormats ?? []);
  let onlyWithAttachments = $state(savedState?.onlyWithAttachments ?? false);
  let onlyDownloaded = $state(savedState?.onlyDownloaded ?? false);

  let sortBy = $state<'added' | 'published' | 'title'>(savedState?.sortBy ?? 'added');
  let sortOrder = $state<'asc' | 'desc'>(savedState?.sortOrder ?? 'desc');
  let currentSortValue = $derived(`${sortBy}_${sortOrder}`);

  $effect(() => {
    navigationState.saveViewState(navigationState.entryKey, {
      searchQuery,
      searchOpen,
      selectedServices,
      selectedFormats,
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
    selectedServices.length +
    selectedFormats.length +
    (onlyWithAttachments ? 1 : 0) +
    (onlyDownloaded ? 1 : 0)
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
      toast.success(i18n.t('library.stash_renamed'));
      manageOpen = false;
      stickyManageOpen = false;
    } catch (error) {
      toast.error(error instanceof Error ? error.message : String(error));
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
      toast.success(i18n.t('library.stash_cleared'));
      manageOpen = false;
      stickyManageOpen = false;
    } catch (error) {
      toast.error(error instanceof Error ? error.message : String(error));
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
    if (selectedServices.includes(service)) {
      selectedServices = selectedServices.filter((s) => s !== service);
    } else {
      selectedServices = [...selectedServices, service];
    }
  }

  function toggleFormat(fmt: string) {
    if (selectedFormats.includes(fmt)) {
      selectedFormats = selectedFormats.filter((f) => f !== fmt);
    } else {
      selectedFormats = [...selectedFormats, fmt];
    }
  }

  function resetFilters() {
    selectedServices = [];
    selectedFormats = [];
    onlyWithAttachments = false;
    onlyDownloaded = false;
  }



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

  function matchesFormat(post: PawchivePost, format: string): boolean {
    const file = post.file as any;
    const attachments = (post.attachments as any[]) || [];
    
    const hasImage = !!(
      file?.type?.startsWith('image/') || 
      attachments.some(a => a.type?.startsWith('image/')) || 
      file?.name?.match(/\.(jpe?g|png|gif|webp|bmp)$/i) || 
      attachments.some(a => a.name?.match(/\.(jpe?g|png|gif|webp|bmp)$/i))
    );
    const hasVideo = !!(
      file?.type?.startsWith('video/') || 
      attachments.some(a => a.type?.startsWith('video/')) ||
      file?.name?.match(/\.(mp4|mkv|webm|mov|avi)$/i) || 
      attachments.some(a => a.name?.match(/\.(mp4|mkv|webm|mov|avi)$/i)) ||
      post.content?.includes('<video') || 
      post.content?.includes('iframe')
    );
    const hasAudio = !!(
      file?.type?.startsWith('audio/') || 
      attachments.some(a => a.type?.startsWith('audio/')) ||
      file?.name?.match(/\.(mp3|wav|ogg|flac|m4a|aac)$/i) || 
      attachments.some(a => a.name?.match(/\.(mp3|wav|ogg|flac|m4a|aac)$/i))
    );
    const hasArchive = !!(
      file?.name?.match(/\.(zip|rar|7z|tar|gz)$/i) || 
      attachments.some(a => a.name?.match(/\.(zip|rar|7z|tar|gz)$/i))
    );
    const hasText = !!(
      file?.name?.match(/\.(txt|md|pdf|doc|docx|epub)$/i) || 
      attachments.some(a => a.name?.match(/\.(txt|md|pdf|doc|docx|epub)$/i))
    );

    if (format === 'image') return hasImage;
    if (format === 'video') return hasVideo;
    if (format === 'audio') return hasAudio;
    if (format === 'archive') return hasArchive;
    if (format === 'text') return hasText;
    return false;
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

    if (selectedServices.length > 0) {
      list = list.filter(post => selectedServices.includes(post.service));
    }

    if (selectedFormats.length > 0) {
      list = list.filter(post => {
        return selectedFormats.some(fmt => matchesFormat(post, fmt));
      });
    }

    if (onlyWithAttachments) {
      list = list.filter(post => (post.attachment_count ?? post.attachments?.length ?? 0) > 0);
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
    <span class="manage-stash-badge">{libraryState.selectedCollection?.item_count ?? 0}</span>
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
  <div class="filter-heading">
    <strong>{i18n.t('feed.filters')}</strong>
    {#if activeFilterCount > 0}
      <button type="button" use:ripple onclick={resetFilters}>{i18n.t('feed.reset_filters')}</button>
    {/if}
  </div>

  <span class="filter-label">{i18n.t('feed.platform')}</span>
  <div class="service-options">
    <Button
      variant={selectedServices.length === 0 ? 'accent' : 'ghost'}
      size="sm"
      onclick={() => selectedServices = []}
      class="filter-chip"
    >
      <IconGlobe class="w-[14px] h-[14px]" />
      <span>{i18n.t('feed.all_platforms')}</span>
    </Button>
    {#each services as service}
      <Button
        variant={selectedServices.includes(service) ? 'accent' : 'ghost'}
        size="sm"
        onclick={() => toggleService(service)}
        class="filter-chip"
      >
        <ServiceIcon service={service} class="w-[14px] h-[14px]" />
        <span>{service}</span>
      </Button>
    {/each}
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

  <div class="view-option" class:active={onlyDownloaded}>
    <Checkbox
      checked={onlyDownloaded}
      onchange={(v) => onlyDownloaded = v}
    />
    <button onclick={() => onlyDownloaded = !onlyDownloaded}>
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
        <span class="tab-count">{libraryState.collections.reduce((sum, c) => sum + c.item_count, 0)}</span>
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
      icon={IconFilter}
    >
      {@render filterInnerContent()}
    </PopoverMenu>
  {:else}
    <PopoverMenu
      bind:open={filtersOpen}
      title={i18n.t('feed.filters')}
      badge={activeFilterCount}
      active={activeFilterCount > 0}
      icon={IconFilter}
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

<PageShell scrollable={true} scrollKey={navigationState.entryKey}>
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
        stateKey={`library:${libraryState.selectedCollectionId ?? 'all'}:services=${selectedServices.join(',')}:formats=${selectedFormats.join(',')}:attachments=${onlyWithAttachments}:downloaded=${onlyDownloaded}:sort=${sortBy}_${sortOrder}`}
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

  .tab-count {
    opacity: 0.55;
    font-size: 12px;
    font-weight: 500;
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

  .manage-stash-badge {
    font-size: 11px;
    padding: 2px 8px;
    border-radius: var(--radius-full);
    background: rgba(255, 255, 255, 0.08);
    color: var(--text-secondary);
    font-weight: 500;
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
