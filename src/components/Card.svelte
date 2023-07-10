<script lang="ts">
    export let href: string;
    export let imgSrc: string;
    export let title: string;

    export let width: keyof typeof cardWidth = 1;
    const cardWidth = {
        1: "min-w-[7rem] col-span-1",
        2: "min-w-[15rem] col-span-2",
        3: "min-w-[23rem] col-span-3",
        4: "min-w-[31rem] col-span-4",
    };

    const widthClass = cardWidth[width];
</script>

<a
    class="{widthClass}  hover:scale-105 duration-150 div text-sm bg-[#202020] rounded flex justify-center items-center flex-col rounded-xl p-3 m-3"
    {href}
    target="_blank"
>
    <!-- svelte-ignore a11y-missing-attribute -->
    <img class="w-8 my-5" src={imgSrc} />
    <p>{title}</p>
</a>
