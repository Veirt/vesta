import { useEffect, useState } from "react";
import { pingService } from "@/shared/api/client";
import type { DashboardService } from "@/shared/schemas/core";
import { layoutStyle } from "@/features/widgets/widget-schemas";

function PingStatus({ id }: { id: string }) {
  const [isUp, setIsUp] = useState<boolean | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    const check = () => {
      void pingService(id, controller.signal)
        .then((up) => {
          if (!controller.signal.aborted) setIsUp(up);
        })
        .catch(() => {
          if (!controller.signal.aborted) setIsUp(false);
        });
    };

    check();
    const interval = window.setInterval(check, 30_000);
    return () => {
      controller.abort();
      window.clearInterval(interval);
    };
  }, [id]);

  return (
    <span
      className="service-ping"
      data-up={isUp === null ? undefined : String(isUp)}
      aria-label={
        isUp === null ? "Checking service status" : isUp ? "Service online" : "Service unavailable"
      }
      role="status"
    >
      <span className="status-dot" aria-hidden="true" />
    </span>
  );
}

export function ServiceCard({
  service,
  editing = false,
}: {
  service: DashboardService;
  editing?: boolean;
}) {
  const content = (
    <>
      {service.icon ? (
        <img className="service-icon" src={service.icon} alt="" loading="lazy" decoding="async" />
      ) : (
        <span className="service-icon-fallback" aria-hidden="true">
          {service.title.charAt(0).toLocaleUpperCase() || "S"}
        </span>
      )}
      <span className="service-copy">
        <span className="service-title">{service.title}</span>
        {service.href && service.show_destination && (
          <span className="service-subtitle">{service.href}</span>
        )}
      </span>
      {service.has_ping ? (
        <PingStatus id={service.id} />
      ) : (
        <span className="service-open-mark" aria-hidden="true">
          ↗
        </span>
      )}
    </>
  );

  const className = `service-card${editing ? " h-full" : " service-layout-item"}`;
  const style = editing ? undefined : layoutStyle(service);
  return !editing && service.href ? (
    <a className={className} style={style} href={service.href} target="_blank" rel="noreferrer">
      {content}
    </a>
  ) : (
    <div className={className} style={style}>
      {content}
    </div>
  );
}
