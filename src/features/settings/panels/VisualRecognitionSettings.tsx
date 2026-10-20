import { roomModeLabel } from "../../roomAutomation/RoomCollaborationMode";
import type { CollaborationMode } from "../../roomAutomation/types";
import { useEffect, useRef, useState } from "react";
import { ChevronDown, ScanLine, Settings2, Bug, RefreshCw, Image, FolderOpen } from "lucide-react";
import { Toggle } from "../../../components/ui/Toggle";
import { Button } from "../../../components/ui/Button";
import "./featureSettings.css";
import { invokeCommand } from "../../../platform/tauri";
import type { AccountMeta } from "../../../store/types";

interface Roi { x: number; y: number; width: number; height: number }
interface Config {
  diagnostic_enabled: boolean;
  chat_send: boolean; chat_follow: boolean; chat_account_id: string; chat_roi: Roi; poll_ms: number;
  area_enabled: boolean; area_roi: Roi; drops_enabled: boolean; drops_enforce: boolean;
  labels_visible: boolean; drop_threshold: number;
}
interface Preview { image_url: string; mask_url: string; tz_image_url?: string | null; candidate_lines: number; line_analysis: Array<{matched: boolean; crosses: number; reason: string}> }
interface Snapshot { generation: number; config: Config }
interface Status { running: boolean; ready: boolean; message: string; checked: number; matched: number; rejected: number; unavailable: number }

// A save continues across tab unmounts; new editors wait before loading persisted state.
let pendingSave: Promise<void> = Promise.resolve();

