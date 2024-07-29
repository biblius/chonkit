<script>
  import { toast } from "@zerodevx/svelte-toast";
  import { getContext, onMount } from "svelte";
  const { apiUrl } = getContext("apiUrl");

  let models = [];
  let collections = [];
  let error = null;

  async function getCollections() {
    try {
      const res = await fetch(`${apiUrl}/embeddings/collections`);
      if (!res.ok) {
        throw new Error("Failed to fetch collections");
      }
      collections = await res.json();
    } catch (e) {
      console.error("Error fetching collections:", e);
      toast.push("Error while embedding: " + e);
    }
  }

  onMount(async () => {
    try {
      await getCollections();
      const res = await fetch(`${apiUrl}/embeddings/models`);
      if (!res.ok) {
        throw new Error("Failed to fetch models");
      }
      models = await res.json();
    } catch (e) {
      console.error("Error fetching models:", e);
      toast.push("Error while embedding: " + e);
    }
  });
</script>

<div>
  <button>Embed</button>
  {#if error}
    <p>{error}</p>
  {/if}
  {#each models as model}
    <li>{model}</li>
  {/each}
  {#each collections as col}
    <li>{col}</li>
  {/each}
</div>
