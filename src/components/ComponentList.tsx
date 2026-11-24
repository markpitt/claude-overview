import type { Component } from "../types";
import { componentStatusLabel } from "../lib/format";
import { LevelChip } from "./Chip";

export function ComponentList({ components, compact = false }: { components: Component[]; compact?: boolean }) {
  return (
    <ul className={compact ? "rows components compact" : "rows components"}>
      {components.map((c) => (
        <li key={c.id}>
          <span className="name">
            {c.name}
            {!compact && c.group && <span className="meta"> · {c.group}</span>}
          </span>
          <LevelChip level={c.level}>{componentStatusLabel(c.status)}</LevelChip>
        </li>
      ))}
    </ul>
  );
}
