<script lang="ts">
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { subscriptionState } from '$lib/state/subscriptionState.svelte';
  import { libraryState } from '$lib/state/libraryState.svelte';
  import { i18n } from '$lib/i18n';
  import { notify } from '$lib/utils/toast';
  import Modal from '$lib/components/ui/Modal.svelte';
  import BottomSheet from '$lib/components/ui/BottomSheet.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import SettingItem from '$lib/components/ui/SettingItem.svelte';
  import NumberStepper from '$lib/components/ui/NumberStepper.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import type { InitialImport, DownloadScope } from '$lib/types/subscription';

  import IconDelete from '~icons/fluent/delete-24-regular';
  import IconCheckmark from '~icons/fluent/checkmark-24-regular';
  import IconPersonAdd from '~icons/fluent/person-add-24-regular';
  import IconArrowClockwise from '~icons/fluent/arrow-clockwise-24-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';
  import IconWarning from '~icons/fluent/warning-24-filled';

  const target = $derived(subscriptionState.dialogTarget);
  const isOpen = $derived(subscriptionState.dialogOpen && Boolean(target));

  const isBatch = $derived(Boolean(target?.batch && target.batch.length > 0));
  const existingSub = $derived(
    target && !isBatch ? subscriptionState.forCreator(target.service, target.creatorId) : undefined
  );
  const isEditing = $derived(Boolean(existingSub));

  let enabled = $state(true);
  let saveToLibrary = $state(true);
  let destination = $state('00000000-0000-0000-0000-000000000001');
  let initialImport = $state<InitialImport>('none');
  let autoDownload = $state(false);
  let downloadScope = $state<DownloadScope>('primary');
  let interval = $state(30);
  let intervalUnit = $state<'minutes' | 'hours' | 'days'>('minutes');
  let intervalValue = $state(30);

  let saving = $state(false);
  let checkingNow = $state(false);
  let savedPostCount = $state<number | null>(null);
  let countingPosts = $state(false);
  let confirmClearOpen = $state(false);
  let clearingPosts = $state(false);

  function minutesToUnit(mins: number): { value: number; unit: 'minutes' | 'hours' | 'days' } {
    if (mins >= 1440 && mins % 1440 === 0) {
      return { value: Math.min(7, Math.floor(mins / 1440)), unit: 'days' };
    }
    if (mins >= 60 && mins % 60 === 0) {
      return { value: Math.min(168, Math.floor(mins / 60)), unit: 'hours' };
    }
    return { value: Math.min(1440, Math.max(5, mins)), unit: 'minutes' };
  }

  function unitToMinutes(val: number, u: 'minutes' | 'hours' | 'days'): number {
    if (u === 'days') return Math.min(10080, Math.max(1, val) * 1440);
    if (u === 'hours') return Math.min(10080, Math.max(1, val) * 60);
    return Math.min(10080, Math.max(5, val));
  }

  function handleIntervalUnitChange(newUnit: 'minutes' | 'hours' | 'days') {
    if (newUnit === intervalUnit) return;
    const currentTotalMins = interval;
    intervalUnit = newUnit;
    if (newUnit === 'days') {
      intervalValue = Math.max(1, Math.min(7, Math.round(currentTotalMins / 1440) || 1));
    } else if (newUnit === 'hours') {
      intervalValue = Math.max(1, Math.min(168, Math.round(currentTotalMins / 60) || 1));
    } else {
      intervalValue = Math.max(5, Math.min(1440, currentTotalMins));
    }
    interval = unitToMinutes(intervalValue, intervalUnit);
  }

  function handleIntervalValueChange(val: number) {
    intervalValue = val;
    interval = unitToMinutes(val, intervalUnit);
  }

  const intervalMin = $derived(intervalUnit === 'minutes' ? 5 : 1);
  const intervalMax = $derived(intervalUnit === 'days' ? 7 : intervalUnit === 'hours' ? 168 : 1440);
  const intervalStep = $derived(intervalUnit === 'minutes' ? 15 : 1);

  const unitOptions = $derived([
    { value: 'minutes', label: i18n.t('subscriptions.unit_minutes') },
    { value: 'hours', label: i18n.t('subscriptions.unit_hours') },
    { value: 'days', label: i18n.t('subscriptions.unit_days') }
  ]);

  function formatRelativeTime(dateStr?: string | null): string {
    if (!dateStr) return i18n.t('subscriptions.never');
    const date = new Date(dateStr.endsWith('Z') ? dateStr : `${dateStr.replace(' ', 'T')}Z`);
    if (isNaN(date.getTime())) return dateStr;
    if (Math.abs(date.getTime() - Date.now()) < 45_000) return i18n.t('subscriptions.just_now');
    return i18n.relativeTime(date);
  }

  $effect(() => {
    if (isOpen && target) {
      if (existingSub) {
        enabled = existingSub.enabled;
        saveToLibrary = existingSub.save_to_library;
        destination = existingSub.destination_collection_id;
        initialImport = existingSub.initial_import;
        autoDownload = existingSub.auto_download;
        downloadScope = existingSub.download_scope;
        interval = existingSub.poll_interval_minutes;

        const decomposed = minutesToUnit(existingSub.poll_interval_minutes);
        intervalValue = decomposed.value;
        intervalUnit = decomposed.unit;

        countingPosts = true;
        subscriptionState
          .countPosts(existingSub.id)
          .then((count) => {
            savedPostCount = count;
          })
          .catch(() => {
            savedPostCount = null;
          })
          .finally(() => {
            countingPosts = false;
          });
      } else {
        enabled = true;
        saveToLibrary = true;
        const defaultInbox = libraryState.collections.find((c) => c.kind === 'inbox');
        destination = defaultInbox ? defaultInbox.id : '00000000-0000-0000-0000-000000000001';
        initialImport = 'none';
        autoDownload = false;
        downloadScope = 'primary';
        interval = 30;
        intervalValue = 30;
        intervalUnit = 'minutes';
        savedPostCount = null;
      }
    }
  });

  const stashOptions = $derived(libraryState.stashOptions);

  const destinationStashName = $derived.by(() => {
    const col = libraryState.collections.find((c) => c.id === destination);
    return col ? libraryState.getStashDisplayName(col) : 'Inbox';
  });

  const titleText = $derived.by(() => {
    if (isBatch) {
      return i18n.t('subscriptions.batch_subscribe_title', { count: target?.batch?.length ?? 0 });
    }
    if (isEditing) {
      return `${i18n.t('subscriptions.edit')} · ${target?.creatorName ?? ''}`;
    }
    return `${i18n.t('subscriptions.subscribe')} · ${target?.creatorName ?? ''}`;
  });

  function handleClose() {
    if (saving || clearingPosts || checkingNow) return;
    subscriptionState.closeDialog();
  }

  async function handleCheckNow() {
    if (!existingSub || checkingNow || saving) return;
    checkingNow = true;
    try {
      await subscriptionState.refresh(existingSub.id);
      await libraryState.refreshCollections();
      await libraryState.refreshSavedKeys();
      if (libraryState.selectedCollectionId === destination) {
        await libraryState.refresh();
      }
      const count = await subscriptionState.countPosts(existingSub.id);
      savedPostCount = count;
      notify.success(i18n.t('subscriptions.check_now_success', { creator: target?.creatorName ?? '' }));
    } catch (err) {
      notify.error(i18n.t('subscriptions.check_now_failed'), err);
    } finally {
      checkingNow = false;
    }
  }

  async function handleSave() {
    if (!target) return;
    saving = true;
    try {
      if (isBatch && target.batch) {
        for (const creator of target.batch) {
          await subscriptionState.save({
            service: creator.service,
            creator_id: creator.id,
            creator_name: creator.name || creator.id,
            destination_collection_id: saveToLibrary ? destination : undefined,
            save_to_library: saveToLibrary,
            enabled,
            initial_import: initialImport,
            auto_download: autoDownload,
            download_scope: downloadScope,
            poll_interval_minutes: interval
          });
        }
        notify.success(
          i18n.t('subscriptions.saved'),
          i18n.t('counts.creators', { count: target.batch.length })
        );
      } else {
        await subscriptionState.save({
          service: target.service,
          creator_id: target.creatorId,
          creator_name: target.creatorName,
          destination_collection_id: saveToLibrary ? destination : undefined,
          save_to_library: saveToLibrary,
          enabled,
          initial_import: initialImport,
          auto_download: autoDownload,
          download_scope: downloadScope,
          poll_interval_minutes: interval
        });
        notify.success(i18n.t('subscriptions.saved'), target.creatorName);
      }
      subscriptionState.closeDialog();
    } catch (err) {
      notify.error(i18n.t('subscriptions.action_error'), err);
    } finally {
      saving = false;
    }
  }

  function handleUnsubscribeClick() {
    if (!existingSub || !target) return;
    const sub = existingSub;
    const name = target.creatorName;
    subscriptionState.closeDialog();
    subscriptionState.promptUnsubscribe(sub, name);
  }

  async function handleClearPostsConfirm() {
    if (!existingSub || !target) return;
    clearingPosts = true;
    try {
      const cleared = await subscriptionState.clearPosts(existingSub.id);
      savedPostCount = 0;
      await libraryState.refreshCollections();
      await libraryState.refreshSavedKeys();
      if (libraryState.selectedCollectionId === destination) {
        await libraryState.refresh();
      }
      notify.success(
        i18n.t('subscriptions.clear_posts_success', {
          count: cleared,
          stash: destinationStashName
        }),
        { glyph: 'cleared' }
      );
      confirmClearOpen = false;
    } catch (err) {
      notify.error(i18n.t('subscriptions.action_error'), err);
    } finally {
      clearingPosts = false;
    }
  }
