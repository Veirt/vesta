<script lang="ts">
    import Card from "./shared/Card.svelte";

    export let imgSrc = "";
    export let title = "";
    export let href: string;
    export let width: number;
</script>

<Card
    tag="a"
    flex="center"
    column
    animation
    {width}
    attributes={{ href, target: "_blank" }}
>
    <!-- svelte-ignore a11y-missing-attribute -->
    <img class="max-w-[2rem] min-h-[2rem] object-contain my-3" src={imgSrc} />
    <p class="text-center">{title}</p>
</Card>
