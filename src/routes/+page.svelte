<script lang="ts">
    import type { PageData } from "./$types";
    import Card from "../components/shared/Card.svelte";
    import Category from "../components/Category.svelte";
    import Service from "../components/Service.svelte";

    export let data: PageData;
    const config = data.config;
    const categories = Object.keys(config);
</script>

<div class="container flex justify-start flex-row h-screen flex-wrap">
    {#each categories as category}
        <Category {...config[category]}>
            {#each config[category].services as serviceProps}
                <Card
                    tag="a"
                    flex="center"
                    column
                    animation
                    attributes={{ href: serviceProps.href, target: "_blank" }}
                    width={serviceProps.width}
                >
                    <Service {...serviceProps} />
                </Card>
            {/each}
        </Category>
    {/each}
</div>
