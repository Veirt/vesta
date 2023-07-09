<script>
    import Card from "../components/Card.svelte";
</script>

<div class="container flex justify-center items-center h-1/2 flex-wrap">
    <Card
        title="Jellyfin"
        href="http://jellyfin.veirt.moe"
        imgSrc="logo/jellyfin.svg"
    />
    <Card
        title="Sonarr"
        href="http://sonarr.veirt.moe"
        imgSrc="logo/sonarr.png"
        width={2}
    />
    <Card
        title="Prowlarr"
        href="http://prowlarr.veirt.moe"
        imgSrc="logo/prowlarr.png"
    />
</div>

