import { useId, useState } from "react";
import { ChevronDown, PackageOpen, RotateCcw, Trash2 } from "lucide-react";
import { Button } from "../../../components/ui/Button";
import { Toggle } from "../../../components/ui/Toggle";
import { ModWaypointSettings } from "./ModWaypointSettings";
import type { AccountMeta, ModCapsule } from "../../../store/types";
import { capsuleBaseModLabel, capsuleFeatureLabels } from "../../modCapsules/model";

interface Props {
  capsule: ModCapsule;
  accounts: AccountMeta[];
  en: boolean;
  minimalMode: boolean;
  busy: boolean;
  onUpdate: (arguments_: string) => Promise<boolean>;
  onDelete: () => void;
  onProcess: () => void;
  onUnpack: () => void;
  unpacking: boolean;
  onToggleDeathExit: (enabled: boolean) => void;
}

/** A library entry owns its disclosure and edit draft, never the shared catalog. */
export function ModCatalogEntry({ capsule, accounts, en, minimalMode, busy, onUpdate, onDelete, onProcess, onUnpack, unpacking, onToggleDeathExit }: Props) {
  const [expanded, setExpanded] = useState(false);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(capsule.launch_arguments);
  const featureLabels = capsuleFeatureLabels(capsule, en, minimalMode);
  const assignedNames = capsule.assigned_account_ids
    .map(id => accounts.find(account => account.id === id)?.display_name || id)
    .join(en ? ", " : "、");
  const detailsId = useId();
  const scanned = capsule.origin === "scanned";
  const supportsDeathExit = scanned && capsule.feature_groups.includes("auto_exit_on_death");
  const save = async () => {
    if (draft.trim() && !busy && await onUpdate(draft.trim())) setEditing(false);
  };

  return <article className="mod-library-entry" aria-label={capsule.name}>
    <div className="mod-library-entry-summary">
      <PackageOpen size={18} aria-hidden="true" />
      <div className="mod-library-entry-copy">
        <h3>{capsule.name}</h3>
        <p>{scanned ? (capsule.requires_unpack ? (en ? "MPQ archive" : "MPQ 压缩包") : (en ? "Installed Mod" : "已安装 Mod")) : (en ? "Custom launch preset" : "自定义启动预设")}
          {capsule.processed && ` · ${capsuleBaseModLabel(capsule, en)}`}</p>
        {featureLabels.length > 0 && <p className="mod-library-feature-list">{featureLabels.join(" · ")}</p>}
        <small>{assignedNames ? `${en ? "Used by" : "使用账号"}：${assignedNames}` : (en ? "No accounts assigned" : "尚未分配给账号")}</small>
        {capsule.issue && <p className="mod-catalog-error" role="status">{capsule.issue}</p>}
        {capsule.update_required && <small className="mod-library-update">{en ? "Feature update available" : "功能需要更新"}</small>}
      </div>
      <Button size="sm" variant="ghost" aria-expanded={expanded} aria-controls={detailsId}
        aria-label={`${en ? "Configure" : "设置"} ${capsule.name}`}
        onClick={() => setExpanded(value => !value)}>
        {en ? "Configure" : "设置"}<ChevronDown size={13} className={expanded ? "is-expanded" : undefined} />
      </Button>
    </div>
    {expanded && <div id={detailsId} className="mod-library-entry-details">
      {scanned && !capsule.requires_unpack && ["LiteHub", "BoHub"].includes(capsule.name) &&
        <ModWaypointSettings edition={capsule.edition} modName={capsule.name} en={en} disabled={busy} />}
      {supportsDeathExit && <div className="mod-catalog-feature-control">
        <span><b>{en ? "Auto-exit on death" : "死亡自动退房"}</b>
          <small id={`${detailsId}-death`}>{en ? "Close this Mod's game before changing; applies on the next launch." : "关闭使用此 Mod 的游戏后可修改，下次启动生效。"}</small></span>
        <Toggle checked={capsule.auto_exit_on_death_enabled === true} disabled={busy}
          ariaLabel={`${capsule.name} ${en ? "Auto-exit on death" : "死亡自动退房"}`} descriptionId={`${detailsId}-death`}
          onChange={onToggleDeathExit} />
      </div>}
      <div className="mod-library-arguments-heading">
        <label htmlFor={`${detailsId}-args`}>{en ? "Shared launch arguments" : "共享启动参数"}</label>
        {!editing && <Button size="sm" variant="ghost" disabled={busy} onClick={() => { setDraft(capsule.launch_arguments); setEditing(true); }}>{en ? "Edit" : "编辑"}</Button>}
      </div>
      {editing ? <div className="mod-library-argument-editor">
        <input id={`${detailsId}-args`} className="settings-input" autoFocus value={draft} disabled={busy}
          onChange={event => setDraft(event.target.value)} onKeyDown={event => {
            if (event.key === "Escape") { event.stopPropagation(); setEditing(false); }
            if (event.key === "Enter") void save();
          }} />
        <Button size="sm" variant="primary" disabled={busy || !draft.trim()} onClick={() => void save()}>{en ? "Save" : "保存"}</Button>
        <Button size="sm" variant="ghost" disabled={busy} onClick={() => setEditing(false)}>{en ? "Cancel" : "取消"}</Button>
      </div> : <code className="mod-library-arguments">{capsule.launch_arguments}</code>}
      <p>{en ? "Changes apply to accounts and launch groups that reference this preset." : "修改会同步到引用此预设的账号与启动方案。"}</p>
      {scanned && capsule.requires_unpack && <p>{en
        ? "Unpack first to process this Mod. The original MPQ is kept in the adjacent back folder; the Mod name stays the same."
        : "先解压，再加工。原 MPQ 会保存在同目录 back 文件夹中，Mod 名称不变。"}</p>}
      <div className="mod-library-entry-actions">
        {scanned && capsule.requires_unpack && <Button size="sm" variant="secondary" disabled={busy} onClick={onUnpack}>
          {unpacking ? (en ? "Unpacking…" : "正在解压…") : (en ? "Unpack" : "解压")}
        </Button>}
        {scanned && !capsule.requires_unpack && (capsule.source_eligible || capsule.update_required || capsule.processed) &&
          <Button size="sm" variant="secondary" disabled={busy} onClick={onProcess}>{en ? "Add game features" : "加工功能"}</Button>}
        {scanned && capsule.default_launch_arguments && capsule.launch_arguments !== capsule.default_launch_arguments &&
          <Button size="sm" variant="ghost" disabled={busy} onClick={() => void onUpdate(capsule.default_launch_arguments!)}><RotateCcw size={12} />{en ? "Restore arguments" : "恢复默认参数"}</Button>}
        {capsule.deletable && <Button size="sm" variant="ghost" disabled={busy} onClick={onDelete}>
          <Trash2 size={12} />{scanned ? (en ? "Delete Mod…" : "删除 Mod…") : (en ? "Delete preset…" : "删除预设…")}
        </Button>}
      </div>
    </div>}
  </article>;
}
