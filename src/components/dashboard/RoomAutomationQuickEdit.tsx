import { RoomCollaborationMode, roomModeLabel } from "../../features/roomAutomation/RoomCollaborationMode";
import type { CollaborationMode } from "../../features/roomAutomation/types";
import { readText, writeText } from "@tauri-apps/plugin-clipboard-manager";
import { copyRoomRule, parseRoomRule } from "../../features/roomAutomation/ruleShare";
import {
  AlertCircle,
  Copy,
  ClipboardPaste,
  ChevronDown,
  DoorOpen,
  LoaderCircle,
  Settings2,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";

import { roomAutomationGateway } from "../../features/roomAutomation/gateway";
import type {
  RoomAutomationConfig,
  RoomAutomationConfigSnapshot,
} from "../../features/roomAutomation/types";
import {
  normalizeSettingsLanguage,
  type SettingsLanguage,
} from "../../features/settings/settingsRegistry";
import { Button } from "../ui/Button";
import "./roomQuickSettings.css";
import { AnchoredPanel } from "../ui/AnchoredPanel";
import { showToast } from "../ui/Toast";

interface RoomAutomationQuickEditProps {
  active: boolean;
  language?: string | null;
  onOpenSettings: () => void;
}

interface RoomNamingDraft {
  collaborationMode: CollaborationMode;
  prefix: string;
  password: string;
  nextSequence: string;
  sequenceWidth: string;
}

interface QuickEditCopy {
  trigger: string;
  triggerTitle: string;
  title: string;
  description: string;
  preview: string;
  prefix: string;
  sequence: string;
  width: string;
  password: string;
  passwordPlaceholder: string;
  fullSettings: string;
  saving: string;
  savedWithWarning: string;
  saveFailed: string;
  invalidRoomText: string;
  roomTooLong: string;
  invalidSequence: string;
}

const QUICK_EDIT_COPY: Record<SettingsLanguage, QuickEditCopy> = {
  "zh-CN": {
    trigger: "跟房配置",
    triggerTitle: "修改下一局跟房配置",
    title: "跟房设置",
    description: "切换本机拼房身份，与队友同步房间规则。",
    preview: "预览",
    prefix: "房名开头",
    sequence: "下一个序号",
    width: "位数",
    password: "房间密码",
    passwordPlaceholder: "无密码",
    fullSettings: "完整设置",
    saving: "应用中",
    savedWithWarning: "房间规则与模式已保存，但运行配置未能立即应用",
    saveFailed: "无法更新下一局房间",
    invalidRoomText: "房名前缀不能为空；支持中文和可粘贴的特殊字符，不支持换行或控制字符。",
    roomTooLong: "生成后的房名和密码不能超过 15 个字符。",
    invalidSequence: "序号应为 0–4294967295 的整数，位数应为 1–6。",
  },
  "en-US": {
    trigger: "Follow Config",
    triggerTitle: "Edit the next room automation settings",
    title: "Next room",
    description: "Choose this computer’s role and synchronize the room rule.",
    preview: "Preview",
    prefix: "Room prefix",
    sequence: "Next sequence",
    width: "Width",
    password: "Room password",
    passwordPlaceholder: "No password",
    fullSettings: "Full settings",
    saving: "Applying",
    savedWithWarning: "Room rule and mode were saved, but the runtime could not apply them immediately",
    saveFailed: "Could not update the next room",
    invalidRoomText: "A room name prefix is required. Chinese and pasteable special characters are supported; line breaks and control characters are not.",
    roomTooLong: "Generated room names and passwords are limited to 15 UTF-16 characters.",
    invalidSequence: "Sequence must be an integer from 0–4294967295; width must be 1–6.",
  },
};

function namingDraft(config: RoomAutomationConfig): RoomNamingDraft {
  return {
    collaborationMode: config.collaboration_mode ?? "default",
    prefix: config.name_prefix,
    password: config.password,
    nextSequence: String(config.next_sequence),
    sequenceWidth: String(config.sequence_width),
  };
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function validateDraft(draft: RoomNamingDraft, copy: QuickEditCopy): string | null {
  const hasRoomControlCharacters = (value: string) => Array.from(value).some((character) => {
    const code = character.codePointAt(0)!;
    return code < 0x20 || (code >= 0x7f && code <= 0x9f) || code === 0x2028 || code === 0x2029;
  });
  if (!draft.prefix || hasRoomControlCharacters(draft.prefix) || hasRoomControlCharacters(draft.password)) {
    return copy.invalidRoomText;
  }
  const nextSequence = Number(draft.nextSequence);
  const sequenceWidth = Number(draft.sequenceWidth);
  if (!draft.nextSequence.trim() || !draft.sequenceWidth.trim()
    || !Number.isSafeInteger(nextSequence) || nextSequence < 0 || nextSequence > 4_294_967_295
    || !Number.isInteger(sequenceWidth) || sequenceWidth < 1 || sequenceWidth > 6) {
    return copy.invalidSequence;
  }
  const roomName = `${draft.prefix}${String(nextSequence).padStart(sequenceWidth, "0")}`;
  const utf16Length = (value: string) => value.length;
  if (utf16Length(roomName) > 15 || utf16Length(draft.password) > 15) return copy.roomTooLong;
  return null;
}

export function RoomAutomationQuickEdit({
  active,
  language,
  onOpenSettings,
}: RoomAutomationQuickEditProps) {
  const locale = normalizeSettingsLanguage(language);
  const copy = QUICK_EDIT_COPY[locale];
  const triggerRef = useRef<HTMLButtonElement>(null);
  const openRef = useRef(false);
  const draftDirtyRef = useRef(false);
  const resetChatHistoryRef = useRef(false);
  const draftRef = useRef<RoomNamingDraft | null>(null);
  const editedFields = useRef(new Set<keyof RoomNamingDraft>());
  const closingRef = useRef<Promise<boolean> | null>(null);
  const pastePending = useRef<Promise<void> | null>(null);
  const activeRef = useRef(active);
  activeRef.current = active;
  const mountedRef = useRef(true);
  const flushRef = useRef<() => Promise<boolean>>(async () => true);
  const [snapshot, setSnapshot] = useState<RoomAutomationConfigSnapshot | null>(null);
  const [draft, setDraft] = useState<RoomNamingDraft | null>(null);
  const [open, setOpen] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      void flushRef.current();
    };
  }, []);

  useEffect(() => {
    if (!active) {
      void flushRef.current();
      setSnapshot(null);
      setDraft(null);
      setOpen(false);
      return;
    }
    let disposed = false;
    let stopSync: (() => void) | undefined;
    void roomAutomationGateway.startSync({
      onConfig: (next) => {
        if (disposed) return;
        setSnapshot((current) => !current || next.generation >= current.generation ? next : current);
        // Keep an untouched open popover live when the settings page commits
        // an edit. Once the user starts typing, preserve that local draft until
        // it is saved when the editor closes.
        if (!draftDirtyRef.current) {
          draftRef.current = namingDraft(next.config);
          setDraft(draftRef.current);
          draftDirtyRef.current = false;
        }
        if (!next.config.enabled) void flushRef.current();
      },
      onStatus: () => undefined,
    }).then((stop) => {
      if (disposed) stop();
      else stopSync = stop;
    }).catch(() => {
      if (!disposed) setSnapshot(null);
    });
    return () => {
      disposed = true;
      stopSync?.();
    };
  }, [active]);

  const preview = useMemo(() => {
    if (!draft) return "";
    if (!draft.nextSequence.trim() || !draft.sequenceWidth.trim()) return "—";
    const nextSequence = Number(draft.nextSequence);
    const sequenceWidth = Number(draft.sequenceWidth);
    if (!Number.isSafeInteger(nextSequence) || !Number.isInteger(sequenceWidth)) return "—";
    const previewWidth = Math.min(6, Math.max(1, sequenceWidth));
    return `${draft.prefix}${String(nextSequence).padStart(previewWidth, "0")}`;
  }, [draft]);

  const updateDraft = (patch: Partial<RoomNamingDraft>) => {
    if (!draftRef.current) return;
    draftRef.current = { ...draftRef.current, ...patch };
    Object.keys(patch).forEach(key => editedFields.current.add(key as keyof RoomNamingDraft));
    draftDirtyRef.current = true;
    if (mountedRef.current) { setDraft(draftRef.current); setError(null); }
  };

  const closeEditor = (): Promise<boolean> => {
    if (closingRef.current) return closingRef.current;
    if (pastePending.current) return pastePending.current.then(() => flushRef.current());
    const pending = draftRef.current;
    if (!pending || !draftDirtyRef.current) {
      openRef.current = false;
      if (mountedRef.current) setOpen(false);
      return Promise.resolve(true);
    }
    const fail = (message: string) => {
      if (mountedRef.current && activeRef.current) { setError(message); setOpen(true); openRef.current = true; }
      else showToast("error", message);
      return false;
    };
    const invalid = validateDraft(pending, copy);
    if (invalid) return Promise.resolve(fail(invalid));
    const fields = new Set(editedFields.current);
    const reset = resetChatHistoryRef.current;
    if (mountedRef.current) { setSaving(true); setError(null); }
    const task = (async () => {
      try {
        const latest = await roomAutomationGateway.getConfig();
        // Preserve live sequence updates and settings this editor did not change.
        const candidate: RoomAutomationConfig = { ...latest.config };
        if (fields.has("collaborationMode")) candidate.collaboration_mode = pending.collaborationMode;
        if (fields.has("prefix")) candidate.name_prefix = pending.prefix;
        if (fields.has("password")) candidate.password = pending.password;
        if (fields.has("nextSequence")) candidate.next_sequence = Number(pending.nextSequence);
        if (fields.has("sequenceWidth")) candidate.sequence_width = Number(pending.sequenceWidth);
        const mergedError = validateDraft(namingDraft(candidate), copy);
        if (mergedError) return fail(mergedError);
        const outcome = reset
          ? await roomAutomationGateway.saveConfig(latest.generation, candidate, true)
          : await roomAutomationGateway.saveConfig(latest.generation, candidate);
        resetChatHistoryRef.current = false;
        draftDirtyRef.current = false;
        editedFields.current.clear();
        draftRef.current = namingDraft(outcome.snapshot.config);
        openRef.current = false;
        if (mountedRef.current) {
          setSnapshot(outcome.snapshot);
          setDraft(draftRef.current);
          setOpen(false);
        }
        if (outcome.apply_warning) showToast("warning", `${copy.savedWithWarning}: ${outcome.apply_warning}`);
        return true;
      } catch (saveError) {
        return fail(`${copy.saveFailed}: ${errorMessage(saveError)}`);
      } finally {
        closingRef.current = null;
        if (mountedRef.current) setSaving(false);
      }
    })();
    closingRef.current = task;
    return task;
  };
  flushRef.current = closeEditor;

  const openEditor = () => {
    if (openRef.current) { void closeEditor(); return; }
    if (!snapshot || closingRef.current) return;
    if (draftDirtyRef.current) { openRef.current = true; setOpen(true); return; }
    setError(null);
    resetChatHistoryRef.current = false;
    editedFields.current.clear();
    draftRef.current = namingDraft(snapshot.config);
    setDraft(draftRef.current);
    draftDirtyRef.current = false;
    openRef.current = true;
    setOpen(true);
  };

  if (!active || !snapshot?.config.enabled) return null;

  return (
    <div className="room-automation-quick">
      <button
        ref={triggerRef}
        type="button"
        className="control-btn room-automation-quick-trigger"
        data-active={open ? "true" : undefined}
        title={copy.triggerTitle}
        aria-haspopup="dialog"
        aria-expanded={open}
        onClick={openEditor}
      >
        <span className="room-automation-quick-dot" aria-hidden="true" />
        <DoorOpen size={13} strokeWidth={1.9} aria-hidden="true" />
        <span>{copy.trigger}</span>
        {snapshot?.config.collaboration_mode && snapshot.config.collaboration_mode !== "default" && <span className="text-2xs text-text-muted">{roomModeLabel(snapshot.config.collaboration_mode, locale === "en-US")}</span>}
        <ChevronDown size={12} strokeWidth={1.9} aria-hidden="true" />
      </button>

      {draft && (
        <AnchoredPanel open={open} anchor={triggerRef} title={copy.title} width={420}
          className="room-naming-panel" closeLabel={locale === "en-US" ? "Close" : "关闭"} onClose={() => { void closeEditor(); }}>
        <form
          className="room-automation-quick-popover"
          onSubmit={(event) => {
            event.preventDefault();
            void closeEditor();
          }}
        >
          <header className="room-automation-quick-header">
            <div>
              <p>{copy.description}</p>
            </div>
            <div className="room-automation-quick-preview">
              <span>{copy.preview}</span>
              <strong data-i18n-skip>{preview}</strong>
            </div>
          </header>

          <fieldset className="room-quick-edit-fields" disabled={saving}>
          <RoomCollaborationMode compact value={draft.collaborationMode} onChange={collaborationMode => updateDraft({ collaborationMode })} disabled={saving} en={locale === "en-US"} />
          <div className="room-quick-toolbar">
            <span>{locale === "en-US" ? "Room rule" : "房间规则"}</span>
            <div>
            <Button type="button" size="sm" variant="ghost" disabled={saving} onClick={async () => {
              const invalid = validateDraft(draft, copy);
              if (invalid) { setError(invalid); return; }
              try {
                await writeText(copyRoomRule({ ...snapshot.config, name_prefix: draft.prefix, password: draft.password,
                  next_sequence: Number(draft.nextSequence), sequence_width: Number(draft.sequenceWidth) }));
                showToast("success", locale === "en-US" ? "Room rule copied (includes password)" : "规则已复制（包含房间密码）");
              } catch (e) { setError(errorMessage(e)); }
            }}><Copy size={13} aria-hidden="true" />{locale === "en-US" ? "Copy" : "复制"}</Button>
            <Button type="button" size="sm" variant="ghost" disabled={saving} onClick={async () => {
              let finishPaste!: () => void;
              pastePending.current = new Promise(resolve => { finishPaste = resolve; });
              setSaving(true); setError(null);
              try {
                const rule = parseRoomRule(await readText());
                resetChatHistoryRef.current = true;
                updateDraft({ prefix: rule.name_prefix, password: rule.password, nextSequence: String(rule.next_sequence), sequenceWidth: String(rule.sequence_width) });
              } catch (e) { setError(errorMessage(e)); } finally { pastePending.current = null; finishPaste(); if (mountedRef.current) setSaving(false); }
            }}><ClipboardPaste size={13} aria-hidden="true" />{locale === "en-US" ? "Paste" : "粘贴"}</Button>
            </div>
          </div>
          <div className="room-automation-quick-fields">
            <label>
              <span>{copy.prefix}</span>
              <input
                type="text"
                className="line-input"
                maxLength={15}
                value={draft.prefix}
                onChange={(event) => updateDraft({ prefix: event.target.value })}
              />
            </label>
            <label>
              <span>{copy.sequence}</span>
              <input
                type="number"
                className="line-input"
                min={0}
                max={4_294_967_295}
                step={1}
                value={draft.nextSequence}
                onChange={(event) => updateDraft({ nextSequence: event.target.value })}
              />
            </label>
            <label>
              <span>{copy.width}</span>
              <input
                type="number"
                className="line-input"
                min={1}
                max={6}
                step={1}
                value={draft.sequenceWidth}
                onChange={(event) => updateDraft({ sequenceWidth: event.target.value })}
              />
            </label>
            <label className="room-automation-quick-password">
              <span>{copy.password}</span>
              <input
                type="password"
                className="line-input"
                maxLength={15}
                placeholder={copy.passwordPlaceholder}
                value={draft.password}
                onChange={(event) => updateDraft({ password: event.target.value })}
              />
            </label>
          </div>

          </fieldset>
          {error && (
            <p className="room-automation-quick-error" role="alert">
              <AlertCircle size={13} aria-hidden="true" />
              <span>{error}</span>
            </p>
          )}

          <footer className="room-automation-quick-footer">
            <span className="room-quick-save-status" role="status">
              {saving && <LoaderCircle className="animate-spin" size={12} aria-hidden="true" />}
              {saving ? copy.saving : locale === "en-US" ? "Saves when you leave" : "离开时自动保存"}
            </span>
            <button type="button" className="room-automation-quick-settings" disabled={saving}
              onClick={async () => { if (await closeEditor()) onOpenSettings(); }}>
              <Settings2 size={12} strokeWidth={1.9} aria-hidden="true" />
              {copy.fullSettings}
            </button>
          </footer>
        </form>
        </AnchoredPanel>
      )}
    </div>
  );
}
