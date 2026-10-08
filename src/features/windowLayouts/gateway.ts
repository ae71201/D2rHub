import { invokeCommand } from "../../platform/tauri";
import type { LayoutMonitor } from "../../store/types";

export interface LayoutRestoreResult {
  applied: string[];
  failures: string[];
}

export const getLayoutMonitors = () => invokeCommand<LayoutMonitor[]>("get_game_layout_monitors");
export const getLayoutAccountOrder = () => invokeCommand<string[]>("get_game_layout_account_order");
export const restoreGameLayout = () => invokeCommand<LayoutRestoreResult>("restore_game_window_layout");
