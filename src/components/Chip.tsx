import type { ReactNode } from "react";
import type { Level, Marker } from "../types";

export type Tone = "success" | "warning" | "danger" | "muted" | "plain";

/** A pill. Status chips carry a symbol as well as their colour, so the colour is never the only signal. */
export function Chip({ tone = "plain", symbol, children }: { tone?: Tone; symbol?: string; children: ReactNode }) {
  return (
    <span className={`chip chip-${tone}`}>
      {symbol && <span aria-hidden="true">{symbol} </span>}
      {children}
    </span>
  );
}

export function levelTone(level: Level): Tone {
  switch (level) {
    case "ok":
      return "success";
    case "warn":
      return "warning";
    case "bad":
      return "danger";
    default:
      return "muted";
  }
}

export function levelSymbol(level: Level): string {
  return level === "bad" ? "!" : "●";
}

/** A status chip for an overall or per-service level. */
export function LevelChip({ level, children }: { level: Level; children: ReactNode }) {
  return (
    <Chip tone={levelTone(level)} symbol={levelSymbol(level)}>
      {children}
    </Chip>
  );
}

/** Tone and symbol for why a Claude Code session is waiting. */
export function markerChip(kind: Marker["kind"]): { tone: Tone; symbol: string } {
  switch (kind) {
    case "permission":
    case "question":
      return { tone: "danger", symbol: "!" };
    case "finished":
      return { tone: "success", symbol: "✓" };
    default:
      return { tone: "warning", symbol: "●" };
  }
}
