<script>
    import fs from "fs";
    import toml from "toml";
    import Card from "../components/Card.svelte";
    import Category from "../components/Category.svelte";

    const settings = fs.readFileSync("./settings.toml", "utf-8");

    const parsedSettings = toml.parse(settings);
    const categories = Object.keys(parsedSettings);
</script>

<div class="container flex justify-start flex-row h-1/2 flex-wrap">
    {#each categories as category}
        <Category {...parsedSettings[category]}>
            {#each Object.values(parsedSettings[category]).splice(2) as serviceProps}
                <Card {...serviceProps} />
            {/each}
        </Category>
    {/each}
</div>
