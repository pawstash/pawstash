<script lang="ts">
  import { onDestroy } from 'svelte';
  import { themeState } from '$lib/theme/themeState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { uiScaleState } from '$lib/state/uiScaleState.svelte';
  import { i18n } from '$lib/i18n';
  import { missingFontFamily } from '$lib/utils/fonts';
  import { UI_SCALE_MAX, UI_SCALE_MIN, UI_SCALE_STEP } from '$lib/theme/tokens';
  import type { AppSettings } from '$lib/types/config';
  import SettingItem from '$lib/components/ui/SettingItem.svelte';
  import Slider from '$lib/components/ui/Slider.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import ShortcutInput from '$lib/components/ui/ShortcutInput.svelte';
  import SettingsGroup from '../SettingsGroup.svelte';
  import SettingChoice from '../SettingChoice.svelte';
  import { settingsController as ctl } from '../settingsController.svelte';
  import IconFlagUs from '~icons/circle-flags/us';
  import IconFlagRu from '~icons/circle-flags/ru';

  let settings = $derived(ctl.settings);
  let panicEnabled = $derived(settings.panic_button_enabled ?? settings.boss_key_enabled ?? true);
  let panicShortcut = $derived(settings.panic_button_shortcut || settings.boss_key_shortcut || 'H');

  let fontTimer: ReturnType<typeof setTimeout> | undefined;
  let missingFont = $derived.by(() => {
    const value = themeState.tokens.fontFamily?.trim();
    return value ? missingFontFamily(value) : null;
  });

  function commitFont(value: string) {
    clearTimeout(fontTimer);
    fontTimer = undefined;
    themeState.setFontFamily(value);
  }

  function handleFontInput(event: Event) {
    const value = (event.target as HTMLInputElement).value;
    clearTimeout(fontTimer);
    fontTimer = setTimeout(() => {
      fontTimer = undefined;
      themeState.setFontFamily(value);
    }, 220);
  }

  function handleFontKeydown(event: KeyboardEvent) {
    if (event.key !== 'Enter') return;
    event.preventDefault();
    commitFont((event.target as HTMLInputElement).value);
    (event.target as HTMLInputElement).blur();
  }

  onDestroy(() => clearTimeout(fontTimer));
</script>

<SettingsGroup label={i18n.t('settings.group_text')}>
  <SettingChoice
    title={i18n.t('settings.language')}
    options={[
      { value: 'en', label: 'English', icon: IconFlagUs },
      { value: 'ru', label: 'Русский', icon: IconFlagRu }
    ]}
    value={i18n.currentLocale}
    onchange={(locale) => i18n.setLocale(locale as typeof i18n.currentLocale)}
  />

  <SettingItem
    title={i18n.t('settings.font_family')}
    description={i18n.t('settings.font_family_desc')}
    warning={missingFont ? i18n.t('settings.font_family_missing', { font: missingFont }) : undefined}
  >
    <Input
      value={themeState.tokens.fontFamily || ''}
      placeholder={i18n.t('settings.font_family_placeholder')}
      clearable={true}
      oninput={handleFontInput}
      onchange={(event) => commitFont((event.target as HTMLInputElement).value)}
      onkeydown={handleFontKeydown}
    />
  </SettingItem>
</SettingsGroup>

<SettingsGroup label={i18n.t('settings.group_display')}>
  <SettingItem
    title={i18n.t('settings.ui_scale')}
    description={layoutState.hasKeyboard
      ? `${i18n.t('settings.ui_scale_desc')} ${i18n.t('settings.ui_scale_desc_keyboard')}`
      : i18n.t('settings.ui_scale_desc')}
    stacked
  >
    <div class="ui-scale">
      <Slider
        class="ui-scale__slider"
        min={UI_SCALE_MIN}
        max={UI_SCALE_MAX}
        step={UI_SCALE_STEP}
        ticks
        value={uiScaleState.effective}
        ariaLabel={i18n.t('settings.ui_scale')}
        formatValue={(value) => `${value}%`}
        onchange={(value) => uiScaleState.set(value)}
      />
      {#if uiScaleState.percent}
        <Button variant="ghost" size="sm" onclick={() => uiScaleState.set(0)}>
          {i18n.t('settings.ui_scale_system', { percent: uiScaleState.systemPercent })}
        </Button>
      {/if}
    </div>
  </SettingItem>

  <SettingChoice
    title={i18n.t('settings.layout_mode')}
    description={i18n.t('settings.layout_mode_desc')}
    options={(['auto', 'desktop', 'mobile'] as AppSettings['layout_mode'][]).map((mode) => ({
      value: mode,
      label: i18n.t(`settings.layout_${mode}`)
    }))}
    value={settings.layout_mode || 'auto'}
    onchange={(mode) => void ctl.save('layout_mode', mode)}
  />

  <SettingChoice
    title={i18n.t('settings.toast_position')}
    description={i18n.t('settings.toast_position_desc')}
    options={[
      { value: 'auto', label: i18n.t('settings.toast_auto') },
      { value: 'top-center', label: i18n.t('settings.toast_top_center') },
      { value: 'top-right', label: i18n.t('settings.toast_top_right') },
      { value: 'top-left', label: i18n.t('settings.toast_top_left') },
      { value: 'bottom-center', label: i18n.t('settings.toast_bottom_center') },
      { value: 'bottom-right', label: i18n.t('settings.toast_bottom_right') },
      { value: 'bottom-left', label: i18n.t('settings.toast_bottom_left') }
    ] as { value: NonNullable<AppSettings['toast_position']>; label: string }[]}
    value={settings.toast_position || 'auto'}
    onchange={(position) => void ctl.save('toast_position', position)}
  />
</SettingsGroup>

{#if layoutState.hasKeyboard}
  <SettingsGroup label={i18n.t('settings.group_privacy')}>
    <SettingItem
      title={i18n.t('settings.panic_button')}
      description={i18n.t('settings.panic_button_desc')}
      checked={panicEnabled}
      onCheckedChange={(enabled) => ctl.setPanicKey(panicShortcut, enabled)}
    />
    {#if panicEnabled}
      <SettingItem title={i18n.t('settings.panic_button_shortcut')} description={i18n.t('settings.panic_button_shortcut_desc')}>
        <ShortcutInput
          value={panicShortcut}
          defaultValue="H"
          onchange={(shortcut) => ctl.setPanicKey(shortcut || 'H', panicEnabled)}
        />
      </SettingItem>
    {/if}
  </SettingsGroup>
{/if}

<style>
  .ui-scale {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: flex-end;
    gap: 4px 16px;
    width: 100%;
  }

  .ui-scale > :global(.ui-scale__slider) {
    flex: 1 1 240px;
    min-width: 0;
  }
</style>
