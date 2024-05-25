<script>
  // @ts-nocheck

  import { onMount, getContext } from "svelte";
  const { chunk } = getContext("documentMain");

  let size = 1000;
  let overlap = 500;

  let delimiters = ["\n\n", "\n", " ", ""];

  let delimitersString = "";
  let delimiterBuffer = "";

  const config = () => {
    return {
      recursive: {
        config: {
          size,
          overlap,
        },
        delimiters,
      },
    };
  };

  function setSliders() {
    const sizeSlider = document.getElementById("chunk-size-slider");
    const overlapSlider = document.getElementById("chunk-overlap-slider");

    size = parseInt(sizeSlider.value);
    overlap = parseInt(overlapSlider.value);

    if (overlap > size) {
      overlap = size;
      overlapSlider.value = sizeSlider.value;
    }
  }

  export function _chunk() {
    if (delimiterBuffer !== delimitersString) {
      delimitersString = delimiterBuffer;
    }
    setSliders();
    chunk(config());
  }

  function updateDelimiters() {
    delimiters.push(delimiterBuffer);
    delimiters = delimiters.map(el => el.replaceAll('\n', '\\n').replaceAll(' ', '<space>'));
    delimiterBuffer = "";
  }

  onMount(() => {
    _chunk();
  });
</script>

<h2>Recursive</h2>

<label for="chunk-size">Size: {size}</label>
<input
  type="range"
  id="chunk-size-slider"
  name="chunk-size"
  min="1"
  max="2000"
  bind:value={size}
  on:change={_chunk}
/>

<label for="chunk-overlap">Overlap: {overlap}</label>
<input
  type="range"
  id="chunk-overlap-slider"
  name="chunk-overlap"
  min="0"
  max="1000"
  bind:value={overlap}
  on:change={_chunk}
/>

<div>
  Delimiters:
  {#each delimiters as delim}
    <p>{delim}</p>
  {/each}
</div>

<label for="chunk-delimiters">Add delimiter:</label>
<input
  type="text"
  id="chunk-delimiters-input"
  name="chunk-delimiters"
  bind:value={delimiterBuffer}
/>
<button
  on:click={updateDelimiters}>+</button
>

<button on:click={() => _chunk()}>Chunk</button>

<style>
</style>
