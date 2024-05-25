<script>
  // @ts-nocheck

  import { onMount, getContext } from "svelte";
  const { chunk } = getContext("documentMain");

  let size = 1000;
  let overlap = 500;

  const config = () => {
    return {
      slidingWindow: {
        config: {
          size,
          overlap,
        },
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
    setSliders();
    chunk(config());
  }

  onMount(() => {
    _chunk();
  });
</script>

<h2>Sliding window</h2>

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

<button on:click={() => _chunk()}>Chunk</button>

<style>
</style>
