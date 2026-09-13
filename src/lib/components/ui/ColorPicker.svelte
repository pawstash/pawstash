<script lang="ts">
  import type { Snippet } from 'svelte';
  import { ripple } from '$lib/motion';
  import { parseColorToRgb, ACCENT_PRESETS, getContrastInk } from '$lib/theme/palette';
  import { themeState } from '$lib/theme/themeState.svelte';
  import { i18n } from '$lib/i18n/i18nState.svelte';
  import PopoverMenu from '$lib/components/ui/PopoverMenu.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import { notify } from '$lib/utils/toast';
  import IconPalette from '~icons/fluent/color-20-regular';
  import IconCheck from '~icons/fluent/checkmark-24-regular';
  import IconCopy from '~icons/fluent/copy-24-regular';
  import IconEyedropper from '~icons/fluent/eyedropper-24-regular';

  interface Props {
    value?: string;
    label?: string;
    align?: 'left' | 'right';
    size?: 'sm' | 'base' | 'md' | 'lg';
    showPalette?: boolean;
    disabled?: boolean;
    class?: string;
    onchange?: (color: string) => void;
    trigger?: Snippet<[{ toggle: (e: MouseEvent) => void; open: boolean; currentHex: string }]>;
  }

  let {
    value = '#000000',
    label,
    align = 'right',
    size,
    showPalette = true,
    disabled = false,
    class: extraClass = '',
    onchange,
    trigger: customTrigger
  }: Props = $props();

  const M3_TONES = [0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 95];

  const resolvedLabel = $derived(label ?? i18n.t('color_picker.label') ?? 'Color');

  const accentPresets = $derived(
    ACCENT_PRESETS.map((preset) => ({
      hex: preset.hex,
      name: i18n.t(`color_picker.accent_${preset.id}`) || preset.id
    }))
  );

  const activeMaterialRoles = $derived([
    { name: i18n.t('color_picker.role_primary') || 'Primary accent', hex: themeState.palette.primary },
    { name: i18n.t('color_picker.role_container') || 'Container accent', hex: themeState.palette.quadrants[1] },
    { name: i18n.t('color_picker.role_deep') || 'Deep accent', hex: themeState.palette.quadrants[2] },
    { name: i18n.t('color_picker.role_subtle') || 'Subtle accent', hex: themeState.palette.quadrants[3] }
  ]);

  let hsl = $state({ h: 0, s: 0, l: 0 });
  let hexDisplay = $state('#000000');
  let lastPropValue = $state('');
  let isCopied = $state(false);
  let copyTimeout: ReturnType<typeof setTimeout> | undefined;

  function hslToHex(h: number, s: number, l: number): string {
    l /= 100;
    const a = (s * Math.min(l, 1 - l)) / 100;
    const f = (n: number) => {
      const k = (n + h / 30) % 12;
      const color = l - a * Math.max(Math.min(k - 3, 9 - k, 1), -1);
      return Math.round(255 * color).toString(16).padStart(2, '0');
    };
    return `#${f(0)}${f(8)}${f(4)}`.toUpperCase();
  }

  function hexToHsl(hex: string): { h: number; s: number; l: number } {
    const rgb = parseColorToRgb(hex);
    const r = rgb.r / 255;
    const g = rgb.g / 255;
    const b = rgb.b / 255;
    const max = Math.max(r, g, b);
    const min = Math.min(r, g, b);
    let h = hsl?.h ?? 0;
    let s = 0;
    const l = (max + min) / 2;

    if (max !== min) {
      const d = max - min;
      s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
      switch (max) {
        case r:
          h = (g - b) / d + (g < b ? 6 : 0);
          break;
        case g:
          h = (b - r) / d + 2;
          break;
        case b:
          h = (r - g) / d + 4;
          break;
      }
      h /= 6;
      h = Math.round(h * 360);
    }

    return {
      h,
      s: Math.round(s * 100),
      l: Math.round(l * 100)
    };
  }

  $effect(() => {
    if (value && value.toUpperCase() !== lastPropValue.toUpperCase()) {
      lastPropValue = value.toUpperCase();
      hexDisplay = value.toUpperCase();
      const parsed = hexToHsl(value);
      if (parsed.s === 0 || parsed.l === 0 || parsed.l === 100) {
        hsl = { h: hsl.h, s: parsed.s, l: parsed.l };
      } else {
        hsl = parsed;
      }
    }
  });

  function applyColor(newHex: string) {
    const formatted = newHex.toUpperCase();
    hexDisplay = formatted;
    lastPropValue = formatted;
    const parsed = hexToHsl(formatted);
    if (parsed.s > 0 && parsed.l > 0 && parsed.l < 100) {
      hsl = parsed;
    } else {
      hsl = { h: hsl.h, s: parsed.s, l: parsed.l };
    }
    onchange?.(formatted);
  }

  function updateHsl(newH: number, newS: number, newL: number) {
    if (newL === 0 && (newH !== hsl.h || (newS !== hsl.s && newS > 0))) {
      newL = 50;
      if (newS === 0) newS = 85;
    }
    if (newS === 0 && newH !== hsl.h) {
      newS = 85;
      if (newL === 0) newL = 50;
    }
    hsl = { h: newH, s: newS, l: newL };
    const hex = hslToHex(hsl.h, hsl.s, hsl.l);
    hexDisplay = hex;
    lastPropValue = hex;
    onchange?.(hex);
  }

  function setTone(tone: number) {
    updateHsl(hsl.h, hsl.s, tone);
  }

  function handleHexInput(e: Event) {
    const raw = (e.target as HTMLInputElement).value.trim();
    const clean = raw.startsWith('#') ? raw : `#${raw}`;
    hexDisplay = clean.toUpperCase();
    if (/^#[0-9A-Fa-f]{6}$/.test(clean)) {
      applyColor(clean);
    }
  }

  async function copyHex() {
    try {
      await navigator.clipboard.writeText(hexDisplay);
      isCopied = true;
      clearTimeout(copyTimeout);
      copyTimeout = setTimeout(() => {
        isCopied = false;
      }, 1600);
      notify.success(i18n.t('color_picker.copied_hex', { hex: hexDisplay }) || `Copied ${hexDisplay}`);
    } catch {
    }
  }

  function handleHexBlur() {
    if (!/^#[0-9A-Fa-f]{6}$/.test(hexDisplay)) {
      hexDisplay = currentHex;
    }
  }

  const currentHex = $derived(hslToHex(hsl.h, hsl.s, hsl.l));
