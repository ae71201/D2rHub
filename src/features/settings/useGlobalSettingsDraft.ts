import { useMemo, useReducer, useRef } from "react";
import type { GlobalConfig } from "../../store/types";
import type { GlobalConfigPatch } from "../../utils/globalConfigPatch";
import { GlobalSettingsSession } from "./globalSettingsSession";

interface Options {
  open: boolean;
  committedConfig: GlobalConfig | null;
  readCommitted: () => GlobalConfig | null;
  persistPatch: (patch: GlobalConfigPatch) => Promise<GlobalConfig>;
}

export function useGlobalSettingsDraft({ open, committedConfig, readCommitted, persistPatch }: Options) {
  const [revision, refresh] = useReducer((value: number) => value + 1, 0);
  const reference = useRef({ open, session: new GlobalSettingsSession(committedConfig) });
  if (reference.current.open !== open) {
    reference.current = { open, session: new GlobalSettingsSession(committedConfig) };
  }
  const session = reference.current.session;
  session.rebase(committedConfig);
  const config = useMemo(() => session.snapshot(), [session, committedConfig, revision]);

  const updateConfig = (updater: (config: GlobalConfig) => void) => {
    session.rebase(readCommitted());
    session.update(updater);
    refresh();
  };

  const persistDraft = async (candidate?: GlobalConfig) => {
    try {
      const saving = session.save(config, candidate, persistPatch, readCommitted);
      refresh();
      return await saving;
    } finally {
      refresh();
    }
  };

  return {
    config,
    updateConfig,
    persistDraft,
    getDraft: () => {
      session.rebase(readCommitted());
      return session.snapshot();
    },
    hasChanges: Object.keys(session.patch()).length > 0,
    hasPendingChanges: () => Object.keys(session.patch()).length > 0,
  };
}
