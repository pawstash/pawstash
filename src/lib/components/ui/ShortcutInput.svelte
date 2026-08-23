<script lang="ts">
  import { i18n } from '$lib/i18n';
  import { ripple, tooltip } from '$lib/motion';
  import PopoverMenu from '$lib/components/ui/PopoverMenu.svelte';
  import IconArrowReset from '~icons/fluent/arrow-reset-24-regular';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';
  import IconCheck from '~icons/fluent/checkmark-24-regular';
  import IconList from '~icons/fluent/list-24-regular';
  import IconKeyboard from '~icons/fluent/keyboard-24-regular';

  interface Props {
    value?: string;
    defaultValue?: string;
    presets?: string[];
    disabled?: boolean;
    onchange?: (value: string) => void;
    class?: string;
  }

  let {
    value = $bindable('H'),
    defaultValue = 'H',
    presets = ['H', 'Escape', 'Alt+X', 'Alt+Q', 'F11', 'F12', 'Ctrl+Alt+H', 'Ctrl+Shift+H', 'Space'],
    disabled = false,
    onchange,
    class: extraClass = ''
  }: Props = $props();

  let fieldEl = $state<HTMLDivElement>();
  let isRecording = $state(false);
  let liveKeys = $state<string[]>([]);
  let pendingValue = $state<string | null>(null);
  let finalizeTimer: ReturnType<typeof setTimeout> | undefined;

  let displayTokens = $derived.by(() => {
    if (isRecording) {
      if (liveKeys.length > 0) return liveKeys;
      return [];
    }
    if (!value || value.trim().length === 0) return [];
    return value.split('+').map((token) => token.trim()).filter(Boolean);
  });

  function startRecording() {
    if (disabled) return;
    if (finalizeTimer) clearTimeout(finalizeTimer);
    isRecording = true;
    liveKeys = [];
    pendingValue = null;
    fieldEl?.focus();
  }

  function stopRecording() {
    if (finalizeTimer) clearTimeout(finalizeTimer);
    isRecording = false;
    liveKeys = [];
    pendingValue = null;
  }

  function finalizeRecording() {
    if (finalizeTimer) clearTimeout(finalizeTimer);
    if (pendingValue) {
      value = pendingValue;
      onchange?.(pendingValue);
    }
    isRecording = false;
    liveKeys = [];
    pendingValue = null;
  }

  function normalizeKeyName(key: string, code: string): string | null {
    if (key === 'Control' || key === 'Alt' || key === 'Shift' || key === 'Meta') {
      return null;
    }
    if (/^F\d{1,2}$/i.test(key)) return key.toUpperCase();
    if (key === ' ' || code === 'Space') return 'Space';
    if (key === 'Escape') return 'Escape';
    if (key === 'Backspace') return 'Backspace';
    if (key === 'Delete') return 'Delete';
    if (key === 'Enter') return 'Enter';
    if (key === 'Tab') return 'Tab';
    if (key === 'ArrowUp') return 'Up';
    if (key === 'ArrowDown') return 'Down';
    if (key === 'ArrowLeft') return 'Left';
    if (key === 'ArrowRight') return 'Right';
    if (code.startsWith('Key')) return code.slice(3).toUpperCase();
    if (code.startsWith('Digit')) return code.slice(5);
    if (key.length === 1) return key.toUpperCase();
    return key;
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (!isRecording) {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        startRecording();
      }
      return;
    }

    event.preventDefault();
    event.stopPropagation();

    const modifiers: string[] = [];
    if (event.ctrlKey) modifiers.push('Control');
    if (event.altKey) modifiers.push('Alt');
    if (event.shiftKey) modifiers.push('Shift');
    if (event.metaKey) modifiers.push('Super');

    const primaryKey = normalizeKeyName(event.key, event.code);

    if (primaryKey) {
      const parts = [...modifiers, primaryKey];
      const combined = parts.join('+');
      pendingValue = combined;
      liveKeys = parts;
      if (finalizeTimer) clearTimeout(finalizeTimer);
      finalizeTimer = setTimeout(() => {
        finalizeRecording();
      }, 500);
    } else {
      liveKeys = [...modifiers];
    }
  }

  function handleKeyUp(event: KeyboardEvent) {
    if (!isRecording) return;
    event.preventDefault();
    event.stopPropagation();

    if (pendingValue) {
      if (finalizeTimer) clearTimeout(finalizeTimer);
      finalizeTimer = setTimeout(() => {
        finalizeRecording();
      }, 120);
      return;
    }

    const modifiers: string[] = [];
    if (event.ctrlKey) modifiers.push('Control');
    if (event.altKey) modifiers.push('Alt');
    if (event.shiftKey) modifiers.push('Shift');
    if (event.metaKey) modifiers.push('Super');
    liveKeys = modifiers;
  }

  function handleBlur() {
    if (isRecording) {
      if (pendingValue) {
        finalizeRecording();
      } else {
        stopRecording();
      }
    }
  }

  function applyPreset(preset: string) {
    if (disabled) return;
    value = preset;
    onchange?.(preset);
    if (isRecording) stopRecording();
  }

  function resetToDefault() {
    if (disabled) return;
    value = defaultValue;
    onchange?.(defaultValue);
    if (isRecording) stopRecording();
  }

  function clearShortcut() {
    if (disabled) return;
    value = '';
    onchange?.('');
    if (isRecording) stopRecording();
  }
