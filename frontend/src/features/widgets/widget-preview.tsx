import { PluginWidgetPreview } from "@/features/widgets/plugin-iframe";
import { WidgetContent } from "@/features/widgets/widget-content";
import { PLUGIN_NAME_PREFIX, previewData } from "@/features/widgets/widget-schemas";

export function WidgetPreview({ name }: { name: string }) {
  const data = previewData(name);
  return (
    <div className="widget-preview">
      {name.startsWith(PLUGIN_NAME_PREFIX) ? (
        <PluginWidgetPreview name={name} />
      ) : (
        <WidgetContent name={name} data={data} title={name} />
      )}
      <span className="widget-preview-note">Sample preview</span>
    </div>
  );
}
