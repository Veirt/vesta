<script lang="ts">
    import type { PageData } from "./$types";
    import Group from "../components/Group.svelte";
    import ServiceCard from "../components/ServiceCard.svelte";

    export let data: PageData;
    const config = data.config;
    const groups = Object.keys(config);
</script>

<div class="container flex justify-start flex-row h-screen flex-wrap">
    {#each groups as group}
        <Group {...config[group]}>
            {#each config[group].services as serviceProps}
                <ServiceCard {...serviceProps} />
            {/each}
        </Group>
    {/each}
</div>
