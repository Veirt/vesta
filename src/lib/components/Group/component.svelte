<script lang="ts">
    import { groupColumn } from "./types";
    import type { GroupColumn } from "./types";

    export let name: string;
    export let columns: GroupColumn = 1;
    export let services: Service[];

    const columnNum = groupColumn[columns].column;
    const width = groupColumn[columns].width;
</script>

<div class="container m-5 {width}">
    <p class=" ml-4 font-bold block">{name}</p>
    <div class="grid {columnNum}">
        <slot />
    </div>
</div>
