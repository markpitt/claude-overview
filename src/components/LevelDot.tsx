import type { Level } from "../types";

export function LevelDot({ level, size = 10 }: { level: Level; size?: number }) {
  return (
    <span
      className={`dot dot-${level}`}
      style={{ width: size, height: size }}
      aria-hidden="true"
    />
  );
}
