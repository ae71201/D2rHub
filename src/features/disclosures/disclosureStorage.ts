import type { OptionalModuleTabId } from "../settings/settingsRegistry";
import { invokeCommand } from "../../platform/tauri";

// Independent of the app version. Bump together with the backend when the notice changes.
export const APPLICATION_DISCLOSURE_REVISION = 1;

const APPLICATION_DISCLOSURE_KEY = "d2rhub-disclosure-accepted-version";
const MODULE_DISCLOSURE_KEY = "d2rhub-disclosure-accepted-modules";

function readAcceptedModules(): OptionalModuleTabId[] {
  try {
    const parsed: unknown = JSON.parse(localStorage.getItem(MODULE_DISCLOSURE_KEY) || "[]");
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((value): value is OptionalModuleTabId => (
      value === "overlays"
      || value === "pet"
      || value === "automation"
      || value === "room-automation"
    ));
  } catch {
    return [];
  }
}

export async function hasAcceptedApplicationDisclosure(): Promise<boolean> {
  const revision = await invokeCommand<number | null>("get_application_disclosure_acceptance");
  if (revision !== null) return revision === APPLICATION_DISCLOSURE_REVISION;
  // Import historical app-version receipts only for the first notice revision.
  // Never let an old WebView receipt acknowledge a future notice change.
  let legacyVersion: string | null = null;
  try {
    legacyVersion = localStorage.getItem(APPLICATION_DISCLOSURE_KEY);
  } catch {
    return false;
  }
  // The version-keyed notice shipped from 0.9.11 through 0.9.103.
  const match = /^0\.9\.(\d+)$/.exec(legacyVersion ?? "");
  if (!match || APPLICATION_DISCLOSURE_REVISION !== 1) return false;
  const patch = Number(match[1]);
  if (patch < 11 || patch > 103) return false;
  await acceptApplicationDisclosure();
  return true;
}

export async function acceptApplicationDisclosure(): Promise<void> {
  await invokeCommand("accept_application_disclosure", { revision: APPLICATION_DISCLOSURE_REVISION });
}

export function hasAcceptedModuleDisclosure(module: OptionalModuleTabId): boolean {
  return readAcceptedModules().includes(module);
}

export function acceptModuleDisclosure(module: OptionalModuleTabId): void {
  try {
    localStorage.setItem(
      MODULE_DISCLOSURE_KEY,
      JSON.stringify(Array.from(new Set([...readAcceptedModules(), module]))),
    );
  } catch {
    // The module can still be added for this session. A later attempt may ask again.
  }
}
