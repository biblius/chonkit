<script>
  /** @type {any} */
  export let icon = undefined;

  export let color = "#166534D9";

  export let text;
</script>

<div class="container">
  <div>
    <svelte:component this={icon} {color} />
  </div>
  <p>
    {text}
  </p>
</div>

<style scoped>
  .container {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 0.5rem;
  }
  p {
    margin: 0;
    padding: 0;
    text-wrap: wrap;
    font-size: clamp(1rem, 0.126vw + 0.975rem, 1.2rem);
    word-break: break-all;
    color: white;
    transition: all ease-in-out 300ms;
  }

  p:hover {
    cursor: pointer;
    color: #646cff;
  }
</style>
