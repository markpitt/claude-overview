import type { ReactNode } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";

/** Opens in the system browser, never inside the app's webview. */
export function Link({ href, children, className }: { href: string; children: ReactNode; className?: string }) {
  return (
    <a
      href={href}
      className={className}
      onClick={(e) => {
        e.preventDefault();
        void openUrl(href);
      }}
    >
      {children}
    </a>
  );
}
