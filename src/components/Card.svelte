<script lang="ts">
    export let href: string;
    export let imgSrc: string;
    export let title: string;

    export let width: keyof typeof cardWidth = 1;
    const cardWidth = {
        1: "w-32",
        2: "w-64",
    };
    const widthClass = cardWidth[width];
</script>

<a {href} target="_blank">
    <div
        class="{widthClass} hover:scale-105 duration-150 div text-sm bg-[#202020] rounded flex justify-center items-center flex-col p-5 rounded-xl m-3"
    >
        <!-- svelte-ignore a11y-missing-attribute -->
        <img class="w-8 my-5" src={imgSrc} />
        <p>{title}</p>
    </div>
</a>
