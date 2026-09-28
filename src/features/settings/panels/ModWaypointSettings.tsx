import { useEffect, useId, useRef, useState } from "react";
import { Button } from "../../../components/ui/Button";
import { invokeCommand } from "../../../platform/tauri/client";

export interface WaypointConfig {
  selected: string[];
  defaults: string[];
  options: { id: string; act: number; label_zh: string; label_en: string }[];
  etag: string;
}

export function ModWaypointSettings({ edition, modName, en, disabled }: {
  edition: string; modName: string; en: boolean; disabled: boolean;
}) {
  const id = useId();
  const [config, setConfig] = useState<WaypointConfig | null>(null);
  const [draft, setDraft] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const request = useRef(0);
  const saveLock = useRef(false);
  const load = async () => {
    const token = ++request.current;
    setLoading(true); setError(""); setMessage("");
    try {
      const next = await invokeCommand<WaypointConfig | null>("get_mod_waypoints", { edition, modName });
      if (token !== request.current) return;
      setConfig(next); setDraft(next?.selected ?? []);
    } catch (e) {
      if (token === request.current) setError(String(e));
    } finally {
      if (token === request.current) setLoading(false);
    }
  };
  useEffect(() => { void load(); return () => { request.current++; }; }, [edition, modName]);
  const nonempty = draft.filter(Boolean);
  const duplicate = new Set(nonempty).size !== nonempty.length;
  const dirty = config !== null && JSON.stringify(config.selected) !== JSON.stringify(draft);
  const busy = disabled || loading || saving;
  const save = async () => {
    if (!config || !dirty || busy || duplicate || saveLock.current) return;
    saveLock.current = true; setSaving(true); setError(""); setMessage("");
    const token = request.current;
    try {
      const next = await invokeCommand<WaypointConfig>("save_mod_waypoints", {
        edition, modName, selected: draft, etag: config.etag,
      });
      if (token !== request.current) return;
      setConfig(next); setDraft(next.selected);
      setMessage(en ? "Saved. Restart the game to apply." : "已保存，重新启动游戏后生效。");
    } catch (e) {
      if (token === request.current) setError(String(e));
    } finally { saveLock.current = false; setSaving(false); }
  };
  return <section className="mod-waypoint-settings" aria-labelledby={`${id}-title`}>
    <h4 id={`${id}-title`}>{en ? "Act IV waypoint shortcuts" : "第四幕快捷传送"}</h4>
    <p>{en
      ? "Customize slots 4–9 in the Act IV waypoint tab. The first three stay unchanged. Close the game before saving; existing waypoint unlock requirements still apply."
      : "设置第四幕传送页第 4–9 项，保留原有前三项。保存前请关闭游戏；仍需满足游戏原有的传送点解锁条件。"}</p>
    {loading && <p role="status">{en ? "Loading…" : "正在读取传送列表…"}</p>}
    {!loading && !config && !error && <p>{en ? "Install a Mod version with waypoint shortcuts to configure this list." : "此 Mod 版本尚未包含快捷传送功能，请先安装新版成品。"}</p>}
    {!loading && config && <>
      <div className="mod-waypoint-grid">
        {draft.map((value, index) => <label key={index} htmlFor={`${id}-${index}`}>
          <span>{en ? `Slot ${index + 4}` : `第 ${index + 4} 项`}</span>
          <select id={`${id}-${index}`} className="settings-input" value={value} disabled={busy}
            onChange={e => { const next = [...draft]; next[index] = e.target.value; setDraft(next); setMessage(""); }}>
            <option value="">{en ? "None" : "不添加"}</option>
            {[1, 2, 3, 5].map(act => <optgroup key={act} label={en ? `Act ${act}` : `第 ${act} 幕`}>
              {config.options.filter(o => o.act === act).map(o => <option key={o.id} value={o.id}>{en ? o.label_en : o.label_zh}</option>)}
            </optgroup>)}
          </select>
        </label>)}
      </div>
      {duplicate && <p role="alert">{en ? "Choose different destinations or leave slots empty." : "新增目的地不能重复；不用的位置可选择“不添加”。"}</p>}
      <div className="mod-library-entry-actions">
        <Button size="sm" variant="primary" disabled={busy || !dirty || duplicate} onClick={() => void save()}>{saving ? (en ? "Saving…" : "正在保存…") : (en ? "Save waypoints" : "保存传送列表")}</Button>
        <Button size="sm" variant="ghost" disabled={busy} onClick={() => { setDraft([...config.defaults]); setMessage(""); }}>{en ? "Restore default order" : "恢复默认顺序"}</Button>
        <Button size="sm" variant="ghost" disabled={busy} onClick={() => void load()}>{en ? "Reload" : "重新读取"}</Button>
      </div>
    </>}
    {error && <p className="mod-catalog-error" role="alert">{error}<Button size="sm" variant="ghost" disabled={busy} onClick={() => void load()}>{en ? "Reload" : "重新读取"}</Button></p>}
    {message && <p role="status">{message}</p>}
  </section>;
}
