<script lang="ts">
  import { i18n } from '$lib/i18n';
  import { notify } from '$lib/utils/toast';
  import { apiOpenInBrowser } from '$lib/utils/ipc';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import SegmentedControl from '$lib/components/ui/SegmentedControl.svelte';
  import IconCopy from '~icons/fluent/copy-24-regular';
  import IconCheck from '~icons/fluent/checkmark-20-regular';
  import IconOpen from '~icons/fluent/open-24-regular';
  import IconWindows from '~icons/fluent/desktop-24-regular';
  import IconLinux from '~icons/fluent/laptop-24-regular';

  interface Props {
    open: boolean;
    onclose: () => void;
  }

  let { open = false, onclose }: Props = $props();

  const isLinux = typeof navigator !== 'undefined' && /Linux/i.test(navigator.userAgent) && !/Android/i.test(navigator.userAgent);
  const initialTab = isLinux ? 'linux' : 'windows';

  let activeTab = $state<'windows' | 'linux'>('windows');
  let copied = $state<string | null>(null);
  let copyTimer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    if (open) {
      activeTab = initialTab;
    }
  });

  const tabOptions = [
    { value: 'windows', label: 'Windows', icon: IconWindows },
    { value: 'linux', label: 'Linux', icon: IconLinux }
  ];

  const UBUNTU_CMD = 'sudo apt install gstreamer1.0-plugins-bad gstreamer1.0-libav';
  const ARCH_CMD = 'sudo pacman -S gst-plugins-bad gst-libav';
  const FEDORA_CMD = 'sudo dnf install gstreamer1-plugins-bad-freeworld gstreamer1-libav';

  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      copied = text;
      notify.success(i18n.t('post.codec_copied') || 'Copied to clipboard');
      if (copyTimer) clearTimeout(copyTimer);
      copyTimer = setTimeout(() => { copied = null; }, 2000);
    } catch {
      notify.error('Failed to copy');
    }
  }
</script>

<Modal
  isOpen={open}
  title={i18n.t('post.codec_guide_title') || 'Кодек H.265 / HEVC'}
  size="md"
  {onclose}
>
  <div class="codec-modal-body">
    <p class="codec-intro">
      {i18n.t('post.codec_why_desc') || 'Для воспроизведения видео в формате H.265 (HEVC) установите кодек в систему.'}
    </p>

    <!-- Predefined SegmentedControl Component -->
    <div class="segmented-wrapper">
      <SegmentedControl
        options={tabOptions}
        value={activeTab}
        onchange={(v) => activeTab = v}
        compact={true}
      />
    </div>

    <!-- Windows Tab -->
    {#if activeTab === 'windows'}
      <div class="tab-content">
        <span class="cmd-label">Microsoft Store:</span>
        <div class="store-row">
          <Button
            variant="ghost"
            class="store-btn"
            onclick={() => void apiOpenInBrowser('https://apps.microsoft.com/detail/9nmzlz57r3t7')}
          >
            <IconOpen class="w-[15px] h-[15px]" />
            <span>{i18n.t('post.codec_windows_m2_btn') || 'Открыть расширение в Microsoft Store'}</span>
          </Button>
        </div>

        <p class="tab-note">
          {i18n.t('post.codec_alt_player_desc')}
        </p>
      </div>
    {:else if activeTab === 'linux'}
      <div class="tab-content">
        <span class="cmd-label">Ubuntu / Debian:</span>
        <div class="cmd-box">
          <code class="cmd-text">{UBUNTU_CMD}</code>
          <button type="button" class="copy-btn" onclick={() => void copy(UBUNTU_CMD)} title="Copy">
            {#if copied === UBUNTU_CMD}<IconCheck class="w-[15px] h-[15px] text-[var(--accent-primary)]" />{:else}<IconCopy class="w-[15px] h-[15px]" />{/if}
          </button>
        </div>

        <span class="cmd-label mt-2">Arch Linux:</span>
        <div class="cmd-box">
          <code class="cmd-text">{ARCH_CMD}</code>
          <button type="button" class="copy-btn" onclick={() => void copy(ARCH_CMD)} title="Copy">
            {#if copied === ARCH_CMD}<IconCheck class="w-[15px] h-[15px] text-[var(--accent-primary)]" />{:else}<IconCopy class="w-[15px] h-[15px]" />{/if}
          </button>
        </div>

        <span class="cmd-label mt-2">Fedora:</span>
        <div class="cmd-box">
          <code class="cmd-text">{FEDORA_CMD}</code>
          <button type="button" class="copy-btn" onclick={() => void copy(FEDORA_CMD)} title="Copy">
            {#if copied === FEDORA_CMD}<IconCheck class="w-[15px] h-[15px] text-[var(--accent-primary)]" />{:else}<IconCopy class="w-[15px] h-[15px]" />{/if}
          </button>
        </div>
      </div>
    {/if}
  </div>
</Modal>

<style>
  .codec-modal-body {
    display: flex;
    flex-direction: column;
    gap: var(--floating-item-gap, 10px);
    padding: var(--floating-padding, 6px) var(--floating-card-px, 10px) var(--floating-card-px, 10px);
    box-sizing: border-box;
  }

  .codec-intro {
    font-size: 13px;
    color: var(--text-secondary);
    line-height: 1.5;
    margin: 0;
  }

  .segmented-wrapper {
    display: flex;
    width: 100%;
    margin-top: 2px;
  }

  .segmented-wrapper :global(.segmented-control-container) {
    width: 100%;
    display: flex;
  }

  .segmented-wrapper :global(.tab-btn) {
    flex: 1;
    justify-content: center;
  }

  .tab-content {
    display: flex;
    flex-direction: column;
    gap: var(--floating-gap, 6px);
    margin-top: 2px;
  }

  .cmd-label {
    font-size: 11.5px;
    font-weight: 600;
    color: var(--text-secondary);
    margin-top: 2px;
  }

  .cmd-box {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--floating-item-gap, 10px);
    padding: var(--floating-card-py, 6px) var(--floating-card-px, 10px);
    background: rgba(0, 0, 0, 0.4);
    border: var(--floating-border);
    border-radius: var(--floating-item-radius, 12px);
  }

  .cmd-text {
    font-family: monospace;
    font-size: 11.5px;
    color: var(--text-primary);
    word-break: break-all;
    user-select: all;
    line-height: 1.35;
  }

  .copy-btn {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm, 6px);
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    flex-shrink: 0;
    transition: color var(--duration-fast, 150ms) var(--ease-expo), background var(--duration-fast, 150ms) var(--ease-expo);
  }

  .copy-btn:hover {
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.08);
  }

  .store-row {
    display: flex;
    margin-top: 4px;
  }

  :global(.store-btn) {
    height: var(--floating-item-height, 36px) !important;
    padding: 0 var(--floating-item-px, 12px) !important;
    font-size: var(--floating-item-font-size, 13.5px) !important;
    border-radius: var(--radius-full, 9999px) !important;
    gap: var(--floating-item-gap, 10px) !important;
    background: rgba(255, 255, 255, 0.06) !important;
  }

  .tab-note {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.45;
    margin: 4px 0 0;
  }
</style>
