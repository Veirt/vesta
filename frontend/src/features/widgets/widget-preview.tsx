import { WidgetContent } from "@/features/widgets/widget-content";
import { previewData } from "@/features/widgets/widget-schemas";

export function WidgetPreview({ name }: { name: string }) {
  const data = previewData(name);
  return (
    <div className="widget-preview">
      <WidgetContent name={name} data={data} title={name} />
      <span className="widget-preview-note">Sample preview</span>
    </div>
  );
}