</script>

{#snippet modalBody()}
  <div class="sub-modal-body flex flex-col gap-3 text-[var(--text-primary)]">
    {#if isEditing && existingSub}
      <div class="sub-status-card">
        <div class="sub-status-header">
          <div class="sub-status-indicator">
            <span
              class="sub-status-dot"
              class:is-active={enabled && !existingSub.last_error}
              class:is-paused={!enabled}
              class:is-error={Boolean(existingSub.last_error)}
            ></span>
            <span class="sub-status-text">
              {#if !enabled}
                {i18n.t('subscriptions.status_paused')}
              {:else if existingSub.last_error}
                {i18n.t('subscriptions.status_error')}
              {:else}
                {i18n.t('subscriptions.status_active')}
              {/if}
            </span>
          </div>

          <Button
            variant="ghost"
            data-tone="accent"
            size="sm"
            onclick={handleCheckNow}
            disabled={checkingNow || saving}
            class="sub-check-now-btn"
          >
            {#if checkingNow}
              <IconLoading class="w-4 h-4 mr-1.5 shrink-0" />
              <span>{i18n.t('subscriptions.checking')}</span>
            {:else}
              <IconArrowClockwise class="w-4 h-4 mr-1.5 shrink-0" />
              <span>{i18n.t('subscriptions.check_now')}</span>
            {/if}
          </Button>
        </div>

        <div class="sub-status-meta">
          <div class="sub-meta-item">
            <span class="sub-meta-label">{i18n.t('subscriptions.last_checked')}:</span>
            <span class="sub-meta-value">{formatRelativeTime(existingSub.last_checked_at)}</span>
          </div>
          {#if enabled}
            <div class="sub-meta-item">
              <span class="sub-meta-label">{i18n.t('subscriptions.next_check')}:</span>
              <span class="sub-meta-value">{formatRelativeTime(existingSub.next_check_at)}</span>
            </div>
          {/if}
        </div>

        {#if existingSub.last_error}
          <div class="sub-error-banner">
            <IconWarning class="w-4 h-4 shrink-0 text-[var(--status-error)]" />
            <div class="sub-error-content">
              <span class="sub-error-message">{existingSub.last_error}</span>
              {#if existingSub.failure_count > 1}
                <span class="sub-error-count">
                  {i18n.t('subscriptions.failure_count', { count: existingSub.failure_count })}
                </span>
              {/if}
            </div>
          </div>
        {/if}
      </div>
    {/if}

    <div class="settings-list subscription-settings">
      <SettingItem
        title={i18n.t('subscriptions.active')}
        description={i18n.t('subscriptions.active_desc')}
        checked={enabled}
        onCheckedChange={(value) => (enabled = value)}
      />

      <SettingItem
        title={i18n.t('subscriptions.save_to_library')}
        description={i18n.t('subscriptions.save_to_library_desc')}
        checked={saveToLibrary}
        onCheckedChange={(value) => (saveToLibrary = value)}
      />

      {#if saveToLibrary}
        <SettingItem
          title={i18n.t('subscriptions.destination')}
          description={i18n.t('subscriptions.destination_desc')}
        >
          <Select
            options={stashOptions}
            value={destination}
            ariaLabel={i18n.t('subscriptions.destination')}
            onchange={(v) => (destination = v)}
          />
        </SettingItem>

        <SettingItem
          title={i18n.t('subscriptions.initial_import')}
          description={i18n.t('subscriptions.initial_import_desc')}
        >
          <Select
            options={[
              { value: 'none', label: i18n.t('subscriptions.import_none') },
              { value: 'latest', label: i18n.t('subscriptions.import_latest') },
              { value: 'all', label: i18n.t('subscriptions.import_all') }
            ]}
            value={initialImport}
            ariaLabel={i18n.t('subscriptions.initial_import')}
            onchange={(v) => (initialImport = v as InitialImport)}
          />
        </SettingItem>
      {/if}

      <SettingItem
        title={i18n.t('subscriptions.auto_download')}
        description={i18n.t('subscriptions.auto_download_desc')}
        checked={autoDownload}
        onCheckedChange={(value) => (autoDownload = value)}
      />

      {#if autoDownload}
        <SettingItem
          title={i18n.t('subscriptions.download_scope')}
          description={i18n.t('subscriptions.download_scope_desc')}
        >
          <Select
            options={[
              { value: 'primary', label: i18n.t('subscriptions.primary_file') },
              { value: 'all', label: i18n.t('subscriptions.all_files') }
            ]}
            value={downloadScope}
            ariaLabel={i18n.t('subscriptions.download_scope')}
            onchange={(v) => (downloadScope = v as DownloadScope)}
          />
        </SettingItem>
      {/if}

      <SettingItem
        title={i18n.t('subscriptions.interval')}
        description={i18n.t('subscriptions.interval_desc')}
        class="interval-setting-item"
      >
        <div class="interval-control-group flex items-center gap-2">
          <NumberStepper
            value={intervalValue}
            min={intervalMin}
            max={intervalMax}
            step={intervalStep}
            size="sm"
            onchange={handleIntervalValueChange}
          />
          <div class="interval-unit-select">
            <Select
              options={unitOptions}
              value={intervalUnit}
              onchange={(v) => handleIntervalUnitChange(v as 'minutes' | 'hours' | 'days')}
              size="sm"
              ariaLabel={i18n.t('subscriptions.interval_unit')}
            />
          </div>
        </div>
      </SettingItem>
    </div>

    {#if isEditing && existingSub}
      <div class="settings-list subscription-settings">
        <SettingItem
          title={i18n.t('subscriptions.clear_posts')}
          description={i18n.t('subscriptions.clear_posts_desc')}
        >
          <div class="flex items-center gap-3">
            {#if countingPosts}
              <span class="text-xs text-[var(--text-muted)]">...</span>
            {:else if savedPostCount !== null}
              <span class="text-xs font-medium text-[var(--text-muted)]">
                {savedPostCount > 0
                  ? i18n.t('subscriptions.posts_in_stash', { count: savedPostCount, stash: destinationStashName })
                  : i18n.t('subscriptions.no_posts_in_stash', { stash: destinationStashName })}
              </span>
            {/if}

            <Button
              variant="ghost"
              data-tone="danger"
              size="sm"
              onclick={() => (confirmClearOpen = true)}
              disabled={clearingPosts || countingPosts || !savedPostCount}
            >
              {#if clearingPosts}
                <IconLoading class="w-4 h-4 mr-1.5 shrink-0" />
              {:else}
                <IconDelete class="w-4 h-4 mr-1.5 shrink-0" />
              {/if}
              <span>{i18n.t('subscriptions.clear_posts')}</span>
            </Button>
          </div>
        </SettingItem>
      </div>
    {/if}
  </div>
{/snippet}

{#snippet modalFooter()}
  <div class="dialog-actions">
    {#if isEditing}
      <Button
        variant="ghost"
        data-tone="danger"
        size="md"
        class="mr-auto"
        onclick={handleUnsubscribeClick}
        disabled={saving || clearingPosts || checkingNow}
      >
        <IconDelete class="w-4 h-4 mr-1.5 shrink-0" />
        <span class="truncate">{i18n.t('subscriptions.unsubscribe')}</span>
      </Button>
    {/if}

    <Button
      variant="ghost"
      size="md"
      onclick={handleClose}
      disabled={saving || clearingPosts || checkingNow}
    >
      <span class="truncate">{i18n.t('common.cancel')}</span>
    </Button>

    <Button
      variant="ghost"
      data-tone="accent"
      size="md"
      onclick={handleSave}
      disabled={saving || clearingPosts || checkingNow}
    >
      {#if saving}
        <IconLoading class="w-4 h-4 mr-1.5 shrink-0" />
      {:else if isEditing}
        <IconCheckmark class="w-4 h-4 mr-1.5 shrink-0" />
      {:else}
        <IconPersonAdd class="w-4 h-4 mr-1.5 shrink-0" />
      {/if}
      <span class="truncate">
        {i18n.t(isEditing ? 'subscriptions.save' : 'subscriptions.subscribe')}
      </span>
    </Button>
  </div>
{/snippet}

{#if isOpen}
  {#if layoutState.isMobile}
    <BottomSheet
      {isOpen}
      title={titleText}
      closeLabel={i18n.t('common.close')}
      onclose={handleClose}
    >
      {#snippet children()}
        {@render modalBody()}
      {/snippet}
      {#snippet footer()}
        {@render modalFooter()}
      {/snippet}
    </BottomSheet>
  {:else}
    <Modal
      {isOpen}
      title={titleText}
      size="lg"
      borderlessFooter
      onclose={handleClose}
    >
      {#snippet children()}
        {@render modalBody()}
      {/snippet}
      {#snippet footer()}
        {@render modalFooter()}
      {/snippet}
    </Modal>
  {/if}
{/if}

{#if confirmClearOpen && target}
  <ConfirmDialog
    isOpen={confirmClearOpen}
    title={i18n.t('subscriptions.clear_posts_confirm', { creator: target.creatorName, stash: destinationStashName })}
    description={i18n.t('subscriptions.clear_posts_confirm_desc', { count: savedPostCount ?? 0, stash: destinationStashName })}
    confirmLabel={i18n.t('subscriptions.clear_posts')}
    confirmVariant="danger"
    heroIcon={IconWarning}
    loading={clearingPosts}
    onclose={() => (confirmClearOpen = false)}
    onconfirm={handleClearPostsConfirm}
  />
{/if}

<style>
  .subscription-settings {
    --settings-tile: var(--surface-container-highest);
  }

  .sub-modal-body {
    min-width: 0;
    width: 100%;
  }

  .sub-status-card {
    display: flex;
    flex-direction: column;
    gap: calc(10px * var(--ui-scale, 1));
    padding: calc(12px * var(--ui-scale, 1)) calc(14px * var(--ui-scale, 1));
    background: var(--settings-row-bg, var(--bg-card));
    border-radius: calc(var(--radius-sm) * var(--ui-scale, 1));
    box-sizing: border-box;
  }

  .sub-status-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: calc(12px * var(--ui-scale, 1));
  }

  .sub-status-indicator {
    display: inline-flex;
    align-items: center;
    gap: calc(8px * var(--ui-scale, 1));
  }

  .sub-status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .sub-status-dot.is-active {
    background: #22c55e;
    box-shadow: 0 0 8px rgba(34, 197, 94, 0.4);
  }

  .sub-status-dot.is-paused {
    background: var(--text-muted);
  }

  .sub-status-dot.is-error {
    background: var(--danger-primary, #ef4444);
    box-shadow: 0 0 8px rgba(239, 68, 68, 0.4);
  }

  .sub-status-text {
    font-size: calc(13.5px * var(--ui-scale, 1));
    font-weight: 600;
    color: var(--text-primary);
  }

  .sub-status-meta {
    display: flex;
    align-items: center;
    gap: calc(16px * var(--ui-scale, 1));
    flex-wrap: wrap;
    font-size: calc(12px * var(--ui-scale, 1));
    color: var(--text-secondary);
  }

  .sub-meta-item {
    display: inline-flex;
    align-items: center;
    gap: calc(5px * var(--ui-scale, 1));
  }

  .sub-meta-label {
    color: var(--text-muted);
  }

  .sub-meta-value {
    color: var(--text-primary);
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }

  .sub-error-banner {
    display: flex;
    align-items: flex-start;
    gap: calc(8px * var(--ui-scale, 1));
    padding: calc(8px * var(--ui-scale, 1)) calc(10px * var(--ui-scale, 1));
    border-radius: calc(var(--radius-xs, 6px) * var(--ui-scale, 1));
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.2);
    font-size: calc(12px * var(--ui-scale, 1));
    color: var(--text-primary);
  }

  .sub-error-content {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .sub-error-message {
    word-break: break-word;
    color: var(--danger-primary, #ef4444);
  }

  .sub-error-count {
    color: var(--text-muted);
    font-size: calc(11px * var(--ui-scale, 1));
  }

  .sub-modal-body :global(.setting-item) {
    --settings-control-max-width: 210px;
  }

  .sub-modal-body :global(.setting-item__title) {
    white-space: nowrap;
  }

  .sub-modal-body :global(.select-root) {
    width: 210px !important;
    max-width: 210px !important;
    min-width: 0 !important;
  }

  .sub-modal-body :global(.interval-setting-item) {
    --settings-control-max-width: 210px;
  }

  .interval-control-group {
    display: flex;
    align-items: center;
    gap: calc(6px * var(--ui-scale, 1));
    flex-shrink: 0;
  }

  .sub-modal-body :global(.interval-setting-item .interval-unit-select .select-root) {
    width: 86px !important;
    max-width: 86px !important;
    min-width: 76px !important;
  }

  :global(.bottom-sheet) .sub-modal-body {
    padding-bottom: 12px;
  }

  :global(.bottom-sheet) .sub-modal-body :global(.setting-item) {
    min-height: calc(48px * var(--ui-scale, 1));
    padding: calc(6px * var(--ui-scale, 1)) calc(12px * var(--ui-scale, 1));
    gap: calc(10px * var(--ui-scale, 1));
    --settings-control-max-width: 160px;
  }

  :global(.bottom-sheet) .sub-modal-body :global(.setting-item__title) {
    font-size: calc(13px * var(--ui-scale, 1));
    white-space: nowrap;
  }

  :global(.bottom-sheet) .sub-modal-body :global(.select-root) {
    width: 160px !important;
    max-width: 160px !important;
    height: 38px !important;
  }

  :global(.bottom-sheet) .sub-modal-body :global(.select-trigger) {
    height: 38px !important;
    min-height: 38px !important;
    padding: 0 10px !important;
    font-size: 13px !important;
  }

  :global(.bottom-sheet) .sub-modal-body :global(.interval-setting-item) {
    --settings-control-max-width: 200px;
  }

  :global(.bottom-sheet) .sub-modal-body :global(.interval-setting-item .interval-unit-select .select-root) {
    width: 78px !important;
    max-width: 78px !important;
    min-width: 68px !important;
    height: 34px !important;
  }

  :global(.bottom-sheet) .sub-modal-body :global(.interval-setting-item .interval-unit-select .select-trigger) {
    height: 34px !important;
    min-height: 34px !important;
    padding: 0 8px !important;
    font-size: 12px !important;
  }

  :global(.bottom-sheet) .sub-modal-body :global(.number-stepper) {
    height: 34px !important;
  }

  :global(.bottom-sheet) .sub-modal-body :global(.number-stepper__button) {
    width: 28px !important;
    height: 28px !important;
  }

  @media (max-width: 500px) {
    .sub-modal-body :global(.setting-item) {
      --settings-control-max-width: 160px;
    }

    .sub-modal-body :global(.setting-item__title) {
      white-space: normal;
    }

    .sub-modal-body :global(.select-root) {
      width: 160px !important;
      max-width: 160px !important;
    }

    .sub-modal-body :global(.interval-setting-item) {
      --settings-control-max-width: 200px;
    }
  }
</style>
