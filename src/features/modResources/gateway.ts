import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { invokeCommand } from "../../platform/tauri";
import type { ModResourceState } from "./types";

export const modResourcesGateway = {
  read: (edition: string, refresh = false) =>
    invokeCommand<ModResourceState>("get_mod_resources", { edition, refresh }),
  install: (edition: string, resourceId: string, localFile: string | null) =>
    invokeCommand<{ path: string }>("install_mod_resource", { edition, resourceId, localFile }),
  chooseFile: async () => {
    const selected = await openFileDialog({
      multiple: false,
      directory: false,
      filters: [{ name: "ZIP", extensions: ["zip"] }],
    });
    return typeof selected === "string" ? selected : null;
  },
  openFolder: (edition: string) => invokeCommand("open_mods_directory", { edition }),
  openExternal: async (url: string) => {
    const { open } = await import("@tauri-apps/plugin-shell");
    await open(url);
  },
};
