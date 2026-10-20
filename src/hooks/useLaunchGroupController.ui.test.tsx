import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useLaunchGroupController } from "./useLaunchGroupController";
import { useAccounts } from "../store/accounts";
import { useGlobalConfig } from "../store/globalConfig";
import { useLaunch } from "../store/launch";
import type { AccountMeta, GlobalConfig, LaunchGroup, SchemeEnvironment } from "../store/types";
import { invokeCommand } from "../platform/tauri";

vi.mock("../platform/tauri", () => ({ invokeCommand: vi.fn(), listenEvent: vi.fn(), emitEvent: vi.fn() }));
vi.mock("../components/ui/Toast", () => ({ showToast: vi.fn() }));

const environment: SchemeEnvironment = {
  settings: { rune_audio_enabled: false, rune_audio_target_account: "", rune_audio_external_target: null,
    rune_audio_tracked_categories: ["runes"], enable_stats_overlay: false, enable_tz_overlay: false, auto_close_browser: false },
  room: { enabled: false, primary_account_id: "", follower_account_ids: [], auto_followers_enabled: false,
    auto_followers_delay_secs: 3, shortcut: "F9", join_shortcut: "F10", name_prefix: "run", password: "",
    next_sequence: 1, sequence_width: 3, background_text_strategy: "post_keys", strategy_version: 26,
    flow: { step_delay_ms: 50, character_delay_ms: 50 } },
  layout: null,
};
const accounts = ["a", "b"].map((id, order) => ({ id, display_name: id, initialized: true, order, region: "KR", auth_mode: "token", mod_args: "", position_presets: [] })) as unknown as AccountMeta[];
const scheme: LaunchGroup = { id: "one", name: "三开", revision: 4, account_ids: ["b", "a"],
  environment, members: accounts.slice().reverse().map(account => ({ account_id: account.id, mod_args: "", graphics_configured: true, resolution: "1280x720", fps: 30,
    snapshot: { game_settings: { Quality: 2 }, position: null, region: "KR", language: null, voicelanguage: null } })) };
const mutatingCalls = () => vi.mocked(invokeCommand).mock.calls.filter(([command]) => !command.startsWith("get_"));

beforeEach(() => {
  vi.clearAllMocks();
  useAccounts.setState({ accounts: structuredClone(accounts) });
  useGlobalConfig.setState({ saving: false, config: { launch_groups: [structuredClone(scheme)] } as GlobalConfig, load: vi.fn(async () => {}) });
  useLaunch.setState({ startSavedScheme: vi.fn(async () => {}) });
  vi.mocked(invokeCommand).mockImplementation(async command => {
    if (command === "get_launch_scheme_defaults") return structuredClone(environment) as never;
    if (command === "get_account_settings") return { "Screen Resolution (Windowed)": "1920x1080", "Framerate Cap": 60, Quality: 3 } as never;
    return undefined as never;
  });
});
afterEach(cleanup);

describe("independent launch scheme drafts", () => {
  it("cancelling nested edits never writes the scheme, account or current settings", async () => {
    const before = structuredClone(useGlobalConfig.getState().config);
    const { result } = renderHook(() => useLaunchGroupController());
    await act(async () => result.current.edit(scheme));
    act(() => result.current.updateEnvironment(env => { env.room.password = "draft"; env.settings.rune_audio_tracked_categories.push("gems"); }));
    act(() => result.current.updateMember("a", { fps: 144 }));
    expect(environment.room.password).toBe("");
    expect(useGlobalConfig.getState().config).toEqual(before);
    act(() => result.current.cancelDraft());
    expect(result.current.draft).toBeNull();
    expect(mutatingCalls()).toEqual([]);
  });

  it("saves only through the scheme endpoint with its original revision and own order", async () => {
    const { result } = renderHook(() => useLaunchGroupController());
    await act(async () => result.current.edit(scheme));
    act(() => result.current.updateMember("b", { fps: 144 }));
    await act(async () => result.current.saveDraft());
    expect(invokeCommand).toHaveBeenCalledWith("save_launch_scheme", expect.objectContaining({ expectedRevision: 4,
      scheme: expect.objectContaining({ account_ids: ["b", "a"], members: [expect.objectContaining({ account_id: "b", fps: 144 }), expect.objectContaining({ account_id: "a", fps: 30 })] }) }));
    expect(useGlobalConfig.getState().config?.launch_groups[0]).toEqual(scheme);
  });

  it("copies game settings when adding an account and keeps a duplicate independent", async () => {
    const { result } = renderHook(() => useLaunchGroupController());
    await act(async () => result.current.create());
    act(() => result.current.toggleAccount("a"));
    await waitFor(() => expect(result.current.draft?.members[0].snapshot).toBeTruthy());
    expect(result.current.draft?.members[0]).toMatchObject({ resolution: "1920x1080", fps: 60, snapshot: { game_settings: { Quality: 3 } } });
    await act(async () => result.current.edit(scheme));
    act(() => result.current.duplicate());
    expect(result.current.draft?.id).toBeNull();
    expect(result.current.draft?.revision).toBeNull();
    act(() => result.current.updateEnvironment(env => { env.room.next_sequence = 42; }));
    expect(scheme.environment?.room.next_sequence).toBe(1);
  });

  it("launches by saved identity and never projects settings from the frontend", () => {
    const { result } = renderHook(() => useLaunchGroupController());
    act(() => result.current.launch(scheme));
    expect(useLaunch.getState().startSavedScheme).toHaveBeenCalledWith("one", 4);
    expect(mutatingCalls()).toEqual([]);
  });

  it("requires explicit editing for an old scheme without assigning global module settings", async () => {
    const { result } = renderHook(() => useLaunchGroupController());
    await act(async () => result.current.launch({ ...scheme, environment: null }));
    expect(useLaunch.getState().startSavedScheme).not.toHaveBeenCalled();
    expect(result.current.draft?.environment.room.enabled).toBe(false);
    expect(mutatingCalls()).toEqual([]);
  });
});
