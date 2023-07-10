<script>
    import Card from "../components/Card.svelte";
    import Category from "../components/Category.svelte";
</script>

<div class="container flex justify-start flex-row h-1/2 flex-wrap">
    <Category columns={2} name="Media">
        <Card
            title="Jellyfin"
            href="http://jellyfin.veirt.moe"
            imgSrc="logo/jellyfin.svg"
        />
        <Card
            title="Sonarr"
            href="http://sonarr.veirt.moe"
            imgSrc="logo/sonarr.png"
        />
        <Card
            title="Prowlarr"
            href="http://prowlarr.veirt.moe"
            imgSrc="logo/prowlarr.png"
        />
        <Card
            title="QBittorrent"
            href="https://qbittorrent.veirt.moe"
            imgSrc="logo/qbittorrent.svg"
        />
    </Category>
</div>
