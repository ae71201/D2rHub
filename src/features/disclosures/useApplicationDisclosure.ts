import { useCallback, useEffect, useState } from "react";
import { invokeCommand } from "../../platform/tauri";
import { useAccounts } from "../../store/accounts";
import {
  acceptApplicationDisclosure,
  hasAcceptedApplicationDisclosure,
} from "./disclosureStorage";

interface ApplicationDisclosureState {
  checking: boolean;
  required: boolean;
  version: string | null;
  accepting: boolean;
  error: { stage: "storage" | "runtime"; message: string } | null;
  retry: () => void;
  accept: () => Promise<void>;
}

export function useApplicationDisclosure(
  ready: boolean,
  runtimeReady = true,
): ApplicationDisclosureState {
  const [checking, setChecking] = useState(true);
  const [required, setRequired] = useState(false);
  const [version, setVersion] = useState<string | null>(null);
  const [accepting, setAccepting] = useState(false);
  const [attempt, setAttempt] = useState(0);
  const [error, setError] = useState<ApplicationDisclosureState["error"]>(null);

  const retry = useCallback(() => {
    setChecking(true);
    setError(null);
    setAttempt(value => value + 1);
  }, []);

  useEffect(() => {
    if (!ready) return;
    let cancelled = false;
    setChecking(true);
    setError(null);

    void (async () => {
      let stage: "storage" | "runtime" = "storage";
      try {
        const currentVersion = await invokeCommand<string>("get_app_version").catch(() => "unknown");
        if (cancelled) return;
        setVersion(currentVersion.replace(/^v/i, "").trim() || "unknown");
        const accepted = await hasAcceptedApplicationDisclosure();
        if (cancelled) return;
        setRequired(!accepted);
        if (!accepted || !runtimeReady) return;

        stage = "runtime";
        await invokeCommand<boolean>("activate_application_runtime");
        if (cancelled) return;
        await useAccounts.getState().loadAccounts();
      } catch (cause) {
        if (!cancelled) {
          setRequired(false);
          setError({ stage, message: String(cause) });
        }
      } finally {
        if (!cancelled) setChecking(false);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [attempt, ready, runtimeReady]);

  const accept = useCallback(async () => {
    if (!required || accepting) return;
    setAccepting(true);
    try {
      // Commit consent before starting services. A startup failure must not erase it.
      await acceptApplicationDisclosure();
      setRequired(false);
      retry();
    } finally {
      setAccepting(false);
    }
  }, [accepting, required, retry]);

  return { checking, required, version, accepting, accept, error, retry };
}
