import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invokeCommand, listenEvent } from "../../../platform/tauri";
import { useBatchProcessing } from "./useBatchProcessing";
import { batchPlan } from "./batchModel";
import { modState, testAccount, testCatalog } from "./testFixtures";

vi.mock("../../../platform/tauri", () => ({ invokeCommand: vi.fn(), listenEvent: vi.fn() }));
const plain = (name: string) => ({ ...modState.installed_mods[0], name, feature_groups: [], audio_ready: false });
const state = { ...modState, installed_mods: [plain("One"), plain("Two"), plain("Three")] };
const features = { includeAudioTelemetry: false, includeRoomTools: true, includeEscNextGame: false, includeAutoExitOnDeath: false };
beforeEach(() => { vi.mocked(invokeCommand).mockReset(); vi.mocked(listenEvent).mockReset().mockResolvedValue(() => {}); });

describe("batch processing", () => {
  it("refreshes installed outputs before editing another processing plan", async () => {
    let installed = state;
    vi.mocked(invokeCommand).mockImplementation(async command => {
      if (command === "get_audio_mod_setup_state") return installed as never;
      installed = { ...state, installed_mods: [...state.installed_mods, plain("One-Hub")] };
      return installed as never;
    });
    const { result } = renderHook(() => useBatchProcessing([testAccount], testCatalog(), { current: false }, false));
    act(() => result.current.open("CN"));
    await waitFor(() => expect(result.current.inspection).toBeTruthy());
    act(() => result.current.select(["One"]));
    act(() => result.current.preview());
    await act(() => result.current.run());
    act(() => result.current.edit());
    act(() => result.current.preview());
    expect(result.current.rows?.[0].state).toBe("blocked");
  });
  it("preflights occupied and duplicate output names without changing files", () => {
    const rows = batchPlan(state.installed_mods, ["One", "Two", "Three"], "create", features,
      { One: "Two", Two: "New", Three: "new" }, false);
    expect(rows.map(row => row.state)).toEqual(["blocked", "pending", "blocked"]);
    expect(invokeCommand).not.toHaveBeenCalled();
  });
  it("uses the saved catalog edition for a legacy account without a region", async () => {
    vi.mocked(invokeCommand).mockResolvedValue(state as never);
    const { result } = renderHook(() => useBatchProcessing([testAccount], testCatalog(), { current: false }, false));
    act(() => result.current.open("CN"));
    await waitFor(() => expect(result.current.inspection).toBeTruthy());
    expect(result.current.accountId).toBe("one");
  });
  it("rebuilds with each Mod's own modules and blocks missing originals", () => {
    const rows = batchPlan([{ ...plain("Tools"), feature_groups: ["in_game_room_tools"], source_mod_name: "Missing" }],
      ["Tools"], "rebuild", { ...features, includeAudioTelemetry: true }, {}, false);
    expect(rows[0].features.includeAudioTelemetry).toBe(false);
    expect(rows[0].features.includeRoomTools).toBe(true);
    expect(rows[0].state).toBe("blocked");
    const unknown = batchPlan([{ ...plain("Future"), feature_groups: ["future_module"] }], ["Future"], "rebuild", features, {}, false);
    expect(unknown[0].message).toContain("未知模块");
  });
  it("runs sequentially, continues after failures, and retries only failed rows without assigning accounts", async () => {
    let release!: () => void;
    let count = 0;
    vi.mocked(invokeCommand).mockImplementation(async (command, args) => {
      if (command === "get_audio_mod_setup_state") return state as never;
      if (command === "prepare_audio_mod") {
        count++;
        if (count === 1) await new Promise<void>(resolve => { release = resolve; });
        if ((args as { modName: string }).modName === "Two-Hub" && count === 2) throw new Error("source unavailable");
      }
      return {} as never;
    });
    const catalog = testCatalog();
    const lock = { current: false };
    const { result } = renderHook(() => useBatchProcessing([{ ...testAccount, region: "CN" }], catalog, lock, false));
    act(() => result.current.open("CN"));
    await waitFor(() => expect(result.current.inspection).toBeTruthy());
    act(() => result.current.select(["One", "Two", "Three"]));
    act(() => result.current.preview());
    let running!: Promise<void>;
    act(() => { running = result.current.run(); });
    await waitFor(() => expect(count).toBe(1));
    expect(lock.current).toBe(true);
    await act(async () => { release(); await running; });
    expect(result.current.rows?.map(row => row.state)).toEqual(["success", "failed", "success"]);
    await act(() => result.current.run(true));
    expect(count).toBe(4);
    expect(result.current.rows?.every(row => row.state === "success")).toBe(true);
    expect(invokeCommand).not.toHaveBeenCalledWith("apply_audio_mod_to_account", expect.anything());
    expect(lock.current).toBe(false);
  });
  it("finishes the current item and skips unstarted items after stopping", async () => {
    let release!: () => void;
    vi.mocked(invokeCommand).mockImplementation(async command => {
      if (command === "get_audio_mod_setup_state") return state as never;
      await new Promise<void>(resolve => { release = resolve; });
      return {} as never;
    });
    const { result } = renderHook(() => useBatchProcessing([{ ...testAccount, region: "CN" }], testCatalog(), { current: false }, false));
    act(() => result.current.open("CN"));
    await waitFor(() => expect(result.current.inspection).toBeTruthy());
    act(() => result.current.select(["One", "Two"]));
    act(() => result.current.preview());
    let running!: Promise<void>;
    act(() => { running = result.current.run(); });
    await waitFor(() => expect(release).toBeTruthy());
    act(() => result.current.stop());
    await act(async () => { release(); await running; });
    expect(result.current.rows?.map(row => row.state)).toEqual(["success", "skipped"]);
  });
});
