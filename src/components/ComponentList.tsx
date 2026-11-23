import type { Component } from "../types";
import { componentStatusLabel } from "../lib/format";
import { LevelDot } from "./LevelDot";

export function ComponentList({ components }: { components: Component[] }) {
  return (
    <ul className="components">
      {components.map((c) => (
        <li key={c.id}>
          <LevelDot level={c.level} />
          <span className="name">
            {c.name}
            {c.group && <span className="muted small"> · {c.group}</span>}
          </span>
          <span className={`state state-${c.level}`}>{componentStatusLabel(c.status)}</span>
        </li>
      ))}
    </ul>
  );
}
