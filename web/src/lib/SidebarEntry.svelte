<script>
  import { onMount, getContext } from "svelte";
  import Icon from "./icon/Icon.svelte";
  import MdIcon from "./icon/MarkdownIcon.svelte";
  import DirIcon from "./icon/DirectoryIcon.svelte";

  const { apiUrl } = getContext("apiUrl");

  const { loadDocument, getCurrentMainId, selectListItem } =
    getContext("documentMain");

  export let id;
  export let name;
  export let isDir;
  export let nesting = 0;

  let children = [];
  let loaded = false;

  let open = false;

  /**
   * Open/close a sidebar directory entry
   * @param docId {string}
   */
  function toggle(docId) {
    open = !open;

    if (loaded) {
      return;
    }

    loadSideElement(docId);

    loaded = true;
  }

  /**
   * Fetch and append the children of the target directory
   * @param {string} id
   */
  async function loadSideElement(id) {
    const res = await fetch(`${apiUrl}/files/${id}`);
    const data = await res.json();
    children = data;
  }

  onMount(() => {
    // Always load the root directory elements to save the extra click
    if (nesting === 0) {
      toggle(id);
    }

    if (id === getCurrentMainId()) {
      selectListItem(id);
    }
  });
</script>

<li
  style="margin-left: {nesting}rem;"
  on:click={() => (isDir ? toggle(id) : loadDocument(id))}
>
  <p id={`side_${id}`} class="sidebar-entry">
    {#if name.endsWith(".md")}
      <Icon icon={MdIcon} text={name} />
    {:else}
      <Icon icon={DirIcon} text={name} />
    {/if}
  </p>
</li>

{#if open}
  {#each children as child}
    <svelte:self
      id={child.id}
      name={child.name}
      isDir={child.isDir}
      nesting={nesting + 0.3}
    />
  {/each}
{/if}

<style>
  li {
    position: relative;
    height: fit-content;
    width: fit-content;
  }

  p {
    box-sizing: border-box;
    position: relative;
    text-wrap: wrap;
    padding-left: 0.5rem;
    font-size: 0.7em;
    word-break: break-all;
  }

  p:hover {
    cursor: pointer;
  }
</style>
