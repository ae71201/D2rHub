import { useCallback, useEffect, useRef, useState } from "react";
import { invokeCommand } from "../../../platform/tauri";
import type { AudioModSetupState } from "../../../store/types";

interface InstallationInspection { edition: "CN" | "Global"; modName?: string }
interface Inspection { state: AudioModSetupState | null; scannedAt: number | null; loading: boolean; error: string | null }
const empty: Inspection = { state: null, scannedAt: null, loading: false, error: null };

/** Account-scoped reads with no navigation, draft, or configuration side effects. */
export function useModInspection(active: boolean, accountId: string, identity = "", installation?: InstallationInspection) {
  const cache = useRef(new Map<string, { state: AudioModSetupState; scannedAt: number }>());
  const requests = useRef(new Map<string, { version: number; promise: Promise<AudioModSetupState> }>());
  const versions = useRef(new Map<string, number>());
  const identities = useRef(new Map<string, string>());
  const revision = useRef(0);
  const displayedIdentity = useRef(identity);
  const [inspection, setInspection] = useState<Inspection>(empty);
  const current = useRef({ active, accountId, identity, installation });
  current.current = { active, accountId, identity, installation };

  const invalidate = useCallback((id: string) => {
    versions.current.set(id, (versions.current.get(id) ?? 0) + 1);
    cache.current.delete(id);
  }, []);
  const accept = useCallback((state: AudioModSetupState, targetId = state.account_id || current.current.accountId) => {
    const entry = { state, scannedAt: Date.now() };
    versions.current.set(targetId, (versions.current.get(targetId) ?? 0) + 1);
    cache.current.set(targetId, entry);
    if (current.current.active && current.current.accountId === targetId) {
      displayedIdentity.current = current.current.identity;
      setInspection({ ...entry, loading: false, error: null });
    }
  }, []);

  const inspect = useCallback((id: string, force = false, external = current.current.installation): Promise<AudioModSetupState> => {
    if (force) invalidate(id);
    const cached = cache.current.get(id);
    if (cached) return Promise.resolve(cached.state);
    const version = versions.current.get(id) ?? 0;
    const pending = requests.current.get(id);
    if (pending?.version === version) return pending.promise;
    const request = (async () => {
      try {
        const state = await invokeCommand<AudioModSetupState>("get_audio_mod_setup_state", external && id.startsWith("installation:")
          ? { accountId: "", edition: external.edition, modName: external.modName || null } : { accountId: id });
        if ((versions.current.get(id) ?? 0) !== version) {
          const newer = cache.current.get(id);
          if (newer) return newer.state;
          throw new Error("Mod inspection changed; retry the current account.");
        }
        cache.current.set(id, { state, scannedAt: Date.now() });
        return state;
      } finally { if (requests.current.get(id)?.version === version) requests.current.delete(id); }
    })();
    requests.current.set(id, { version, promise: request });
    return request;
  }, [invalidate]);

  const refresh = useCallback(async (force = true) => {
    const id = current.current.accountId;
    if (!id) return null;
    const ticket = ++revision.current;
    setInspection(previous => ({ ...previous, loading: true, error: null }));
    try {
      const state = await inspect(id, force);
      if (ticket === revision.current && current.current.active && current.current.accountId === id) accept(state, id);
      return state;
    } catch (error) {
      if (ticket === revision.current && current.current.active && current.current.accountId === id) setInspection({ ...empty, error: String(error) });
      return null;
    }
  }, [accept, inspect]);

  useEffect(() => {
    ++revision.current;
    if (accountId && identities.current.get(accountId) !== identity) {
      if (identities.current.has(accountId)) invalidate(accountId);
      identities.current.set(accountId, identity);
    }
    if (!active || !accountId) { setInspection(empty); return; }
    const cached = cache.current.get(accountId);
    if (cached) { displayedIdentity.current = identity; setInspection({ ...cached, loading: false, error: null }); }
    else { setInspection(empty); void refresh(false); }
    return () => { ++revision.current; };
  }, [active, accountId, identity, invalidate, refresh]);

  return { ...inspection,
    state: displayedIdentity.current === identity && (installation ? inspection.state?.account_id === "" : inspection.state?.account_id === accountId) ? inspection.state : null,
    refresh, inspect, accept };
}
