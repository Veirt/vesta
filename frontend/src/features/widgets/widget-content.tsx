import {
  AdGuardTile,
  ClockTile,
  JellyfinTile,
  ProwlarrTile,
  QuickLinksTile,
  SonarrCalendarTile,
  SystemStatsTile,
  WeatherTile,
  WidgetError,
} from "@/features/widgets/builtins";
import { PluginWidgetContent } from "@/features/widgets/plugin-iframe";
import { PLUGIN_NAME_PREFIX, type WidgetData } from "@/features/widgets/widget-schemas";

export function WidgetContent({
  name,
  data,
  title,
}: {
  name: string;
  data: WidgetData;
  title?: string;
}) {
  if (name.startsWith(PLUGIN_NAME_PREFIX)) {
    return <PluginWidgetContent name={name} data={data} title={title} />;
  }
  switch (name) {
    case "Clock":
      return <ClockTile data={data} />;
    case "QuickLinks":
      return <QuickLinksTile data={data} />;
    case "SystemStats":
      return <SystemStatsTile data={data} />;
    case "Weather":
      return <WeatherTile data={data} />;
    case "SonarrCalendar":
      return <SonarrCalendarTile data={data} />;
    case "JellyfinRandomShow":
      return <JellyfinTile data={data} />;
    case "Prowlarr":
      return <ProwlarrTile data={data} />;
    case "AdGuard":
      return <AdGuardTile data={data} />;
    default:
      return <WidgetError message={`Unsupported widget: ${name}`} />;
  }
}
