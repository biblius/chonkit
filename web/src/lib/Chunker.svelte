<script>
  import SwChunker from "./chunker/SwChunker.svelte";
  import SswChunker from "./chunker/SswChunker.svelte";
  import RecChunker from "./chunker/RecChunker.svelte";

  /** @type string */
  export let id;

  /** @type string[] */
  export let chunks;

  const options = [
    { name: "Sliding Window", component: SwChunker },
    { name: "Snapping Window", component: SswChunker },
    { name: "Recursive", component: RecChunker },
  ];

  let selectedChunker = options[0];
</script>

<div id="chunk-container">
  <select bind:value={selectedChunker}>
    {#each options as option}
      <option value={option}>{option.name}</option>
    {/each}
  </select>

  {#if id}
    <svelte:component this={selectedChunker.component} />

    <div id="chunk-container">
      <h3>Chunks</h3>
      {#each chunks as chunk}
        <p class="chunk">{chunk}</p>
      {/each}
    </div>
  {/if}
</div>

<style>
  .chunk {
    font-size: 0.8em;
    word-break: break-all;
  }
</style>
