<script>
  import { getContext, onMount } from "svelte";
  const { apiUrl } = getContext("apiUrl");

  let models = [];
  let collections = [];

  async function getCollections() {
    const res = await fetch(`${apiUrl}/embeddings/collections`);
    collections = await res.json();
  }

  onMount(async () => {
    await getCollections();
    const res = await fetch(`${apiUrl}/embeddings/models`);
    models = await res.json();
  });
</script>

<div>
  <button>Embed</button>
  {#each models as model}
    <li>
      {model}
    </li>
  {/each}
  {#each collections as col}
    <li>
      {col}
    </li>
  {/each}
</div>