</script>

<PopoverMenu
  {align}
  width="calc(310px * var(--ui-scale, 1))"
  menuClass="color-picker-popover-portal"
  class="color-picker-root {customTrigger ? 'has-custom-trigger' : ''} {extraClass}"
>
  {#snippet trigger({ toggle, open })}
    {#if customTrigger}
      {@render customTrigger({ toggle, open, currentHex })}
    {:else}
      <button
        type="button"
        onclick={toggle}
        use:ripple
        {disabled}
        class="color-picker-trigger {size ? `size-${size}` : ''}"
        class:is-open={open}
        class:is-disabled={disabled}
        aria-label={resolvedLabel}
        aria-haspopup="dialog"
        aria-expanded={open}
      >
        <span class="trigger-color-circle" style="background-color: {currentHex};"></span>
        <span class="trigger-label">{currentHex}</span>
        <span class="trigger-icon" class:is-open={open}>
          <IconPalette />
        </span>
      </button>
    {/if}
  {/snippet}

  {#snippet children({ close })}
    <div class="color-picker-dialog">
      <div class="preview-card">
        <div class="preview-glow" style="background-color: {currentHex};"></div>
        <div class="preview-content">
          <input
            type="text"
            class="preview-hex-input"
            value={hexDisplay}
            oninput={handleHexInput}
            onblur={handleHexBlur}
            onkeydown={(e) => {
              if (e.key === 'Enter') (e.currentTarget as HTMLInputElement).blur();
            }}
            maxlength={7}
            spellcheck="false"
            autocomplete="off"
            aria-label={i18n.t('color_picker.hex_code') || 'Hex color code'}
          />

          <div class="preview-actions">
            <Button
              variant="ghost"
              size="sm"
              class="action-icon-btn"
              onclick={copyHex}
              aria-label={i18n.t('color_picker.copy_hex') || 'Copy HEX'}
            >
              {#if isCopied}
                <span class="text-emerald-400">
                  <IconCheck />
                </span>
              {:else}
                <IconCopy />
              {/if}
            </Button>

            <div class="action-picker-wrapper">
              <Button
                variant="ghost"
                size="sm"
                class="action-icon-btn"
                aria-hidden="true"
                tabindex={-1}
              >
                <IconEyedropper />
              </Button>
              <input
                type="color"
                value={currentHex}
                oninput={(e) => applyColor(e.currentTarget.value)}
                onchange={(e) => applyColor(e.currentTarget.value)}
                class="action-picker-native-input"
                aria-label={i18n.t('color_picker.system_picker') || 'System Color Picker'}
              />
            </div>
          </div>
        </div>
      </div>

      <div class="color-picker-body">
        <div class="dialog-section">
          <span class="section-title">{i18n.t('color_picker.tonal') || 'Tonal'}</span>
          <div class="tonal-strip">
            {#each M3_TONES as tone}
              {@const toneHex = hslToHex(hsl.h, hsl.s, tone)}
              {@const active = Math.abs(hsl.l - tone) <= 4}
              <button
                type="button"
                use:ripple
                class="tone-cell"
                class:is-active={active}
                style="background-color: {toneHex};"
                onclick={() => setTone(tone)}
                aria-label={i18n.t('color_picker.tone_value', { tone }) || `Tone ${tone}`}
              >
                {#if active}
                  <span class="swatch-check ink-{getContrastInk(toneHex)}">
                    <IconCheck />
                  </span>
                {/if}
              </button>
            {/each}
          </div>
        </div>

        {#if showPalette}
          <div class="dialog-section">
            <span class="section-title">{i18n.t('color_picker.palette') || 'Palette'}</span>

            <div class="material-roles-grid">
              {#each activeMaterialRoles as role}
                {@const active = currentHex.toUpperCase() === role.hex.toUpperCase()}
                <button
                  type="button"
                  use:ripple
                  class="role-pill"
                  class:is-active={active}
                  style="background-color: {role.hex};"
                  onclick={() => applyColor(role.hex)}
                  aria-label={role.name}
                >
                  {#if active}
                    <span class="swatch-check ink-{getContrastInk(role.hex)}">
                      <IconCheck />
                    </span>
                  {/if}
                </button>
              {/each}
            </div>

            <div class="presets-grid">
              {#each accentPresets as preset}
                {@const active = currentHex.toUpperCase() === preset.hex.toUpperCase()}
                <button
                  type="button"
                  use:ripple
                  class="key-chip"
                  class:is-active={active}
                  style="background-color: {preset.hex};"
                  onclick={() => applyColor(preset.hex)}
                  aria-label={preset.name}
                >
                  {#if active}
                    <span class="swatch-check ink-{getContrastInk(preset.hex)}">
                      <IconCheck />
                    </span>
                  {/if}
                </button>
              {/each}
            </div>
          </div>
        {/if}

        <div class="sliders-section">
          <div class="slider-row">
            <span class="slider-label">{i18n.t('color_picker.hue') || 'Hue'}</span>
            <input
              type="range"
              min="0"
              max="360"
              step="1"
              value={hsl.h}
              oninput={(e) => updateHsl(Number(e.currentTarget.value), hsl.s, hsl.l)}
              class="m3-slider-input hue-gradient-track"
              aria-label={i18n.t('color_picker.hue') || 'Hue'}
            />
            <span class="slider-value">{hsl.h}°</span>
          </div>

          <div class="slider-row">
            <span class="slider-label">{i18n.t('color_picker.tone') || 'Tone'}</span>
            <input
              type="range"
              min="0"
              max="100"
              step="1"
              value={hsl.l}
              oninput={(e) => updateHsl(hsl.h, hsl.s, Number(e.currentTarget.value))}
              class="m3-slider-input"
              style="background: linear-gradient(to right, #000000 0%, {hslToHex(hsl.h, hsl.s, 50)} 50%, #ffffff 100%);"
              aria-label={i18n.t('color_picker.tone') || 'Tone'}
            />
            <span class="slider-value">{hsl.l}%</span>
          </div>

          <div class="slider-row">
            <span class="slider-label">{i18n.t('color_picker.chroma') || 'Chroma'}</span>
            <input
              type="range"
              min="0"
              max="100"
              step="1"
              value={hsl.s}
              oninput={(e) => updateHsl(hsl.h, Number(e.currentTarget.value), hsl.l)}
              class="m3-slider-input"
              style="background: linear-gradient(to right, {hslToHex(hsl.h, 0, hsl.l)} 0%, {hslToHex(hsl.h, 100, hsl.l)} 100%);"
              aria-label={i18n.t('color_picker.chroma') || 'Chroma'}
            />
            <span class="slider-value">{hsl.s}%</span>
          </div>
        </div>
      </div>
    </div>
  {/snippet}
</PopoverMenu>

<style>
  :global(.color-picker-root) {
    position: relative;
    display: inline-flex;
    width: auto;
    max-width: fit-content;
    flex-shrink: 0;
  }

  :global(.color-picker-root:not(.has-custom-trigger)) {
    margin-left: auto;
  }

  :global(.color-picker-root.has-custom-trigger) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    margin-left: 0;
  }

  .color-picker-trigger {
    display: inline-flex;
    align-items: center;
    justify-content: flex-start;
    gap: calc(8px * var(--ui-scale, 1));
    width: auto;
    min-width: calc(108px * var(--ui-scale, 1));
    height: calc(var(--control-height, 36px) * var(--ui-scale, 1));
    padding: 0 calc(12px * var(--ui-scale, 1)) 0 calc(10px * var(--ui-scale, 1));
    background: var(--select-bg);
    border: none;
    border-radius: var(--radius-full);
    color: var(--text-primary);
    font-size: calc(var(--control-font-size, 13px) * var(--ui-scale, 1));
    font-family: var(--font-sans);
    line-height: normal;
    cursor: pointer;
    text-align: left;
    outline: none;
    box-sizing: border-box;
    user-select: none;
    transition:
      background var(--duration-fast) var(--ease-expo),
      color var(--duration-fast) var(--ease-expo);
  }

  .color-picker-trigger.size-sm {
    --control-height: var(--control-height-sm, 32px);
    --control-font-size: var(--control-font-sm, 12px);
    --control-icon-size: var(--control-icon-sm, 14px);
    --control-padding-x: var(--control-padding-sm, 10px);
  }

  .color-picker-trigger.size-base {
    --control-height: var(--control-height-base, 36px);
    --control-font-size: var(--control-font-base, 13px);
    --control-icon-size: var(--control-icon-base, 16px);
    --control-padding-x: var(--control-padding-base, 12px);
  }

  .color-picker-trigger.size-md {
    --control-height: var(--control-height-md, 40px);
    --control-font-size: var(--control-font-md, 13.5px);
    --control-icon-size: var(--control-icon-md, 18px);
    --control-padding-x: var(--control-padding-md, 14px);
  }

  .color-picker-trigger.size-lg {
    --control-height: var(--control-height-lg, 46px);
    --control-font-size: var(--control-font-lg, 14px);
    --control-icon-size: var(--control-icon-lg, 20px);
    --control-padding-x: var(--control-padding-lg, 16px);
  }

  .color-picker-trigger:hover {
    background: var(--select-bg-hover);
  }

  .color-picker-trigger.is-open {
    background: var(--select-bg-hover);
  }

  .color-picker-trigger:focus-visible {
    outline: calc(1.5px * var(--ui-scale, 1)) solid var(--accent-primary);
    outline-offset: calc(-1.5px * var(--ui-scale, 1));
  }

  .color-picker-trigger.is-disabled,
  .color-picker-trigger:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    pointer-events: none;
  }

  .trigger-color-circle {
    width: calc(var(--control-icon-size, 16px) * var(--ui-scale, 1));
    height: calc(var(--control-icon-size, 16px) * var(--ui-scale, 1));
    border-radius: var(--radius-full);
    flex-shrink: 0;
    border: none !important;
    box-shadow: none !important;
    outline: none !important;
  }

  .trigger-label {
    flex: 1 1 auto;
    min-width: 0;
    white-space: nowrap;
    font-family: var(--font-mono);
    font-size: calc(var(--control-font-size, 13px) * var(--ui-scale, 1));
    font-weight: 500;
    letter-spacing: 0.02em;
    line-height: normal;
    color: var(--text-primary);
  }

  .trigger-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    opacity: 0.5;
    color: var(--text-secondary);
    transition:
      opacity var(--duration-fast) var(--ease-expo),
      color var(--duration-fast) var(--ease-expo);
    flex-shrink: 0;
    margin-left: auto;
  }

  .color-picker-trigger:hover .trigger-icon,
  .color-picker-trigger.is-open .trigger-icon {
    opacity: 0.85;
    color: var(--text-primary);
  }

  .trigger-icon :global(svg) {
    width: calc(var(--control-icon-size, 16px) * var(--ui-scale, 1)) !important;
    height: calc(var(--control-icon-size, 16px) * var(--ui-scale, 1)) !important;
  }

  :global(.popover-menu-dropdown.color-picker-popover-portal) {
    padding: 0 !important;
    border: none !important;
    overflow: hidden !important;
  }

  .color-picker-dialog {
    display: flex;
    flex-direction: column;
    padding: 0;
    box-sizing: border-box;
    width: 100%;
    --swatch-ink-light: #ffffff;
    --swatch-ink-dark: #000000;
  }

  .preview-card {
    position: relative;
    width: 100%;
    height: calc(58px * var(--ui-scale, 1));
    border: none;
    box-sizing: border-box;
    flex-shrink: 0;
  }

  .preview-glow {
    position: absolute;
    top: 50%;
    right: calc(-70px * var(--ui-scale, 1));
    width: calc(320px * var(--ui-scale, 1));
    height: calc(320px * var(--ui-scale, 1));
    border-radius: var(--radius-full);
    pointer-events: none;
    filter: blur(calc(56px * var(--ui-scale, 1)));
    opacity: 0.28;
    transform: translateY(-50%);
  }

  .preview-content {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 calc(var(--space-3) * var(--ui-scale, 1));
    z-index: 1;
  }

  .color-picker-body {
    display: flex;
    flex-direction: column;
    gap: calc(10px * var(--ui-scale, 1));
    padding: calc(8px * var(--ui-scale, 1)) calc(var(--space-3) * var(--ui-scale, 1)) calc(var(--space-3) * var(--ui-scale, 1));
    box-sizing: border-box;
  }

  .preview-hex-input {
    font-family: var(--font-mono);
    font-size: calc(23px * var(--ui-scale, 1));
    font-weight: 900;
    letter-spacing: -0.01em;
    line-height: 1.1;
    color: var(--text-primary);
    text-shadow: 0 2px 8px rgba(0, 0, 0, 0.75);
    background: transparent;
    border: none;
    outline: none !important;
    box-shadow: none !important;
    padding: 0;
    margin: 0;
    width: calc(130px * var(--ui-scale, 1));
    cursor: text;
    -webkit-tap-highlight-color: transparent;
  }

  .preview-hex-input:hover,
  .preview-hex-input:focus,
  .preview-hex-input:focus-visible,
  .preview-hex-input:active {
    outline: none !important;
    box-shadow: none !important;
    border: none !important;
    background: transparent !important;
  }

  .preview-actions {
    display: flex;
    align-items: center;
    gap: calc(var(--space-1) * var(--ui-scale, 1));
  }

  .action-picker-wrapper {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .action-picker-native-input {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    opacity: 0;
    cursor: pointer;
    border: none;
    padding: 0;
    margin: 0;
    z-index: 5;
  }

  :global(.action-icon-btn) {
    width: calc(30px * var(--ui-scale, 1)) !important;
    height: calc(30px * var(--ui-scale, 1)) !important;
    padding: 0 !important;
    background: rgba(0, 0, 0, 0.28) !important;
    border: none !important;
    backdrop-filter: blur(12px) !important;
    color: #ffffff !important;
    border-radius: calc(var(--radius-sm) * var(--ui-scale, 1)) !important;
  }

  :global(.action-icon-btn:hover) {
    background: rgba(0, 0, 0, 0.44) !important;
  }

  .dialog-section {
    display: flex;
    flex-direction: column;
    gap: calc(5px * var(--ui-scale, 1));
  }

  .section-title {
    font-family: var(--font-sans);
    font-size: calc(var(--text-xs) * var(--ui-scale, 1));
    font-weight: var(--font-weight-medium);
    letter-spacing: normal;
    text-transform: none;
    color: var(--text-secondary);
    line-height: var(--leading-tight, 1.25);
  }

  .tonal-strip {
    display: grid;
    grid-template-columns: repeat(11, 1fr);
    height: calc(24px * var(--ui-scale, 1));
    border-radius: calc(var(--radius-sm) * var(--ui-scale, 1));
    overflow: hidden;
    border: none;
  }

  .tone-cell {
    position: relative;
    width: 100%;
    height: 100%;
    border: none;
    padding: 0;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    outline: none;
    transition: transform var(--duration-fast) var(--ease-expo);
  }

  .tone-cell:hover {
    transform: scaleY(1.18);
    z-index: 2;
  }

  .tone-cell.is-active {
    box-shadow: inset 0 0 0 calc(2px * var(--ui-scale, 1)) var(--accent-primary);
  }

  .tone-cell:focus-visible {
    outline: calc(2px * var(--ui-scale, 1)) solid var(--accent-primary);
    outline-offset: calc(-2px * var(--ui-scale, 1));
    z-index: 3;
  }

  .swatch-check {
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .swatch-check.ink-light {
    color: var(--swatch-ink-light);
  }

  .swatch-check.ink-dark {
    color: var(--swatch-ink-dark);
  }

  .tonal-strip .swatch-check :global(svg) {
    width: calc(11px * var(--ui-scale, 1));
    height: calc(11px * var(--ui-scale, 1));
  }

  .material-roles-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: calc(6px * var(--ui-scale, 1));
  }

  .role-pill {
    position: relative;
    width: 100%;
    height: calc(22px * var(--ui-scale, 1));
    border-radius: var(--radius-full);
    border: none;
    cursor: pointer;
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    outline: none;
    transition:
      transform var(--duration-fast) var(--ease-expo),
      box-shadow var(--duration-fast) var(--ease-expo);
  }

  .role-pill:hover {
    transform: scale(1.05);
  }

  .role-pill.is-active {
    box-shadow:
      0 0 0 calc(2px * var(--ui-scale, 1)) var(--bg-dropdown),
      0 0 0 calc(3.5px * var(--ui-scale, 1)) var(--accent-primary);
  }

  .role-pill:focus-visible {
    outline: calc(2px * var(--ui-scale, 1)) solid var(--accent-primary);
    outline-offset: calc(2px * var(--ui-scale, 1));
  }

  .presets-grid {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: calc(6px * var(--ui-scale, 1));
  }

  .key-chip {
    position: relative;
    width: calc(24px * var(--ui-scale, 1));
    height: calc(24px * var(--ui-scale, 1));
    border-radius: var(--radius-full);
    border: none;
    cursor: pointer;
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    outline: none;
    transition:
      transform var(--duration-fast) var(--ease-expo),
      box-shadow var(--duration-fast) var(--ease-expo);
  }

  .key-chip:hover {
    transform: scale(1.18);
  }

  .key-chip.is-active {
    box-shadow:
      0 0 0 calc(2px * var(--ui-scale, 1)) var(--bg-dropdown),
      0 0 0 calc(3.5px * var(--ui-scale, 1)) var(--accent-primary);
    transform: scale(1.1);
  }

  .key-chip:focus-visible {
    outline: calc(2px * var(--ui-scale, 1)) solid var(--accent-primary);
    outline-offset: calc(2px * var(--ui-scale, 1));
  }

  .material-roles-grid .swatch-check :global(svg),
  .presets-grid .swatch-check :global(svg) {
    width: calc(12px * var(--ui-scale, 1));
    height: calc(12px * var(--ui-scale, 1));
  }

  .sliders-section {
    display: flex;
    flex-direction: column;
    gap: calc(7px * var(--ui-scale, 1));
    padding-top: calc(4px * var(--ui-scale, 1));
    border: none;
  }

  .slider-row {
    display: flex;
    align-items: center;
    gap: calc(8px * var(--ui-scale, 1));
  }

  .slider-label {
    width: calc(44px * var(--ui-scale, 1));
    font-family: var(--font-sans);
    font-size: calc(var(--text-xs) * var(--ui-scale, 1));
    font-weight: var(--font-weight-medium);
    letter-spacing: normal;
    text-transform: none;
    color: var(--text-secondary);
    line-height: var(--leading-tight, 1.25);
    flex-shrink: 0;
  }

  .slider-value {
    width: calc(34px * var(--ui-scale, 1));
    text-align: right;
    font-family: var(--font-mono);
    font-size: calc(var(--text-xs) * var(--ui-scale, 1));
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .m3-slider-input {
    flex: 1;
    min-width: 0;
    height: calc(6px * var(--ui-scale, 1));
    border-radius: var(--radius-full);
    -webkit-appearance: none;
    appearance: none;
    outline: none !important;
    cursor: pointer;
    border: none;
    box-shadow: none !important;
  }

  .m3-slider-input:focus {
    outline: none;
  }

  .m3-slider-input:focus-visible::-webkit-slider-thumb {
    box-shadow:
      0 1px 4px rgba(0, 0, 0, 0.4),
      0 0 0 calc(3px * var(--ui-scale, 1)) var(--accent-primary);
  }

  .m3-slider-input::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: calc(15px * var(--ui-scale, 1));
    height: calc(15px * var(--ui-scale, 1));
    border-radius: var(--radius-full);
    background: #ffffff;
    border: none;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.4);
    cursor: pointer;
    transition: transform var(--duration-fast) var(--ease-expo);
  }

  .m3-slider-input::-webkit-slider-thumb:hover {
    transform: scale(1.15);
  }

  .hue-gradient-track {
    background: linear-gradient(
      to right,
      #ff0000 0%,
      #ffff00 17%,
      #00ff00 33%,
      #00ffff 50%,
      #0000ff 67%,
      #ff00ff 83%,
      #ff0000 100%
    );
  }

  @media (max-width: 640px) {
    .tonal-strip {
      grid-template-columns: repeat(auto-fit, minmax(calc(44px * var(--ui-scale, 1)), 1fr));
      height: auto;
      gap: calc(2px * var(--ui-scale, 1));
    }

    .tone-cell {
      height: calc(44px * var(--ui-scale, 1));
    }

    .tone-cell:hover {
      transform: none;
    }

    .role-pill {
      height: calc(44px * var(--ui-scale, 1));
    }

    .presets-grid {
      grid-template-columns: repeat(auto-fit, minmax(calc(44px * var(--ui-scale, 1)), 1fr));
    }

    .key-chip {
      width: 100%;
      height: calc(44px * var(--ui-scale, 1));
      border-radius: calc(var(--radius-md) * var(--ui-scale, 1));
    }

    .m3-slider-input {
      height: calc(12px * var(--ui-scale, 1));
    }

    .m3-slider-input::-webkit-slider-thumb {
      width: calc(24px * var(--ui-scale, 1));
      height: calc(24px * var(--ui-scale, 1));
    }
  }
</style>
