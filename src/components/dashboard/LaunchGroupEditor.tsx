import { useState } from "react";
import { createPortal } from "react-dom";
import { ArrowDown, ArrowUp, Copy, Trash2 } from "lucide-react";
import type { LaunchGroupController } from "../../hooks/useLaunchGroupController";
import { useGlobalConfig } from "../../store/globalConfig";
import { useAccounts } from "../../store/accounts";
import type { ModCapsulePool, SchemeEnvironment } from "../../store/types";
import { DisplaySection, GraphicsSection, AudioSection, GameplaySection, AutomapSection } from "../../features/settings/gameSettings/fields";
import { Modal } from "../ui/Modal";
import { invokeCommand } from "../../platform/tauri";
import { getLayoutMonitors } from "../../features/windowLayouts/gateway";
import { presetSlots } from "../../features/windowLayouts/model";
import type { RoomAutomationConfigSnapshot } from "../../features/roomAutomation/types";
import { showToast } from "../ui/Toast";
import "./launchSchemeEditor.css";
import { SchemeLayoutPreview } from "./SchemeLayoutPreview";

export function LaunchGroupEditor({ launchGroups: controller, modCapsulePool }: {
  launchGroups: LaunchGroupController; modCapsulePool?: ModCapsulePool | null;
}) {
  const config = useGlobalConfig(state => state.config);
  const accounts = useAccounts(state => state.accounts);
  const [tab, setTab] = useState("accounts");
  const [layoutBusy, setLayoutBusy] = useState(false);
  const draft = controller.draft;
  if (!draft || !config || controller.pendingDelete) return null;
  const env = draft.environment, room = env.room, settings = env.settings;
  const selected = accounts.filter(account => draft.members.some(member => member.account_id === account.id));
  const name = (id: string) => accounts.find(account => account.id === id)?.display_name || id;
  const edit = controller.updateEnvironment;
  const setting = <K extends keyof SchemeEnvironment["settings"]>(key: K, value: SchemeEnvironment["settings"][K]) => edit(env => { env.settings[key] = value; });
  const errors: string[] = [];
  if (room.enabled) {
    if (room.primary_account_id && !selected.some(account => account.id === room.primary_account_id)) errors.push("跟房：请选择方案内的主号");
    if (!room.follower_account_ids.length || room.follower_account_ids.some(id => !selected.some(account => account.id === id) || id === room.primary_account_id)) errors.push("跟房：请选择有效的小号");
  }
  if (settings.rune_audio_enabled && !selected.some(account => account.id === settings.rune_audio_target_account)) errors.push("识别：请选择方案内的账号");
  if (env.layout && env.layout.windows.length < draft.members.length) errors.push("布局：位置数量少于账号数量");
  const ready = draft.name.trim() && draft.members.length && draft.members.every(member => member.snapshot && member.resolution && member.fps != null) && !errors.length;
  const createLayout = async () => {
    setLayoutBusy(true);
    try {
      const monitors = await getLayoutMonitors();
      if (!monitors.length) throw new Error("未检测到显示器");
      edit(env => { env.layout = { id: crypto.randomUUID(), name: "方案布局", monitors, windows: presetSlots(monitors, Math.max(1, draft.members.length), "1080p") }; });
    } catch (error) { showToast("error", String(error)); }
    finally { setLayoutBusy(false); }
  };
  const importCurrent = async () => {
    setLayoutBusy(true);
    try {
      let currentRoom = room;
      if (config.installed_optional_modules?.includes("room-automation")) currentRoom = (await invokeCommand<RoomAutomationConfigSnapshot>("room_automation_get_config")).config;
      edit(env => {
        for (const key of Object.keys(env.settings) as (keyof SchemeEnvironment["settings"])[]) {
          const value = config[key];
          if (value !== undefined) Object.assign(env.settings, { [key]: structuredClone(value) });
        }
        env.settings.rune_audio_external_target = null;
        env.room = structuredClone(currentRoom);
        env.layout = structuredClone(config.window_layout_enabled ? config.window_layouts?.find(layout => layout.id === config.active_window_layout_id) ?? null : null);
      });
      showToast("info", "已填入草稿，检查后保存方案；当前运行不会改变");
    } catch (error) { showToast("error", `读取当前设置失败：${error}`); }
    finally { setLayoutBusy(false); }
  };
  const options = <><option value="">请选择账号</option>{selected.map(account => <option key={account.id} value={account.id}>{name(account.id)}</option>)}</>;
  return createPortal(<Modal open title={draft.id ? `编辑方案：${draft.name}` : "新建启动方案"} width="max-w-4xl"
    onClose={controller.cancelDraft} dismissible={!controller.busy && !layoutBusy} footer={<>
      {draft.id && <button className="control-btn" disabled={controller.busy} onClick={controller.duplicate}><Copy size={14} />复制为新方案</button>}
      <button className="control-btn" disabled={controller.busy || layoutBusy} onClick={controller.cancelDraft}>取消</button>
      <button className="primary-cta" disabled={!ready || controller.busy || layoutBusy} onClick={() => void controller.saveDraft()}>{controller.busy ? "正在保存…" : "保存方案"}</button>
    </>}>
    <div className="scheme-form">
      <label>方案名称<input autoFocus maxLength={32} value={draft.name} disabled={controller.busy} onChange={event => controller.renameDraft(event.target.value)} /></label>
      <p className="scheme-note">保存仅影响以后按此方案启动。运行中的调整不会写回方案。</p>
      <nav className="scheme-tabs" aria-label="方案设置">{[["accounts", "账号与启动"], ["room", "跟房"], ["recognition", "识别"], ["layout", "窗口布局"]].map(([id, label]) => <button type="button" key={id} aria-pressed={tab === id} onClick={() => setTab(id)}>{label}</button>)}</nav>
      <fieldset disabled={controller.busy || layoutBusy} className="scheme-body">
        {tab === "accounts" && <>
          <div className="scheme-account-picker" aria-label="选择方案账号">{accounts.map(account => <label key={account.id}><input type="checkbox" disabled={!account.initialized} checked={draft.members.some(member => member.account_id === account.id)} onChange={() => controller.toggleAccount(account.id)} />{name(account.id)}{!account.initialized && "（未初始化）"}</label>)}</div>
          {!draft.members.length && <p className="scheme-note">先选择账号，再配置跟房、识别和布局。</p>}
          {draft.members.map((member, index) => {
            const account = accounts.find(account => account.id === member.account_id), snapshot = member.snapshot;
            const update = (patch: Partial<typeof member>) => controller.updateMember(member.account_id, patch);
            const edition = modCapsulePool?.accounts.find(account => account.account_id === member.account_id)?.edition;
            const mods = modCapsulePool?.capsules.filter(capsule => capsule.edition === edition && capsule.ready) ?? [];
            return <section className="scheme-account" key={member.account_id}>
              <div className="scheme-row"><strong>{index + 1}. {name(member.account_id)}</strong><span className="scheme-spacer" />
                <button className="icon-btn" aria-label={`上移 ${name(member.account_id)}`} disabled={index === 0} onClick={() => controller.moveMember(member.account_id, -1)}><ArrowUp size={14}/></button>
                <button className="icon-btn" aria-label={`下移 ${name(member.account_id)}`} disabled={index === draft.members.length - 1} onClick={() => controller.moveMember(member.account_id, 1)}><ArrowDown size={14}/></button>
                <button className="icon-btn" aria-label={`移除 ${name(member.account_id)}`} onClick={() => controller.toggleAccount(member.account_id)}><Trash2 size={14}/></button>
              </div>
              <div className="scheme-fields">
                <label>Mod<select value={member.mod_args ?? ""} onChange={event => update({ mod_args: event.target.value })}><option value="">不使用 Mod</option>{member.mod_args && !mods.some(mod => mod.launch_arguments === member.mod_args) && <option value={member.mod_args}>{member.mod_args}</option>}{mods.map(mod => <option key={mod.id} value={mod.launch_arguments}>{mod.name}</option>)}</select></label>
                <label>分辨率{env.layout ? <span className="scheme-note">由布局位置 {index + 1} 决定</span> : <input aria-label={`${name(member.account_id)} 分辨率`} value={member.resolution ?? ""} placeholder="1920x1080" disabled={!snapshot} onChange={event => update({ resolution: event.target.value, graphics_configured: true })}/>}</label>
                <label>FPS（0 为不限）<input type="number" min={0} max={500} disabled={!snapshot} value={member.fps ?? 30} onChange={event => update({ fps: Number(event.target.value), graphics_configured: true })}/></label>
              </div>
              {!snapshot ? <p className="scheme-note">正在读取独立游戏设置…</p> : <details><summary>位置、语言与启动参数</summary><div className="scheme-fields">
                <label>位置<select disabled={!!env.layout} value={snapshot.position ? member.position_preset_id ?? "snapshot" : ""} onChange={event => { const position = account?.position_presets?.find(position => position.id === event.target.value); update({ position_preset_id: position?.id ?? null, snapshot: { ...snapshot, position: position ? [position.x, position.y] : null } }); }}><option value="">不指定位置</option>{snapshot.position && <option value={member.position_preset_id ?? "snapshot"}>已保存位置（{snapshot.position.join(", ")}）</option>}{account?.position_presets?.filter(position => position.id !== member.position_preset_id).map(position => <option key={position.id} value={position.id}>{position.name}</option>)}</select></label>
                {!env.layout && snapshot.position && <>{["X", "Y"].map((axis, coordinate) => <label key={axis}>{axis}<input type="number" value={snapshot.position![coordinate]} onChange={event => { const position: [number, number] = [...snapshot.position!]; position[coordinate] = Number(event.target.value); update({ snapshot: { ...snapshot, position } }); }}/></label>)}</>}
                <label>区服<select disabled={account?.auth_mode !== "token"} value={snapshot.region ?? ""} onChange={event => update({ snapshot: { ...snapshot, region: event.target.value } })}>{(snapshot.region === "CN" ? ["CN"] : ["KR", "NA", "EU"]).map(region => <option key={region} value={region}>{({ CN: "国服", KR: "亚洲", NA: "美洲", EU: "欧洲" } as Record<string, string>)[region]}</option>)}</select></label>
                <label>文字语言<input value={snapshot.language ?? ""} placeholder="跟随游戏默认" onChange={event => update({ snapshot: { ...snapshot, language: event.target.value || null } })}/></label>
                <label>语音语言<input value={snapshot.voicelanguage ?? ""} placeholder="跟随游戏默认" onChange={event => update({ snapshot: { ...snapshot, voicelanguage: event.target.value || null } })}/></label>
                <label className="scheme-wide">启动参数<input value={member.mod_args ?? ""} onChange={event => update({ mod_args: event.target.value })}/></label>
              </div><p className="scheme-note">其他游戏设置已复制为独立快照；账号默认设置的变化不会影响此方案。</p></details>}
              {snapshot && <details><summary>完整游戏设置</summary>{[...(!env.layout ? [DisplaySection] : []), GraphicsSection, AudioSection, GameplaySection, AutomapSection].map((Section, sectionIndex) => <Section key={sectionIndex} settings={{ ...snapshot.game_settings, "Screen Resolution (Windowed)": member.resolution, "Framerate Cap": member.fps }} update={(key, value) => {
                if (key === "Screen Resolution (Windowed)") update({ resolution: String(value) });
                else if (key === "Framerate Cap" || key === "Framerate Target") update({ fps: Number(value) });
                else update({ snapshot: { ...snapshot, game_settings: { ...snapshot.game_settings, [key]: value } } });
              }}/>)}</details>}
            </section>;
          })}
          <details><summary>启动行为</summary><div className="scheme-fields"><label><input type="checkbox" checked={settings.launch_trim_memory ?? true} onChange={event => setting("launch_trim_memory", event.target.checked)}/>启动后整理内存</label><label><input type="checkbox" checked={settings.auto_close_browser} onChange={event => setting("auto_close_browser", event.target.checked)}/>启动时关闭登录浏览器</label><label>保留战网<select value={settings.keep_battle_net_account_id ?? ""} onChange={event => setting("keep_battle_net_account_id", event.target.value || null)}><option value="">不保留</option>{selected.filter(account => account.auth_mode === "bnet").map(account => <option key={account.id} value={account.id}>{name(account.id)}</option>)}</select></label></div></details>
        </>}
        {tab === "room" && <>
          <label className="scheme-toggle"><input type="checkbox" checked={room.enabled} onChange={event => edit(env => { env.room.enabled = event.target.checked; })}/>启用自动跟房</label>
          {room.enabled && <><div className="scheme-fields"><label>主号<select value={room.primary_account_id} onChange={event => edit(env => { env.room.primary_account_id = event.target.value; env.room.follower_account_ids = env.room.follower_account_ids.filter(id => id !== event.target.value); })}>{options}</select></label><label>房名前缀<input value={room.name_prefix} onChange={event => edit(env => { env.room.name_prefix = event.target.value; })}/></label><label>密码<input type="password" autoComplete="off" value={room.password} onChange={event => edit(env => { env.room.password = event.target.value; })}/></label><label>起始序号<input type="number" min={1} value={room.next_sequence} onChange={event => edit(env => { env.room.next_sequence = Number(event.target.value); })}/></label></div>
          <p>跟随账号</p><div className="scheme-account-picker">{selected.filter(account => account.id !== room.primary_account_id).map(account => <label key={account.id}><input type="checkbox" checked={room.follower_account_ids.includes(account.id)} onChange={event => edit(env => { env.room.follower_account_ids = event.target.checked ? [...env.room.follower_account_ids, account.id] : env.room.follower_account_ids.filter(id => id !== account.id); })}/>{name(account.id)}</label>)}</div>
          <div className="scheme-fields"><label><input type="checkbox" checked={room.auto_followers_enabled} onChange={event => edit(env => { env.room.auto_followers_enabled = event.target.checked; })}/>建房后自动跟随</label><label>建房后等待（秒）<input type="number" min={0} step={0.1} value={room.auto_followers_delay_secs} onChange={event => edit(env => { env.room.auto_followers_delay_secs = Number(event.target.value); })}/></label><label>跟随方式<select value={room.follower_join_mode ?? "simultaneous"} onChange={event => edit(env => { env.room.follower_join_mode = event.target.value as "simultaneous" | "interval"; })}><option value="simultaneous">同时派发</option><option value="interval">间隔派发</option></select></label>{room.follower_join_mode === "interval" && <label>派发间隔（秒）<input type="number" min={0} step={0.1} value={room.follower_join_interval_secs ?? 3} onChange={event => edit(env => { env.room.follower_join_interval_secs = Number(event.target.value); })}/></label>}</div>
          <details><summary>快捷键与输入时序</summary><div className="scheme-fields"><label>建房快捷键<input value={room.shortcut} onChange={event => edit(env => { env.room.shortcut = event.target.value; })}/></label><label>跟随快捷键<input value={room.join_shortcut} onChange={event => edit(env => { env.room.join_shortcut = event.target.value; })}/></label><label>序号位数<input type="number" min={1} max={8} value={room.sequence_width} onChange={event => edit(env => { env.room.sequence_width = Number(event.target.value); })}/></label>{([["step_delay_ms", "步骤间隔"], ["character_delay_ms", "字符间隔"], ["key_hold_ms", "按键保持"], ["form_settle_ms", "表单等待"]] as const).map(([key, label]) => <label key={key}>{label}（毫秒）<input type="number" min={0} value={room.flow[key] ?? 50} onChange={event => edit(env => { env.room.flow[key] = Number(event.target.value); })}/></label>)}</div></details>
          <p className="scheme-note">启动只准备跟房配置，不会自动建房或进房。参与账号需要带房间工具的 Mod。</p></>}
        </>}
        {tab === "recognition" && <>
          <label className="scheme-toggle"><input type="checkbox" checked={settings.rune_audio_enabled} onChange={event => setting("rune_audio_enabled", event.target.checked)}/>启用识别</label>
          {settings.rune_audio_enabled && <><div className="scheme-fields"><label>识别账号<select value={settings.rune_audio_target_account} onChange={event => setting("rune_audio_target_account", event.target.value)}>{options}</select></label>{room.primary_account_id && <button className="control-btn" onClick={() => setting("rune_audio_target_account", room.primary_account_id)}>使用跟房主号</button>}</div>
          <p>记录内容</p><div className="scheme-account-picker">{[["runes", "符文"], ["gems", "宝石"], ["charms", "护身符"], ["jewels", "珠宝"], ["keys", "钥匙"], ["organs", "器官"], ["essences", "精华"]].map(([id, label]) => <label key={id}><input type="checkbox" checked={settings.rune_audio_tracked_categories.includes(id)} onChange={event => setting("rune_audio_tracked_categories", event.target.checked ? [...settings.rune_audio_tracked_categories, id] : settings.rune_audio_tracked_categories.filter(category => category !== id))}/>{label}</label>)}</div>
          <div className="scheme-fields"><label>最低符文编号<input type="number" min={1} max={33} value={settings.rune_audio_min_rune_number ?? 1} onChange={event => setting("rune_audio_min_rune_number", Number(event.target.value))}/></label><label>最低宝石等级<input type="number" min={1} max={5} value={settings.rune_audio_min_gem_level ?? 1} onChange={event => setting("rune_audio_min_gem_level", Number(event.target.value))}/></label></div><p className="scheme-note">识别目标独立保存，改变跟房主号不会自动改变识别目标。</p></>}
          <div className="scheme-account-picker"><label><input type="checkbox" checked={settings.enable_stats_overlay} onChange={event => setting("enable_stats_overlay", event.target.checked)}/>统计悬浮窗</label><label><input type="checkbox" checked={settings.enable_tz_overlay} onChange={event => setting("enable_tz_overlay", event.target.checked)}/>恐怖地带悬浮窗</label></div>
        </>}
        {tab === "layout" && <>
          <div className="scheme-fields"><label>布局模板<select value="" onChange={event => { const layout = config.window_layouts?.find(layout => layout.id === event.target.value); if (layout) edit(env => { env.layout = structuredClone(layout); }); }}><option value="">选择模板，复制到方案…</option>{config.window_layouts?.map(layout => <option key={layout.id} value={layout.id}>{layout.name}</option>)}</select></label><button className="control-btn" disabled={!draft.members.length} onClick={() => void createLayout()}>生成 {draft.members.length || 1} 窗口布局</button><button className="control-btn" onClick={() => edit(env => { env.layout = null; })}>不使用统一布局</button></div>
          {env.layout ? <><SchemeLayoutPreview layout={env.layout} names={draft.members.map(member => name(member.account_id))}/><p className="scheme-note">布局内容属于此方案。账号对应固定位置，启动失败或补启动不会改变其他账号的位置。</p><label>布局名称<input value={env.layout.name} onChange={event => edit(env => { env.layout!.name = event.target.value; })}/></label>
          {draft.members.map((member, index) => { const slot = env.layout!.windows[index]; return slot && <section className="scheme-account" key={member.account_id}><strong>位置 {index + 1} · {name(member.account_id)}</strong><div className="scheme-fields"><label>显示器<select value={slot.monitor_id} onChange={event => edit(env => { env.layout!.windows[index].monitor_id = event.target.value; })}>{env.layout!.monitors.map(monitor => <option key={monitor.id} value={monitor.id}>{monitor.name}</option>)}</select></label>{([["x", "X"], ["y", "Y"], ["width", "分辨率宽"], ["height", "分辨率高"]] as const).map(([key, label]) => <label key={key}>{label}<input type="number" value={slot[key]} onChange={event => edit(env => { env.layout!.windows[index][key] = Number(event.target.value); })}/></label>)}</div></section>; })}</> : <p className="scheme-note">使用方案内各账号自己的位置与分辨率。</p>}
        </>}
      </fieldset>
      {controller.loadError && <p role="alert" className="scheme-error">{controller.loadError}。可移除后重新添加该账号以重试。</p>}
      {!!errors.length && <div role="alert" className="scheme-error">{errors.map(error => <p key={error}>{error}</p>)}</div>}
      <div className="scheme-row"><button className="control-btn" disabled={controller.busy || layoutBusy} onClick={() => void importCurrent()}>从当前跟房、识别和布局填入</button><span className="scheme-spacer"/>{draft.id && <button className="control-btn" disabled={controller.busy || layoutBusy} onClick={controller.requestDraftDelete}>删除方案</button>}</div>
    </div>
  </Modal>, document.body);
}
