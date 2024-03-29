<script lang="ts">
    import axios from "axios";
    import { addMinutes, formatTime, formatYYYY_MM_DD } from "$lib/utils/date";
    import type { CalendarEntry, Calendar } from "$lib/widgets/SonarrCalendar/types";
    import Card from "$lib/components/Card/component.svelte";
    import Error from "$lib/components/shared/Error.svelte";
    import { onMount } from "svelte";

    export let group: string;
    export let title: string;
    export let width: CardWidth;
    export let height: CardHeight;
    export let widget: ServiceWidget;

    let currentDate: Date;

    function unaired(calendarEntry: CalendarEntry) {
        return currentDate < new Date(calendarEntry.airDateUtc);
    }

    function onAir(calendarEntry: CalendarEntry) {
        const airDate = new Date(calendarEntry.airDateUtc);
        const airedDate = addMinutes(new Date(calendarEntry.airDateUtc), calendarEntry.series.runtime);

        return currentDate >= airDate && currentDate <= airedDate;
    }

    function missing(calendarEntry: CalendarEntry) {
        const airedDate = addMinutes(new Date(calendarEntry.airDateUtc), calendarEntry.series.runtime);

        return !calendarEntry.hasFile && currentDate > airedDate;
    }

    function downloaded(calendarEntry: CalendarEntry) {
        return calendarEntry.hasFile;
    }

    function downloading(calendarEntry: CalendarEntry) {
        return calendarEntry.downloading;
    }

    function formatAirTime(calendarEntry: CalendarEntry) {
        const airDate = formatTime(calendarEntry.airDateUtc);
        const airedDate = formatTime(addMinutes(new Date(calendarEntry.airDateUtc), calendarEntry.series.runtime));

        return `${airDate} - ${airedDate}`;
    }

    function formatEpisode(calendarEntry: CalendarEntry) {
        const season = calendarEntry.seasonNumber;
        const episode = calendarEntry.episodeNumber;
        const formattedEpisode = episode.toString().padStart(2, "0");

        return `${season}x${formattedEpisode}`;
    }

    function formatSeriesUrl(titleSlug: string) {
        const url = new URL(widget.url!);
        url.pathname = `series/${titleSlug}`;
        return url.toString();
    }

    async function fetchCalendar() {
        const calendar = await axios.get<Calendar>("/api/sonarr", {
            params: { group, title },
        });

        const calendarGroupedByDate = Object.groupBy(calendar.data, (data: CalendarEntry) => {
            const date = new Date(data.airDateUtc);
            const formattedDate = formatYYYY_MM_DD(date);

            return formattedDate;
        });

        return calendarGroupedByDate;
    }

    onMount(() => {
        currentDate = new Date();
    });
</script>

<Card tag="div" column {width} {height} class="overflow-y-auto no-scrollbar">
    {#await fetchCalendar() then calendar}
        {#if Object.keys(calendar).length === 0}
            <div class="flex justify-center items-center min-w-full min-h-full text-xl font-bold">No entry</div>
        {/if}
        {#each Object.keys(calendar) as date}
            <div class="flex justify-center py-2 my-2 min-w-full rounded bg-accent">
                <a href={`${widget.url}/calendar`} class="font-semibold text-center">
                    {date}
                </a>
            </div>
            {#each calendar[date] as calendarEntry}
                <div
                    class="px-2 my-1"
                    class:unaired={unaired(calendarEntry)}
                    class:downloading={downloading(calendarEntry)}
                    class:downloaded={downloaded(calendarEntry)}
                    class:onAir={onAir(calendarEntry)}
                    class:missing={missing(calendarEntry)}
                >
                    <a href={formatSeriesUrl(calendarEntry.series.titleSlug)} class="line-clamp-1 hover:brightness-125">
                        {calendarEntry.series.title}
                    </a>
                    <span class="block text-xs text-slate-400">
                        {formatEpisode(calendarEntry)}
                    </span>
                    <span class="text-xs text-slate-500">
                        {formatAirTime(calendarEntry)}
                    </span>
                </div>
            {/each}
        {/each}
    {:catch err}
        <Error type="Widget" details={err} />
    {/await}
</Card>

<style lang="postcss">
    .unaired {
        @apply border-l-2 border-l-sky-800;
    }

    .onAir {
        @apply border-l-2 border-l-yellow-300;
    }

    .missing {
        @apply border-l-2 border-l-red-500;
    }

    .downloaded {
        @apply border-l-2 border-l-green-700;
    }

    .downloading {
        @apply border-l-2 border-l-violet-900;
    }
</style>