</script>

<div
  bind:this={fieldEl}
  class="shortcut-box {extraClass}"
  class:is-recording={isRecording}
  class:is-disabled={disabled}
  tabindex="0"
  role="button"
  aria-label={i18n.t('settings.panic_key_record_label') || 'Record shortcut'}
  onclick={startRecording}
  onkeydown={handleKeyDown}
  onkeyup={handleKeyUp}
  onblur={handleBlur}
>
  <!-- Left Inset Keyboard Icon -->
  <div class="left-icon">
    <IconKeyboard style="width: 19px; height: 19px;" />
  </div>

  <!-- Key Tokens Display -->
  <div class="shortcut-keys">
    {#if isRecording && liveKeys.length === 0}
      <span class="recording-prompt">
        {i18n.t('settings.panic_key_press_prompt') || 'Нажмите комбинацию клавиш...'}
      </span>
    {:else if displayTokens.length > 0}
      {#each displayTokens as token, idx (idx + token)}
        <kbd class="key-chip">{token}</kbd>
        {#if idx < displayTokens.length - 1}
          <span class="plus-sep">+</span>
        {/if}
      {/each}
      {#if isRecording}
        <span class="plus-sep">+</span>
        <span class="waiting-chip">?</span>
      {/if}
    {:else}
      <span class="placeholder-text">
        {i18n.t('settings.panic_key_none') || 'Не назначено (кликните для записи)'}
      </span>
    {/if}
  </div>

  <!-- Right Inset Action Buttons (Presets Popover + Reset + Clear) -->
  <div class="right-actions">
    {#if presets.length > 0}
      <PopoverMenu align="right" width="220px">
        {#snippet trigger({ toggle, open })}
          <button
            type="button"
            use:ripple
            use:tooltip={i18n.t('settings.panic_key_presets') || 'Пресеты'}
            class="icon-btn"
            class:is-open={open}
            onclick={(e) => {
              e.stopPropagation();
              toggle(e);
            }}
            aria-label="Presets"
            tabindex="-1"
          >
            <IconList style="width: 18px; height: 18px;" />
          </button>
        {/snippet}
        {#snippet children({ close })}
          <div class="preset-menu-list">
            {#each presets as preset}
              <button
                type="button"
                class="preset-item"
                class:selected={value === preset}
                onclick={() => {
                  applyPreset(preset);
                  close();
                }}
              >
                <code class="preset-badge">{preset}</code>
                {#if value === preset}
                  <IconCheck style="width: 15px; height: 15px; color: var(--accent); flex-shrink: 0;" />
                {/if}
              </button>
            {/each}
          </div>
        {/snippet}
      </PopoverMenu>
    {/if}

    {#if value && value !== defaultValue}
      <button
        type="button"
        class="icon-btn"
        use:ripple
        use:tooltip={i18n.t('settings.panic_key_reset') || 'Сбросить'}
        onclick={(e) => {
          e.stopPropagation();
          resetToDefault();
        }}
        aria-label="Reset shortcut"
        tabindex="-1"
      >
        <IconArrowReset style="width: 18px; height: 18px;" />
      </button>
    {/if}

    {#if value}
      <button
        type="button"
        class="icon-btn"
        use:ripple
        use:tooltip={i18n.t('settings.panic_key_clear') || 'Очистить'}
        onclick={(e) => {
          e.stopPropagation();
          clearShortcut();
        }}
        aria-label="Clear shortcut"
        tabindex="-1"
      >
        <IconDismiss style="width: 18px; height: 18px;" />
      </button>
    {/if}
  </div>
</div>

<style>
  .shortcut-box {
    display: flex;
    align-items: center;
    width: 100%;
    height: 46px;
    padding: 0 14px;
    gap: 10px;
    background: var(--bg-card);
    border: var(--border-width) solid var(--border-color);
    border-radius: var(--radius-full);
    box-sizing: border-box;
    cursor: pointer;
    outline: none !important;
    box-shadow: none !important;
    user-select: none;
    transition: background var(--duration-fast) var(--ease-expo),
                border-color var(--duration-fast) var(--ease-expo);
  }

  .shortcut-box:hover,
  .shortcut-box:focus {
    background: var(--bg-card-hover);
    border-color: var(--border-color-hover);
  }

  .shortcut-box:focus-visible,
  .shortcut-box.is-recording {
    background: var(--bg-card-hover);
    border-color: var(--border-color-focus);
    outline: none !important;
    box-shadow: none !important;
  }

  .shortcut-box.is-disabled {
    opacity: 0.5;
    pointer-events: none;
  }

  .left-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    opacity: 0.65;
    flex-shrink: 0;
    transition: color var(--duration-fast), opacity var(--duration-fast);
  }

  .shortcut-box:hover .left-icon,
  .shortcut-box:focus .left-icon,
  .shortcut-box.is-recording .left-icon {
    color: var(--accent);
    opacity: 1;
  }

  .shortcut-keys {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1;
    min-width: 0;
    overflow: hidden;
  }

  .key-chip {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    height: 24px;
    padding: 0 8px;
    background: rgba(255, 255, 255, 0.08);
    border: none;
    border-radius: var(--radius-sm, 6px);
    box-shadow: none;
    color: var(--text-primary, #fff);
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    font-weight: 600;
    line-height: 1;
  }

  .plus-sep {
    color: var(--text-muted, rgba(255, 255, 255, 0.35));
    font-size: 12px;
    font-weight: 600;
  }

  .waiting-chip {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border: 1px dashed var(--accent);
    border-radius: var(--radius-sm, 6px);
    color: var(--accent);
    font-size: 12px;
    font-weight: 700;
    animation: rec-pulse 1s infinite ease-in-out;
  }

  .recording-prompt {
    display: flex;
    align-items: center;
    color: var(--accent);
    font-size: 13px;
    font-weight: 500;
  }

  @keyframes rec-pulse {
    0%, 100% { opacity: 1; transform: scale(1); }
    50% { opacity: 0.4; transform: scale(1.2); }
  }

  .placeholder-text {
    color: var(--text-muted);
    opacity: 0.6;
    font-size: 13px;
  }

  .right-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
    margin-left: auto;
  }

  .icon-btn {
    width: 24px;
    height: 24px;
    border: none;
    outline: none;
    background: transparent;
    display: flex;
    align-items: center;
    justify-content: center;
    color: white;
    opacity: 0.45;
    cursor: pointer;
    padding: 0;
    border-radius: var(--radius-sm, 6px);
    transition: opacity 160ms ease, color 160ms ease;
  }

  .icon-btn:hover,
  .icon-btn.is-open {
    opacity: 0.95;
    color: var(--accent);
  }

  .preset-menu-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 2px;
    width: 100%;
    box-sizing: border-box;
  }

  .preset-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    width: 100%;
    padding: 8px 12px;
    border-radius: var(--radius-md, 8px);
    border: none;
    background: transparent;
    cursor: pointer;
    text-align: left;
    transition: background var(--duration-fast), color var(--duration-fast);
    box-sizing: border-box;
    flex-shrink: 0;
  }

  .preset-item:hover,
  .preset-item.selected {
    background: rgba(255, 255, 255, 0.08);
  }

  .preset-item.selected {
    box-shadow: inset 2px 0 0 var(--accent);
  }

  .preset-badge {
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 600;
    color: var(--accent);
    background: rgba(255, 255, 255, 0.06);
    padding: 2px 6px;
    border-radius: 4px;
    flex-shrink: 0;
  }
</style>
