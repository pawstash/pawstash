<script lang="ts">
  import IconGlobe from '~icons/fluent/globe-24-filled';
  import IconPatreon from '~icons/simple-icons/patreon';
  import IconPixiv from '~icons/simple-icons/pixiv';
  import IconDiscord from '~icons/simple-icons/discord';
  import IconGumroad from '~icons/simple-icons/gumroad';
  import IconOnlyfans from '~icons/simple-icons/onlyfans';

  interface Props {
    service: string;
    class?: string;
  }

  let { service, class: extraClass = '' }: Props = $props();
  const icons: Record<string, any> = {
    patreon: IconPatreon,
    fanbox: IconPixiv,
    pixiv: IconPixiv,
    discord: IconDiscord,
    gumroad: IconGumroad,
    onlyfans: IconOnlyfans
  };
  let Icon = $derived(icons[service.toLowerCase()] ?? IconGlobe);
</script>

<Icon class={extraClass} />
