import { IconServer } from "@tabler/icons-react";
import { Card } from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import type { DashboardNode } from "@/shared/schemas/core";
import { formatAge, formatUptime, nodeAge, nodeStatus } from "@/features/servers/node-health";
import { Status } from "@/features/servers/status";

/** Servers table section; click a row to feature it in the overview cards. */
function NodesTable({
  nodes,
  visibleNodes,
  selectedNodeId,
  now,
  searchText,
  filteredCount,
  onSelectNode,
}: {
  nodes: DashboardNode[];
  visibleNodes: DashboardNode[];
  selectedNodeId: string | undefined;
  now: number;
  searchText: string;
  filteredCount: number;
  onSelectNode: (id: string) => void;
}) {
  return (
    <section id="nodes" aria-labelledby="nodes-heading" className="space-y-3">
      <div className="flex items-center justify-between gap-3">
        <h2 id="nodes-heading" className="text-lg font-semibold tracking-tight">
          Servers
        </h2>
        {searchText && (
          <span className="text-xs text-muted-foreground">{filteredCount} matching results</span>
        )}
      </div>
      <Card className="gap-0 overflow-hidden border-border/70 bg-card/50 py-0 shadow-none">
        <div className="overflow-x-auto">
          <Table>
            <TableHeader>
              <TableRow className="border-border/70 hover:bg-transparent">
                <TableHead className="min-w-44 pl-4">Server</TableHead>
                <TableHead>Status</TableHead>
                <TableHead>Uptime</TableHead>
                <TableHead>CPU</TableHead>
                <TableHead>Memory</TableHead>
                <TableHead>Disk</TableHead>
                <TableHead>Load</TableHead>
                <TableHead className="pr-4">Last seen</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {visibleNodes.map((node) => {
                const nodeStats = node.stats;
                const nodeSeen = nodeAge(node, now);
                const status = nodeStatus(node, now);
                return (
                  <TableRow
                    key={node.id}
                    className={`border-border/60 ${node.id === selectedNodeId ? "bg-accent/30" : ""}`}
                  >
                    <TableCell className="pl-4">
                      <button
                        type="button"
                        className="group flex min-h-10 items-center gap-2.5 text-left focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
                        aria-label={`Show ${node.name} in the overview`}
                        onClick={() => onSelectNode(node.id)}
                      >
                        {node.icon ? (
                          <img
                            className="size-8 shrink-0 rounded-md border border-border bg-muted object-contain p-1"
                            src={node.icon}
                            alt=""
                            loading="lazy"
                            decoding="async"
                          />
                        ) : (
                          <span className="grid size-8 shrink-0 place-items-center rounded-md border border-border bg-muted text-muted-foreground">
                            <IconServer size={17} stroke={1.7} aria-hidden="true" />
                          </span>
                        )}
                        <span className="grid">
                          <span className="font-medium group-hover:text-primary">{node.name}</span>
                          <span className="text-[0.68rem] text-muted-foreground">
                            {node.hostname ? `Hostname · ${node.hostname}` : `ID · ${node.id}`}
                          </span>
                          {node.device_name && (
                            <span className="text-[0.68rem] text-muted-foreground">
                              Device · {node.device_name}
                            </span>
                          )}
                        </span>
                      </button>
                    </TableCell>
                    <TableCell>
                      <Status status={status} />
                    </TableCell>
                    <TableCell className="tabular-nums">
                      {nodeStats ? formatUptime(nodeStats.uptime_secs) : "—"}
                    </TableCell>
                    <TableCell className="tabular-nums">
                      {nodeStats ? `${nodeStats.cpu_usage_percent.toFixed(1)}%` : "—"}
                    </TableCell>
                    <TableCell className="tabular-nums">
                      {nodeStats ? `${nodeStats.memory.usage_percent.toFixed(1)}%` : "—"}
                    </TableCell>
                    <TableCell className="tabular-nums">
                      {nodeStats ? `${nodeStats.disk.usage_percent.toFixed(1)}%` : "—"}
                    </TableCell>
                    <TableCell className="tabular-nums">
                      {nodeStats?.load ? nodeStats.load.one.toFixed(2) : "—"}
                    </TableCell>
                    <TableCell className="pr-4 text-muted-foreground tabular-nums">
                      {formatAge(nodeSeen)}
                    </TableCell>
                  </TableRow>
                );
              })}
              {visibleNodes.length === 0 && (
                <TableRow>
                  <TableCell colSpan={8} className="h-32 text-center text-sm text-muted-foreground">
                    {nodes.length === 0
                      ? "No servers configured. Add a server in Settings to start monitoring."
                      : "No servers match your search."}
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </div>
      </Card>
    </section>
  );
}

export { NodesTable };
