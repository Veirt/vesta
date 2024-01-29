<script lang="ts">
    import axios from "axios";
    import Error from "./shared/Error.svelte";

    export let title = "";
    export let group = "";

    async function fetchPing() {
        const pingRes = await axios.get("/api/ping", {
            params: { title, group },
        });

        return pingRes.data;
    }
</script>

{#await fetchPing()}
    <div class="w-2 h-2 visibility-hidden" />
{:then data}
    {#if data.statusCode < 500}
        <div class="self-end mr-4 w-2 h-2 bg-green-500 rounded-full" />
    {:else}
        <div class="self-end mr-4 w-2 h-2 bg-red-500 rounded-full" />
    {/if}
{:catch error}
    <Error type="Service" details={error} />
{/await}
