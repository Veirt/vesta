<script lang="ts">
    export let imgSrc = "";
    export let title = "";
</script>

<!-- svelte-ignore a11y-missing-attribute -->
<img class="max-w-[2rem] min-h-[2rem] object-contain my-3" src={imgSrc} />
<p class="text-center">{title}</p>
