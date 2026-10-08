import {
  AlertTriangle,
  CheckCircle2,
  ChevronDown,
  Package,
  Play,
  RotateCw,
} from "lucide-react";
import { useEffect, useState } from "react";
import { Button } from "../../../components/ui/Button";
import { RangeSlider } from "../../../components/ui/RangeSlider";
import { Toggle } from "../../../components/ui/Toggle";
import { showToast } from "../../../components/ui/Toast";
import { invokeCommand } from "../../../platform/tauri";
import type { AccountMeta, AudioModSetupState, GlobalConfig, ModCapsulePool } from "../../../store/types";
import {
  AUDIO_TELEMETRY_CAPSULE_FEATURE,
  capsuleSelectionForAccount,
  compatibleCapsulesForAccount,
  selectedCapsuleForAccount,
} from "../../modCapsules/model";
import { validateTrackingTarget } from "../../../utils/trackingTarget";
import {
  AGGREGATE_ITEM_FILTERS,
  CHARM_FILTERS,
  DEFAULT_TRACKING_CATEGORIES,
  GEM_LEVELS,
  TRACKING_CATEGORIES,
  type RuneAudioStatus,
  type ExternalAudioInstance,
} from "../audioModuleModel";

import { defaultExternalAudioTarget, type ExternalAudioTarget } from "../../../utils/recognitionSource";
import { ExternalRecognitionTarget } from "./ExternalRecognitionTarget";

type TrackingTarget = ReturnType<typeof validateTrackingTarget>;

interface AutomationPanelProps {
  config: GlobalConfig;
  updateConfig: (updater: (config: GlobalConfig) => void) => void;
  persistConfig: (draft: GlobalConfig, quiet?: boolean) => Promise<unknown>;
  initializedTrackingAccounts: AccountMeta[];
  trackingTarget: TrackingTarget;
  audioStatus: RuneAudioStatus | null;
  audioModState: AudioModSetupState | null;
  audioModStateLoading: boolean;
  modCapsulePool?: ModCapsulePool | null;
  assigningCapsuleAccountId?: string | null;
  onAssignModCapsule?: (accountId: string, capsuleId: string) => Promise<unknown>;
  onPrepareModCapsule?: (accountId: string, capsuleId: string) => void;
  onOpenModProcessing?: () => void;
  audioPreparing: boolean;
  hasInitializedAudioAccount: boolean;
  hasAudioTarget: boolean;
  hasReadyAudioMod: boolean;
  isAudioEnableRequested: boolean;
  isAudioRecognitionActive: boolean;
  externalInstances?: ExternalAudioInstance[];
  externalInstancesError?: string | null;
  onExternalTargetChange?: (target: ExternalAudioTarget) => Promise<void>;
  onSelectExternalInstance?: (instance: ExternalAudioInstance) => Promise<void>;
  onAudioTargetChange: (accountId: string) => Promise<void>;
  onAudioToggle: (enabled: boolean) => Promise<boolean | void>;
  onToggleDiagnosticRecording: () => Promise<void>;
  onClose: () => void;
  onInitializeAccount: () => void;
}

