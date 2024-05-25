<script>
  /** @type {any} */
  export let icon = undefined;

  export let color = "#166534D9";

  export let text;
</script>

<div>
  <svelte:component this={icon} {color} />
  {text}
</div>
