<script lang="ts">
  import { onMount } from 'svelte';
  import { syncState } from '$lib/state/syncState.svelte';
  import { accountState } from '$lib/state/accountState.svelte';
  import { configState } from '$lib/state/configState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { i18n } from '$lib/i18n';
  import PageShell from '$lib/components/layout/PageShell.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import IconLock from '~icons/fluent/lock-closed-24-regular';
  import IconKey from '~icons/fluent/key-24-regular';
  import IconPassword from '~icons/fluent/password-24-regular';
  import IconSignOut from '~icons/fluent/sign-out-24-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';
  import IconCheck from '~icons/fluent/checkmark-24-regular';
  import IconCopy from '~icons/fluent/copy-24-regular';
  import IconArrowDownload from '~icons/fluent/arrow-download-24-regular';
  import IconSync from '~icons/fluent/arrow-sync-24-regular';
  import IconDice from '~icons/fluent/cube-24-regular';
  import IconArrowLeft from '~icons/fluent/arrow-left-24-regular';
  import IconPerson from '~icons/fluent/person-24-regular';
  import IconPersonAdd from '~icons/fluent/person-add-24-regular';
  import IconGlobe from '~icons/fluent/globe-24-regular';
  import IconLaptop from '~icons/fluent/laptop-24-regular';
  import IconOpen from '~icons/fluent/open-24-regular';
  import { invoke } from '@tauri-apps/api/core';
  import { notify } from '$lib/utils/toast';

  type SubView =
    | 'menu'
    | 'connect'
    | 'create'
    | 'recover'
    | 'pawchive_login'
    | 'recovery_kit'
    | 'change_password'
    | 'unlock';

  let activeView = $state<SubView>('menu');

  let syncServer = $state('https://pawstash.nichind.dev');
  let syncAccount = $state('');
  let syncPassword = $state('');
  let syncDevice = $state('Pawstash Desktop');
  let syncRecoveryInput = $state('');

  let unlockPassword = $state('');
  let currentPassword = $state('');
  let newPassword = $state('');
  let recoveryKitText = $state('');

  let pawchiveUsername = $state('');
  let pawchivePassword = $state('');
  let pawchiveLoginError = $state('');
  let registerUrl = $derived(
    `https://${configState.settings.api_domain.replace(/^https?:\/\//, '')}/account/register`
  );
  let pawchiveProfileUrl = $derived(
    `https://${configState.settings.api_domain.replace(/^https?:\/\//, '')}/account`
  );

  async function openExternalUrl(url: string) {
    try {
      await invoke('open_in_browser', { url });
    } catch {
      window.open(url, '_blank');
    }
  }

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
    void accountState.refresh().catch((error) =>
      notify.error(i18n.t('profile.check_error'), error)
    );
  });

  async function runSyncAction(work: () => Promise<unknown>, successMessage?: string) {
    try {
      await work();
      if (successMessage) notify.success(successMessage);
      activeView = 'menu';
    } catch (error: any) {
      const errStr = String(error?.message || error);
      if (!syncState.status.configured && activeView === 'create' && errStr.toLowerCase().includes('account already exists')) {
        notify.error(i18n.t('sync.account_taken'));
        syncAccount = generateRandomId();
      } else {
        notify.error(i18n.t('sync.error'), errStr);
      }
    } finally {
      syncPassword = '';
    }
  }

  async function handleSyncSubmit() {
    if (activeView === 'create') {
      await runSyncAction(async () => {
        await syncState.create(syncServer, syncAccount, syncPassword, syncDevice);
        notify.success(i18n.t('sync.configured'));
        await openRecoveryView();
      });
    } else if (activeView === 'connect') {
      await runSyncAction(
        () => syncState.connect(syncServer, syncAccount, syncPassword, syncDevice),
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

  function downloadRecoveryFile(ext: 'json' | 'txt') {
    if (!recoveryKitText) return;
    const blob = new Blob([recoveryKitText], { type: ext === 'json' ? 'application/json' : 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `pawstash-recovery-kit-${syncState.status.account_id || 'vault'}.${ext}`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    notify.success(i18n.t('sync.recovery_copied'));
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

  async function loginPawchive() {
    if (accountState.loading || !pawchiveUsername.trim() || !pawchivePassword) return;
    pawchiveLoginError = '';
    try {
      await accountState.login(pawchiveUsername, pawchivePassword);
      notify.success(i18n.t('profile.connected'), pawchiveUsername);
      activeView = 'menu';
    } catch (error) {
      pawchiveLoginError = String(error);
      notify.error(i18n.t('profile.login_error'), pawchiveLoginError);
    } finally {
      pawchivePassword = '';
    }
  }

  async function logoutPawchive() {
    try {
      await accountState.logout();
      notify.success(i18n.t('profile.logged_out'));
    } catch (error) {
      notify.error(i18n.t('profile.logout_error'), error);
    }
  }

  let parsedRecovery = $derived.by(() => {
    if (!recoveryKitText) return null;
    try {
      const data = JSON.parse(recoveryKitText);
      return {
        accountId: data.account_id || syncState.status.account_id,
        serverUrl: data.server_url || syncState.status.server_url,
        vaultKey: data.secrets?.vault_key || ''
      };
    } catch {
      return null;
    }
  });
</script>

<PageShell scrollable={true} scrollKey={navigationState.entryKey}>
  <div class="flex items-center justify-center min-h-[calc(100vh-100px)] w-full p-4 sm:p-6 box-border">
    <div class="flex flex-col gap-6 w-full max-w-[360px] my-auto">

      {#if activeView === 'menu'}
        {#if syncState.status.configured}
          <div class="flex flex-col gap-4">
            <div class="flex flex-col items-center text-center gap-1.5">
              <svg viewBox="0 0 602 602" fill="none" class="w-12 h-12 mb-0.5" xmlns="http://www.w3.org/2000/svg">
                <defs>
                  <linearGradient id="logo-profile-connected" x1="301" y1="0" x2="-2.17166e-05" y2="584.337" gradientUnits="userSpaceOnUse">
                    <stop stop-color="#FCD8D2"/>
                    <stop offset="1" stop-color="#FEB8AD"/>
                  </linearGradient>
                </defs>
                <g transform="translate(0, 8.5)">
                  <path fill="url(#logo-profile-connected)" d="M130.548 56.3212L414.821 178.14L301 226.902L18.361 105.771C24.725 99.2782 32.508 94.0322 41.366 90.6352L130.548 56.3212ZM188.082 34.2192L254.732 8.59119C284.529 -2.86373 317.514 -2.86373 347.311 8.59119L560.677 90.6352C569.492 94.0752 577.275 99.2352 583.639 105.771L469.431 154.705L188.082 34.2192ZM601.742 144.815L322.5 264.484V584.834C330.957 583.401 339.227 581.136 347.311 578.04L560.677 495.953C572.841 491.269 583.301 483.01 590.677 472.264C598.054 461.517 602.002 448.788 602 435.753V150.835C602 148.829 601.9 146.822 601.699 144.815M279.5 584.834V264.484L0.300999 144.815C0.130354 146.818 0.0299598 148.826 0 150.835V435.753C0.00172613 448.793 3.95568 461.526 11.3404 472.273C18.7252 483.02 29.1939 491.276 41.366 495.953L254.689 578.04C262.773 581.136 271.043 583.401 279.5 584.834Z" />
                </g>
              </svg>
              <h2 class="text-xl font-semibold text-white font-outfit m-0">
                {syncState.status.account_id}
              </h2>
              <span class="text-xs text-white/50 font-light">
                {syncState.status.unlocked
                  ? i18n.t('sync.revision', { revision: syncState.status.revision, cursor: syncState.status.cursor })
                  : i18n.t('sync.locked')}
              </span>
            </div>

            <div class="flex flex-col gap-2.5 pt-1">
              <Button
                variant="accent"
                class="w-full"
                disabled={syncState.busy || !syncState.status.unlocked || !syncState.status.enabled}
                onclick={() => void runSyncAction(() => syncState.sync(), i18n.t('sync.completed'))}
              >
                {#if syncState.busy}
                  <IconLoading class="mr-2" />
                  {i18n.t('sync.status_syncing')}
                {:else}
                  <IconSync class="mr-2" />
                  {i18n.t('sync.sync_now')}
                {/if}
              </Button>

              <div class="flex flex-col gap-2">
                <div class="grid grid-cols-2 gap-2">
                  <Button variant="primary" class="w-full !px-2.5" onclick={() => void openRecoveryView()}>
                    <IconKey class="w-4 h-4 mr-1.5 text-white/60 shrink-0" />
                    <span class="truncate">{i18n.t('sync.export_recovery_kit')}</span>
                  </Button>

                  {#if !syncState.status.unlocked}
                    <Button variant="primary" class="w-full !px-2.5" onclick={() => (activeView = 'unlock')}>
                      <IconLock class="w-4 h-4 mr-1.5 text-amber-400 shrink-0" />
                      <span class="text-amber-300 truncate">{i18n.t('sync.unlock')}</span>
                    </Button>
                  {:else}
                    <Button variant="primary" class="w-full !px-2.5" onclick={() => void runSyncAction(() => syncState.lock())}>
                      <IconLock class="w-4 h-4 mr-1.5 text-white/60 shrink-0" />
                      <span class="truncate">{i18n.t('sync.lock')}</span>
                    </Button>
                  {/if}
                </div>

                <Button variant="primary" class="w-full" onclick={() => (activeView = 'change_password')}>
                  <IconPassword class="w-4 h-4 mr-2 text-white/60 shrink-0" />
                  <span>{i18n.t('sync.change_password')}</span>
                </Button>
              </div>

              <Button
                variant="danger"
                class="w-full mt-1"
                disabled={syncState.busy}
                onclick={() => void runSyncAction(() => syncState.disconnect())}
              >
                <IconSignOut class="w-4 h-4 mr-1.5" />
                <span>{i18n.t('sync.disconnect')}</span>
              </Button>
            </div>
          </div>

          <div class="relative flex py-1 items-center">
            <div class="flex-grow border-t border-white/[0.08]"></div>
            <span class="flex-shrink mx-3 text-[11px] font-medium text-white/40 uppercase tracking-widest font-outfit">
              Pawchive
            </span>
            <div class="flex-grow border-t border-white/[0.08]"></div>
          </div>

          <div class="flex flex-col gap-2">
            {#if accountState.session.authenticated}
              <div class="flex flex-col gap-2 w-full">
                <Button
                  variant="ghost"
                  class="w-full font-mono"
                  onclick={() => void openExternalUrl(pawchiveProfileUrl)}
                  title="Open Pawchive Profile"
                >
                  <IconPerson class="w-4 h-4 mr-2 text-white/60 shrink-0" />
                  <span class="truncate">@{accountState.session.username}</span>
                  <IconOpen class="w-3.5 h-3.5 ml-1.5 text-white/40 shrink-0" />
                </Button>

                <Button
                  variant="ghost"
                  class="w-full text-red-400 hover:text-red-300"
                  disabled={accountState.loading}
                  onclick={() => void logoutPawchive()}
                >
                  <IconSignOut class="w-4 h-4 mr-1.5 text-red-400/80" />
                  <span>{i18n.t('profile.logout')}</span>
                </Button>
              </div>
            {:else}
              <Button
                variant="ghost"
                class="w-full"
                onclick={() => (activeView = 'pawchive_login')}
              >
                <IconGlobe class="w-5 h-5 mr-2 text-white/60" />
                <span>{i18n.t('profile.login')}</span>
              </Button>
            {/if}
          </div>

        {:else}
          <div class="flex flex-col items-center text-center gap-2">
            <svg viewBox="0 0 602 602" fill="none" class="w-14 h-14 mb-1" xmlns="http://www.w3.org/2000/svg">
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
            <h2 class="text-2xl font-semibold text-white font-outfit m-0">
              Pawstash Cloud
            </h2>
            <p class="text-xs text-white/50 font-light leading-relaxed m-0 px-2">
              {i18n.t('sync.setup_desc')}
            </p>
          </div>

          <div class="flex flex-col gap-2.5 w-full pt-1">
            <Button
              variant="accent"
              class="w-full"
              onclick={() => (activeView = 'connect')}
            >
              <IconPerson class="w-5 h-5 mr-2" />
              {i18n.t('sync.mode_connect')}
            </Button>

            <Button
              variant="primary"
              class="w-full"
              onclick={() => {
                syncAccount = generateRandomId();
                activeView = 'create';
              }}
            >
              <IconPersonAdd class="w-5 h-5 mr-2" />
              {i18n.t('sync.mode_create')}
            </Button>

            <Button
              variant="ghost"
              class="w-full"
              onclick={() => (activeView = 'recover')}
            >
              <IconKey class="w-5 h-5 mr-2 text-white/50" />
              <span>{i18n.t('sync.mode_recover')}</span>
            </Button>
          </div>

          <div class="relative flex py-1 items-center">
            <div class="flex-grow border-t border-white/[0.08]"></div>
            <span class="flex-shrink mx-3 text-[11px] font-medium text-white/40 uppercase tracking-widest font-outfit">
              Pawchive
            </span>
            <div class="flex-grow border-t border-white/[0.08]"></div>
          </div>

          <div class="flex flex-col gap-2">
            {#if accountState.session.authenticated}
              <div class="flex flex-col gap-2 w-full">
                <Button
                  variant="ghost"
                  class="w-full font-mono"
                  onclick={() => void openExternalUrl(pawchiveProfileUrl)}
                  title="Open Pawchive Profile"
                >
                  <IconPerson class="w-4 h-4 mr-2 text-white/60 shrink-0" />
                  <span class="truncate">@{accountState.session.username}</span>
                  <IconOpen class="w-3.5 h-3.5 ml-1.5 text-white/40 shrink-0" />
                </Button>

                <Button
                  variant="ghost"
                  class="w-full text-red-400 hover:text-red-300"
                  disabled={accountState.loading}
                  onclick={() => void logoutPawchive()}
                >
                  <IconSignOut class="w-4 h-4 mr-1.5 text-red-400/80" />
                  <span>{i18n.t('profile.logout')}</span>
                </Button>
              </div>
            {:else}
              <Button
                variant="ghost"
                class="w-full"
                onclick={() => (activeView = 'pawchive_login')}
              >
                <IconGlobe class="w-5 h-5 mr-2 text-white/60" />
                <span>{i18n.t('profile.login')}</span>
              </Button>
            {/if}
          </div>
        {/if}

      {:else if activeView === 'connect'}
        <div class="flex items-center gap-2 mb-1">
          <Button variant="ghost" class="!w-[42px] !h-[42px] !p-0" onclick={() => (activeView = 'menu')}>
            <IconArrowLeft class="w-5 h-5" />
          </Button>
          <h2 class="text-lg font-semibold text-white font-outfit m-0">
            {i18n.t('sync.connect_account')}
          </h2>
        </div>

        <form
          class="flex flex-col gap-3"
          onsubmit={(e) => { e.preventDefault(); void handleSyncSubmit(); }}
        >
          <div class="flex flex-col gap-1">
            <span class="text-xs text-white/60 font-medium">{i18n.t('sync.server_url')}</span>
            <Input icon={IconGlobe} clearable={true} bind:value={syncServer} placeholder="https://pawstash.nichind.dev" />
          </div>

          <div class="flex flex-col gap-1">
            <span class="text-xs text-white/60 font-medium">{i18n.t('sync.account_id')}</span>
            <Input icon={IconPerson} clearable={true} bind:value={syncAccount} placeholder="account-id" />
          </div>

          <div class="flex flex-col gap-1">
            <span class="text-xs text-white/60 font-medium">{i18n.t('sync.master_password')}</span>
            <Input
              icon={IconKey}
              bind:value={syncPassword}
              type="password"
              placeholder="••••••••••••"
              autocomplete="current-password"
            />
          </div>

          <div class="flex flex-col gap-1">
            <span class="text-xs text-white/60 font-medium">{i18n.t('sync.device_name')}</span>
            <Input icon={IconLaptop} clearable={true} bind:value={syncDevice} placeholder="Pawstash Desktop" />
          </div>

          <Button
            type="submit"
            variant="accent"
            class="w-full mt-2"
            disabled={syncState.busy || !syncAccount.trim() || !syncPassword || syncPassword.length < 12}
          >
            {#if syncState.busy}
              <IconLoading class="w-4 h-4 mr-2" />
            {/if}
            {i18n.t('sync.connect_account')}
          </Button>
        </form>

      {:else if activeView === 'create'}
        <div class="flex items-center gap-2 mb-1">
          <Button variant="ghost" class="!w-[42px] !h-[42px] !p-0" onclick={() => (activeView = 'menu')}>
            <IconArrowLeft class="w-5 h-5" />
          </Button>
          <h2 class="text-lg font-semibold text-white font-outfit m-0">
            {i18n.t('sync.create_account')}
          </h2>
        </div>

        <form
          class="flex flex-col gap-3"
          onsubmit={(e) => { e.preventDefault(); void handleSyncSubmit(); }}
        >
          <div class="flex flex-col gap-1">
            <span class="text-xs text-white/60 font-medium">{i18n.t('sync.server_url')}</span>
            <Input icon={IconGlobe} clearable={true} bind:value={syncServer} placeholder="https://pawstash.nichind.dev" />
          </div>

          <div class="flex flex-col gap-1">
            <span class="text-xs text-white/60 font-medium">{i18n.t('sync.account_id')}</span>
            <Input
              icon={IconPerson}
              clearable={true}
              bind:value={syncAccount}
              placeholder="account-id"
              actionIcon={IconDice}
              actionTooltip="Generate random ID"
              onAction={() => (syncAccount = generateRandomId())}
            />
          </div>

          <div class="flex flex-col gap-1">
            <span class="text-xs text-white/60 font-medium">{i18n.t('sync.master_password')}</span>
            <Input
              icon={IconKey}
              bind:value={syncPassword}
              type="password"
              placeholder="•••••••••••• (min. 12 chars)"
              autocomplete="new-password"
            />
          </div>

          <div class="flex flex-col gap-1">
            <span class="text-xs text-white/60 font-medium">{i18n.t('sync.device_name')}</span>
            <Input icon={IconLaptop} clearable={true} bind:value={syncDevice} placeholder="Pawstash Desktop" />
          </div>

          <Button
            type="submit"
            variant="accent"
            class="w-full mt-2"
            disabled={syncState.busy || !syncAccount.trim() || !syncPassword || syncPassword.length < 12}
          >
            {#if syncState.busy}
              <IconLoading class="w-4 h-4 mr-2" />
            {/if}
            {i18n.t('sync.create_account')}
          </Button>
        </form>

      {:else if activeView === 'recover'}
        <div class="flex items-center gap-2 mb-1">
          <Button variant="ghost" class="!w-[42px] !h-[42px] !p-0" onclick={() => (activeView = 'menu')}>
            <IconArrowLeft class="w-5 h-5" />
          </Button>
          <h2 class="text-lg font-semibold text-white font-outfit m-0">
            {i18n.t('sync.recover_account')}
          </h2>
        </div>

        <form
          class="flex flex-col gap-3"
          onsubmit={(e) => { e.preventDefault(); void handleSyncSubmit(); }}
        >
          <div class="flex flex-col gap-1">
            <span class="text-xs text-white/60 font-medium">{i18n.t('sync.recovery_kit')}</span>
            <textarea
              class="recovery-textarea"
              bind:value={syncRecoveryInput}
              placeholder={`{"format":"pawstash-recovery-v1",...}`}
            ></textarea>
          </div>

          <div class="flex flex-col gap-1">
            <span class="text-xs text-white/60 font-medium">{i18n.t('sync.new_password')}</span>
            <Input
              icon={IconKey}
              bind:value={syncPassword}
              type="password"
              placeholder="•••••••••••• (min. 12 chars)"
              autocomplete="new-password"
            />
          </div>

          <div class="flex flex-col gap-1">
            <span class="text-xs text-white/60 font-medium">{i18n.t('sync.device_name')}</span>
            <Input icon={IconLaptop} clearable={true} bind:value={syncDevice} placeholder="Pawstash Desktop" />
          </div>

          <Button
            type="submit"
            variant="accent"
            class="w-full mt-2"
            disabled={syncState.busy || !syncRecoveryInput.trim() || !syncPassword || syncPassword.length < 12}
          >
            {#if syncState.busy}
              <IconLoading class="w-4 h-4 mr-2" />
            {/if}
            {i18n.t('sync.recover_account')}
          </Button>
        </form>

      {:else if activeView === 'recovery_kit'}
        <div class="flex items-center gap-2 mb-1">
          <Button variant="ghost" class="!w-[42px] !h-[42px] !p-0" onclick={() => (activeView = 'menu')}>
            <IconArrowLeft class="w-5 h-5" />
          </Button>
          <h2 class="text-lg font-semibold text-white font-outfit m-0">
            {i18n.t('sync.recovery_kit_title')}
          </h2>
        </div>

        <div class="flex flex-col gap-3.5">
          <p class="text-xs text-white/60 leading-relaxed m-0">
            {i18n.t('sync.recovery_warning')}
          </p>

          <div class="flex items-center justify-between p-3.5 rounded-2xl bg-[var(--bg-card)] border border-[var(--border-color)]">
            <span class="text-xs text-white/50">{i18n.t('sync.account_id')}</span>
            <span class="font-mono text-xs font-semibold text-white tracking-wider">
              {parsedRecovery?.accountId || syncState.status.account_id}
            </span>
          </div>

          <div class="flex flex-col gap-1.5">
            <span class="text-xs text-white/60 font-medium">{i18n.t('sync.vault_key')}</span>
            <div class="font-mono text-[12px] text-white/90 break-all select-all tracking-wide py-3 px-4 rounded-2xl bg-[var(--bg-card)] border border-[var(--border-color)]">
              {parsedRecovery?.vaultKey || recoveryKitText}
            </div>
          </div>

          <div class="flex flex-col gap-2 pt-1">
            <Button variant="accent" class="w-full" onclick={copyRecoveryKit}>
              <IconCopy class="mr-2" />
              {i18n.t('sync.copy_recovery')}
            </Button>

            <Button variant="ghost" class="w-full" onclick={() => downloadRecoveryFile('json')}>
              <IconArrowDownload class="mr-2" />
              {i18n.t('sync.download_json')}
            </Button>
          </div>
        </div>

      {:else if activeView === 'change_password'}
        <div class="flex items-center gap-2 mb-1">
          <Button variant="ghost" class="!w-[42px] !h-[42px] !p-0" onclick={() => (activeView = 'menu')}>
            <IconArrowLeft class="w-5 h-5" />
          </Button>
          <h2 class="text-lg font-semibold text-white font-outfit m-0">
            {i18n.t('sync.change_password_title')}
          </h2>
        </div>

        <form onsubmit={(e) => { e.preventDefault(); void handleChangePasswordSubmit(); }} class="flex flex-col gap-3">
          <div class="flex flex-col gap-1">
            <span class="text-xs text-white/70 font-medium">{i18n.t('sync.current_password')}</span>
            <Input
              icon={IconKey}
              type="password"
              bind:value={currentPassword}
              placeholder="••••••••••••"
              autocomplete="current-password"
            />
          </div>

          <div class="flex flex-col gap-1">
            <span class="text-xs text-white/70 font-medium">{i18n.t('sync.new_password')}</span>
            <Input
              icon={IconKey}
              type="password"
              bind:value={newPassword}
              placeholder="•••••••••••• (min. 12 chars)"
              autocomplete="new-password"
            />
          </div>

          <Button
            variant="accent"
            type="submit"
            class="w-full mt-2"
            disabled={syncState.busy || !currentPassword || newPassword.length < 12}
          >
            <IconCheck class="mr-2" />
            {i18n.t('sync.change_password')}
          </Button>
        </form>

      {:else if activeView === 'unlock'}
        <div class="flex items-center gap-2 mb-1">
          <Button variant="ghost" class="!w-[42px] !h-[42px] !p-0" onclick={() => (activeView = 'menu')}>
            <IconArrowLeft class="w-5 h-5" />
          </Button>
          <h2 class="text-lg font-semibold text-white font-outfit m-0">
            {i18n.t('sync.unlock_title')}
          </h2>
        </div>

        <form onsubmit={(e) => { e.preventDefault(); void handleUnlockSubmit(); }} class="flex flex-col gap-3">
          <p class="text-xs text-white/60 leading-relaxed m-0">
            {i18n.t('sync.master_password_desc')}
          </p>

          <Input
            icon={IconLock}
            type="password"
            bind:value={unlockPassword}
            placeholder={i18n.t('sync.master_password')}
            autocomplete="current-password"
          />

          <Button
            variant="accent"
            type="submit"
            class="w-full mt-2"
            disabled={syncState.busy || !unlockPassword}
          >
            <IconLock class="mr-2" />
            {i18n.t('sync.unlock')}
          </Button>
        </form>

      {:else if activeView === 'pawchive_login'}
        <div class="flex items-center gap-2 mb-1">
          <Button variant="ghost" class="!w-[42px] !h-[42px] !p-0" onclick={() => (activeView = 'menu')}>
            <IconArrowLeft class="w-5 h-5" />
          </Button>
          <h2 class="text-lg font-semibold text-white font-outfit m-0">
            {i18n.t('profile.login_title')}
          </h2>
        </div>

        <form
          class="flex flex-col gap-3"
          onsubmit={(e) => { e.preventDefault(); void loginPawchive(); }}
        >
          <div class="flex flex-col gap-1">
            <span class="text-xs text-white/60 font-medium">{i18n.t('profile.username')}</span>
            <Input
              icon={IconPerson}
              clearable={true}
              bind:value={pawchiveUsername}
              placeholder="Username"
              autocomplete="username"
            />
          </div>

          <div class="flex flex-col gap-1">
            <span class="text-xs text-white/60 font-medium">{i18n.t('profile.password')}</span>
            <Input
              icon={IconKey}
              bind:value={pawchivePassword}
              type="password"
              placeholder="••••••••"
              autocomplete="current-password"
            />
          </div>

          {#if pawchiveLoginError}
            <p class="text-xs text-red-400 m-0 py-0.5" role="alert">{pawchiveLoginError}</p>
          {/if}

          <Button
            type="submit"
            variant="accent"
            class="w-full mt-2"
            disabled={accountState.loading || !pawchiveUsername.trim() || !pawchivePassword}
          >
            {#if accountState.loading}
              <IconLoading class="w-4 h-4 mr-2" />
              {i18n.t('profile.signing_in')}
            {:else}
              <IconCheck class="w-4 h-4 mr-2" />
              {i18n.t('profile.login')}
            {/if}
          </Button>

          <p class="text-xs text-white/40 text-center m-0 pt-1">
            {i18n.t('profile.no_account')}
            <a
              href={registerUrl}
              target="_blank"
              rel="noreferrer"
              class="text-[var(--accent)] hover:underline font-medium ml-1"
            >
              {i18n.t('profile.register')}
            </a>
          </p>
        </form>
      {/if}

    </div>
  </div>
</PageShell>

<style>
  .recovery-textarea {
    width: 100%;
    min-height: 100px;
    padding: 12px 14px;
    border-radius: var(--radius-lg, 16px);
    background: var(--bg-card);
    border: var(--border-width, 1px) solid var(--border-color);
    color: var(--text-primary, #ffffff);
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    line-height: 1.4;
    resize: vertical;
    outline: none;
    box-sizing: border-box;
    transition: border-color var(--duration-fast, 150ms) var(--ease-out, ease-out);
  }

  .recovery-textarea:focus {
    border-color: var(--border-color-focus);
    box-shadow: 0 0 0 2px var(--accent-glow);
  }
</style>
