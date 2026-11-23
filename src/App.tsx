import { getCurrentWindow } from "@tauri-apps/api/window";
import { MainWindow } from "./views/MainWindow";
import { Popover } from "./views/Popover";

// One bundle serves both windows; the window label picks the view.
const label = getCurrentWindow().label;

export default function App() {
  return label === "popover" ? <Popover /> : <MainWindow />;
}
