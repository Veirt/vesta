<script lang="ts">
    export let details: string = "";
</script>

<div
    class="m-auto text-center bg-red-700 text-slate-50 text-xl p-5 rounded-xl"
    id="error"
>
    <p>Error Details:</p>
    <p>{details}</p>
</div>
