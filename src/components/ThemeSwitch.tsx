import { chooseTheme, useTheme, type ThemeChoice } from "../lib/theme";

const OPTIONS: { id: ThemeChoice; label: string }[] = [
  { id: "dark", label: "Dark" },
  { id: "light", label: "Light" },
  { id: "system", label: "System" },
];

/** Dark, Light or System, for both windows. */
export function ThemeSwitch() {
  const choice = useTheme();
  return (
    <div className="pill-group" role="group" aria-label="Theme">
      {OPTIONS.map((o) => (
        <button
          key={o.id}
          type="button"
          className={choice === o.id ? "pill active" : "pill"}
          aria-pressed={choice === o.id}
          onClick={() => chooseTheme(o.id)}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}
