<script lang="ts">
    import type { PageData } from "./$types";
    import Card from "../components/Card.svelte";
    import Category from "../components/Category.svelte";

    export let data: PageData;
    const config = data.config;
    const categories = Object.keys(config);
</script>

<div class="container flex justify-start flex-row h-screen flex-wrap">
    {#each categories as category}
        <Category {...config[category]}>
            {#each config[category].services as serviceProps}
                <Card {...serviceProps} />
            {/each}
        </Category>
    {/each}
</div>
