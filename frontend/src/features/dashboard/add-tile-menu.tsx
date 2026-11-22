import type { ReactNode } from "react";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { WidgetDescriptor } from "@/shared/schemas/core";

export function AddTileMenu({
  widgets,
  align,
  trigger,
  onAddService,
  onAddWidget,
}: {
  widgets: WidgetDescriptor[];
  align: "start" | "end";
  trigger: ReactNode;
  onAddService: () => void;
  onAddWidget: (name: string) => void;
}) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>{trigger}</DropdownMenuTrigger>
      <DropdownMenuContent align={align}>
        <DropdownMenuItem onSelect={onAddService}>Add service</DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuLabel>Widgets</DropdownMenuLabel>
        {widgets.length === 0 ? (
          <DropdownMenuItem disabled>No widgets available</DropdownMenuItem>
        ) : (
          widgets.map((widget) => (
            <DropdownMenuItem key={widget.name} onSelect={() => onAddWidget(widget.name)}>
              {widget.title}
            </DropdownMenuItem>
          ))
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
