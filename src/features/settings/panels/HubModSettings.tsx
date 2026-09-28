import { useEffect, useId, useRef, useState } from "react";
import { RefreshCw, Save } from "lucide-react";
import { Button } from "../../../components/ui/Button";
import { invokeCommand } from "../../../platform/tauri/client";
import { ModWaypointSettings } from "./ModWaypointSettings";

export interface HubSettings {
  base_mod: string;
  game_data_version: string;
  waypoints_supported: boolean;
  etag: string;
}

export function HubModSettings({ edition, modName, en, disabled }: {
  edition: string; modName: string; en: boolean; disabled: boolean;
}) {
  const id = useId();
  const [config, setConfig] = useState<HubSettings | null>(null);
  const [draft, setDraft] = useState("");
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
      const next = await invokeCommand<HubSettings | null>("get_hub_mod_settings", { edition, modName });
      if (token !== request.current) return;
      setConfig(next ?? null); setDraft(next?.game_data_version ?? "");
    } catch (e) {
      if (token === request.current) setError(String(e));
    } finally { if (token === request.current) setLoading(false); }
  };
  useEffect(() => {
    setConfig(null); setDraft(""); void load();
    return () => { request.current++; };
  }, [edition, modName]);
  const valid = /^\d{1,10}$/.test(draft) && Number(draft) > 0 && Number(draft) <= 4294967295;
  const busy = disabled || saving || loading;
  const save = async () => {
    if (!config || !valid || busy || draft === config.game_data_version || saveLock.current) return;
    saveLock.current = true; setSaving(true); setError(""); setMessage("");
    const token = request.current;
    try {
      const next = await invokeCommand<HubSettings>("save_hub_mod_data_version", {
        edition, modName, gameDataVersion: draft, etag: config.etag,
      });
      if (token !== request.current) return;
      setConfig(next); setDraft(next.game_data_version);
      setMessage(en ? "Saved. Restart the game to apply." : "已保存，重新启动游戏后生效。");
    } catch (e) {
      if (token === request.current) setError(String(e));
    } finally { saveLock.current = false; setSaving(false); }
  };
  if (!config && !error) return null;
  return <>
    <section className="hub-mod-version" aria-labelledby={`${id}-label`}>
      <div className="hub-mod-version-row">
        <label id={`${id}-label`} htmlFor={`${id}-version`}>{en ? "Game data version" : "游戏数据版本"}</label>
        {config && <div className="hub-mod-version-controls">
          <input id={`${id}-version`} className="settings-input" inputMode="numeric" maxLength={10}
            value={draft} disabled={busy} aria-invalid={!valid} aria-describedby={`${id}-note`}
            onChange={e => { setDraft(e.target.value); setMessage(""); }}
            onKeyDown={e => { if (e.key === "Enter") { e.preventDefault(); void save(); } }} />
          <Button size="sm" variant="primary" disabled={busy || !valid || draft === config.game_data_version}
            aria-label={en ? "Save data version" : "保存数据版本"} title={en ? "Save data version" : "保存数据版本"} onClick={() => void save()}><Save size={14} /></Button>
        </div>}
        <Button size="sm" variant="ghost" disabled={busy} aria-label={en ? "Reload Mod settings" : "重新读取 Mod 设置"}
          title={en ? "Reload Mod settings" : "重新读取 Mod 设置"} onClick={() => void load()}><RefreshCw size={14} /></Button>
      </div>
      <p id={`${id}-note`}>{en ? "Close the game before saving. This changes the version marker only; it does not adapt the Mod to a game update." : "保存前请关闭游戏。仅修改版本标记，不会自动适配游戏更新。"}</p>
      {config && !valid && <p className="mod-catalog-error" role="alert">{en ? "Enter a positive integer up to 4294967295." : "请输入 1–4294967295 范围内的整数。"}</p>}
      {error && <p className="mod-catalog-error" role="alert">{error}</p>}
      {message && <p role="status">{message}</p>}
    </section>
    {config?.waypoints_supported && <ModWaypointSettings key={`${edition}:${modName}`} edition={edition} modName={modName} en={en} disabled={disabled || saving} />}
  </>;
}
