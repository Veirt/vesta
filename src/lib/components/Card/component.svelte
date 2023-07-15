<script lang="ts">
    import { cardWidth, cardHeight } from "./types";
    import type { CardWidth, CardHeight } from "./types";

    export let tag: "a" | "div" = "div";
    export let attributes = {};
    export let flex = "left";
    export let column = false;
    export let row = false;
    export let animation = false;
    export let width: CardWidth = 1;
    export let height: CardHeight = 1;

    const widthClass = cardWidth[width];
    const heightClass = cardHeight[height];
</script>

<svelte:element
    this={tag}
    {...attributes}
    class:animation
    class:column
    class:row
    class="{widthClass} {heightClass} {flex} {$$props.class} text-xs bg-black-2 rounded-xl py-2 m-2"
>
    <slot />
</svelte:element>

<style lang="postcss">
    .left {
        @apply flex justify-start items-start;
    }
    .right {
        @apply flex justify-end items-end;
    }

    .center {
        @apply flex justify-center items-center;
    }

    .between {
        @apply flex justify-between items-center;
    }

    .column {
        @apply flex-col;
    }

    .row {
        @apply flex-row;
    }

    .animation {
        @apply hover:scale-105 duration-150;
    }
</style>
