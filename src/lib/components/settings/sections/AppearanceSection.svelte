<script lang="ts">
  import { themeState } from '$lib/theme/themeState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { i18n } from '$lib/i18n';
  import { COLOR_MODES, PRESET_QUADRANTS, SCHEME_VARIANTS, type ContrastLevel } from '$lib/theme/palette';
  import { generateAccentPalette } from '$lib/theme/paletteEngine';
  import type { AccentColor } from '$lib/theme/tokens';
  import type { AppSettings } from '$lib/types/config';
  import SettingItem from '$lib/components/ui/SettingItem.svelte';
  import PaletteCircle from '$lib/components/ui/PaletteCircle.svelte';
  import ColorPicker from '$lib/components/ui/ColorPicker.svelte';
  import SettingsGroup from '../SettingsGroup.svelte';
  import SettingChoice from '../SettingChoice.svelte';
  import BackgroundSection from './BackgroundSection.svelte';
  import { settingsController as ctl } from '../settingsController.svelte';
  import IconModeSystem from '~icons/fluent/desktop-24-regular';
  import IconModeLight from '~icons/fluent/weather-sunny-24-regular';
  import IconModeDark from '~icons/fluent/weather-moon-24-regular';

  const PRESETS: { id: AccentColor; quadrants: [string, string, string, string] }[] = [
    { id: '#D69085', quadrants: PRESET_QUADRANTS['#D69085'] },
    { id: 'rose', quadrants: PRESET_QUADRANTS.rose },
    { id: 'violet', quadrants: PRESET_QUADRANTS.violet },
    { id: 'cyan', quadrants: PRESET_QUADRANTS.cyan },
    { id: 'emerald', quadrants: PRESET_QUADRANTS.emerald },
    { id: 'amber', quadrants: PRESET_QUADRANTS.amber },
    { id: 'indigo', quadrants: PRESET_QUADRANTS.indigo }
  ];
  const CUSTOM_ACCENT_SEED = '#8b5cf6';

  let settings = $derived(ctl.settings);
  let accent = $derived(themeState.tokens.accent);
  let isCustomAccent = $derived(
    accent !== 'system' && !PRESETS.some((preset) => preset.id.toLowerCase() === accent.toLowerCase())
  );
  let customQuadrants = $derived<[string, string, string, string]>(
    isCustomAccent && accent.startsWith('#')
      ? generateAccentPalette(accent).quadrants
      : ['#d69085', '#f59e0b', '#10b981', '#6366f1']
  );
  let showWindow = $derived((layoutState.hasWindowFrame && !layoutState.isMobile) || layoutState.isLinux);
</script>

<SettingsGroup label={i18n.t('settings.group_theme')}>
  <SettingChoice
    title={i18n.t('settings.color_mode')}
    description={i18n.t('settings.color_mode_desc')}
    options={COLOR_MODES.map((mode) => ({
      value: mode,
      label: i18n.t(`settings.color_mode_${mode}`),
      icon: mode === 'system' ? IconModeSystem : mode === 'light' ? IconModeLight : IconModeDark
    }))}
    value={themeState.tokens.colorMode}
    onchange={(mode) => themeState.setColorMode(mode)}
  />

  <SettingItem title={i18n.t('settings.accent_color')} description={i18n.t('settings.accent_color_desc')}>
    <div class="accent-swatches">
      {#if themeState.systemPalette}
        <PaletteCircle
          quadrants={themeState.systemPalette.quadrants}
          active={accent === 'system'}
          label={i18n.t('settings.system_accent')}
          onclick={() => themeState.setAccent('system')}
        />
      {/if}
      {#each PRESETS as preset (preset.id)}
        <PaletteCircle
          quadrants={preset.quadrants}
          active={accent.toLowerCase() === preset.id.toLowerCase()}
          label={i18n.t('settings.set_accent', { color: preset.id })}
          onclick={() => themeState.setAccent(preset.id)}
        />
      {/each}
      <ColorPicker
        value={isCustomAccent ? accent : CUSTOM_ACCENT_SEED}
        label={i18n.t('settings.custom_accent')}
        showPalette={false}
        onchange={(color) => themeState.setAccent(color)}
      >
        {#snippet trigger({ toggle })}
          <PaletteCircle
            quadrants={customQuadrants}
            active={isCustomAccent}
            label={i18n.t('settings.custom_accent')}
            onclick={(event) => {
              if (!isCustomAccent) themeState.setAccent(accent.startsWith('#') ? accent : CUSTOM_ACCENT_SEED);
              toggle(event);
            }}
          />
        {/snippet}
      </ColorPicker>
    </div>
  </SettingItem>

  <SettingItem
    title={i18n.t('settings.dynamic_accent')}
    description={i18n.t('settings.dynamic_accent_desc')}
    checked={settings.dynamic_accent}
    onCheckedChange={(value) => void ctl.save('dynamic_accent', value)}
  />

  <SettingChoice
    title={i18n.t('settings.color_scheme')}
    description={themeState.schemeIsForcedNeutral
      ? i18n.t('settings.color_scheme_neutral_source')
      : i18n.t('settings.color_scheme_desc')}
    options={SCHEME_VARIANTS.map((variant) => ({
      value: variant,
      label: i18n.t(`settings.scheme_${variant.replace('-', '_')}`)
    }))}
    value={themeState.tokens.schemeVariant}
    onchange={(variant) => themeState.setSchemeVariant(variant)}
  />

  <SettingChoice
    title={i18n.t('settings.scheme_contrast')}
    description={i18n.t('settings.scheme_contrast_desc')}
    options={[
      { value: -1 as ContrastLevel, label: i18n.t('settings.contrast_reduced') },
      { value: 0 as ContrastLevel, label: i18n.t('settings.contrast_default') },
      { value: 0.5 as ContrastLevel, label: i18n.t('settings.contrast_medium') },
      { value: 1 as ContrastLevel, label: i18n.t('settings.contrast_high') }
    ]}
    value={themeState.tokens.contrastLevel}
    onchange={(level) => themeState.setContrastLevel(level)}
  />
</SettingsGroup>

<BackgroundSection />

{#if showWindow}
  <SettingsGroup label={i18n.t('settings.group_window')}>
    {#if layoutState.hasWindowFrame && !layoutState.isMobile}
      <SettingChoice
        title={i18n.t('settings.titlebar_style')}
        description={i18n.t('settings.titlebar_style_desc')}
        options={[
          { value: 'auto', label: i18n.t('settings.titlebar_style_auto') },
          { value: 'windows', label: 'Windows' },
          { value: 'macos', label: 'macOS' }
        ] as { value: NonNullable<AppSettings['titlebar_style']>; label: string }[]}
        value={settings.titlebar_style || 'auto'}
        onchange={(style) => void ctl.save('titlebar_style', style)}
      />
    {/if}

    {#if layoutState.isLinux}
      <SettingItem
        title={i18n.t('settings.linux_transparent_window')}
        description={i18n.t('settings.linux_transparent_window_desc')}
        checked={settings.linux_transparent_window ?? false}
        onCheckedChange={(value) => void ctl.save('linux_transparent_window', value)}
      />
    {/if}
  </SettingsGroup>
{/if}

<style>
  .accent-swatches {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
</style>
