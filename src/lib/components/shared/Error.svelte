<script lang="ts">
    type ErrorType = "Service" | "Widget";

    export let details: string = "";
    export let type: ErrorType = "Service";
</script>

{#if type === "Service"}
    <div class="m-auto text-sm text-center text-red-500">
        <p>Error</p>
    </div>
{:else if type === "Widget"}
    <div class="p-5 m-auto text-xl text-center bg-red-700 rounded-xl text-slate-50">
        <p>Error Details:</p>
        <p>{details}</p>
    </div>
{/if}
