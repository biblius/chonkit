<script>
  import { onMount } from "svelte";
  import { setContext } from "svelte";
  import SidebarEntry from "./lib/SidebarEntry.svelte";
  import showdown from "showdown";
  import SwChunker from "./lib/chunker/SwChunker.svelte";
  import SswChunker from "./lib/chunker/SswChunker.svelte";
  import RecChunker from "./lib/chunker/RecChunker.svelte";
  import { SvelteToast } from "@zerodevx/svelte-toast";
  import { toast } from "@zerodevx/svelte-toast";

  const baseUrl = import.meta.env.VITE_BASE_URL ?? "";

  const converter = new showdown.Converter({
    ghCodeBlocks: true,
    ghCompatibleHeaderId: true,
    tables: true,
  });

  let id, content, meta;

  const options = [
    { name: "Sliding Window", component: SwChunker },
    { name: "Snapping Window", component: SswChunker },
    { name: "Recursive", component: RecChunker },
  ];

  let selectedChunker = options[0];

  let chunks = [];

  // Set the document ID to the one in the URL
  const pageUrl = window.location.href;
  const documentId = pageUrl.substring(baseUrl.length + 1);

  onMount(() => {
    if (documentId) {
      loadDocument(documentId);
    }
  });

  /**
   * Fetch a document from the backend and display it on the page.
   * @param {?string} docId The UUID of the document
   */
  async function loadDocument(docId) {
    // Prevent loading the same document
    if (id && id === docId) {
      return;
    }

    // Unselect the current doc and select the new one in the sidebar
    selectListItem(docId);

    // Fetch the new document, display popup if not found
    const base = `${baseUrl}/document`;
    const url = docId ? `${base}/${docId}` : base;

    const response = await fetch(url);

    if (response.status === 404) {
      if (docId) {
        toast.push(`Not found`);
      }

      // In case of no index, return the default page
      content = "Chonk knawledge.";
    }

    const data = await response.json();

    // Display the contents
    displayMain(data);

    // Push state to history
    if (docId) {
      let historyUrl = url.replace("/document", "");
      history.pushState(data, "", historyUrl);
    } else {
      history.pushState(data, "", baseUrl);
    }
  }

  /**
   * Set the id, meta and content to the currently selected document
   * @param {{id: string, meta: object, content: string}} documentData
   */
  function displayMain(documentData) {
    id = documentData.meta.id;
    meta = documentData.meta;
    content = converter.makeHtml(documentData.content);
  }

  async function loadSidebar() {
    const res = await fetch(`${baseUrl}/side`);
    const data = await res.json();
    return data;
  }

  /**
   * Unselect the last, then select the currently focused entry in the sidebar
   * @param {string} entryId
   */
  function selectListItem(entryId) {
    const newSelected = document.getElementById(`side_${entryId}`);
    if (newSelected) {
      newSelected.classList.add("sidebar-selected");
    }
  }

  function getCurrentMainId() {
    return id;
  }

  /**
   * Chunk a document using a specific chunker configuration.
   * Mainly called from Chunker components.
   **/
  async function chunk(config) {
    if (!id) {
      return [];
    }

    const url = `${baseUrl}/document/${id}/chunk`;

    try {
      const res = await fetch(url, {
        headers: {
          "content-type": "application/json",
        },
        method: "POST",
        body: JSON.stringify(config),
      });
      chunks = await res.json();
      toast.push("Chonked!");
    } catch (e) {
      console.error("Error in response", e);
      toast.push("There was an error: " + e);
    }
  }

  setContext("baseUrl", { baseUrl });

  setContext("documentMain", {
    loadDocument,
    getCurrentMainId,
    selectListItem,
    chunk,
  });
</script>

<nav>
  <h1>
    <a href="/"> Chonkit! </a>
  </h1>
  {#await loadSidebar()}
    Loading...
  {:then entries}
    <ul>
      {#each entries as entry}
        <SidebarEntry id={entry.id} name={entry.name} isDir={entry.is_dir} />
      {/each}
    </ul>
  {/await}
</nav>
<div class="mega-wrapper">
  <main>
    {#if id}
      {@html content}
      <SvelteToast />
    {:else}
      <h2>Welcome!</h2>
      <p>Select a document to commence the chonkenking.</p>
    {/if}
  </main>

  <aside>
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
  </aside>
</div>

<style>
  nav {
    position: sticky;
    left: 0;
    top: 0;
    margin: 0 0 1rem 0;
    padding: 0 2rem;
    width: 10%;
    height: 100%;
  }

  nav h1 {
    width: 100%;
    margin: 1rem 0;
    font-size: 1.6em;
    text-align: center;
  }

  @media screen and (max-width: 1000px) {
    nav h1 {
      font-size: 1.6em;
    }
  }

  ul {
    list-style-type: none;
    padding: 0;
  }

  .mega-wrapper {
    display: flex;
    flex-direction: row;
    overflow-x: scroll;
    @media screen and (max-width: 1100px) {
      flex-direction: column;
    }
  }

  main {
    padding: 2rem;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 1rem;
    width: 100%;
  }

  aside {
    margin: 1rem 0;
    padding: 2rem;
    width: 100%;
  }

  h1 {
    font-size: 1em;
  }

  .chunk {
    font-size: 0.7em;
    word-break: break-all;
  }
</style>
