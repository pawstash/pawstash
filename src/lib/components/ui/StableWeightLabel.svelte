<script lang="ts">
  interface Props {
    text: string;
    reserveWeight?: string;
  }

  let {
    text,
    reserveWeight = 'var(--font-weight-medium)'
  }: Props = $props();
</script>

<span class="stable-weight-label" style:--stable-weight={reserveWeight}>
  <span class="stable-weight-label__content">{text}</span>
  <span class="stable-weight-label__measure" aria-hidden="true">{text}</span>
</span>

<style>
  .stable-weight-label {
    display: inline-grid;
    min-width: max-content;
    vertical-align: top;
  }

  .stable-weight-label__content,
  .stable-weight-label__measure {
    grid-area: 1 / 1;
    white-space: nowrap;
  }

  .stable-weight-label__measure {
    visibility: hidden;
    font-weight: var(--stable-weight);
    pointer-events: none;
    user-select: none;
  }
</style>
