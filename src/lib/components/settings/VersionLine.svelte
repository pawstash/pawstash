<script lang="ts">
  import { i18n } from '$lib/i18n';
  import { tooltip } from '$lib/motion';
  import { APP_VERSION, BUILD_TIME, COMMIT_HASH } from '$lib/version';

  let { inset = 24 }: { inset?: number } = $props();

  let builtOn = $derived.by(() => {
    const date = new Date(BUILD_TIME);
    if (Number.isNaN(date.getTime())) return BUILD_TIME;
    return i18n.date(date, { day: 'numeric', month: 'short', year: 'numeric', hour: '2-digit', minute: '2-digit' });
  });
</script>

<p class="version-line" style:padding-inline={`${inset}px`} use:tooltip={`${i18n.t('settings.built_on')} ${builtOn} · ${COMMIT_HASH}`}>
  Pawstash {APP_VERSION}
</p>

<style>
  .version-line {
    flex-shrink: 0;
    margin: 0;
    padding: 12px 24px 16px;
    color: var(--text-secondary);
    font-family: var(--font-sans);
    font-size: var(--type-body-sm-size);
    line-height: var(--type-body-sm-line);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    user-select: text;
    contain: inline-size;
  }
</style>
