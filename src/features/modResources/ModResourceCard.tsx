import { useEffect, useRef } from "react";
import { Download, ExternalLink, FolderOpen, MemoryStick } from "lucide-react";
import { Button } from "../../components/ui/Button";
import { ProgressBar } from "../../components/ui/ProgressBar";
import { LIGHTWEIGHT_PROFILES } from "../modCapsules/lightweightModel";
import { resourceInstallLocation, resourceStatusMessage } from "./model";
import type { ModResourceAsset, ModResourceState } from "./types";
import type { ModResourcesController } from "./useModResources";

const MEMORY_REFERENCE: Record<string, number> = { NullHub: 300, BoHub: 500, LiteHub: 800 };

interface Props {
  asset: ModResourceAsset;
  state: ModResourceState;
  controller: ModResourcesController;
  installed: boolean;
  en: boolean;
}

export function ModResourceCard({ asset, state, controller, installed, en }: Props) {
  const processor = asset.id === "processor";
  const local = state.mods?.find(mod => mod.id === asset.id);
  const task = controller.taskByAsset.get(asset.id);
  const active = controller.activeAsset === asset.id || task?.state === "running";
  const feedback = controller.feedback?.assetId === asset.id ? controller.feedback : null;
  const installationCancelled = task?.state === "cancelled" && feedback?.installAfterTaskId !== undefined
    && task.task_id > feedback.installAfterTaskId;
  const profile = LIGHTWEIGHT_PROFILES.find(value => value.name === asset.id);
  const location = resourceInstallLocation(asset, state);
  const incompatible = !processor && state.game_data_version !== asset.game_data_version;
  const current = processor ? state.processor.ready && !state.processor.update_available
    : (local ? !!local.installed_version && !local.update_available : installed);
  const blocked = controller.busy || current || local?.protected || incompatible || (!processor && !location);
  const status = resourceStatusMessage(local, en);
  const progress = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (active) progress.current?.scrollIntoView?.({ block: "nearest", behavior: "smooth" });
  }, [active]);

  const actionLabel = active ? (en ? "Working…" : "正在处理…")
    : local?.protected ? (en ? "Preserved" : "保留现有 Mod")
      : current ? (en ? "Installed" : "已是当前版本")
        : processor && state.processor.installed_path ? (en ? "Update processor" : "更新加工器")
          : local?.update_available ? (en ? "Verify & update" : "校验并原位更新")
            : (en ? "Download & install" : "下载并安装");

  return <article className="resource-card" data-processor={processor || undefined} aria-label={processor ? (en ? "Mod processor" : "Mod 加工器") : asset.id} aria-busy={active}>
    <div className="resource-summary">
    <div className="resource-title">
      <strong>{processor ? (en ? "Independent Mod processor" : "独立 Mod 加工器") : asset.id}</strong>
      <span>{en ? "Download" : "下载"} {(asset.size / 1048576).toFixed(1)} MB</span>
    </div>
    <p>{processor
      ? (en ? "Adds selected features to your own Mods. Installed separately from Hub." : "为已有 Mod 添加所选功能，独立安装和更新。")
      : (en ? profile?.enDetail : profile?.detail)}</p>
    </div>
    <div className="resource-metadata">
    {!processor && MEMORY_REFERENCE[asset.id] && <div className="resource-memory" title={en ? "Reference only; actual use varies with the scene and settings." : "仅供参考，实际占用随场景和设置变化。"}>
      <MemoryStick size={13} aria-hidden="true" /><span>{en ? "Memory reference" : "内存参考"}</span>
      <strong>{en ? "~" : "约 "}{MEMORY_REFERENCE[asset.id]} MB</strong>
    </div>}
    <small>{processor ? `${en ? "Recommended" : "推荐版本"} ${state.processor.recommended_version}`
      : `${en ? "Game data" : "游戏数据版本"} ${asset.game_data_version}`}</small>
    {processor && state.processor.installed_path && <p className="resource-note">
      {state.processor.legacy ? (en ? "Legacy bundled processor found" : "检测到旧版内置加工器") : (en ? "Installed processor" : "已安装加工器")}
      {` · ${state.processor.installed_version ?? (en ? "Unknown version" : "版本未知")}`}
      {!state.processor.ready && (en ? ". Install the compatible version below before processing." : "。请安装兼容版本后再加工。")}
    </p>}
    {status && <p className="resource-note">{status}</p>}
    {processor && state.processor.ready && state.processor.update_available && <p className="resource-note">
      {en ? "Your installed processor remains usable while an update is available." : "有兼容新版本，当前加工器仍可使用。"}
    </p>}
    {incompatible && <p className="resource-note">{en
      ? `Current game: ${state.game_data_version ?? "not configured"}. This package requires ${asset.game_data_version}.`
      : `当前游戏版本：${state.game_data_version ?? "尚未配置或无法识别"}，此成品需要 ${asset.game_data_version}。`}</p>}
    {!processor && !location && <p className="resource-note">{en ? "Configure a game directory first" : "请先在运行环境中设置游戏目录"}</p>}
    </div>
    <div className="resource-actions">
      <Button size="sm" variant="primary" disabled={!!blocked} onClick={() => void controller.install(asset, false)}>
        <Download size={13} />{actionLabel}
      </Button>
      <Button size="sm" variant="ghost" disabled={!location} onClick={() => void controller.openFolder(asset)}><FolderOpen size={12} />{en ? "Open folder" : "打开目录"}</Button>
    </div>
    <details className="resource-details">
      <summary>{en ? "Manual download" : "手动下载"}</summary>
      <div className="resource-actions">
        <Button size="sm" variant="ghost" disabled={!!blocked} onClick={() => void controller.install(asset, true)}>{en ? "Import downloaded file" : "导入已下载文件"}</Button>
        <Button size="sm" variant="ghost" onClick={() => void controller.openExternal(asset)}><ExternalLink size={12} />{en ? "Browser download" : "浏览器下载"}</Button>
      </div>
    </details>
    {active && <div ref={progress} className="resource-progress" role="status" aria-live="polite">
      <div className="resource-progress-heading">
        <span>{task?.state === "running" ? task.message : (en ? "Preparing…" : "正在准备…")}</span>
        {task?.state === "running" && <span>{task.progress}%</span>}
      </div>
      <ProgressBar label={en ? `${asset.id} progress` : `${asset.id} 进度`} value={task?.state === "running" ? task.progress : undefined} />
      <div className="resource-progress-heading">
        <small>{en ? "You can leave this page; progress stays in Background tasks." : "可离开此页面，进度会保留在后台任务中。"}</small>
        {task?.state === "running" && <Button size="sm" variant="ghost" disabled={task.cancel_requested} onClick={() => void controller.cancel(asset, task)}>
          {task.cancel_requested ? (en ? "Cancelling…" : "正在取消…") : (en ? "Cancel" : "取消")}
        </Button>}
      </div>
    </div>}
    {feedback?.error && !installationCancelled && <p className="resource-error" role="alert">{feedback.error}</p>}
    {feedback?.installedPath && <p className="resource-success" role="status">{en ? "Installation complete. Use Open folder to view the files." : "安装完成，可通过“打开目录”查看文件。"}</p>}
    {!active && !feedback && task?.state === "failed" && <p className="resource-error" role="alert">{task.message}</p>}
    {!active && task?.state === "cancelled" && (!feedback || installationCancelled) && <p className="resource-note" role="status">{en ? "Installation cancelled. You can try again." : "安装已取消，可以重新开始。"}</p>}
  </article>;
}
