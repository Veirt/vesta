<script lang="ts">
    import type { PageData } from "./$types";
    import Group from "../components/Group.svelte";
    import ServiceCard from "../components/ServiceCard.svelte";

    export let data: PageData;
    const config = data.config;
    const groups = Object.keys(config);

    async function loadWidget(name: string) {
        const component = await import(`../components/widgets/${name}.svelte`);
        return component.default;
    }
</script>

<div class="container flex justify-start flex-row h-screen flex-wrap">
    {#each groups as group}
        <Group {...config[group]}>
            {#each config[group].services as serviceProps}
                {#if !serviceProps.widget}
                    <ServiceCard {...serviceProps} />
                {:else}
                    {#await loadWidget(serviceProps.widget) then widget}
                        <svelte:component this={widget} {...serviceProps} />
                    {/await}
                {/if}
            {/each}
        </Group>
    {/each}
</div>
