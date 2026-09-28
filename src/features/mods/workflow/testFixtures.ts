import { vi } from "vitest";
import type { AccountMeta, AudioModSetupState, ModCapsulePool } from "../../../store/types";
import type { ModCapsuleController } from "../../modCapsules/useModCapsulePool";
import { describeModDraft, initialModFeatures } from "./model";
import type { ModProcessingDraft } from "./types";
import type { ModWorkflowController } from "./useModWorkflow";

export const testAccount = { id: "one", display_name: "Leader", initialized: true } as AccountMeta;
export const modState: AudioModSetupState = {
  account_id: "one", account_name: "Leader", current_mod_name: null, launch_arguments: "", has_txt: false,
  ready: false, update_required: false, recipe_version: null, required_recipe_version: 22,
  build_mode: null, source_mod_name: null, feature_groups: [], auto_exit_on_death_enabled: false,
  reason_code: "missing", message: "Not prepared", running_pid: null, session_verified: false,
  active_session_ready: null, active_session_update_required: null, restart_required: false,
  installed_mods: [{ name: "MyExistingMod", audio_ready: true, update_required: false, source_eligible: true,
    feature_groups: ["audio_telemetry", "in_game_room_tools"], audio_reusable: true, auto_exit_on_death_enabled: false }],
};
export const modPool: ModCapsulePool = {
  generation: 1, scanned_at: "2026-09-28", capsules: [],
  accounts: [{ account_id: "one", account_name: "Leader", edition: "CN", selected_capsule_id: null, legacy_mod_arguments: "", issue: null }],
};
export function testCatalog(pool = modPool): ModCapsuleController {
  return { pool, loading: false, assigningAccountId: null, error: null,
    refresh: vi.fn(async () => pool), scan: vi.fn(async () => pool), add: vi.fn(async () => pool), update: vi.fn(async () => pool),
    remove: vi.fn(async () => pool), unpackingCapsuleId: null, unpackProgress: null, unpackResult: null, cancelUnpack: vi.fn(async () => {}),
    unpack: vi.fn(async () => ({ pool, backup_path: null, escaped_name_count: 0 })),
    setAutoExitOnDeathEnabled: vi.fn(async () => pool), assign: vi.fn(async () => pool) };
}
export function workflowFixture({ draft: draftOverrides, state = modState, en = false, ...overrides }: {
  draft?: Partial<ModProcessingDraft>; state?: AudioModSetupState; en?: boolean;
} & Partial<Omit<ModWorkflowController, "draft" | "en">> = {}): ModWorkflowController {
  const draft: ModProcessingDraft = { origin: "recognition", accountId: "one", edition: "CN",
    recipe: { kind: "create", source: "MyExistingMod", name: "D2rHubTools" }, features: initialModFeatures("recognition"), ...draftOverrides };
  const analysis = describeModDraft(draft, state, en);
  return { view: "processing", libraryEdition: draft.edition, openAdd: false, draft,
    inspection: { state, loading: false, error: null, scannedAt: 1,
      refresh: vi.fn(async () => state), inspect: vi.fn(async () => state), accept: vi.fn() },
    analysis, blockedReason: analysis.blockedReason, busy: false, progress: null, error: null, notice: null,
    prepared: false, processorReady: true, en, minimalMode: false, accounts: [testAccount], readyCapsules: [], catalog: testCatalog(),
    preparationTask: { currentTask: null, cancel: vi.fn(async () => {}), cancelError: null, cancelling: false },
    actions: { requestProcessing: vi.fn(), openLibrary: vi.fn(), changeRecipe: vi.fn(), changeFeatures: vi.fn(), chooseTarget: vi.fn(),
      prepare: vi.fn(async () => {}), back: vi.fn(), setProcessorReady: vi.fn(), setLibraryEdition: vi.fn(),
      refresh: vi.fn(async () => state), openResources: vi.fn(), resume: vi.fn() }, ...overrides };
}
