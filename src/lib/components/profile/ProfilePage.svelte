<script lang="ts">
  import { onMount } from 'svelte';
  import { syncState } from '$lib/state/syncState.svelte';
  import { avatarState } from '$lib/state/avatarState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { i18n } from '$lib/i18n';
  import PageShell from '$lib/components/layout/PageShell.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import SettingItem from '$lib/components/ui/SettingItem.svelte';
  import SettingsGroup from '$lib/components/settings/SettingsGroup.svelte';
  import { syncStatusText } from '$lib/utils/syncStatus';
  import IconLock from '~icons/fluent/lock-closed-24-regular';
  import IconKey from '~icons/fluent/key-24-regular';
  import IconSignOut from '~icons/fluent/sign-out-24-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';
  import IconCopy from '~icons/fluent/copy-24-regular';
  import IconArrowDownload from '~icons/fluent/arrow-download-24-regular';
  import IconSync from '~icons/fluent/arrow-sync-24-regular';
  import IconDice from '~icons/fluent/cube-24-regular';
  import IconPerson from '~icons/fluent/person-24-regular';
  import IconGallery from '~icons/fluent/image-multiple-24-regular';
  import IconTrash from '~icons/fluent/delete-24-regular';
  import IconGlobe from '~icons/fluent/globe-24-regular';
  import IconLaptop from '~icons/fluent/laptop-24-regular';
  import IconDevices from '~icons/fluent/phone-laptop-24-regular';
  import IconShield from '~icons/fluent/shield-lock-24-regular';
  import IconChevronDown from '~icons/fluent/chevron-down-24-regular';
  import { notify } from '$lib/utils/toast';
  import { copyText } from '$lib/utils/clipboard';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { tooltip } from '$lib/motion';
  import { onboardingState } from '$lib/state/onboardingState.svelte';
  import { apiTestSyncConnection } from '$lib/utils/ipc';
  import { readRecentLogs } from '$lib/utils/logger';

  type SubView =
    | 'menu'
    | 'connect'
    | 'create'
    | 'recover'
    | 'recovery_kit'
    | 'change_password'
    | 'unlock';

  let activeView = $state<SubView>('menu');

  let syncServer = $state('https://pawstash.nichind.dev');
  let syncAccount = $state('');
  let syncPassword = $state('');
  let syncDevice = $state(layoutState.isAndroid ? 'Pawstash Android' : layoutState.isMobileDevice ? 'Pawstash Mobile' : 'Pawstash Desktop');
  let showAdvanced = $state(false);
  let syncRecoveryInput = $state('');

  let unlockPassword = $state('');
  let currentPassword = $state('');
  let newPassword = $state('');
  let recoveryKitText = $state('');

  let testingConnection = $state(false);
  let copyingLogs = $state(false);
  let confirmDisconnect = $state(false);
  let avatarInput = $state<HTMLInputElement | null>(null);

  const VIEW_TITLES: Record<SubView, string> = {
    menu: 'profile.title',
    connect: 'sync.connect_account',
    create: 'sync.create_account',
    recover: 'sync.recover_account',
    recovery_kit: 'sync.recovery_kit_title',
    change_password: 'sync.change_password_title',
    unlock: 'sync.unlock_title'
  };

  let viewTitle = $derived(i18n.t(VIEW_TITLES[activeView]));

  let serverHost = $derived.by(() => {
    const url = syncState.status.server_url;
    if (!url) return '';
    try {
      return new URL(url).host;
    } catch {
      return url;
    }
  });

  function generateRandomId(): string {
    const chars = 'abcdef0123456789';
    let rand = '';
    for (let i = 0; i < 8; i++) {
      rand += chars[Math.floor(Math.random() * chars.length)];
    }
    return rand;
  }

  onMount(() => {
    syncAccount = generateRandomId();
    if (onboardingState.takeProfileView() === 'create' && !syncState.status.configured) activeView = 'create';
  });

  async function runSyncAction(work: () => Promise<unknown>, successMessage?: string): Promise<boolean> {
    try {
      await work();
      if (successMessage) notify.success(successMessage);
      activeView = 'menu';
      return true;
    } catch (error: any) {
      const errStr = String(error?.message || error);
      if (!syncState.status.configured && activeView === 'create' && errStr.toLowerCase().includes('account already exists')) {
        notify.error(i18n.t('sync.account_taken'), undefined, {
          action: {
            label: i18n.t('sync.generate_new_id'),
            onclick: () => {
              syncAccount = generateRandomId();
            }
          }
        });
        syncAccount = generateRandomId();
      } else if (errStr.toLowerCase().includes('disconnect the current sync account first') || errStr.toLowerCase().includes('already configured')) {
        notify.error(i18n.t('sync.error'), errStr, {
          action: {
            label: i18n.t('sync.disconnect'),
            onclick: async () => {
              try {
                await syncState.disconnect();
                notify.success(i18n.t('sync.disconnected'));
              } catch (e) {
                notify.error(i18n.t('sync.error'), e);
              }
            }
          }
        });
      } else {
        notify.error(i18n.t('sync.error'), errStr, {
          action: { label: i18n.t('common.copy'), onclick: () => copyText(errStr) }
        });
      }
      return false;
    } finally {
      syncPassword = '';
    }
  }

  async function handleTestConnection() {
    if (testingConnection || !syncServer.trim()) return;
    testingConnection = true;
    try {
      await apiTestSyncConnection(syncServer.trim());
      notify.success(
        i18n.t('sync.test_connection_success_title'),
        i18n.t('sync.test_connection_success_desc')
      );
    } catch (e) {
      const err = String((e as any)?.message || e);
      notify.error(i18n.t('sync.test_connection_failed_title'), err, {
        action: { label: i18n.t('common.copy'), onclick: () => copyText(err) }
      });
    } finally {
      testingConnection = false;
    }
  }

  async function handleAvatarPicked(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = '';
    if (!file) return;
    try {
      await avatarState.setFromFile(file);
      notify.success(i18n.t('profile.avatar_saved'));
    } catch (error) {
      notify.error(i18n.t('profile.avatar_failed'), error);
    }
  }

  async function handleClearAvatar() {
    try {
      await avatarState.clear();
      notify.success(i18n.t('profile.avatar_removed'));
    } catch (error) {
      notify.error(i18n.t('profile.avatar_failed'), error);
    }
  }

  async function handleCopySyncLogs() {
    if (copyingLogs) return;
    copyingLogs = true;
    try {
      const raw = await readRecentLogs(400);
      if (!raw || !raw.trim()) {
        notify.info(i18n.t('sync.no_logs'));
        return;
      }
      const lines = raw.split('\n');
      const syncLines = lines.filter((l) => /sync|vault|pwsec|pawstash.*sync/i.test(l));
      const payload = syncLines.length > 0 ? syncLines.join('\n') : lines.slice(-100).join('\n');
      await navigator.clipboard.writeText(payload);
      notify.success(
        i18n.t('sync.logs_copied_title'),
        i18n.t('sync.logs_copied_desc', { count: syncLines.length || 100 })
      );
    } catch (e) {
      notify.error(i18n.t('sync.copy_logs_failed'), e);
    } finally {
      copyingLogs = false;
    }
  }

  async function handleSyncSubmit() {
    const account = syncAccount.trim();
    const server = syncServer.trim();
    if (activeView === 'create' || activeView === 'connect') {
      if (account.length < 8) {
        notify.error(
          i18n.t('sync.account_id_too_short_title'),
          i18n.t('sync.account_id_too_short_desc')
        );
        return;
      }
      const validIdRegex = /^[A-Za-z0-9_-]{8,128}$/;
      if (!validIdRegex.test(account)) {
        notify.error(
          i18n.t('sync.account_id_invalid_title'),
          i18n.t('sync.account_id_invalid_desc')
        );
        return;
      }
    }

    if (activeView === 'create') {
      if (await runSyncAction(() => syncState.create(server, account, syncPassword, syncDevice), i18n.t('sync.configured'))) {
        await openRecoveryView();
      }
    } else if (activeView === 'connect') {
      await runSyncAction(
        () => syncState.connect(server, account, syncPassword, syncDevice),
        i18n.t('sync.configured')
      );
    } else if (activeView === 'recover') {
      await runSyncAction(
        () => syncState.recover(syncRecoveryInput, syncPassword, syncDevice),
        i18n.t('sync.configured')
      );
    }
  }

  async function openRecoveryView() {
    try {
      recoveryKitText = await syncState.getRecoveryKit();
      activeView = 'recovery_kit';
    } catch (error) {
      notify.error(i18n.t('sync.error'), error);
    }
  }

  async function copyRecoveryKit() {
    await runSyncAction(() => syncState.copyRecoveryKit(), i18n.t('sync.recovery_copied'));
  }

  function downloadRecoveryFile() {
    if (!recoveryKitText) return;
    const blob = new Blob([recoveryKitText], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `pawstash-recovery-kit-${syncState.status.account_id || 'vault'}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    notify.success(i18n.t('sync.recovery_saved'));
  }

  async function handleUnlockSubmit() {
    if (!unlockPassword) return;
    await runSyncAction(async () => {
      await syncState.unlock(unlockPassword);
      unlockPassword = '';
      notify.success(i18n.t('sync.status_ready'));
    });
  }

  async function handleChangePasswordSubmit() {
    if (!currentPassword || newPassword.length < 12) return;
    await runSyncAction(async () => {
      await syncState.changePassword(currentPassword, newPassword);
      currentPassword = '';
      newPassword = '';
      notify.success(i18n.t('sync.password_changed'));
    });
  }

  let parsedRecovery = $derived.by(() => {
    if (!recoveryKitText) return null;
    try {
      const data = JSON.parse(recoveryKitText);
      return {
        accountId: data.account_id || syncState.status.account_id,
        vaultKey: data.secrets?.vault_key || ''
      };
    } catch {
      return null;
    }
  });
</script>

{#snippet fieldLabel(text: string)}
  <span class="profile-label">{text}</span>
{/snippet}

{#snippet serverField()}
  <div class="profile-field">
    <div class="profile-label-row">
      {@render fieldLabel(i18n.t('sync.server_url'))}
      <button
        type="button"
        class="profile-link"
        disabled={testingConnection || !syncServer.trim()}
        onclick={() => void handleTestConnection()}
      >
        {#if testingConnection}<IconLoading />{/if}
        {i18n.t('sync.test_connection')}
      </button>
    </div>
    <Input icon={IconGlobe} clearable={true} bind:value={syncServer} placeholder="https://pawstash.nichind.dev" />
  </div>
{/snippet}

{#snippet deviceField()}
  <div class="profile-field">
    {@render fieldLabel(i18n.t('sync.device_name'))}
    <Input icon={IconLaptop} clearable={true} bind:value={syncDevice} placeholder="Pawstash Desktop" />
  </div>
{/snippet}

{#snippet submitButton(label: string, disabled: boolean)}
  <Button type="submit" variant="accent" class="profile-submit" {disabled}>
    {#if syncState.busy}<IconLoading class="mr-2" />{/if}
    {label}
  </Button>
{/snippet}

<PageShell
  title={viewTitle}
  back={activeView === 'menu' ? undefined : () => (activeView = 'menu')}
  scrollKey={navigationState.entryKey}
>
  <div class="profile-body">
    {#if activeView === 'menu'}
      <input
        class="avatar-input"
        type="file"
        accept="image/*"
        bind:this={avatarInput}
        onchange={handleAvatarPicked}
      />

      {#if syncState.status.configured}
        <section class="account-card">
          <div class="account-avatar">
            <button
              type="button"
              class="account-avatar-change"
              aria-label={i18n.t('profile.change_photo')}
              disabled={avatarState.busy}
              use:tooltip={{ text: i18n.t('profile.change_photo'), placement: 'top' }}
              onclick={() => avatarInput?.click()}
            >
              {#if avatarState.dataUrl}
                <img src={avatarState.dataUrl} alt="" />
              {:else}
                <IconPerson />
              {/if}
              <span class="account-avatar-overlay" aria-hidden="true"><IconGallery /></span>
            </button>
            {#if avatarState.dataUrl}
              <button
                type="button"
                class="account-avatar-remove"
                aria-label={i18n.t('profile.remove_photo')}
                disabled={avatarState.busy}
                use:tooltip={{ text: i18n.t('profile.remove_photo'), placement: 'top' }}
                onclick={() => void handleClearAvatar()}
              >
                <IconTrash />
              </button>
            {/if}
          </div>
          <div class="account-text">
            <h2 class="account-id">{syncState.status.account_id}</h2>
            <span class="account-status">
              {syncStatusText()}{#if serverHost}{` · ${serverHost}`}{/if}
            </span>
          </div>
          <Button
            variant="tonal"
            disabled={syncState.busy || !syncState.status.unlocked}
            onclick={() => void runSyncAction(() => syncState.sync(), i18n.t('sync.completed'))}
          >
            {#if syncState.busy}<IconLoading class="mr-2" />{:else}<IconSync class="mr-2" />{/if}
            {i18n.t('sync.sync_now')}
          </Button>
        </section>

        {#if syncState.status.last_error}
          <SettingsGroup>
            <SettingItem
              title={i18n.t('sync.sync_failed')}
              description={syncState.status.last_error}
              action
              danger
              onclick={() => copyText(syncState.status.last_error || '')}
            />
          </SettingsGroup>
        {/if}

        <SettingsGroup>
          {#if syncState.status.unlocked}
            <SettingItem
              title={i18n.t('sync.lock')}
              description={i18n.t('sync.lock_desc')}
              action
              onclick={() => void runSyncAction(() => syncState.lock())}
            />
          {:else}
            <SettingItem
              title={i18n.t('sync.unlock')}
              description={i18n.t('sync.locked')}
              onclick={() => (activeView = 'unlock')}
            />
          {/if}
          <SettingItem
            title={i18n.t('sync.export_recovery_kit')}
            description={i18n.t('sync.recovery_warning')}
            disabled={!syncState.status.unlocked}
            onclick={() => void openRecoveryView()}
          />
          <SettingItem title={i18n.t('sync.change_password')} onclick={() => (activeView = 'change_password')} />
        </SettingsGroup>

        <SettingsGroup>
          <SettingItem
            title={i18n.t('sync.copy_logs')}
            action
            disabled={copyingLogs}
            onclick={() => void handleCopySyncLogs()}
          />
          <SettingItem
            title={i18n.t('sync.disconnect')}
            action
            danger
            disabled={syncState.busy}
            onclick={() => (confirmDisconnect = true)}
          />
        </SettingsGroup>
      {:else}
        <div class="profile-setup">
          <svg viewBox="0 0 602 602" fill="none" class="setup-mark" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
            <defs>
              <linearGradient id="logo-profile-grad" x1="301" y1="0" x2="-2.17166e-05" y2="584.337" gradientUnits="userSpaceOnUse">
                <stop stop-color="#FCD8D2"/>
                <stop offset="1" stop-color="#FEB8AD"/>
              </linearGradient>
            </defs>
            <g transform="translate(0, 8.5)">
              <path fill="url(#logo-profile-grad)" d="M130.548 56.3212L414.821 178.14L301 226.902L18.361 105.771C24.725 99.2782 32.508 94.0322 41.366 90.6352L130.548 56.3212ZM188.082 34.2192L254.732 8.59119C284.529 -2.86373 317.514 -2.86373 347.311 8.59119L560.677 90.6352C569.492 94.0752 577.275 99.2352 583.639 105.771L469.431 154.705L188.082 34.2192ZM601.742 144.815L322.5 264.484V584.834C330.957 583.401 339.227 581.136 347.311 578.04L560.677 495.953C572.841 491.269 583.301 483.01 590.677 472.264C598.054 461.517 602.002 448.788 602 435.753V150.835C602 148.829 601.9 146.822 601.699 144.815M279.5 584.834V264.484L0.300999 144.815C0.130354 146.818 0.0299598 148.826 0 150.835V435.753C0.00172613 448.793 3.95568 461.526 11.3404 472.273C18.7252 483.02 29.1939 491.276 41.366 495.953L254.689 578.04C262.773 581.136 271.043 583.401 279.5 584.834Z" />
            </g>
          </svg>
          <h2 class="setup-title">{i18n.t('sync.title')}</h2>
          <ul class="setup-benefits">
            <li><IconDevices /><span>{i18n.t('sync.benefit_devices')}</span></li>
            <li><IconShield /><span>{i18n.t('sync.benefit_private')}</span></li>
          </ul>
          <div class="setup-actions">
            <Button
              variant="accent"
              onclick={() => {
                syncAccount = generateRandomId();
                activeView = 'create';
              }}
            >
              {i18n.t('sync.mode_create')}
            </Button>
            <Button variant="tonal" onclick={() => (activeView = 'connect')}>
              {i18n.t('sync.mode_connect')}
            </Button>
            <Button variant="ghost" onclick={() => (activeView = 'recover')}>
              {i18n.t('sync.mode_recover')}
            </Button>
          </div>
        </div>

        <SettingsGroup>
          <SettingItem
            title={i18n.t('sync.copy_logs')}
            action
            disabled={copyingLogs}
            onclick={() => void handleCopySyncLogs()}
          />
        </SettingsGroup>
      {/if}

    {:else if activeView === 'connect' || activeView === 'create'}
      <form class="profile-form" onsubmit={(e) => { e.preventDefault(); void handleSyncSubmit(); }}>
        {#if activeView === 'connect'}
          <div class="profile-field">
            {@render fieldLabel(i18n.t('sync.account_id'))}
            <Input icon={IconPerson} clearable={true} bind:value={syncAccount} placeholder="account-id" />
          </div>
        {/if}
        <div class="profile-field">
          {@render fieldLabel(i18n.t('sync.master_password'))}
          <Input
            icon={IconKey}
            bind:value={syncPassword}
            type="password"
            placeholder="••••••••••••"
            autocomplete={activeView === 'create' ? 'new-password' : 'current-password'}
          />
          {#if activeView === 'create'}
            <span class="profile-hint">{i18n.t('sync.password_hint_create')}</span>
          {/if}
        </div>

        <button type="button" class="profile-advanced" aria-expanded={showAdvanced} onclick={() => (showAdvanced = !showAdvanced)}>
          {i18n.t('sync.advanced')}
          <IconChevronDown class={showAdvanced ? 'is-open' : ''} />
        </button>
        {#if showAdvanced}
          {@render serverField()}
          {#if activeView === 'create'}
            <div class="profile-field">
              {@render fieldLabel(i18n.t('sync.account_id'))}
              <Input
                icon={IconPerson}
                clearable={true}
                bind:value={syncAccount}
                placeholder="account-id"
                actionIcon={IconDice}
                actionTooltip={i18n.t('sync.generate_id')}
                onAction={() => (syncAccount = generateRandomId())}
              />
            </div>
          {/if}
          {@render deviceField()}
        {/if}

        {@render submitButton(
          i18n.t(activeView === 'create' ? 'sync.create_account' : 'sync.connect_account'),
          syncState.busy || !syncAccount.trim() || syncPassword.length < 12
        )}
      </form>

    {:else if activeView === 'recover'}
      <form class="profile-form" onsubmit={(e) => { e.preventDefault(); void handleSyncSubmit(); }}>
        <div class="profile-field">
          {@render fieldLabel(i18n.t('sync.recovery_kit'))}
          <textarea
            class="recovery-textarea"
            bind:value={syncRecoveryInput}
            placeholder={`{"format":"pawstash-recovery-v1",...}`}
          ></textarea>
        </div>
        <div class="profile-field">
          {@render fieldLabel(i18n.t('sync.new_password'))}
          <Input
            icon={IconKey}
            bind:value={syncPassword}
            type="password"
            placeholder={i18n.t('sync.password_placeholder')}
            autocomplete="new-password"
          />
        </div>
        {@render deviceField()}
        {@render submitButton(
          i18n.t('sync.recover_account'),
          syncState.busy || !syncRecoveryInput.trim() || syncPassword.length < 12
        )}
      </form>

    {:else if activeView === 'recovery_kit'}
      <div class="profile-form">
        <p class="profile-note">{i18n.t('sync.recovery_warning')}</p>
        <div class="profile-field">
          {@render fieldLabel(i18n.t('sync.account_id'))}
          <div class="recovery-value">{parsedRecovery?.accountId || syncState.status.account_id}</div>
        </div>
        <div class="profile-field">
          {@render fieldLabel(i18n.t('sync.vault_key'))}
          <div class="recovery-value is-key">{parsedRecovery?.vaultKey || recoveryKitText}</div>
        </div>
        <div class="profile-actions">
          <Button variant="accent" onclick={copyRecoveryKit}>
            <IconCopy class="mr-2" />
            {i18n.t('sync.copy_recovery')}
          </Button>
          <!-- Android's WebView has no download handler, so a blob link would do nothing there. -->
          {#if !layoutState.isMobileDevice}
            <Button variant="tonal" onclick={downloadRecoveryFile}>
              <IconArrowDownload class="mr-2" />
              {i18n.t('sync.download_json')}
            </Button>
          {/if}
          <Button variant="ghost" onclick={() => (activeView = 'menu')}>
            {i18n.t('sync.recovery_done')}
          </Button>
        </div>
      </div>

    {:else if activeView === 'change_password'}
      <form class="profile-form" onsubmit={(e) => { e.preventDefault(); void handleChangePasswordSubmit(); }}>
        <div class="profile-field">
          {@render fieldLabel(i18n.t('sync.current_password'))}
          <Input icon={IconKey} type="password" bind:value={currentPassword} placeholder="••••••••••••" autocomplete="current-password" />
        </div>
        <div class="profile-field">
          {@render fieldLabel(i18n.t('sync.new_password'))}
          <Input
            icon={IconKey}
            type="password"
            bind:value={newPassword}
            placeholder={i18n.t('sync.password_placeholder')}
            autocomplete="new-password"
          />
        </div>
        {@render submitButton(i18n.t('sync.change_password'), syncState.busy || !currentPassword || newPassword.length < 12)}
      </form>

    {:else if activeView === 'unlock'}
      <form class="profile-form" onsubmit={(e) => { e.preventDefault(); void handleUnlockSubmit(); }}>
        <p class="profile-note">{i18n.t('sync.master_password_desc')}</p>
        <Input
          icon={IconLock}
          type="password"
          bind:value={unlockPassword}
          placeholder={i18n.t('sync.master_password')}
          autocomplete="current-password"
        />
        {@render submitButton(i18n.t('sync.unlock'), syncState.busy || !unlockPassword)}
      </form>
    {/if}
  </div>
</PageShell>

<ConfirmDialog
  isOpen={confirmDisconnect}
  title={i18n.t('sync.disconnect_confirm_title')}
  description={i18n.t('sync.disconnect_confirm_desc')}
  confirmLabel={i18n.t('sync.disconnect')}
  confirmVariant="danger"
  confirmIcon={IconSignOut}
  onconfirm={() => {
    confirmDisconnect = false;
    void runSyncAction(() => syncState.disconnect(), i18n.t('sync.disconnected'));
  }}
  onclose={() => (confirmDisconnect = false)}
/>

<style>
  .profile-body {
    display: flex;
    flex-direction: column;
    gap: 24px;
    width: 100%;
    max-width: var(--content-max-width);
    margin-inline: auto;
    padding-block: 8px 24px;
  }

  .account-card {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 16px 16px 16px 20px;
    border-radius: var(--radius-xl);
    background: var(--settings-tile, var(--content-surface));
  }

  .account-avatar {
    position: relative;
    flex: none;
    width: 64px;
    height: 64px;
  }

  .account-avatar-change {
    position: relative;
    width: 100%;
    height: 100%;
    padding: 0;
    border: 0;
    border-radius: 50%;
    overflow: hidden;
    display: grid;
    place-items: center;
    background: var(--content-surface-high);
    color: var(--text-secondary);
    cursor: pointer;
  }

  .account-avatar-change:focus-visible {
    outline: 2px solid var(--accent-primary);
    outline-offset: 2px;
  }

  .account-avatar-change:disabled {
    cursor: default;
    opacity: 0.7;
  }

  .account-avatar-change img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .account-avatar-change > :global(svg) {
    width: 28px;
    height: 28px;
  }

  .account-avatar-overlay {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: rgb(0 0 0 / 0.42);
    color: #fff;
    opacity: 0;
    pointer-events: none;
    transition: opacity var(--duration-fast) var(--ease-standard);
  }

  .account-avatar-overlay :global(svg) {
    width: 22px;
    height: 22px;
  }

  .account-avatar-change:hover .account-avatar-overlay,
  .account-avatar:focus-within .account-avatar-overlay,
  .account-avatar-change:focus-visible .account-avatar-overlay {
    opacity: 1;
  }

  @media (hover: none) {
    .account-avatar-overlay {
      opacity: 1;
      background: rgb(0 0 0 / 0.32);
    }
  }

  :global([data-layout='mobile']) .account-avatar-overlay {
    opacity: 1;
    background: rgb(0 0 0 / 0.32);
  }

  .account-avatar-remove {
    position: absolute;
    right: -4px;
    bottom: -4px;
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    padding: 0;
    box-sizing: border-box;
    border: 1px solid var(--state-layer-hover);
    border-radius: 50%;
    background: var(--surface-container-high);
    color: var(--status-error);
    cursor: pointer;
    opacity: 0;
    pointer-events: none;
    transition: opacity var(--duration-fast) var(--ease-standard);
  }

  .account-avatar:hover .account-avatar-remove,
  .account-avatar:focus-within .account-avatar-remove {
    opacity: 1;
    pointer-events: auto;
  }

  @media (hover: none) {
    .account-avatar-remove {
      opacity: 1;
      pointer-events: auto;
    }
  }

  :global([data-layout='mobile']) .account-avatar-remove {
    opacity: 1;
    pointer-events: auto;
  }

  .account-avatar-remove :global(svg) {
    width: 14px;
    height: 14px;
  }

  .account-avatar-remove:focus-visible {
    outline: 2px solid var(--accent-primary);
    outline-offset: 1px;
  }

  .account-avatar-remove:disabled {
    opacity: 0.7;
    cursor: default;
  }

  .avatar-input {
    display: none;
  }

  .account-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1 1 auto;
    min-width: 0;
  }

  .account-id {
    margin: 0;
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: var(--type-title-md-size);
    line-height: var(--type-title-md-line);
    font-weight: 500;
    overflow-wrap: anywhere;
  }

  .account-status {
    color: var(--text-secondary);
    font-size: var(--type-body-sm-size);
    line-height: var(--type-body-sm-line);
    overflow-wrap: anywhere;
  }

  .account-card :global(.btn) {
    flex: none;
  }

  :global([data-layout='mobile']) .account-card {
    flex-wrap: wrap;
  }

  :global([data-layout='mobile']) .account-card :global(.btn) {
    flex: 1 1 100%;
    justify-content: center;
  }

  .profile-setup {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 32px 16px 8px;
    text-align: center;
  }

  .setup-mark {
    width: 48px;
    height: 48px;
    margin-bottom: 8px;
  }

  .setup-title {
    margin: 0;
    color: var(--text-primary);
    font-family: var(--font-display);
    font-size: var(--type-title-lg-size);
    line-height: var(--type-title-lg-line);
    font-weight: 400;
  }

  .setup-benefits {
    display: flex;
    flex-direction: column;
    gap: 12px;
    width: min(100%, 360px);
    margin: 4px 0 0;
    padding: 0;
    list-style: none;
    text-align: start;
  }

  .setup-benefits li {
    display: flex;
    align-items: center;
    gap: 14px;
    color: var(--text-primary);
    font-size: var(--type-body-md-size);
    line-height: var(--type-body-md-line);
  }

  .setup-benefits :global(svg) {
    flex: none;
    width: 22px;
    height: 22px;
    color: var(--accent-on-surface);
  }

  .profile-hint {
    color: var(--text-secondary);
    font-size: var(--type-body-sm-size);
    line-height: var(--type-body-sm-line);
  }

  .profile-advanced {
    display: inline-flex;
    align-self: flex-start;
    align-items: center;
    gap: 4px;
    padding: 4px 0;
    border: 0;
    background: none;
    color: var(--text-secondary);
    font: inherit;
    font-size: var(--type-label-lg-size);
    font-weight: 500;
    cursor: pointer;
  }

  .profile-advanced :global(svg) {
    width: 18px;
    height: 18px;
    transition: transform var(--duration-fast) var(--ease-standard);
  }

  .profile-advanced :global(svg.is-open) {
    transform: rotate(180deg);
  }

  .profile-note {
    max-width: 44ch;
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--type-body-md-size);
    line-height: var(--type-body-md-line);
  }

  .setup-actions {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 8px;
    width: min(100%, 320px);
    margin-top: 16px;
  }

  .profile-form {
    display: flex;
    flex-direction: column;
    gap: 16px;
    width: 100%;
    max-width: 440px;
  }

  .profile-field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .profile-label-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .profile-label {
    padding-inline: 4px;
    color: var(--text-secondary);
    font-size: var(--type-label-lg-size);
    line-height: var(--type-label-lg-line);
    font-weight: 500;
  }

  .profile-link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 0 4px;
    border: 0;
    background: transparent;
    color: var(--accent-primary);
    font-size: var(--type-label-lg-size);
    line-height: var(--type-label-lg-line);
  }

  .profile-link:disabled {
    opacity: 0.38;
  }

  @media (hover: hover) {
    .profile-link:not(:disabled):hover {
      text-decoration: underline;
    }
  }

  .profile-form :global(.profile-submit) {
    align-self: flex-start;
    margin-top: 8px;
  }

  :global([data-layout='mobile']) .profile-form :global(.profile-submit) {
    align-self: stretch;
  }

  .profile-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 8px;
  }

  .recovery-value {
    padding: 12px 16px;
    border-radius: var(--radius-md);
    background: var(--surface-container);
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: var(--type-body-sm-size);
    line-height: var(--type-body-sm-line);
    user-select: all;
  }

  .recovery-value.is-key {
    word-break: break-all;
  }

  .recovery-textarea {
    width: 100%;
    min-height: 120px;
    padding: 12px 16px;
    border: 0;
    border-radius: var(--radius-md);
    background: var(--surface-container-high);
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: var(--type-body-sm-size);
    line-height: var(--type-body-sm-line);
    resize: vertical;
    outline: none;
    box-sizing: border-box;
  }

  .recovery-textarea:focus-visible {
    outline: 2px solid var(--accent-primary);
    outline-offset: -2px;
  }
</style>
