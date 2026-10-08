import { useEffect, useId, useRef, useState } from "react";

const RESOLUTION_OPTIONS = ["800x600", "1024x768", "1280x720", "1600x900", "1920x1080", "2560x1440", "3840x2160"];

function normalizeResolution(value: string): string | null {
  const match = value.trim().match(/^(\d+)\s*[x×*]\s*(\d+)$/i);
  if (!match) return null;
  const width = Number(match[1]), height = Number(match[2]);
  if (!Number.isSafeInteger(width) || !Number.isSafeInteger(height)) return null;
  return `${Math.max(800, Math.min(7680, width))}x${Math.max(600, Math.min(4320, height))}`;
}

export function ResolutionInput({ value, onChange, onCommit, label = "分辨率", disabled, onClick }: {
  value: string;
  onChange: (value: string) => void;
  onCommit?: () => void;
  label?: string;
  disabled?: boolean;
  onClick?: React.MouseEventHandler<HTMLInputElement>;
}) {
  const id = useId();
  const [draft, setDraft] = useState(value);
  const [invalid, setInvalid] = useState(false);
  const focused = useRef(false);
  useEffect(() => { if (!focused.current) setDraft(value); }, [value]);
  const commit = () => {
    const normalized = normalizeResolution(draft);
    setInvalid(normalized === null);
    if (normalized === null) return;
    setDraft(normalized);
    if (normalized !== value) onChange(normalized);
    onCommit?.();
  };
  return (
    <div className="resolution-input">
      <div className="combo-input">
        <input aria-label={label} aria-invalid={invalid || undefined} aria-describedby={invalid ? `${id}-error` : undefined}
          value={draft} list={`${id}-options`} disabled={disabled} placeholder="1280x720"
          title="可输入自定义分辨率，最小 800 × 600" onClick={onClick}
          onFocus={() => { focused.current = true; }}
          onChange={event => { setDraft(event.target.value); setInvalid(false); }}
          onBlur={() => { focused.current = false; commit(); }}
          onKeyDown={event => {
            if (event.key === "Enter") { event.preventDefault(); event.stopPropagation(); event.currentTarget.blur(); }
            if (event.key === "Escape" && (draft !== value || invalid)) { event.stopPropagation(); setDraft(value); setInvalid(false); }
          }} />
        <datalist id={`${id}-options`}>{RESOLUTION_OPTIONS.map(option => <option key={option} value={option} />)}</datalist>
      </div>
      {invalid && <span id={`${id}-error`} role="alert" className="resolution-error">请输入宽 × 高，例如 1280x720</span>}
    </div>
  );
}
