import { useEffect, useMemo, useRef, useState } from "react";
import { Check, FolderOpen, PackageOpen, PackagePlus, Plus, RefreshCw, Trash2, X } from "lucide-react";

import { Button } from "../../../components/ui/Button";
import { ModResourceLibrary } from "./ModResourceLibrary";
import { ModCatalogEntry } from "./ModCatalogEntry";
import "./modLibrary.css";
import { Modal } from "../../../components/ui/Modal";
import { showToast } from "../../../components/ui/Toast";
import type { AccountMeta, ModCapsule } from "../../../store/types";
import type { ModCapsuleController } from "../../modCapsules/useModCapsulePool";

interface ModCatalogManagerProps {
  catalog: ModCapsuleController;
  accounts: AccountMeta[];
  autoOpenAdd?: boolean;
  initialEdition?: string;
  edition?: "CN" | "Global";
  onEditionChange?: (edition: "CN" | "Global") => void;
  language?: string | null;
  minimalMode?: boolean;
  onProcess: (capsule: ModCapsule) => Promise<void> | void;
  onCreate?: (edition: "CN" | "Global") => Promise<void> | void;
}

const COPY = {
  "zh-CN": {
    title: "Mod 管理", description: "管理游戏加载的内容。在账号卡片中选用 Mod，重启游戏后生效。",
    scan: "扫描目录", openFolder: "打开文件夹", openFolderTitle: "打开当前版本的 mods 文件夹", editions: "游戏版本", cn: "国服", global: "国际服", add: "添加自定义参数",
    addLabel: "添加自定义共享参数", customTitle: "自定义共享参数",
    customHelp: "用于保留旧账号或特殊启动写法；普通 Mod 会由目录扫描自动加入。",
    cancel: "取消", save: "保存", addSuccess: "自定义参数已加入 Mod 列表",
    updateSuccess: "共享参数已更新，引用它的账号和启动方案已同步",
    autoExitEnabled: "死亡自动退房已启用，重新启动游戏后生效", autoExitDisabled: "死亡自动退房已停用，重新启动游戏后生效",
    deleteScannedTitle: (name: string) => `删除 Mod“${name}”？`,
    deleteScannedDescription: (name: string) => `删除此参数会同时永久删除游戏目录中的 Mod 文件夹“mods\\${name}”。此操作不可撤销。`,
    deleteScannedAction: "删除参数和 Mod", deleteScannedSuccess: (name: string) => `Mod“${name}”及其参数已删除`,
    deleteCustomTitle: "删除自定义参数？",
    deleteCustomDescription: "只会删除这条自定义参数，不会删除或修改任何 Mod 文件。",
    deleteCustomAction: "删除参数", deleteCustomSuccess: "自定义参数已删除",
    empty: (value: string) => `没有扫描到 ${value === "CN" ? "国服" : "国际服"} Mod`,
  },
  "en-US": {
    title: "Mod Management", description: "Manage what your game loads. Choose a Mod on an account card, then restart the game.",
    scan: "Scan folders", openFolder: "Open folder", openFolderTitle: "Open the mods folder for this game edition", editions: "Game edition", cn: "China", global: "Global", add: "Add custom arguments",
    addLabel: "Add shared custom arguments", customTitle: "Shared custom arguments",
    customHelp: "Keep legacy or specialized launch syntax here. Regular Mods are discovered from the game directory.",
    cancel: "Cancel", save: "Save", addSuccess: "Custom arguments added to the Mod list",
    updateSuccess: "Shared arguments updated across accounts and launch schemes",
    autoExitEnabled: "Auto-exit on death enabled; restart the game to apply", autoExitDisabled: "Auto-exit on death disabled; restart the game to apply",
    deleteScannedTitle: (name: string) => `Delete “${name}”?`,
    deleteScannedDescription: (name: string) => `Deleting these arguments will also permanently delete the corresponding Mod folder, “mods\\${name}”. This cannot be undone.`,
    deleteScannedAction: "Delete arguments and Mod", deleteScannedSuccess: (name: string) => `“${name}” and its arguments were deleted`,
    deleteCustomTitle: "Delete custom arguments?",
    deleteCustomDescription: "Only these custom arguments will be deleted. No Mod files will be deleted or changed.",
    deleteCustomAction: "Delete arguments", deleteCustomSuccess: "Custom arguments deleted",
    empty: (value: string) => `No ${value === "CN" ? "China" : "Global"} Mods found`,
  },
} as const;