export function AutomationPanel({
  config,
  updateConfig,
  persistConfig: persistGlobalDraft,
  initializedTrackingAccounts,
  trackingTarget,
  audioStatus,
  audioModState,
  audioModStateLoading,
  modCapsulePool = null,
  assigningCapsuleAccountId = null,
  onAssignModCapsule,
  onPrepareModCapsule,
  onOpenModProcessing = () => undefined,
  audioPreparing,
  hasInitializedAudioAccount,
  hasAudioTarget,
  hasReadyAudioMod,
  isAudioEnableRequested,
  isAudioRecognitionActive,
  externalInstances = [], externalInstancesError = null, onExternalTargetChange, onSelectExternalInstance,
  onAudioTargetChange: handleAudioTargetChange,
  onAudioToggle: handleAudioToggle,
  onToggleDiagnosticRecording: toggleAudioDiagnosticRecording,
  onClose,
  onInitializeAccount,
}: AutomationPanelProps) {
  const external = config.rune_audio_external_target;
  const isEnglish = config.app_language === "en-US";
  const trackingAccountId = trackingTarget.valid ? trackingTarget.account.id : "";
  const audioCapsules = compatibleCapsulesForAccount(modCapsulePool, trackingAccountId)
    .filter((capsule) => capsule.feature_groups.includes(AUDIO_TELEMETRY_CAPSULE_FEATURE));
  const processingCandidates = compatibleCapsulesForAccount(modCapsulePool, trackingAccountId)
    .filter((capsule) => capsule.source_eligible);
  const audioCapsuleSelection = capsuleSelectionForAccount(modCapsulePool, trackingAccountId);
  const selectedCapsule = selectedCapsuleForAccount(modCapsulePool, trackingAccountId);
  const selectedAudioReady = !!selectedCapsule?.feature_groups.includes(AUDIO_TELEMETRY_CAPSULE_FEATURE);
  const trackedCategories = config.rune_audio_tracked_categories ?? DEFAULT_TRACKING_CATEGORIES;
  const filterSummary = trackedCategories.length
    ? isEnglish
      ? `${trackedCategories.length} drop categories${trackedCategories.includes("runes") ? ` · Runes #${config.rune_audio_min_rune_number ?? 1}+` : ""}`
      : `记录 ${trackedCategories.length} 类掉落${trackedCategories.includes("runes") ? ` · 符文 #${config.rune_audio_min_rune_number ?? 1} 起` : ""}`
    : isEnglish ? "Drops off; scene tracking and run timing remain on." : "暂不记录掉落，场景与刷图计时不受影响";
  const [modSelectionRequired, setModSelectionRequired] = useState(false);
  useEffect(() => {
    if (selectedAudioReady) setModSelectionRequired(false);
  }, [selectedAudioReady]);
  const requestAudioToggle = (enabled: boolean) => {
    void handleAudioToggle(enabled).then((handled) => {
      if (enabled && handled === false) {
        setModSelectionRequired(true);
        showToast("info", audioCapsules.length
          ? (isEnglish ? "Choose a Mod with recognition support first." : "请先选择一个已加工声纹的 Mod")
          : (isEnglish ? "Choose a Mod to prepare for recognition." : "请选择一个要加工声纹的 Mod"));
      }
    });
  };
  return (
<div className="settings-content-grid recognition-layout">
  <div className="spatial-panel recognition-control-panel">
    <div className="flex items-center justify-between py-1">
      <div className="min-w-0 pr-4">
        <span className="text-sm font-bold text-text-secondary">音频声纹自动识别</span>
        <p className="text-xs text-text-muted">{external ? (isEnglish ? "Track drops and run times for the selected game." : "记录所选游戏的掉落与刷图用时。") : isEnglish ? "Track drops and run times for the selected account." : "记录所选账号的掉落与刷图用时。"}</p>
      </div>
      <Toggle
        checked={isAudioEnableRequested}
        disabled={audioPreparing || audioModStateLoading}
        ariaLabel="启用音频声纹自动识别"
        descriptionId="rune-audio-readiness"
        onChange={requestAudioToggle}
      />
    </div>

    {!external && (
    <div
      id="rune-audio-readiness"
      className="recognition-readiness"
      data-state={isAudioRecognitionActive ? "running" : hasReadyAudioMod ? "ready" : "attention"}
      role="status"
      aria-live="polite"
    >
      <div className="flex items-start justify-between gap-3">
        <div className="flex min-w-0 items-start gap-2.5">
          {isAudioRecognitionActive || hasReadyAudioMod
            ? <CheckCircle2 size={16} className={`mt-0.5 shrink-0 ${isAudioRecognitionActive ? "text-success" : "text-accent"}`} />
            : <AlertTriangle size={16} className="mt-0.5 shrink-0 text-warning" />}
          <div className="min-w-0">
            <p className="text-xs font-semibold text-text-primary">
              {isAudioRecognitionActive
                ? "声纹识别已开启"
                : isAudioEnableRequested && hasReadyAudioMod
                  ? audioModState?.restart_required ? (isEnglish ? "Enabled · restart the game" : "已启用，等待游戏重启")
                    : audioStatus?.last_error ? (isEnglish ? "Recognition is unavailable" : "识别暂时不可用") : (isEnglish ? "Enabled · waiting for the game" : "已启用，等待游戏运行")
                : !hasInitializedAudioAccount
                  ? isAudioEnableRequested ? "开启尚未完成：初始化账号" : "先初始化一个游戏账号"
                  : !hasAudioTarget
                    ? isAudioEnableRequested ? "开启尚未完成：选择监听账号" : "第 2 步：选择监听账号"
                    : audioModStateLoading
                      ? "正在检查识别 Mod"
                      : !hasReadyAudioMod
                        ? isAudioEnableRequested ? "开启尚未完成：准备识别 Mod" : "还差一步：准备识别 Mod"
                        : "准备完成，可以开启识别"}
            </p>
            <p className="mt-0.5 text-2xs leading-relaxed text-text-secondary">
              {isAudioRecognitionActive
                ? audioModState?.restart_required
                  ? "配置已完成；请重启该账号的游戏，让新的 Mod 启动参数生效。"
                  : "D2RHub 会锁定所选账号的 D2R 进程，不会录制其他应用声音。"
                : isAudioEnableRequested && hasReadyAudioMod
                  ? audioModState?.restart_required
                    ? (isEnglish ? "Restart the selected account's game. Recognition starts when the updated Mod is loaded." : "重启所选账号的游戏，新 Mod 生效后会自动开始识别。")
                    : audioStatus?.last_error
                      ? (isEnglish ? "Game audio is not being recognized. Open diagnostics to inspect the issue and retry." : "当前未在识别游戏声音。可展开诊断工具查看原因并重试。")
                      : (isEnglish ? "Recognition starts automatically when this account enters the game." : "所选账号进入游戏后会自动开始识别，无需再次开启。")
                : !hasInitializedAudioAccount
                  ? "声纹需要绑定一个可启动的账号。点击右侧按钮完成初始化，再回来选择账号和准备 Mod。"
                  : !hasAudioTarget
                    ? "声音按 D2R 进程隔离捕获；先明确要统计哪个账号。"
                    : !hasReadyAudioMod
                      ? "D2R 需要播放极短的识别音频。D2RHub 会保留你的原 Mod，并自动生成启动参数。"
                      : "所有前置项均已完成。点击开启后，目标游戏运行时会自动开始识别。"}
            </p>
          </div>
        </div>
        {!isAudioRecognitionActive && !(isAudioEnableRequested && hasReadyAudioMod) && (
          <Button
            variant={hasReadyAudioMod ? "primary" : "secondary"}
            size="sm"
            className="shrink-0"
            disabled={audioPreparing || audioModStateLoading}
            onClick={() => {
              if (!hasInitializedAudioAccount) {
                onClose();
                onInitializeAccount();
                return;
              }
              if (!hasAudioTarget) {
                const firstAccount = initializedTrackingAccounts[0];
                if (firstAccount) void handleAudioTargetChange(firstAccount.id);
                return;
              }
              if (!hasReadyAudioMod) {
                onOpenModProcessing();
                return;
              }
              requestAudioToggle(true);
            }}
          >
            {!hasInitializedAudioAccount
              ? "初始化账号"
              : !hasAudioTarget
                ? "选择首个账号"
                : !hasReadyAudioMod
                  ? "开始准备"
                  : "立即开启"}
          </Button>
        )}
      </div>
      <ol className="recognition-checklist" aria-label="声纹识别启用步骤">
        {[
          { label: "初始化账号", complete: hasInitializedAudioAccount },
          { label: "选择监听账号", complete: hasAudioTarget },
          { label: "准备识别 Mod", complete: hasReadyAudioMod },
        ].map((step, index) => (
          <li
            key={step.label}
            className="recognition-checklist-item"
            data-complete={step.complete ? "true" : undefined}
          >
            <span
              className="recognition-checklist-marker"
              aria-hidden="true"
            >
              {step.complete ? "✓" : index + 1}
            </span>
            <span>{step.label}</span>
          </li>
        ))}
      </ol>
    </div>

    )}

    <div className="recognition-target-section">
      <div className="flex items-center justify-between gap-4">
        <div>
          <label htmlFor="rune-audio-target-account" className="text-sm font-semibold text-text-secondary">
            {isEnglish ? "Listening target" : "监听目标"}
          </label>
          <p className="text-2xs text-text-muted">{isEnglish ? "Choose an account or specify a game process" : "选择账号，或指定要监听的游戏进程"}</p>
        </div>
        <select
          id="rune-audio-target-account"
          value={external ? "external" : trackingTarget.valid ? trackingTarget.account.id : ""}
          disabled={audioPreparing || audioModStateLoading}
          aria-describedby="rune-audio-target-help"
          onChange={e => e.target.value === "external"
            ? void onExternalTargetChange?.(defaultExternalAudioTarget(config)) : void handleAudioTargetChange(e.target.value)}
          className="h-8 min-w-36 px-2.5 rounded-lg bg-surface-hover border border-border-default text-text-primary text-xs disabled:opacity-50 disabled:cursor-not-allowed"
        >
          <option value="" disabled>
            {initializedTrackingAccounts.length === 0 ? "暂无可用账号" : "请选择账号"}
          </option>
          <option value="external">{isEnglish ? "Specify game process (no account needed)" : "指定游戏进程（无需账号）"}</option>
          {initializedTrackingAccounts.map(account => (
            <option key={account.id} value={account.id}>{account.display_name || account.id}</option>
          ))}
        </select>
      </div>
      <p id="rune-audio-target-help" aria-live="polite" className="text-2xs text-text-secondary">
        {external ? (isEnglish ? "Choose a process below, including games started through Battle.net or a shortcut." : "在下方指定游戏进程，也支持通过战网或快捷方式启动的游戏。") : initializedTrackingAccounts.length === 0
          ? "上方“初始化账号”会直接打开账号向导；完成后回到这里继续。"
          : trackingTarget.valid
            ? `只识别“${trackingTarget.account.display_name || trackingTarget.account.id}”对应的游戏声音。`
            : "必须先选择目标账号；也可点击上方“选择首个账号”快速继续。"}
      </p>
      {!external && trackingTarget.valid && (
        <div className="recognition-capsule-selector">
          <select
            className="settings-input recognition-capsule-select"
            aria-label={isEnglish ? "Choose a Mod" : "选择 Mod"}
            value={selectedAudioReady ? audioCapsuleSelection?.selected_capsule_id ?? "" : ""}
            data-required={modSelectionRequired ? "true" : undefined}
            disabled={assigningCapsuleAccountId === trackingAccountId
              || (audioCapsules.length ? !onAssignModCapsule : !onPrepareModCapsule)}
            onChange={(event) => {
              const capsule = (audioCapsules.length ? audioCapsules : processingCandidates)
                .find((candidate) => candidate.id === event.target.value);
              if (!capsule) return;
              if (audioCapsules.length) void onAssignModCapsule?.(trackingAccountId, capsule.id);
              else onPrepareModCapsule?.(trackingAccountId, capsule.id);
            }}
          >
            <option value="">{audioCapsules.length
              ? (isEnglish ? "Choose a Mod" : "选择 Mod")
              : (isEnglish ? "Choose a Mod to prepare" : "选择要加工声纹的 Mod")}</option>
            {(audioCapsules.length ? audioCapsules : processingCandidates)
              .map((capsule) => <option value={capsule.id} key={capsule.id}>{capsule.name}</option>)}
          </select>
          <Button size="sm" variant="secondary" className="shrink-0" onClick={onOpenModProcessing}>
            <Package size={12} />
            {audioModState?.ready ? (isEnglish ? "Manage Mod" : "管理 Mod") : (isEnglish ? "Prepare Mod" : "前往加工")}
          </Button>
        </div>
      )}
    </div>

    {external && <ExternalRecognitionTarget config={config} state={audioModState} status={audioStatus} pool={modCapsulePool}
      busy={audioPreparing || audioModStateLoading} instances={externalInstances} instancesError={externalInstancesError}
      onChange={onExternalTargetChange ?? (async () => {})} onSelectInstance={onSelectExternalInstance ?? (async () => {})} onPrepare={onOpenModProcessing} />}

    {config.rune_audio_enabled && (trackingTarget.valid || external) && (
      <details className="group border-t border-border-default/50 pt-3">
        <summary className="flex cursor-pointer list-none items-center justify-between text-xs font-medium text-text-secondary">
          诊断工具
          <ChevronDown size={14} className="transition-transform duration-200 group-open:rotate-180" />
        </summary>
        <div className="mt-3 space-y-3">
    <div className="recognition-monitor">
      <div className="flex items-center justify-between text-xs">
        <span className={audioStatus?.running ? "text-success" : "text-text-secondary"}>
          {audioStatus?.running ? `正在捕获 · PID ${audioStatus.target_pid}` : "监控未运行"}
        </span>
        <span className="text-text-muted">数据包 {audioStatus?.decoded_packets ?? 0}</span>
      </div>
      <div className="grid grid-cols-4 gap-2 text-center text-2xs">
        <div className="rounded bg-surface-hover px-2 py-1.5">
          <span className="block text-text-muted">音频峰值</span>
          <span className="font-mono text-text-primary">
            {audioStatus ? audioStatus.audio_peak.toFixed(4) : "0.0000"}
          </span>
        </div>
        <div className="rounded bg-surface-hover px-2 py-1.5">
          <span className="block text-text-muted">符文</span>
          <span className="font-mono text-text-primary">{audioStatus?.rune_events ?? 0}</span>
        </div>
        <div className="rounded bg-surface-hover px-2 py-1.5">
          <span className="block text-text-muted">物品</span>
          <span className="font-mono text-text-primary">{audioStatus?.item_events ?? 0}</span>
        </div>
        <div className="rounded bg-surface-hover px-2 py-1.5">
          <span className="block text-text-muted">地点信号</span>
          <span className="font-mono text-text-primary">{audioStatus?.scene_heartbeats ?? 0}</span>
        </div>
      </div>
      {audioStatus?.last_marker && (
        <p className="text-2xs text-success">
          最近识别：{audioStatus.last_marker} · {((audioStatus.last_confidence ?? 0) * 100).toFixed(1)}%
        </p>
      )}
      {audioStatus?.last_error && (
        <p className="text-2xs text-danger break-all">{audioStatus.last_error}</p>
      )}
    </div>


          <div className="flex items-center justify-between gap-3">
            <div>
              <span className="text-xs font-semibold text-text-secondary">识别阈值</span>
              <p className="text-2xs text-text-muted">默认 0.56；没有误识别时无需调整</p>
            </div>
            <input
              type="number"
              aria-label="识别阈值"
              min={0.4}
              max={0.95}
              step={0.01}
              value={config.rune_audio_detection_threshold ?? 0.56}
              onChange={event => updateConfig(c => {
                c.rune_audio_detection_threshold = Number(event.target.value);
              })}
              className="h-8 w-24 rounded-lg border border-border-default bg-surface-hover px-2.5 text-xs text-text-primary"
            />
          </div>

          <Button
            variant="secondary"
            size="md"
            onClick={async () => {
              try {
                await persistGlobalDraft(config, true);
                await invokeCommand("restart_rune_audio_monitor");
                showToast("success", "声纹监控已用新配置重启");
              } catch (e) {
                showToast("error", "重启声纹监控失败: " + e);
              }
            }}
            className="w-full"
          >
            <RotateCw size={13} className="shrink-0" />
            应用并重启识别
          </Button>

          <div className="border-t border-border-default/50 pt-3">
            <Button
              variant={audioStatus?.diagnostic_recording ? "danger" : "secondary"}
              size="md"
              disabled={!audioStatus?.running}
              onClick={toggleAudioDiagnosticRecording}
              className="w-full"
            >
              {audioStatus?.diagnostic_recording ? "停止并保存诊断录音" : "开始诊断录音"}
            </Button>
            <p className="mt-1 text-center text-2xs text-text-muted break-all">
              {audioStatus?.diagnostic_recording
                ? "正在录制目标游戏的声音并保存识别事件"
                : audioStatus?.diagnostic_recording_path
                  ? `最近保存：${audioStatus.diagnostic_recording_path}`
                  : "仅录制目标游戏，不录制麦克风或其他应用"}
            </p>
          </div>
        </div>
      </details>
    )}
  </div>

  <div className="recognition-side">
    <details className="spatial-panel group overflow-hidden recognition-filters">
      <summary className="flex cursor-pointer list-none items-center justify-between gap-3 p-4">
        <span>
          <span className="block text-sm font-bold text-text-primary">{isEnglish ? "Drops to record" : "记录哪些掉落"}</span>
          <span className="mt-0.5 block text-xs text-text-muted">{filterSummary}</span>
        </span>
        <ChevronDown size={15} className="shrink-0 text-text-muted transition-transform duration-200 group-open:rotate-180" />
      </summary>

      <div className="border-t border-border-default/50 px-4 pb-4 pt-3">
        <div className="space-y-4">
          {([
            {
              id: "runes",
              label: "符文",
              value: config.rune_audio_min_rune_number ?? 1,
              max: 33,
              valueLabel: `#${config.rune_audio_min_rune_number ?? 1}–#33`,
              detail: "最低编号（含）；滑到 #20 时只记录 #20–#33",
              onChange: (value: number) => updateConfig(next => { next.rune_audio_min_rune_number = value; }),
            },
            {
              id: "gems",
              label: "宝石与骷髅",
              value: config.rune_audio_min_gem_level ?? 1,
              max: 5,
              valueLabel: `${GEM_LEVELS[(config.rune_audio_min_gem_level ?? 1) - 1]}及以上`,
              detail: "五档品质：碎裂、裂开、普通、无瑕疵、完美",
              onChange: (value: number) => updateConfig(next => { next.rune_audio_min_gem_level = value; }),
            },
          ] as const).map(filter => {
            const enabled = (config.rune_audio_tracked_categories ?? DEFAULT_TRACKING_CATEGORIES).includes(filter.id);
            return (
              <div key={filter.id} className={enabled ? "" : "opacity-55"}>
                <div className="flex items-center justify-between gap-3">
                  <label className="flex cursor-pointer items-center gap-2 text-xs font-semibold text-text-secondary">
                    <input
                      type="checkbox"
                      checked={enabled}
                      onChange={event => updateConfig(next => {
                        const current = new Set(next.rune_audio_tracked_categories ?? DEFAULT_TRACKING_CATEGORIES);
                        if (event.target.checked) current.add(filter.id);
                        else current.delete(filter.id);
                        next.rune_audio_tracked_categories = TRACKING_CATEGORIES
                          .map(item => item.id)
                          .filter(id => current.has(id));
                      })}
                      className="accent-[var(--accent)]"
                    />
                    {filter.label}
                  </label>
                  <span className="rounded-md bg-surface-hover px-2 py-0.5 font-mono text-2xs font-semibold text-text-primary">
                    {filter.valueLabel}
                  </span>
                </div>
                <RangeSlider
                  min={1}
                  max={filter.max}
                  step={1}
                  value={filter.value}
                  disabled={!enabled}
                  aria-label={`${filter.label}最低记录等级`}
                  onChange={event => filter.onChange(Number(event.target.value))}
                  className="mt-2 w-full"
                />
                <div className="mt-1 flex items-center justify-between text-2xs text-text-muted">
                  <span>{filter.detail}</span>
                  <span className="ml-3 shrink-0">1 — {filter.max}</span>
                </div>
              </div>
            );
          })}

          <div className="border-t border-border-default/50 pt-3">
            <p className="mb-2 text-2xs font-semibold text-text-muted">护身符 · 分别选择</p>
            <div className="grid grid-cols-3 gap-1.5">
              {CHARM_FILTERS.map(item => {
                const categories = config.rune_audio_tracked_categories ?? DEFAULT_TRACKING_CATEGORIES;
                const codes = config.rune_audio_tracked_charm_codes ?? CHARM_FILTERS.map(filter => filter.code);
                const selected = categories.includes("charms") && codes.includes(item.code);
                return (
                  <label
                    key={item.code}
                    className={`cursor-pointer rounded-lg border px-2 py-2 transition-colors ${selected
                      ? "border-accent/40 bg-accent/5"
                      : "border-border-default bg-surface-hover"}`}
                  >
                    <span className="flex items-start gap-1.5">
                      <input
                        type="checkbox"
                        checked={selected}
                        onChange={event => updateConfig(next => {
                          const currentCategories = new Set(next.rune_audio_tracked_categories ?? DEFAULT_TRACKING_CATEGORIES);
                          const charmCodes = new Set(
                            currentCategories.has("charms")
                              ? next.rune_audio_tracked_charm_codes ?? CHARM_FILTERS.map(filter => filter.code)
                              : [],
                          );
                          if (event.target.checked) charmCodes.add(item.code);
                          else charmCodes.delete(item.code);
                          next.rune_audio_tracked_charm_codes = CHARM_FILTERS
                            .map(filter => filter.code)
                            .filter(code => charmCodes.has(code));
                          if (next.rune_audio_tracked_charm_codes.length > 0) currentCategories.add("charms");
                          else currentCategories.delete("charms");
                          next.rune_audio_tracked_categories = TRACKING_CATEGORIES
                            .map(filter => filter.id)
                            .filter(id => currentCategories.has(id));
                        })}
                        className="mt-0.5 accent-[var(--accent)]"
                      />
                      <span className="min-w-0">
                        <span className="block text-2xs font-semibold leading-tight text-text-secondary">{item.label}</span>
                        <span className="mt-0.5 block truncate text-[9px] text-text-muted">{item.detail}</span>
                      </span>
                    </span>
                  </label>
                );
              })}
            </div>
          </div>

          <div className="border-t border-border-default/50 pt-3">
            <p className="mb-2 text-2xs font-semibold text-text-muted">其他物品 · 按整项选择</p>
            <div className="grid grid-cols-2 gap-x-3 gap-y-1">
              {AGGREGATE_ITEM_FILTERS.map(item => {
                const selected = (config.rune_audio_tracked_categories ?? DEFAULT_TRACKING_CATEGORIES).includes(item.id);
                return (
                  <label key={item.id} className="flex cursor-pointer items-start gap-2 border-b border-border-default/40 py-2 last:border-b-0">
                    <input
                      type="checkbox"
                      checked={selected}
                      onChange={event => updateConfig(next => {
                        const current = new Set(next.rune_audio_tracked_categories ?? DEFAULT_TRACKING_CATEGORIES);
                        if (event.target.checked) current.add(item.id);
                        else current.delete(item.id);
                        next.rune_audio_tracked_categories = TRACKING_CATEGORIES
                          .map(filter => filter.id)
                          .filter(id => current.has(id));
                      })}
                      className="mt-0.5 accent-[var(--accent)]"
                    />
                    <span>
                      <span className="block text-xs font-semibold text-text-secondary">{item.label}</span>
                      <span className="block text-2xs leading-relaxed text-text-muted">{item.detail}</span>
                    </span>
                  </label>
                );
              })}
            </div>
          </div>

          <p className="border-t border-border-default/50 pt-3 text-2xs leading-relaxed text-text-muted">
            所有掉落仅在已确认的野外或地下城场景中记录；主城、主界面和尚未识别地点时一律忽略。修改后点击左侧“应用并重启识别”。
          </p>
        </div>
      </div>
    </details>

    <div className="spatial-panel recognition-notes">
      <div>
        <span className="text-xs font-bold text-text-primary block mb-1">识别说明</span>
        <p className="text-2xs text-text-muted">
          {external ? (isEnglish ? "D2RHub captures only the selected game process, without reading game memory or injecting code." : "D2RHub 只捕获所选游戏进程的声音，不读取游戏内存，也不会向游戏注入代码。") : "D2RHub 只捕获所选账号的游戏声音，不读取游戏内存，也不会向游戏注入代码。"}
        </p>
      </div>
      <p className="text-2xs text-text-secondary">
        过滤器只决定 D2RHub 是否将已接收事件写入统计；Mod 始终包含完整识别声纹。
      </p>
      <p className="text-2xs text-warning">
        声纹按基础物品代码识别；同一代码的暗金、套装或词缀无法仅凭音频区分。
      </p>
      <div className="border-t border-border-default/50 pt-3">
        <Button
          size="sm"
          onClick={async () => {
            try {
              await invokeCommand("open_stats_page");
            } catch (e) {
              showToast("error", `打开统计界面失败: ${e}`);
            }
          }}
        >
          <Play size={10} className="text-success fill-success" />
          打开掉落统计图表
        </Button>
      </div>
    </div>
  </div>
</div>
  );
}
