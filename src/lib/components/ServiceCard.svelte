<script lang="ts">
    import type { CardWidth, CardHeight } from "$lib/components/Card/types";
    import Card from "$lib/components/Card/component.svelte";
    import Indicator from "./Indicator.svelte";

    export let imgSrc = "";
    export let title = "";
    export let group = "";
    export let href: string;
    export let width: CardWidth = 1;
    export let height: CardHeight = 1;
    export let ping: Service["ping"] = undefined;
</script>

<Card
    tag="a"
    flex="center"
    column
    animation
    {width}
    {height}
    attributes={{ href, target: "_blank", rel: "noreferrer" }}
>
    {#if ping}
        <Indicator {title} {group} />
    {/if}
    <!-- svelte-ignore a11y-missing-attribute -->
    <img class="object-contain my-3 w-[2rem] h-[2rem]" src={imgSrc} alt={title} />
    <p class="text-center">{title}</p>
</Card>