export function ModCatalogManager({ catalog, accounts, autoOpenAdd, initialEdition, edition: controlledEdition, onEditionChange, language, minimalMode = false, onProcess, onCreate }: ModCatalogManagerProps) {
  const isEnglish = language === "en-US";
  const copy = COPY[isEnglish ? "en-US" : "zh-CN"];
  const [localEdition, setLocalEdition] = useState<"CN" | "Global">(
    (initialEdition ?? catalog.pool?.capsules[0]?.edition) === "Global" ? "Global" : "CN",
  );
  const edition = controlledEdition ?? localEdition;
  const setEdition = (next: "CN" | "Global") => { setLocalEdition(next); onEditionChange?.(next); };
  const [generateOpen, setGenerateOpen] = useState(false);
  const [downloadBusy, setDownloadBusy] = useState(false);
  const operationLock = useRef(false);
  const [operationBusy, setOperationBusy] = useState(false);
  const busy = catalog.loading || operationBusy;
  const [addOpen, setAddOpen] = useState(false);
  const [addDraft, setAddDraft] = useState("");
  const [deleteTarget, setDeleteTarget] = useState<ModCapsule | null>(null);
  const capsules = useMemo(
    () => (catalog.pool?.capsules ?? []).filter((capsule) => capsule.edition === edition),
    [catalog.pool, edition],
  );

  useEffect(() => {
    if (autoOpenAdd) {
      setAddOpen(true);
      if (initialEdition === "CN" || initialEdition === "Global") setEdition(initialEdition);
    }
  }, [autoOpenAdd, initialEdition]);

  const run = async (operation: () => Promise<unknown>, success: string) => {
    if (operationLock.current) return false;
    operationLock.current = true;
    setOperationBusy(true);
    try {
      await operation();
      showToast("success", success);
      return true;
    } catch (error) {
      showToast("error", String(error));
      return false;
    } finally {
      operationLock.current = false;
      setOperationBusy(false);
    }
  };

  const deleteIsScanned = deleteTarget?.origin === "scanned";
  const deleteTitle = deleteTarget
    ? deleteIsScanned
      ? copy.deleteScannedTitle(deleteTarget.name)
      : copy.deleteCustomTitle
    : "";
  const deleteDescription = deleteTarget
    ? deleteIsScanned
      ? copy.deleteScannedDescription(deleteTarget.name)
      : copy.deleteCustomDescription
    : "";
  const closeDeleteConfirmation = () => {
    if (!busy) setDeleteTarget(null);
  };

  return (
    <div className="mod-catalog-manager mod-library">
      <header className="mod-processing-header">
        <div>
          <h2>{copy.title}</h2>
          <p>{minimalMode
            ? (isEnglish
              ? "Scan, organize, assign, and process shared Mods for your accounts."
              : "扫描、整理、分配并加工账号共用的 Mod。")
            : copy.description}</p>
        </div>
        <div className="mod-processing-header-actions">
          <Button
            size="sm"
            variant="ghost"
            title={copy.openFolderTitle}
            onClick={() => void catalog.openDirectory?.(edition).catch((error) => {
              showToast("error", String(error));
            })}
          >
            <FolderOpen size={13} />{copy.openFolder}
          </Button>
          <Button size="sm" variant="ghost" loading={busy} onClick={() => void catalog.scan()}>
            <RefreshCw size={13} />{copy.scan}
          </Button>
        </div>
      </header>

      <div className="mod-library-navigation">
        <div className="mod-library-views" role="group" aria-label={isEnglish ? "Mod library views" : "Mod 库视图"}>
          <button type="button" aria-pressed={!generateOpen} onClick={() => setGenerateOpen(false)}>{isEnglish ? "Installed" : "已安装"}</button>
          <button type="button" aria-pressed={generateOpen} onClick={() => setGenerateOpen(true)}>{isEnglish ? "Downloads & updates" : "下载与更新"}</button>
        </div>
        <div className="mod-catalog-editions" role="tablist" aria-label={copy.editions}>
          {(["CN", "Global"] as const).map((value) => (
            <button key={value} type="button" role="tab" aria-selected={edition === value} disabled={downloadBusy} onClick={() => { setEdition(value); setAddOpen(false); setAddDraft(""); }}>
              {value === "CN" ? copy.cn : copy.global}
            </button>
          ))}
        </div>
      </div>

      {generateOpen ? <ModResourceLibrary edition={edition} en={isEnglish} catalog={catalog} onBusy={setDownloadBusy} /> : <>
      {addOpen && (
        <section className="mod-catalog-add" aria-label={copy.addLabel}>
          <div>
            <strong>{copy.customTitle}</strong>
            <small>{copy.customHelp}</small>
          </div>
          <input
            className="settings-input"
            value={addDraft}
            disabled={busy}
            aria-label={copy.customTitle}
            autoFocus
            placeholder={isEnglish ? "Example: -mod MyMod -txt -assettestmode 1" : "例如：-mod MyMod -txt -assettestmode 1"}
            onChange={(event) => setAddDraft(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Escape") { event.stopPropagation(); if (!busy) setAddOpen(false); }
              if (event.key === "Enter" && addDraft.trim() && !busy) {
                void run(() => catalog.add(edition, addDraft.trim()), copy.addSuccess).then((saved) => {
                  if (saved) { setAddOpen(false); setAddDraft(""); }
                });
              }
            }}
          />
          <div>
            <Button size="sm" variant="ghost" onClick={() => setAddOpen(false)}><X size={12} />{copy.cancel}</Button>
            <Button
              size="sm"
              variant="primary"
              disabled={!addDraft.trim() || busy}
              onClick={() => void run(() => catalog.add(edition, addDraft.trim()), copy.addSuccess).then((saved) => {
                if (saved) { setAddOpen(false); setAddDraft(""); }
              })}
            ><Check size={12} />{copy.save}</Button>
          </div>
        </section>
      )}

      {catalog.error && <p className="mod-catalog-error" role="status">{catalog.error}</p>}
      <div className="mod-catalog-list" aria-busy={busy}>
        {capsules.map(capsule => <ModCatalogEntry key={capsule.id} capsule={capsule}
          accounts={accounts} en={isEnglish} minimalMode={minimalMode} busy={busy}
          onUpdate={args => run(() => catalog.update(capsule.id, args), copy.updateSuccess)}
          onDelete={() => setDeleteTarget(capsule)}
          onProcess={() => void onProcess(capsule)}
          onToggleDeathExit={enabled => void run(() => catalog.setAutoExitOnDeathEnabled(capsule.id, enabled), enabled ? copy.autoExitEnabled : copy.autoExitDisabled)} />)}
        {!catalog.loading && capsules.length === 0 && (
          <div className="mod-catalog-empty">
            <PackageOpen size={22} />
            <strong>{copy.empty(edition)}</strong>
            <p>{isEnglish ? "Download a ready-to-use Mod, or scan Mods already in your game folder." : "下载成品 Mod，或将已有 Mod 放入游戏目录后重新扫描。"}</p>
            <Button size="sm" variant="primary" onClick={() => setGenerateOpen(true)}><PackagePlus size={13} />{isEnglish ? "Download Mods & processor" : "下载 Mod 与加工器"}</Button>
          </div>
        )}
      </div>
      <details className="mod-library-advanced" open={addOpen || undefined}>
        <summary>{isEnglish ? "Custom processing & launch presets" : "自定义加工与启动预设"}</summary>
        {onCreate && <Button size="sm" variant="secondary" disabled={busy} onClick={() => void onCreate(edition)}>
          <PackagePlus size={13} />{isEnglish ? "Process a new Mod" : "加工新 Mod"}
        </Button>}
        <p>{copy.customHelp}</p>
        <Button size="sm" variant="ghost" disabled={busy} onClick={() => { setAddOpen(true); setAddDraft(""); }}><Plus size={13} />{copy.add}</Button>
      </details>
      </>}
      <Modal
        open={deleteTarget !== null}
        onClose={closeDeleteConfirmation}
        title={deleteTitle}
        width="max-w-sm"
        dismissible={!busy}
        footer={(
          <>
            <Button autoFocus size="sm" variant="secondary" disabled={busy} onClick={closeDeleteConfirmation}>
              {copy.cancel}
            </Button>
            <Button
              size="sm"
              variant="danger"
              loading={busy}
              onClick={() => {
                if (!deleteTarget) return;
                const target = deleteTarget;
                const success = target.origin === "scanned"
                  ? copy.deleteScannedSuccess(target.name)
                  : copy.deleteCustomSuccess;
                void run(() => catalog.remove(target.id), success).then((removed) => {
                  if (removed) setDeleteTarget(null);
                });
              }}
            >
              <Trash2 size={12} />
              {deleteIsScanned ? copy.deleteScannedAction : copy.deleteCustomAction}
            </Button>
          </>
        )}
      >
        <p className="mod-catalog-delete-description">{deleteDescription}</p>
      </Modal>
    </div>
  );
}
