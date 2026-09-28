import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { useGlobalConfig } from "../../store/globalConfig";
import type { AccountMeta, GlobalConfig, ModCapsulePool } from "../../store/types";
import type { ModCapsuleController } from "../modCapsules/useModCapsulePool";
import { roomAutomationGateway } from "../roomAutomation/gateway";
import { useModFeatureCoordination } from "./useModFeatureCoordination";

const accounts = [{ id: "main", display_name: "主号", initialized: true, mod_args: "-mod Rooms -txt" }] as AccountMeta[];
const pool: ModCapsulePool = {
  generation: 1,
  scanned_at: "2026-09-02T00:00:00+08:00",
  capsules: [{
    id: "scan:cn:rooms", edition: "CN", name: "Rooms", origin: "scanned",
    launch_arguments: "-mod Rooms -txt", default_launch_arguments: "-mod Rooms -txt -assettestmode 1",
    feature_groups: ["in_game_room_tools"], processed: true, source_eligible: true,
    update_required: false, ready: true, deletable: false, assigned_account_ids: ["main"],
  }],
  accounts: [{ account_id: "main", account_name: "主号", edition: "CN", selected_capsule_id: "scan:cn:rooms", legacy_mod_arguments: "-mod Rooms -txt", issue: null }],
};

function catalog(assign = vi.fn(async () => pool)): ModCapsuleController {
  return {
    pool, loading: false, assigningAccountId: null, error: null,
    refresh: vi.fn(async () => pool), scan: vi.fn(async () => pool),
    add: vi.fn(async () => pool), update: vi.fn(async () => pool), remove: vi.fn(async () => pool),
    unpackingCapsuleId: null, unpackProgress: null, unpackResult: null, cancelUnpack: vi.fn(async () => {}),
    unpack: vi.fn(async () => ({ pool, backup_path: null, escaped_name_count: 0 })),
    setAutoExitOnDeathEnabled: vi.fn(async () => pool), assign,
  };
}

afterEach(() => vi.restoreAllMocks());

describe("shared Mod feature coordination", () => {
  it("opens a reviewable recognition setup when the selected account has no Mod", async () => {
    const openProcessing = vi.fn();
    const modCatalog = catalog();
    modCatalog.pool = { ...pool, accounts: [{ ...pool.accounts[0], selected_capsule_id: null }] };
    const toggleAudio = vi.fn();
    const { result } = renderHook(() => useModFeatureCoordination({
      accounts, trackingTargetId: "main", modCatalog, toggleAudio, openProcessing, onGlobalCommitted: vi.fn(),
    }));
    await act(() => result.current.toggleRecognition(true));
    expect(openProcessing).toHaveBeenCalledWith({ origin: "recognition", accountId: "main" });
    expect(modCatalog.assign).not.toHaveBeenCalled();
    expect(toggleAudio).not.toHaveBeenCalled();
  });
  it("preserves the selected recognition account and requests review without assigning or starting processing", async () => {
    const readRoom = vi.spyOn(roomAutomationGateway, "getConfig").mockResolvedValue({
      schema_version: 1, generation: 1,
      config: { enabled: true, primary_account_id: "another-room-primary" } as never,
      normalization: {} as never,
    });
    const assign = vi.fn(async () => pool);
    const toggleAudio = vi.fn(async () => undefined);
    const openProcessing = vi.fn();
    const { result } = renderHook(() => useModFeatureCoordination({
      accounts, trackingTargetId: "main", modCatalog: catalog(assign), toggleAudio,
      openProcessing, onGlobalCommitted: vi.fn(),
    }));

    await act(() => result.current.toggleRecognition(true));
    expect(assign).not.toHaveBeenCalled();
    expect(openProcessing).toHaveBeenCalledWith({ accountId: "main", edition: "CN", origin: "recognition", autoStart: false,
      source: { name: "Rooms", processed: true } });
    expect(toggleAudio).not.toHaveBeenCalled();
    expect(readRoom).not.toHaveBeenCalled();
  });

  it("saves room participants as a launch scheme with explicit Mods and inherited other settings", async () => {
    const current = { launch_groups: [] } as unknown as GlobalConfig;
    const saved = { ...current, launch_groups: [] } as GlobalConfig;
    const patch = vi.fn(async (value) => ({ ...saved, ...value }));
    useGlobalConfig.setState({ config: current, patch } as never);
    const { result } = renderHook(() => useModFeatureCoordination({
      accounts, trackingTargetId: "main", modCatalog: catalog(), toggleAudio: vi.fn(),
      openProcessing: vi.fn(), onGlobalCommitted: vi.fn(),
    }));

    await act(() => result.current.saveRoomLaunchScheme(["main"]));
    const group = patch.mock.calls[0][0].launch_groups[0];
    expect(group.account_ids).toEqual(["main"]);
    expect(group.members[0]).toMatchObject({
      account_id: "main", mod_args: "-mod Rooms -txt",
      position_configured: false, graphics_configured: false,
    });
  });
});
