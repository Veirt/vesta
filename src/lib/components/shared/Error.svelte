<script lang="ts">
    type ErrorType = "Service" | "Widget";

    export let details: string = "";
    export let type: ErrorType = "Service";
</script>

{#if type === "Service"}
    <div class="m-auto text-center text-red-500 text-sm">
        <p>Error</p>
    </div>
{:else if type === "Widget"}
    <div class="m-auto text-center bg-red-700 text-slate-50 text-xl p-5 rounded-xl">
        <p>Error Details:</p>
        <p>{details}</p>
    </div>
{/if}
