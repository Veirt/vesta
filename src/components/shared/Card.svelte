<script lang="ts">
    export let tag: "a" | "div" = "div";
    export let attributes = {};

    const cardWidth = {
        1: "md:min-w-[7rem] col-span-1",
        2: "md:min-w-[15rem] col-span-2",
        3: "md:min-w-[23rem] col-span-3",
        4: "md:min-w-[31rem] col-span-4",
    };
    export let width: keyof typeof cardWidth = 1;
    const widthClass = cardWidth[width];

    const cardHeight = {
        1: "min-h-[7rem] max-h-[7rem] row-span-1",
        2: "min-h-[15rem] max-h-[15rem] row-span-2",
        3: "min-h-[23rem] max-h-[23rem] row-span-3",
        4: "min-h-[31rem] max-h-[31rem] row-span-4",
    };
    export let height: keyof typeof cardHeight = 1;
    const heightClass = cardHeight[height];

    export let flex = "left";
    export let column = false;
    export let row = false;
    export let animation = false;
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
