import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { showToast } from "../../components/ui/Toast";
import type { SettingsMap } from "./gameSettings/fields";
import { emitEvent, invokeCommand, listenEvent } from "../../platform/tauri";
import type { AccountMeta } from "../../store/types";
import { FRAMERATE_CAP_KEY, writeFramerateCap } from "../../utils/gameSettings";

type GameSettingsTab = "launch" | "game_display" | "game_graphics" | "game_audio" | "game_gameplay" | "game_automap";

interface AccountSettingsControllerOptions {
  accounts: AccountMeta[];
  loadAccounts: () => Promise<void>;
  renameAccount: (id: string, newName: string) => Promise<boolean>;
}

function changedSettings(base: SettingsMap, next: SettingsMap): SettingsMap {
  return Object.fromEntries(Object.entries(next).filter(([key, value]) => JSON.stringify(value) !== JSON.stringify(base[key])));
}

export function useAccountSettingsController({
  accounts,
  loadAccounts,
  renameAccount,
}: AccountSettingsControllerOptions) {
  const [selectedAccountId, setSelectedAccountId] = useState("");
  const [gameSettings, setGameSettings] = useState<SettingsMap>({});
  const [gameSettingsLoading, setGameSettingsLoading] = useState(false);
  const [gameSettingsLoadError, setGameSettingsLoadError] = useState<string | null>(null);
  const [gameSettingsChanged, setGameSettingsChanged] = useState(false);
  const [gameSettingsSaving, setGameSettingsSaving] = useState(false);
  const [gameSettingsTab, setGameSettingsTab] = useState<GameSettingsTab>("launch");
  const [accountNicknameDraft, setAccountNicknameDraft] = useState("");
  const [accountWinXDraft, setAccountWinXDraft] = useState<number | null>(null);
  const [accountWinYDraft, setAccountWinYDraft] = useState<number | null>(null);
  const gameSettingsRequestRef = useRef(0);
  const savedSettingsRef = useRef<SettingsMap>({});
  const latestSettingsRef = useRef<SettingsMap>({});
  const selectedIdRef = useRef(selectedAccountId);
  selectedIdRef.current = selectedAccountId;
  const previousAccountRef = useRef<AccountMeta | undefined>(undefined);

  const loadGameSettings = useCallback(async (accountId: string) => {
    const requestId = ++gameSettingsRequestRef.current;
    setGameSettingsLoading(true);
    setGameSettingsLoadError(null);
    try {
      const data = await invokeCommand<SettingsMap>("get_account_settings", { accountId });
      if (requestId !== gameSettingsRequestRef.current) return;
      savedSettingsRef.current = data;
      latestSettingsRef.current = data;
      setGameSettings(data);
      setGameSettingsChanged(false);
    } catch (error) {
      if (requestId !== gameSettingsRequestRef.current) return;
      setGameSettings({});
      setGameSettingsChanged(false);
      setGameSettingsLoadError(String(error));
      showToast("error", `加载账号游戏配置失败: ${error}`);
    } finally {
      if (requestId === gameSettingsRequestRef.current) setGameSettingsLoading(false);
    }
  }, []);

  useEffect(() => {
    const account = accounts.find((candidate) => candidate.id === selectedAccountId);
    const previous = previousAccountRef.current;
    previousAccountRef.current = account;
    if (!account) {
      ++gameSettingsRequestRef.current;
      setGameSettings({});
      setGameSettingsChanged(false);
      setGameSettingsLoading(false);
      setGameSettingsLoadError(null);
      return;
    }
    if (previous?.id === account.id) {
      // Refresh saved metadata without replacing fields the user has edited.
      setAccountNicknameDraft((draft) => draft === (previous.display_name || previous.id)
        ? (account.display_name || account.id) : draft);
      setAccountWinXDraft((draft) => draft === (previous.window_x ?? null)
        ? (account.window_x ?? null) : draft);
      setAccountWinYDraft((draft) => draft === (previous.window_y ?? null)
        ? (account.window_y ?? null) : draft);
      // Other accounts' changes must not reload this account's game settings.
      return;
    }
    setAccountNicknameDraft(account.display_name || account.id);
    setAccountWinXDraft(account.window_x ?? null);
    setAccountWinYDraft(account.window_y ?? null);
    void loadGameSettings(selectedAccountId);
  }, [accounts, loadGameSettings, selectedAccountId]);

  useEffect(() => {
    if (!selectedAccountId) return;
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    void listenEvent<{ accountId: string }>("account-settings-updated", (event) => {
      if (event.payload.accountId === selectedAccountId && !gameSettingsSaving && !gameSettingsChanged) {
        void loadGameSettings(selectedAccountId);
      }
    }).then((stopListening) => {
      if (cancelled) stopListening();
      else unlisten = stopListening;
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [gameSettingsChanged, gameSettingsSaving, loadGameSettings, selectedAccountId]);

  const updateGameSetting = (key: string, value: unknown) => {
    if (gameSettingsLoadError) return;
    const previous = latestSettingsRef.current;
    const next = key === FRAMERATE_CAP_KEY ? writeFramerateCap(previous, Number(value)) : { ...previous, [key]: value };
    latestSettingsRef.current = next;
    setGameSettings(next);
    setGameSettingsChanged(true);
  };

  const saveAccount = async (quiet = false) => {
    if (!selectedAccountId) return true;
    // Capture the draft before metadata writes yield; switching accounts during
    // a rename must not send the newly selected account's fields to this one.
    const sentSettings = latestSettingsRef.current;
    const settingsPatch = changedSettings(savedSettingsRef.current, sentSettings);
    setGameSettingsSaving(true);
    try {
      const account = accounts.find((candidate) => candidate.id === selectedAccountId);
      if (!account) return false;
      const nextName = accountNicknameDraft.trim();
      if (nextName && nextName !== (account.display_name || account.id)) {
        if (!(await renameAccount(selectedAccountId, nextName))) return false;
      }
      if (accountWinXDraft !== (account.window_x ?? null) || accountWinYDraft !== (account.window_y ?? null)) {
        await invokeCommand("set_account_window_position", {
          accountId: selectedAccountId,
          windowX: accountWinXDraft,
          windowY: accountWinYDraft,
        });
      }
      if (gameSettingsChanged) {
        const sent = sentSettings;
        const patch = settingsPatch;
        if (Object.keys(patch).length) {
          const saved = await invokeCommand<SettingsMap>("patch_account_settings", { accountId: selectedAccountId, patch });
          if (selectedIdRef.current === selectedAccountId) {
            const pending = changedSettings(sent, latestSettingsRef.current);
            savedSettingsRef.current = saved;
            latestSettingsRef.current = { ...saved, ...pending };
            setGameSettings(latestSettingsRef.current);
            setGameSettingsChanged(Object.keys(pending).length > 0);
          }
          await emitEvent("account-settings-updated", { accountId: selectedAccountId }).catch(error => console.warn("Settings saved; notification failed", error));
        } else setGameSettingsChanged(false);
      }
      await loadAccounts();
      if (!quiet) showToast("success", `账号 "${nextName || account.display_name || account.id}" 的设置已保存`);
      return true;
    } catch (error) {
      showToast("error", `保存账号设置失败: ${error}`);
      return false;
    } finally {
      setGameSettingsSaving(false);
    }
  };

  const snapshotSystemSettings = async () => {
    if (!selectedAccountId) return;
    try {
      const settings = await invokeCommand<SettingsMap>("snapshot_system_settings_to_account", {
        accountId: selectedAccountId,
      });
      savedSettingsRef.current = settings;
      latestSettingsRef.current = settings;
      setGameSettings(settings);
      setGameSettingsLoadError(null);
      setGameSettingsChanged(false);
      await loadAccounts();
      await emitEvent("account-settings-updated", { accountId: selectedAccountId });
      showToast("success", "已快照系统配置到当前账号");
    } catch (error) {
      showToast("error", `快照系统配置失败: ${error}`);
    }
  };

  const toggleCustomizedSettings = async (accountId: string, customized: boolean) => {
    try {
      if (customized) {
        await invokeCommand("snapshot_system_settings_to_account", { accountId });
      } else {
        await invokeCommand("set_settings_customized", { accountId, customized: false });
      }
      await loadAccounts();
      if (accountId === selectedAccountId) await loadGameSettings(accountId);
      await emitEvent("account-settings-updated", { accountId });
    } catch (error) {
      showToast("error", `切换配置模式失败: ${error}`);
    }
  };

  const selectedAccount = accounts.find((account) => account.id === selectedAccountId);
  const accountHasChanges = useMemo(() => {
    if (!selectedAccount) return false;
    return !!(
      (accountNicknameDraft.trim()
        && accountNicknameDraft.trim() !== (selectedAccount.display_name || selectedAccount.id))
      || accountWinXDraft !== (selectedAccount.window_x ?? null)
      || accountWinYDraft !== (selectedAccount.window_y ?? null)
      || gameSettingsChanged
    );
  }, [accountNicknameDraft, accountWinXDraft, accountWinYDraft, gameSettingsChanged, selectedAccount]);

  return {
    selectedAccountId,
    setSelectedAccountId,
    selectedAccount,
    accountHasChanges,
    accountNicknameDraft,
    setAccountNicknameDraft,
    accountWinXDraft,
    setAccountWinXDraft,
    accountWinYDraft,
    setAccountWinYDraft,
    gameSettings,
    gameSettingsLoading,
    gameSettingsLoadError,
    gameSettingsSaving,
    gameSettingsTab,
    setGameSettingsTab,
    loadGameSettings,
    updateGameSetting,
    saveAccount,
    snapshotSystemSettings,
    toggleCustomizedSettings,
  };
}
