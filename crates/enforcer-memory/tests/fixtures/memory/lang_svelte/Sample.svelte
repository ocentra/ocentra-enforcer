<script>
    let count = 0;
</script>

{#if count > 0}
    <p>{count}</p>
{/if}

{#each items as item}
    <li>{item}</li>
{/each}

{#await promise}
    <p>Loading...</p>
{/await}
