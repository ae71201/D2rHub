import { cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState, type ComponentProps } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AccountMeta, GlobalConfig } from "../../../store/types";
import { AutomationPanel } from "./AutomationPanel";
import { ModProcessingPanel } from "./ModProcessingPanel";
import { useModWorkflow } from "../../mods/workflow/useModWorkflow";
import { modState, testCatalog } from "../../mods/workflow/testFixtures";

vi.mock("../../../platform/tauri", () => ({
  invokeCommand: vi.fn(async command => command === "get_audio_mod_setup_state" ? { ...modState, account_id: "leader", installed_mods: [] } : ({ processor: { ready: true, update_available: false } })),
  listenEvent: vi.fn(async () => () => {}),
}));
vi.mock("../../../components/ui/Toast", () => ({ showToast: vi.fn() }));

const account = { id: "leader", display_name: "Leader", initialized: true } as AccountMeta;
const config = { app_language: "zh-CN", rune_audio_enabled: false } as GlobalConfig;

function panelProps(overrides: Partial<ComponentProps<typeof AutomationPanel>> = {}): ComponentProps<typeof AutomationPanel> {
  return {
    config, updateConfig: vi.fn(), persistConfig: vi.fn(async () => config),
    initializedTrackingAccounts: [], trackingTarget: { valid: false, reason: "missing" },
    audioStatus: null, audioModState: null, audioModStateLoading: false, audioPreparing: false,
    hasInitializedAudioAccount: false, hasAudioTarget: false, hasReadyAudioMod: false,
    isAudioEnableRequested: false, isAudioRecognitionActive: false,
    onAudioTargetChange: vi.fn(async () => {}), onAudioToggle: vi.fn(async () => false),
    onToggleDiagnosticRecording: vi.fn(async () => {}), onClose: vi.fn(), onInitializeAccount: vi.fn(),
    ...overrides,
  };
}

function RecognitionFlow({ pendingEnable = false }: { pendingEnable?: boolean }) {
  const [processing, setProcessing] = useState(false);
  const workflow = useModWorkflow({ open: true, active: processing, accounts: [account],
    catalog: testCatalog({ generation: 1, scanned_at: "", capsules: [], accounts: [{ account_id: "leader", account_name: "Leader", edition: "CN", selected_capsule_id: null, legacy_mod_arguments: "", issue: null }] }),
    language: "zh-CN", optionalFeaturesAvailable: true, onApplied: vi.fn(async () => {}),
    onNavigate: origin => setProcessing(origin === "library"),
  });
  return processing ? <ModProcessingPanel workflow={workflow} /> : <AutomationPanel {...panelProps({
    config: { ...config, rune_audio_enabled: pendingEnable }, isAudioEnableRequested: pendingEnable,
    initializedTrackingAccounts: [account], trackingTarget: { valid: true, account },
    hasInitializedAudioAccount: true, hasAudioTarget: true,
    onOpenModProcessing: () => workflow.actions.requestProcessing({ origin: "recognition", accountId: account.id }),
  })} />;
}
afterEach(cleanup);

