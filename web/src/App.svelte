<script lang="ts">
  import { SvelteToast, toast } from "@zerodevx/svelte-toast";
  import showdown from "showdown";
  import { onMount, setContext } from "svelte";
  import Chunker from "./lib/Chunker.svelte";
  import EmbeddingConfig from "./lib/EmbeddingConfig.svelte";
  import SidebarEntry from "./lib/SidebarEntry.svelte";
  import * as Types from "./lib/typedefs";

  /** The chonkening API. */
  const apiUrl = (import.meta as any).env.VITE_API_URL ?? "";

  /** The application document url, i.e. browser url. */
  const baseUrl = (import.meta as any).env.VITE_BASE_URL ?? "";

  const converter = new showdown.Converter({
    ghCodeBlocks: true,
    ghCompatibleHeaderId: true,
    tables: true,
  });

  let id, content, meta;

  /** Current document chunks. */
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
    const base = `${apiUrl}/documents`;
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
      let historyUrl = url.replace("/documents", "").replace(apiUrl, baseUrl);
      history.pushState(data, "", historyUrl);
    } else {
      history.pushState(data, "", apiUrl);
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
    const res = await fetch(`${apiUrl}/files`);
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
   * Mainly called from Chunker components to preview chunks.
   * Updates the chunks in this component.
   * @param {
       { slidingWindow: Types.SlidingWindowInput } |
       { snappingWindow: Types.SnappingWindowInput } |
       { recursive: Types.RecursiveInput }
     } config - Chunk configuration.
   * @returns {Promise<void>}
   **/
  async function chunk(config) {
    if (!id) {
      return;
    }

    const url = `${apiUrl}/documents/${id}/chunk`;

    try {
      const res = await fetch(url, {
        headers: {
          "content-type": "application/json",
        },
        method: "POST",
        body: JSON.stringify(config),
      });
      chunks = await res.json();
    } catch (e) {
      console.error("Error in response", e);
      toast.push("Error while chunking: " + e);
    }
  }

  /**
   * Create embeddings for a document.
   * @param {Types.EmbeddingPayload} config - Embedding configuration.
   * @returns {Promise<void>}
   */
  async function embed(config) {
    if (!id) {
      return;
    }

    config.id = id;
    config.collection = "default";

    const url = `${apiUrl}/embeddings`;

    try {
      const res = await fetch(url, {
        headers: {
          "content-type": "application/json",
        },
        method: "POST",
        body: JSON.stringify(config),
      });
      const response = await res.json();
      toast.push(response);
    } catch (e) {
      console.error("Error in response", e);
      toast.push("Error while embedding: " + e);
    }
  }

  setContext("apiUrl", { apiUrl });

  setContext("documentMain", {
    loadDocument,
    getCurrentMainId,
    selectListItem,
    chunk,
    embed,
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
      <p>Select a document to commence the chonkening.</p>
    {/if}
  </main>
  <aside>
    <EmbeddingConfig />
    <Chunker {id} {chunks} />
    <aside></aside>
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

  aside {
    margin: 1rem 0;
    padding: 2rem;
    width: 100%;
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

  h1 {
    font-size: 1em;
  }

  /* .chunk {
    font-size: 0.7em;
    word-break: break-all;
  } */
</style>
