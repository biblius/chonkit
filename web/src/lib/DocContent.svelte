<script>
  export let content;
  export let meta;
</script>

<svelte:head>
  <title>{meta?.name || "Chonkit"}</title>
</svelte:head>

<section id="markdown-content" class="markdown-body">
  {@html content}
</section>

<style>
  @import "../markdown.css";
</style>
