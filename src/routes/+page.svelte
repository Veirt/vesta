<script lang="ts">
    import type { PageData } from "./$types";
    import Group from "$lib/components/Group/component.svelte";
    import Card from "$lib/components/Card/component.svelte";
    import ServiceCard from "$lib/components/ServiceCard.svelte";
    import Loading from "$lib/components/shared/Loading.svelte";

    export let data: PageData;

    const config = data.config;
    const groups = Object.keys(config);

    async function loadWidget(name: string) {
        const component = await import(`$lib/widgets/${name}/widget.svelte`);
        return component.default;
    }
</script>

<div class="container flex justify-center sm:justify-start flex-row h-screen flex-wrap">
    {#each groups as group}
        <Group {...config[group]}>
            {#each config[group].services as serviceProps (serviceProps.title)}
                {#if !serviceProps.widget}
                    <ServiceCard {...serviceProps} {group} />
                {:else}
                    {#await loadWidget(serviceProps.widget.name)}
                        <Card {...serviceProps} flex="center">
                            <Loading />
                        </Card>
                    {:then widget}
                        <svelte:component this={widget} {...serviceProps} {group} />
                    {/await}
                {/if}
            {/each}
        </Group>
    {/each}
</div>
