<script lang="ts">
    export let href: string;
    export let imgSrc: string;
    export let title: string;

    export let width: keyof typeof cardWidth = 1;
    const cardWidth = {
        1: "md:min-w-[7rem] col-span-1",
        2: "md:min-w-[15rem] col-span-2",
        3: "md:min-w-[23rem] col-span-3",
        4: "md:min-w-[31rem] col-span-4",
    };

    const widthClass = cardWidth[width];
</script>

<a
    class="{widthClass}  hover:scale-105 duration-150 text-xs bg-[#202020] rounded flex justify-center items-center flex-col rounded-xl py-3 m-2"
    {href}
    target="_blank"
>
    <!-- svelte-ignore a11y-missing-attribute -->
    <img class="max-w-[2rem] min-h-[2rem] object-contain my-3" src={imgSrc} />
    <p class="text-center">{title}</p>
</a>
