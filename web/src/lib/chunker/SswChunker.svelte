<script lang="ts">
  import { getContext, onMount } from "svelte";
  interface DocumentMainContext {
    chunk: (config: any) => void;
  }

  const { chunk } = getContext<DocumentMainContext>("documentMain");

  let size = 500;
  let overlap = 10;
  let skipF = ["com", "org", "net", "g.", "e.", "js", "rs", "json", "sh"];
  let skipB = ["www", "etc", "e.g", "i.e"];

  let forwardSkip = "";
  let backwardSkip = "";

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

  function updateSkips() {
    if (forwardSkip) {
      skipF = [forwardSkip, ...skipF];
      forwardSkip = "";
    }
    if (backwardSkip) {
      skipB = [backwardSkip, ...skipB];
      backwardSkip = "";
    }
  }

  function removeSkip(s: string, skipType: string) {
    if (!s && !skipType) return;
    if (skipType === "forwardSkip") {
      let removedArr = skipF.filter((x) => x !== s);
      skipF = removedArr;
    } else {
      let removedArr = skipB.filter((x) => x !== s);
      skipB = removedArr;
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
<h4>Forward skip:</h4>
<div class="skip-wrapper">
  {#each skipF as s}
    <button class="tag" on:click={() => removeSkip(s, "forwardSkip")}>
      <p>{s}</p>
    </button>
  {/each}
</div>

<h4>Backwards skip:</h4>

<div class="skip-wrapper">
  {#each skipB as s}
    <button class="tag" on:click={() => removeSkip(s, "backwardSkip")}>
      <p>{s}</p>
    </button>
  {/each}
</div>

<div class="snap-form-wrapper">
  <label for="chunk-size">Size (character based): {size}</label>
  <input
    type="range"
    id="chunk-size-slider"
    name="chunk-size"
    min="1"
    max="1000"
    bind:value={size}
    on:change={_chunk}
  />

  <label for="chunk-overlap">Overlap (sentence based): {overlap}</label>
  <input
    type="range"
    id="chunk-overlap-slider"
    name="chunk-overlap"
    min="0"
    max="20"
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
  <button on:click={updateSkips}>+</button>

  <label for="chunk-skip-b">Add backward:</label>
  <input
    type="text"
    id="chunk-skip-b"
    name="chunk-skip-b"
    bind:value={backwardSkip}
  />
  <button on:click={updateSkips}>+</button>
</div>

<style>
  .skip-wrapper {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }
  .skip-wrapper .tag {
    border: 1px solid white;
    padding: 0.5rem;
    border-radius: 14px;
    display: flex;
    justify-content: center;
    align-items: center;
    flex: 1;
  }

  .skip-wrapper .tag p {
    margin: 0;
    padding: 0;
  }
  .snap-form-wrapper {
    margin-top: 1rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .snap-form-wrapper input {
    height: 2rem;
  }
</style>
