<script lang="ts">
  import { getContext, onMount } from "svelte";

  // note that here line 59 was giving problems "chunk(chunkConfig());"
  // so i added chunk from context because i think that is was missing here, please check since i dont have full scope of project
  // s ljubavlju hesoyam

  interface DocumentMainContext {
    chunk: (config: any) => void;
  }

  const { chunk } = getContext<DocumentMainContext>("documentMain");

  let size = 1000;
  let overlap = 500;

  const chunkConfig = () => {
    return {
      slidingWindow: {
        config: {
          size,
          overlap,
        },
      },
    };
  };

  //this function was not used
  // const embeddingConfig = () => {
  //   return {
  //     input: {
  //       slidingWindow: {
  //         config: {
  //           size,
  //           overlap,
  //         },
  //       },
  //     },
  //   };
  // };

  function setSliders() {
    const sizeSlider = document.getElementById(
      "chunk-size-slider",
    ) as HTMLInputElement;
    const overlapSlider = document.getElementById(
      "chunk-overlap-slider",
    ) as HTMLInputElement;

    size = parseInt(sizeSlider.value);
    overlap = parseInt(overlapSlider.value);

    if (overlap > size) {
      overlap = size;
      overlapSlider.value = sizeSlider.value;
    }
  }

  function _chunk() {
    setSliders();
    chunk(chunkConfig());
  }

  onMount(() => {
    _chunk();
  });
</script>

<div>
  <h2>Sliding window</h2>
  <div class="chunk-config-controller">
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
  </div>
</div>

<style>
  .chunk-config-controller {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
</style>
