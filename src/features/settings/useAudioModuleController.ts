import { useEffect, useRef, useState } from "react";
import { showToast } from "../../components/ui/Toast";
import { invokeCommand } from "../../platform/tauri";
import type { AccountMeta, GlobalConfig } from "../../store/types";
import { useModInspection } from "../mods/workflow/useModInspection";
import type { ModAppliedResult, ModProcessingRequest } from "../mods/workflow/types";
import type { RuneAudioStatus } from "./audioModuleModel";
import { hasAudioTelemetry } from "../mods/featureContract";
import type { SettingsTabId } from "./settingsRegistry";

interface AudioModuleControllerOptions {
  open: boolean;
  activeTab: SettingsTabId;
  config: GlobalConfig | null;
  initializedAccounts: AccountMeta[];
  trackingTargetId: string;
  updateConfig: (updater: (config: GlobalConfig) => void) => void;
  persistConfig: (draft: GlobalConfig, quiet?: boolean) => Promise<unknown>;
  requestProcessing: (request: ModProcessingRequest) => void;
  optionalFeaturesAvailable?: boolean;
}

/** Recognition runtime and preferences only. Mod recipes belong to the Mod workflow. */
export function useAudioModuleController(options: AudioModuleControllerOptions) {
  const { open, activeTab, config, initializedAccounts, trackingTargetId, optionalFeaturesAvailable = true } = options;
  const latest = useRef(options);
  latest.current = options;
  const active = open && activeTab === "automation" && optionalFeaturesAvailable;
  const target = initializedAccounts.find(account => account.id === trackingTargetId);
  const inspectionIdentity = JSON.stringify([target?.mod_args, target?.is_running, target?.running_pid]);
  const inspection = useModInspection(active, trackingTargetId, inspectionIdentity);
  const [audioStatus, setAudioStatus] = useState<RuneAudioStatus | null>(null);
  const [changing, setChanging] = useState(false);
  const requestRevision = useRef(0);
  const hasReadyAudioMod = !!trackingTargetId && inspection.state?.account_id === trackingTargetId && inspection.state.ready;

  useEffect(() => {
    if (!active) return;
    let disposed = false;
    let timer: number | undefined;
    const poll = async () => {
      try {
        const status = await invokeCommand<RuneAudioStatus>("get_rune_audio_status");
        if (!disposed) setAudioStatus(status);
      } catch (error) { console.warn("读取音频遥测状态失败", error); }
      if (!disposed) timer = window.setTimeout(poll, 1000);
    };
    void poll();
    return () => { disposed = true; if (timer !== undefined) window.clearTimeout(timer); };
  }, [active]);

  const persistAudioEnabledState = async (accountId: string, enabled: boolean) => {
    const current = latest.current.config;
    if (!current) throw new Error("识别设置尚未加载");
    const next = { ...current, rune_audio_target_account: accountId, rune_audio_enabled: enabled };
    latest.current.updateConfig(draft => { draft.rune_audio_target_account = accountId; draft.rune_audio_enabled = enabled; });
    if (!(await latest.current.persistConfig(next, true))) throw new Error("识别设置未能保存，请重试");
  };

  const handleAudioTargetChange = async (accountId: string) => {
    const revision = ++requestRevision.current;
    const wasEnabled = !!latest.current.config?.rune_audio_enabled;
    latest.current.updateConfig(next => { next.rune_audio_target_account = accountId; next.rune_audio_enabled = false; });
    setChanging(true);
    try {
      const state = await inspection.inspect(accountId, true);
      if (revision !== requestRevision.current) return;
      inspection.accept(state);
      if (wasEnabled && state.ready) await persistAudioEnabledState(accountId, true);
      else if (wasEnabled) latest.current.requestProcessing({ origin: "recognition", accountId });
    } catch (error) {
      if (revision === requestRevision.current) showToast("error", `无法检查账号的识别 Mod：${error}`);
    } finally { if (revision === requestRevision.current) setChanging(false); }
  };

  const handleAudioToggle = async (enabled: boolean, preferredAccountId?: string) => {
    const revision = ++requestRevision.current;
    if (!enabled) {
      await persistAudioEnabledState(latest.current.trackingTargetId, false);
      await invokeCommand("stop_rune_audio_monitor").catch(() => undefined);
      return;
    }
    const accountId = preferredAccountId || latest.current.trackingTargetId;
    if (!accountId) { showToast("warning", latest.current.initializedAccounts.length ? "请先选择监听账号" : "请先初始化一个账号"); return; }
    setChanging(true);
    try {
      const state = await inspection.inspect(accountId, true);
      if (revision !== requestRevision.current) return;
      inspection.accept(state);
      if (state.ready) {
        await persistAudioEnabledState(accountId, true);
        if (state.update_required) showToast("warning", "旧版识别 Mod 仍可使用；建议更新以获得即时恐怖区域识别");
        if (state.running_pid && state.active_session_ready === true) await invokeCommand("start_rune_audio_monitor").catch(() => undefined);
        else if (state.restart_required) showToast("warning", "设置已生效，请重启该账号的游戏后开始识别");
      } else {
        latest.current.updateConfig(next => { next.rune_audio_target_account = accountId; next.rune_audio_enabled = false; });
        latest.current.requestProcessing({ origin: "recognition", accountId });
      }
    } catch (error) {
      if (revision === requestRevision.current) showToast("error", `无法开启声纹识别：${error}`);
    } finally { if (revision === requestRevision.current) setChanging(false); }
  };

  const completeModProcessing = async ({ origin, accountId, state }: ModAppliedResult) => {
    inspection.accept(state);
    const current = latest.current.config;
    if (origin === "recognition") await persistAudioEnabledState(accountId, hasAudioTelemetry(state.feature_groups));
    else if (current?.rune_audio_target_account === accountId) {
      await persistAudioEnabledState(accountId, !!current.rune_audio_enabled && hasAudioTelemetry(state.feature_groups));
    }
  };

  const toggleAudioDiagnosticRecording = async () => {
    try {
      if (audioStatus?.diagnostic_recording) {
        const path = await invokeCommand<string | null>("stop_rune_audio_diagnostic_recording");
        setAudioStatus(previous => previous ? { ...previous, diagnostic_recording: false,
          diagnostic_recording_path: path ?? previous.diagnostic_recording_path } : previous);
        if (path) showToast("success", `诊断录音已保存：${path}`);
      } else {
        const path = await invokeCommand<string>("start_rune_audio_diagnostic_recording");
        setAudioStatus(previous => previous ? { ...previous, diagnostic_recording: true, diagnostic_recording_path: path } : previous);
        showToast("success", "诊断录音已开始，仅录制目标 D2R 进程的声音");
      }
    } catch (error) { showToast("error", `切换诊断录音失败: ${error}`); }
  };

  return {
    audioStatus, audioModState: inspection.state, audioModStateLoading: inspection.loading || changing,
    audioModError: inspection.error,
    hasInitializedAudioAccount: initializedAccounts.length > 0, hasAudioTarget: !!trackingTargetId,
    hasReadyAudioMod: !!hasReadyAudioMod, isAudioEnableRequested: !!config?.rune_audio_enabled,
    isAudioRecognitionActive: optionalFeaturesAvailable && !!config?.rune_audio_enabled
      && audioStatus?.running === true && audioStatus.account_id === trackingTargetId,
    refreshAudioModState: inspection.refresh, handleAudioTargetChange, handleAudioToggle,
    completeModProcessing, toggleAudioDiagnosticRecording,
  };
}