describe("Recognition onboarding", () => {
  it("renders the new overview, filter summary and Mod picker in English", () => {
    render(<AutomationPanel {...panelProps({ config: { ...config, app_language: "en-US", rune_audio_min_rune_number: 20 },
      initializedTrackingAccounts: [account], trackingTarget: { valid: true, account },
      hasInitializedAudioAccount: true, hasAudioTarget: true,
    })} />);
    expect(screen.getByText("Track drops and run times for the selected account.")).toBeTruthy();
    expect(screen.getByText("Drops to record")).toBeTruthy();
    expect(screen.getByText("7 drop categories · Runes #20+")).toBeTruthy();
    expect(screen.getByRole("combobox", { name: "Choose a Mod" })).toBeTruthy();
    expect(screen.getByRole("option", { name: "Choose a Mod to prepare" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Prepare Mod" })).toBeTruthy();
    expect(screen.queryByText("记录哪些掉落")).toBeNull();
  });
  it("shows enabled-but-waiting state without another enable action and keeps diagnostics collapsed", () => {
    render(<AutomationPanel {...panelProps({ config: { ...config, rune_audio_enabled: true },
      initializedTrackingAccounts: [account], trackingTarget: { valid: true, account },
      hasInitializedAudioAccount: true, hasAudioTarget: true, hasReadyAudioMod: true,
      isAudioEnableRequested: true, isAudioRecognitionActive: false,
    })} />);
    expect(screen.getByText("已启用，等待游戏运行")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "立即开启" })).toBeNull();
    expect(screen.getByText("诊断工具").closest("details")?.open).toBe(false);
    expect(screen.getByText("音频峰值").closest("details")?.open).toBe(false);
    expect(screen.getByText("记录哪些掉落").closest("details")?.open).toBe(false);
    expect(screen.getByText(/记录 7 类掉落/)).toBeTruthy();
  });
  it("shows every prerequisite and opens account initialization when no account exists", async () => {
    const props = panelProps();
    render(<AutomationPanel {...props} />);
    const steps = screen.getByRole("list", { name: "声纹识别启用步骤" });
    expect(within(steps).getAllByRole("listitem").map(step => step.textContent)).toEqual([
      "1初始化账号", "2选择监听账号", "3准备识别 Mod",
    ]);
    await userEvent.click(screen.getByRole("button", { name: "初始化账号" }));
    expect(props.onClose).toHaveBeenCalledTimes(1);
    expect(props.onInitializeAccount).toHaveBeenCalledTimes(1);
    expect(vi.mocked(props.onClose).mock.invocationCallOrder[0]).toBeLessThan(vi.mocked(props.onInitializeAccount).mock.invocationCallOrder[0]);
  });

  it("keeps recognition clickable before prerequisites are ready so it can guide setup", async () => {
    const onAudioToggle = vi.fn(async () => false);
    render(<AutomationPanel {...panelProps({ onAudioToggle })} />);
    const toggle = screen.getByRole("switch", { name: "启用音频声纹自动识别" }) as HTMLButtonElement;
    expect(toggle.disabled).toBe(false);
    expect(screen.getByText("先初始化一个游戏账号")).toBeTruthy();
    await userEvent.click(toggle);
    expect(onAudioToggle).toHaveBeenCalledWith(true);
    expect(toggle.getAttribute("aria-checked")).toBe("false");
    expect(screen.getByRole("button", { name: "初始化账号" })).toBeTruthy();
  });

  it("lets the user select the first initialized account as the next setup step", async () => {
    const onAudioTargetChange = vi.fn(async () => {});
    render(<AutomationPanel {...panelProps({ initializedTrackingAccounts: [account], hasInitializedAudioAccount: true, onAudioTargetChange })} />);
    await userEvent.click(screen.getByRole("button", { name: "选择首个账号" }));
    expect(onAudioTargetChange).toHaveBeenCalledWith("leader");
  });

  it("opens the dedicated processing form and returns to recognition settings", async () => {
    render(<RecognitionFlow />);
    expect(screen.queryByRole("textbox", { name: "新 Mod 名称" })).toBeNull();
    await userEvent.click(screen.getByRole("button", { name: "前往加工" }));
    expect(screen.getByRole("heading", { name: "Mod 加工" })).toBeTruthy();
    expect(screen.getByRole("textbox", { name: "新 Mod 名称" })).toBeTruthy();
    const requiredAudio = screen.getByRole("checkbox", { name: /声纹识别/ }) as HTMLInputElement;
    expect(requiredAudio.checked).toBe(true);
    expect(requiredAudio.disabled).toBe(true);
    await userEvent.click(screen.getByRole("button", { name: "返回识别设置" }));
    expect(screen.getByRole("switch", { name: "启用音频声纹自动识别" })).toBeTruthy();
    expect(screen.queryByRole("textbox", { name: "新 Mod 名称" })).toBeNull();
  });

  it("opens preparation from the readiness action even when enable was already requested", async () => {
    render(<RecognitionFlow pendingEnable />);
    expect(screen.getByText("开启尚未完成：准备识别 Mod")).toBeTruthy();
    expect(screen.getByRole("switch", { name: "启用音频声纹自动识别" }).getAttribute("aria-checked")).toBe("true");
    await userEvent.click(screen.getByRole("button", { name: "开始准备" }));
    expect(screen.getByRole("heading", { name: "Mod 加工" })).toBeTruthy();
    expect(screen.getByRole("textbox", { name: "新 Mod 名称" })).toBeTruthy();
  });
});
