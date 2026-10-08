import { useCallback, useEffect, useId, useLayoutEffect, useRef, useState, type KeyboardEvent } from "react";
import { createPortal } from "react-dom";
import { Check, ChevronDown, ListChecks, Play, Square, X } from "lucide-react";
import type { BatchMode, BatchSelection } from "../../hooks/useAccountBatch";
import { useI18n } from "../../i18n";
import "../../styles/accountSelection.css";

type SelectionMode = Exclude<BatchMode, null>;
interface Props {
  selection: BatchSelection;
  selectableIds: Record<SelectionMode, string[]>;
  disabled?: boolean;
  uncertain?: boolean;
  onSelectAll: (mode: SelectionMode) => void;
  onClear: () => void;
}

export function AccountSelectionMenu({ selection, selectableIds, disabled = false, uncertain = false, onSelectAll, onClear }: Props) {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  const [position, setPosition] = useState({ left: 8, top: 8 });
  const trigger = useRef<HTMLButtonElement>(null);
  const menu = useRef<HTMLDivElement>(null);
  const menuId = useId();
  const unavailable = disabled || uncertain || !Object.values(selectableIds).some(ids => ids.length);
  const close = useCallback((restoreFocus = false) => {
    setOpen(false);
    if (restoreFocus) trigger.current?.focus();
  }, []);

  useEffect(() => { if (unavailable) close(); }, [unavailable, close]);
  useLayoutEffect(() => {
    if (!open) return;
    const place = () => {
      const rect = trigger.current?.getBoundingClientRect();
      const box = menu.current?.getBoundingClientRect();
      if (!rect || !box) return;
      setPosition({
        left: Math.max(8, Math.min(rect.left, window.innerWidth - box.width - 8)),
        top: Math.max(8, Math.min(rect.bottom + 6, window.innerHeight - box.height - 8)),
      });
    };
    place();
    window.addEventListener("resize", place);
    window.addEventListener("scroll", place, true);
    return () => { window.removeEventListener("resize", place); window.removeEventListener("scroll", place, true); };
  }, [open]);
  useEffect(() => {
    if (!open) return;
    menu.current?.querySelector<HTMLButtonElement>('button:not(:disabled)')?.focus();
    const outside = (event: PointerEvent | FocusEvent) => {
      if (event.target instanceof Node && !trigger.current?.contains(event.target) && !menu.current?.contains(event.target)) close();
    };
    document.addEventListener("pointerdown", outside, true);
    document.addEventListener("focusin", outside);
    return () => { document.removeEventListener("pointerdown", outside, true); document.removeEventListener("focusin", outside); };
  }, [open, close]);

  const navigate = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); close(true); return; }
    if (event.key === "Tab") { close(true); return; }
    const items = Array.from(menu.current?.querySelectorAll<HTMLButtonElement>('button:not(:disabled)') ?? []);
    const current = items.indexOf(document.activeElement as HTMLButtonElement);
    const index = event.key === "Home" ? 0 : event.key === "End" ? items.length - 1
      : event.key === "ArrowDown" ? (current + 1) % items.length
      : event.key === "ArrowUp" ? (current - 1 + items.length) % items.length : null;
    if (index !== null && items.length) { event.preventDefault(); items[index]?.focus(); }
  };
  const selected = !!selection.mode;
  return <div className="account-selection-control" data-i18n-skip>
    <button ref={trigger} type="button" className="control-btn account-selection-trigger" disabled={unavailable}
      data-selected={selected || undefined} aria-haspopup="menu" aria-expanded={open}
      aria-controls={open ? menuId : undefined}
      title={t(uncertain ? "account.selection.uncertain" : "account.selection.menuHint")}
      onClick={() => setOpen(value => !value)}
      onKeyDown={event => {
        if (event.key === "ArrowDown" || event.key === "ArrowUp") { event.preventDefault(); setOpen(true); }
      }}>
      <ListChecks size={13} strokeWidth={1.8} aria-hidden="true" />
      <span>{selected ? t("account.selection.selected", { count: selection.ids.length }) : t("account.selection.open")}</span>
      <ChevronDown size={11} aria-hidden="true" />
    </button>
    {selected && <button type="button" className="control-btn account-selection-clear" disabled={disabled}
      aria-label={t("account.selection.clear")} title={t("account.selection.clearHint")} onClick={onClear}>
      <X size={13} aria-hidden="true" />
    </button>}
    {open && !unavailable && createPortal(<div ref={menu} id={menuId} role="menu" aria-label={t("account.selection.label")}
      className="account-selection-menu" style={position} onKeyDown={navigate} data-i18n-skip>
      {(["launch", "close"] as const).map(mode => {
        const ids = selectableIds[mode];
        const allSelected = selection.mode === mode && ids.length > 0 && ids.every(id => selection.ids.includes(id));
        const Icon = mode === "launch" ? Play : Square;
        return <button key={mode} type="button" role="menuitem" disabled={!ids.length}
          onClick={() => { onSelectAll(mode); close(true); }}>
          <Icon size={13} aria-hidden="true" />
          <span>{t(mode === "launch" ? "account.selection.launch" : "account.selection.close")}</span>
          <small>{ids.length}</small>
          <span className="account-selection-menu-check" aria-hidden="true">{allSelected && <Check size={13} />}</span>
        </button>;
      })}
    </div>, document.body)}
  </div>;
}
