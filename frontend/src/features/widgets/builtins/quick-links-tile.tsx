import * as v from "valibot";
import { QuickLinksPayloadSchema, type WidgetData } from "@/features/widgets/widget-schemas";

export function QuickLinksTile({ data }: { data: WidgetData }) {
  const parsed = v.safeParse(QuickLinksPayloadSchema, data);
  const links = parsed.success ? parsed.output.links : [];

  return (
    <div className="space-y-2">
      <div className="space-y-1">
        {links.map((link) => (
          <a
            key={`${link.title}-${link.url}`}
            href={link.url}
            target="_blank"
            rel="noreferrer"
            className="quick-link"
          >
            {link.icon ? (
              <img
                className="service-icon"
                src={link.icon}
                alt=""
                loading="lazy"
                decoding="async"
              />
            ) : (
              <span className="quick-link-icon">
                {link.title.charAt(0).toLocaleUpperCase() || "?"}
              </span>
            )}
            <span className="quick-link-title">{link.title}</span>
            <span className="service-open-mark" aria-hidden="true">
              ↗
            </span>
          </a>
        ))}
      </div>
    </div>
  );
}