export function VisualRecognitionSettings({ mode, language, accounts = [], primaryAccountId, collaborationMode = "default" }: {
  mode: "chat" | "audio"; language?: string | null; accounts?: AccountMeta[]; primaryAccountId?: string; collaborationMode?: CollaborationMode;
}) {
  const en = language === "en-US";
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [draft, setDraft] = useState<Config | null>(null);
  const [status, setStatus] = useState<Status | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [opened, setOpened] = useState(false);
  const [preview, setPreview] = useState<Preview | null>(null);
  const [previewing, setPreviewing] = useState(false);
  const loaded = useRef(false);
  const [savedMessage, setSavedMessage] = useState("");
  useEffect(() => {
    if (!opened) return;
    let alive = true;
    if (!loaded.current) void pendingSave.then(() => invokeCommand<Snapshot>("vision_get_config")).then(value => {
      if (alive && value) { loaded.current = true; setSnapshot(value); setDraft(value.config); }
    }).catch(e => { if (alive) setError(String(e)); });
    const refresh = () => void invokeCommand<Status>("vision_get_status").then(value => {
      if (alive) setStatus(value);
    }).catch(e => { if (alive) setError(String(e)); });
    refresh(); const timer = window.setInterval(refresh, 1500);
    return () => { alive = false; window.clearInterval(timer); };
  }, [opened]);
  const update = (patch: Partial<Config>) => {
    setSavedMessage("");
    setDraft(value => value ? { ...value, ...patch } : value);
  };
  const save = async (next: Config) => {
    if (!snapshot || busy) return;
    setBusy(true); setError(""); setSavedMessage("");
    const operation = pendingSave.then(async () => {
      // Merge only this editor's changes so the chat/audio pages do not overwrite each other.
      const latest = await invokeCommand<Snapshot>("vision_get_config");
      const patch = Object.fromEntries(Object.entries(next).filter(([key, value]) =>
        JSON.stringify(value) !== JSON.stringify(snapshot.config[key as keyof Config])));
      const value = await invokeCommand<Snapshot>("vision_save_config", {
        expectedGeneration: latest.generation, config: { ...latest.config, ...patch },
      });
      setSnapshot(value); setDraft(value.config);
      setSavedMessage(en ? "Saved; settings survive app restarts." : "已保存，重启软件后仍保留。");
    });
    pendingSave = operation.catch(() => {});
    try { await operation; } catch (e) { setError(`${en ? "Save not confirmed: " : "保存未确认，请重试："}${String(e)}`); }
    finally { setBusy(false); }
  };
  const commit = (patch: Partial<Config>) => {
    if (!draft) return;
    const next = { ...draft, ...patch };
    if (patch.drops_enabled === false || patch.labels_visible === false) next.drops_enforce = false;
    update(next); void save(next);
  };
  const toggle = (key: "diagnostic_enabled" | "chat_send" | "chat_follow" | "area_enabled" | "drops_enabled" | "drops_enforce" | "labels_visible", title: string, description: string, unavailable = false) => (
    <div className="feature-setting-row">
      <div><strong>{title}</strong><p>{description}</p></div>
      <Toggle checked={draft?.[key] ?? false} ariaLabel={title} disabled={busy || unavailable} onChange={value => commit({ [key]: value })} />
    </div>
  );
  const roi = (key: "chat_roi" | "area_roi") => draft && (
    <fieldset className="feature-roi"><legend>{en ? "Capture region · % of window" : "截图范围 · 占游戏窗口的百分比"}</legend>
      {(["x", "y", "width", "height"] as const).map((field, index) => <label key={field}>{en ? field : ["左侧起点", "顶部起点", "宽度", "高度"][index]}
        <input className="settings-input" type="number" min="0" max="100" step="1" value={Math.round(draft[key][field] * 100)} disabled={busy}
          onBlur={() => { if (dirty) void save(draft); }}
          onChange={e => { if (Number.isFinite(e.target.valueAsNumber)) update({ [key]: { ...draft[key], [field]: e.target.valueAsNumber / 100 } }); }} />
      </label>)}
    </fieldset>
  );
  const dirty = !!draft && !!snapshot && JSON.stringify(draft) !== JSON.stringify(snapshot.config);
  return <details className="feature-settings spatial-panel" onToggle={e => { if (e.target === e.currentTarget) setOpened(e.currentTarget.open); }}>
    <summary className="feature-settings-heading">
      <ScanLine size={17} aria-hidden="true" />
      <span><strong>{mode === "chat" ? (en ? "Cross-PC room signals" : "跨电脑聊天信号") : (en ? "Visual recognition" : "视觉辅助识别")}</strong>
        <small>{mode === "chat" ? (en ? "Follow another player's next room" : "通过聊天信号，跟随另一台电脑进入下一局") : (en ? "Supplement area names and verify uncertain drops" : "补充地区名称，核验低分掉落")}</small></span>
      <em>{en ? "Experimental" : "试验"}</em><ChevronDown className="feature-chevron" size={15} aria-hidden="true" />
    </summary>
    <div className="feature-settings-body">
      {!draft && !error && <p className="feature-help" role="status">{en ? "Loading settings…" : "正在读取设置…"}</p>}
      {draft && <>
        <div className="feature-save-state" role="status" aria-live="polite"><span>{en ? "Changes save automatically" : "修改自动保存"}</span><span>{busy ? (previewing ? (en ? "Capturing…" : "正在截图…") : (en ? "Saving…" : "正在保存…")) : error && dirty ? (en ? "Save needs attention" : "保存需处理") : dirty ? (en ? "Editing…" : "编辑中…") : savedMessage ? (en ? "Saved" : "已保存") : ""}</span></div>
        {mode === "chat" ? <>
          <p className="feature-note">{en ? "Current mode: " : "当前模式："}{roomModeLabel(collaborationMode, en)}{en ? ". Change it in Room collaboration above or in quick settings." : "。在上方拼房模式或快捷配置中切换。"}</p>
          <div className="feature-setting-row feature-account"><div><strong>{en ? "Listening account" : "接收信号的账号"}</strong><p>{en ? "Shared follower listens to the primary account. The primary and followers join using the local room rule." : "拼房跟随监听本机主号聊天框，主号与小号按本机规则一起加入。"}</p></div>
            <span className="feature-listening-account">{accounts.find(account => account.id === primaryAccountId)?.display_name || primaryAccountId || (en ? "Choose a primary account" : "请先配置主号")}</span>
          </div>
          <p className="feature-note">{en ? "Signal: + + 501 + +. Detect spaced plus pairs on both sides, then OCR the numeric center for deduplication only. Successful reads pause OCR for 5 seconds; unchanged masks are cached. Update both computers and synchronize room rules yourself." : "信号示例：+ + 501 + +。先检测左右两侧的加号对，再 OCR 中间的数字后缀，仅用于持久去重，仍按本机规则跟房。读取成功后冷却 5 秒，已识别且掩膜未变化时跳过 OCR。两端需更新并自行同步规则。"}</p>
        </> : <>
          {toggle("area_enabled", en ? "Recognize scenes and time runs" : "场景识别与计时", en ? "OCR alone identifies scenes and starts, splits or stops run timers. Red entry text and purple Terror Zone text are read separately every second." : "仅通过 OCR 识别场景并开始、切换或停止计时；每秒检查红字地区名和下方紫色 TZ 文字，分行用原图识别。")}
          <p className="feature-help">{en ? "Turning scene OCR off disables automatic scene timing and drop attribution." : "关闭场景 OCR 后，不再自动识别地点、切换计时或确定掉落归属。"}</p>
          {toggle("drops_enabled", en ? "Verify low-score drops" : "核验低分掉落", en ? "Locate dark or blue item labels, then read original name crops to verify audio. Nearby events share captures." : "定位黑底或悬浮蓝底标签，再用原图读取名称核对声纹；相近事件共用截图，不做二值化 OCR。")}
          {draft.drops_enabled && <div className="feature-dependent">
            {toggle("labels_visible", en ? "Item names stay visible" : "地面物品名称持续可见", en ? "Confirm this in game. This setting does not press keys or change the game." : "请先在游戏中确认名称文字持续显示。此项不会替你修改游戏或按键。")}
            <div className="feature-field-row"><label htmlFor={`drop-threshold-${mode}`}>{en ? "Verify below" : "核验分数低于"}</label><div className="feature-unit-input"><input id={`drop-threshold-${mode}`} className="settings-input" type="number" min="40" max="95" value={Math.round(draft.drop_threshold * 100)} disabled={busy} onChange={e => { if (Number.isFinite(e.target.valueAsNumber)) update({ drop_threshold: e.target.valueAsNumber / 100 }); }} onBlur={() => { if (dirty) void save(draft); }} /><span>%</span></div></div>
            {toggle("drops_enforce", en ? "Record confirmed drops only" : "只记录核验通过的低分掉落", draft.labels_visible ? (en ? "Hidden or unreadable labels may cause missed drops. Off by default." : "遮挡或识别失败时可能漏记，默认关闭。") : (en ? "Confirm visible item names first." : "先确认地面物品名称持续可见。"), !draft.labels_visible)}
            {!draft.labels_visible && <p className="feature-help">{en ? "Verification pauses until item names are visible." : "未确认名称可见前，掉落视觉核验会跳过。"}</p>}
          </div>}
          <p className="feature-help">{en ? "Images are processed locally. OCR resources are prepared only when needed." : "截图仅在本机处理，识别资源按需准备。"}</p>
        </>}
        <details className="feature-disclosure"><summary><Settings2 size={14} aria-hidden="true" />{en ? "Capture and timing" : "截图与识别参数"}<ChevronDown size={14} className="feature-chevron" aria-hidden="true" /></summary><div className="feature-disclosure-body">
          {mode === "chat" && <div className="feature-field-row"><label htmlFor="vision-poll">{en ? "Check interval" : "检查间隔"}</label><div className="feature-unit-input"><input id="vision-poll" className="settings-input" type="number" min="1000" max="10000" step="500" value={draft.poll_ms} disabled={busy} onChange={e => { if (Number.isFinite(e.target.valueAsNumber)) update({ poll_ms: e.target.valueAsNumber }); }} onBlur={() => { if (dirty) void save(draft); }} /><span>ms</span></div></div>}
          <p className="feature-help">{mode === "chat" ? (en ? "Include both plus pairs and the numeric center on the same line." : "范围应覆盖同一行的左右加号和中间数字。") : (en ? "Include the red entry title and purple Terror Zone line below it; a 9% height is recommended. Drops use a separate capture." : "范围应覆盖红字地区名及下方紫色恐怖／恐惧区域文字，高度建议 9%；掉落使用独立截图。")}</p>
          {roi(mode === "chat" ? "chat_roi" : "area_roi")}
          <Button size="sm" disabled={busy || (mode === "chat" && !primaryAccountId)} onClick={async () => {
            setBusy(true); setPreviewing(true); setError("");
            try { setPreview(await invokeCommand<Preview>("vision_preview", { kind: mode === "chat" ? "chat" : "area", accountId: mode === "chat" ? primaryAccountId : null, roi: mode === "chat" ? draft.chat_roi : draft.area_roi })); }
            catch (e) { setError(String(e)); } finally { setBusy(false); setPreviewing(false); }
          }}><Image size={14} aria-hidden="true" />{en ? "Preview capture region" : "预览截图"}</Button>
          {preview && <>
            <figure className="feature-preview"><img src={preview.image_url} alt={en ? "Current capture region" : "当前截图区域"} onError={() => setError(en ? "Preview image could not load. Restart the updated app and retry capture." : "预览图片加载失败，请重启更新后的软件并重新截图。")} /><figcaption>{en ? "Current capture · local preview" : "当前截图 · 仅本机预览"}</figcaption></figure>
            <figure className="feature-preview"><img src={preview.mask_url} alt={mode === "chat" ? (en ? "Color mask" : "颜色掩膜") : (en ? "Original OCR crop" : "OCR 原图裁片")} /><figcaption>{mode === "chat" ? (en ? "Color mask · candidate lines" : "颜色掩膜 · 候选行") : (en ? "Original OCR crop · red-text candidates" : "OCR 原图裁片 · 红字候选")}：{preview.candidate_lines}</figcaption></figure>
            {mode === "audio" && preview.tz_image_url && <figure className="feature-preview"><img src={preview.tz_image_url} alt={en ? "Terror Zone OCR crop" : "TZ 原图裁片"} /><figcaption>{en ? "Terror Zone original OCR crop" : "恐怖／恐惧区域 · OCR 原图裁片"}</figcaption></figure>}
            {mode === "chat" && <p className="feature-help">{en ? "Signal" : "信号"}：{preview.line_analysis.some(line => line.matched) ? (en ? "Detected" : "加号特征已检测到（后缀由运行时 OCR 读取）") : (en ? "Not detected" : "未检测到")} · {preview.line_analysis.map((line,i) => `${i+1}: ${line.crosses} (${({marker: "加号特征匹配",too_few_crosses: "两侧加号不足4个",alignment_or_shape: "尺寸或同行排列不符",pair_spacing: "两侧加号间距不符"} as Record<string,string>)[line.reason] ?? line.reason})`).join("; ")}</p>}
          </>}
        </div></details>
        <details className="feature-disclosure"><summary><Bug size={14} aria-hidden="true" />{en ? "Diagnostics and resources" : "诊断与资源"}<ChevronDown size={14} className="feature-chevron" aria-hidden="true" /></summary><div className="feature-disclosure-body">
          {toggle("diagnostic_enabled", en ? "Diagnostic logging" : "记录诊断日志", en ? "Shared by chat, area and drop recognition. No screenshots are saved." : "聊天、地区与掉落识别共用，仅排查问题时开启，不保存截图。")}
          {status && <><p className="feature-help">{status.message || (en ? "Service idle" : "服务待机")}</p><dl className="feature-metrics">{[[en ? "Checked" : "核验", status.checked], [en ? "Matched" : "匹配", status.matched], [en ? "Unmatched" : "未匹配", status.rejected], [en ? "Uncertain" : "不确定", status.unavailable]].map(([label, count]) => <div key={label}><dt>{label}</dt><dd>{count ?? 0}</dd></div>)}</dl></>}
          <div className="feature-actions"><Button size="sm" onClick={() => { void invokeCommand("open_logs_dir").catch(e => setError(String(e))); }}><FolderOpen size={14} aria-hidden="true" />{en ? "Open logs folder" : "打开日志"}</Button><Button size="sm" disabled={busy} onClick={() => { void save(draft); }}><RefreshCw size={14} aria-hidden="true" />{en ? "Retry resources" : "重试资源准备"}</Button></div>
        </div></details>
        {dirty && <div className="feature-save-footer"><p className="feature-help">{en ? "Keep this page open until saved." : "保存成功前请保留当前页面。"}</p><Button size="sm" disabled={busy} onClick={() => { void save(draft); }}>{en ? "Save changes" : "保存修改"}</Button></div>}
      </>}
      {error && <p role="alert" className="feature-error">{error}</p>}
    </div>
  </details>;
}
