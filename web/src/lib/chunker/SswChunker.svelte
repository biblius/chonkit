<script>
  // @ts-nocheck

  import { onMount, getContext } from "svelte";
  const { chunk } = getContext("documentMain");

  let size = 1000;
  let overlap = 500;
  let skipF = ['com', 'org', 'net', 'g.', 'e.', 'js', 'rs', 'json', 'sh']
  let skipB = ['www', 'etc', 'e.g', 'i.e']

  let forwardSkip = '';
  let backwardSkip = '';

  const config = () => {
    return {
      snappingWindow: {
        config: {
          size,
          overlap,
        },
        skip_f: skipF,
        skip_b: skipB,
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

  function updateSkips() {
    if (forwardSkip) {
      skipF = [forwardSkip, ...skipF]
      forwardSkip = '';
    }
    if (backwardSkip) {
      skipB = [backwardSkip, ...skipB]
      backwardSkip = '';
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

<h2>Snapping window</h2>

<div>
  Forward skip:
  {#each skipF as s}
    <p>{s}</p>
  {/each}
</div>

<div>
  Backward skip:
  {#each skipB as s}
    <p>{s}</p>
  {/each}
</div>

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

<label for="chunk-skip-f">Add forward:</label>
<input
  type="text"
  id="chunk-skip-f"
  name="chunk-skip-f"
  bind:value={forwardSkip}
/>
<button
  on:click={updateSkips}>+</button
>

<label for="chunk-skip-b">Add backward:</label>
<input
  type="text"
  id="chunk-skip-b"
  name="chunk-skip-b"
  bind:value={backwardSkip}
/>
<button
  on:click={updateSkips}>+</button
>

<button on:click={() => _chunk()}>Chunk</button>

<style>
</style>
