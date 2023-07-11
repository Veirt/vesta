<script lang="ts">
    import fs from "fs";
    import toml from "toml";
    import Card from "../components/Card.svelte";
    import Category from "../components/Category.svelte";

    interface Service {
        title: string;
        href: string;
        imgSrc: string;
    }

    interface VestaConfig {
        [category: string]: {
            name: string;
            columns: 1 | 2 | 3 | 4;
            services: Service[];
        };
    }

    const config = fs.readFileSync("./config.toml", "utf-8");

    const parsedConfig: VestaConfig = toml.parse(config);
    // console.log(parsedConfig["media"]);
    const categories = Object.keys(parsedConfig);
    console.log(categories);
</script>

<div class="container flex justify-start flex-row h-screen flex-wrap">
    {#each categories as category}
        <Category {...parsedConfig[category]}>
            {#each parsedConfig[category].services as serviceProps}
                <Card {...serviceProps} />
            {/each}
        </Category>
    {/each}
</div>
