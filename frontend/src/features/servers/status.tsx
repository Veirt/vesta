import { nodeStatus, statusLabel } from "@/features/servers/node-health";
function Status({ status }: { status: ReturnType<typeof nodeStatus> }) {
  const styles = {
    online: "bg-emerald-400",
    delayed: "bg-amber-400",
    collecting: "bg-muted-foreground",
    unavailable: "bg-red-400",
  };
  return (
    <span className="inline-flex items-center gap-2 whitespace-nowrap text-xs text-muted-foreground">
      <span className={`size-1.5 rounded-full ${styles[status]}`} aria-hidden="true" />
      {statusLabel(status)}
    </span>
  );
}

export { Status };
