import { PluginWidgetPreview } from "@/features/widgets/plugin-iframe";
import { WidgetContent } from "@/features/widgets/widget-content";
import { PLUGIN_NAME_PREFIX, previewData } from "@/features/widgets/widget-schemas";

export function WidgetPreviewBody({ name, title }: { name: string; title?: string }) {
  if (name.startsWith(PLUGIN_NAME_PREFIX)) {
    return <PluginWidgetPreview name={name} title={title} />;
  }
  return <WidgetContent name={name} data={previewData(name)} title={title ?? name} />;
}

export function WidgetPreview({ name }: { name: string }) {
  return (
    <div className="widget-preview">
      <WidgetPreviewBody name={name} />
      <span className="widget-preview-note">Sample preview</span>
    </div>
  );
}
