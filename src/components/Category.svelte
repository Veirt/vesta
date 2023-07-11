<script lang="ts">
    export let name: string;

    const categoryClass = {
        1: { column: "grid-cols-1", width: "w-[8rem]" },
        2: { column: "grid-cols-2", width: "w-[16rem]" },
        3: { column: "grid-cols-3", width: "w-[24rem]" },
        4: { column: "grid-cols-4", width: "w-[32rem]" },
    };
    export let columns: keyof typeof categoryClass = 1;

    const columnNum = categoryClass[columns].column;
    const width = categoryClass[columns].width;
</script>

<div class="container m-5 {width}">
    <p class=" ml-4 font-bold block">{name}</p>
    <div class="grid {columnNum}">
        <slot />
    </div>
</div>
