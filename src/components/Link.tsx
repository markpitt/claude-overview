import type { ReactNode } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";

/** Opens in the system browser, never inside the app's webview. */
export function Link({ href, children }: { href: string; children: ReactNode }) {
  return (
    <a
      href={href}
      onClick={(e) => {
        e.preventDefault();
        void openUrl(href);
      }}
    >
      {children}
    </a>
  );
}
