import type { CSSProperties } from "react";
import type { DashboardItem as DashboardItemData } from "@/shared/schemas/core";
import { ServiceCard, WidgetCard } from "@/features/dashboard/tile-renderers";
import type { DashboardGroupSection } from "@/shared/lib/layout";

/** Read-only dashboard sections: group heading plus a grid of tiles. */
function DashboardViewGroups({
  sections,
  query,
}: {
  sections: DashboardGroupSection<DashboardItemData>[];
  query: string;
}) {
  return (
    <div className={sections.some((section) => section.label) ? "dashboard-groups" : undefined}>
      {sections.map((section) => (
        <section
          className="dashboard-view-group"
          key={section.id}
          style={{ "--group-span": section.width } as CSSProperties}
        >
          {section.label && (
            <div className="dashboard-group-heading-row">
              <div className="dashboard-group-heading-copy">
                {section.icon && <img className="dashboard-group-icon" src={section.icon} alt="" />}
                <div className="min-w-0">
                  <h3 className="dashboard-group-heading">{section.label}</h3>
                  {section.description && (
                    <p className="dashboard-group-description">{section.description}</p>
                  )}
                </div>
              </div>
            </div>
          )}
          {section.items.length === 0 ? (
            <p className="dashboard-group-empty">
              {query
                ? "No tiles in this section match the search."
                : "No tiles in this section yet."}
            </p>
          ) : (
            <div className={`dashboard-grid${query ? " dashboard-grid-search" : ""}`}>
              {section.items.map((item) =>
                item.kind === "service" ? (
                  <ServiceCard key={item.id} service={item} />
                ) : (
                  <WidgetCard key={item.id} widget={item} />
                ),
              )}
            </div>
          )}
        </section>
      ))}
    </div>
  );
}

export { DashboardViewGroups };
