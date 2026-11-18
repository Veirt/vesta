import { useEffect, useMemo, useRef, useState } from "react";
import * as v from "valibot";
import {
  buildPluginAssetUrl,
  PluginWidgetPayloadSchema,
  parsePluginWidgetName,
  type WidgetData,
} from "@/features/widgets/widget-schemas";

/**
 * Embeds a plugin's static asset bundle in a sandboxed iframe (opaque origin)
 * and publishes the tile's configuration through the postMessage contract:
 * `{ type: "vesta:config", title, config }`.
 */
export function PluginIframe({
  pluginId,
  entry,
  title,
  config,
}: {
  pluginId: string;
  entry: string;
  title: string;
  config: WidgetData;
}) {
  const frameRef = useRef<HTMLIFrameElement>(null);
  const [frameLoaded, setFrameLoaded] = useState(false);
  const configKey = useMemo(() => JSON.stringify(config ?? {}), [config]);

  useEffect(() => {
    if (!frameLoaded) return;
    frameRef.current?.contentWindow?.postMessage(
      { type: "vesta:config", title, config: JSON.parse(configKey) },
      "*",
    );
  }, [frameLoaded, configKey, title]);

  const src = buildPluginAssetUrl(pluginId, entry);

  return (
    <iframe
      className="plugin-widget-frame"
      ref={frameRef}
      src={src}
      title={`${title} plugin widget`}
      sandbox="allow-scripts allow-forms"
      onLoad={() => setFrameLoaded(true)}
    />
  );
}

export function PluginWidgetContent({
  name,
  data,
  title,
}: {
  name: string;
  data: WidgetData;
  title?: string;
}) {
  const parsedName = parsePluginWidgetName(name);
  const parsedPayload = v.safeParse(PluginWidgetPayloadSchema, data);

  if (!parsedName || !parsedPayload.success) {
    return (
      <div className="widget-error" role="alert">
        This plugin widget has no saved configuration yet. Add it to the dashboard to render.
      </div>
    );
  }

  return (
    <PluginIframe
      pluginId={parsedName.pluginId}
      entry={parsedPayload.output.entry}
      title={title ?? parsedName.widgetId}
      config={parsedPayload.output.config}
    />
  );
}
