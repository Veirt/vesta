export function WidgetError({ message }: { message: string }) {
  return (
    <div className="widget-error" role="alert">
      {message}
    </div>
  );
}

export function StatRow({
  label,
  percent,
  value,
}: {
  label: string;
  percent: number;
  value: string;
}) {
  return (
    <div className="widget-stat">
      <div className="widget-stat-heading">
        <span>{label}</span>
        <span className="widget-value">{value}</span>
      </div>
      <div className="widget-meter">
        <div
          className="widget-meter-fill"
          style={{ width: `${Math.max(0, Math.min(100, percent))}%` }}
        />
      </div>
    </div>
  );
}
