import type { ReactNode } from "react";

/** A card title with an optional link top right. */
export function CardHead({ title, children }: { title: string; children?: ReactNode }) {
  return (
    <div className="card-head">
      <h2>{title}</h2>
      {children}
    </div>
  );
}

/** A link that moves to another tab. A button, so it needs no URL. */
export function TabLink({ onClick, children }: { onClick: () => void; children: ReactNode }) {
  return (
    <button type="button" className="link" onClick={onClick}>
      {children}
    </button>
  );
}
