import * as v from "valibot";
import { WidgetError } from "@/features/widgets/builtins/shared";
import { WeatherPayloadSchema, type WidgetData } from "@/features/widgets/widget-schemas";

export function WeatherTile({ data }: { data: WidgetData }) {
  const parsed = v.safeParse(WeatherPayloadSchema, data);
  if (!parsed.success) return <WidgetError message="Invalid weather payload." />;
  const { latitude, longitude, units, current } = parsed.output;
  const unit = units === "fahrenheit" ? "°F" : "°C";

  return (
    <div className="weather-widget">
      <div className="text-center">
        <div className="widget-secondary">
          {latitude.toFixed(2)}°, {longitude.toFixed(2)}°
        </div>
      </div>
      <div className="flex items-center justify-between">
        <div className="flex-1">
          <div className="weather-temperature widget-value">
            {current.temperature.toFixed(0)}
            {unit}
          </div>
          <div className="widget-secondary">
            Feels {current.apparent_temperature.toFixed(0)}
            {unit}
          </div>
          <div className="weather-description">{current.description}</div>
        </div>
        <div className="shrink-0 text-3xl" aria-hidden="true">
          {current.icon}
        </div>
      </div>
      <div className="weather-details widget-divider">
        <div className="text-center">
          <div className="widget-secondary">Humidity</div>
          <div className="weather-detail-value widget-value">{current.relative_humidity}%</div>
        </div>
        <div className="text-center">
          <div className="widget-secondary">Wind</div>
          <div className="weather-detail-value widget-value">
            {current.wind_speed.toFixed(0)} {current.wind_direction}
          </div>
        </div>
      </div>
    </div>
  );
}
