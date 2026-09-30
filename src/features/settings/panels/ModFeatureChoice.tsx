import { Check } from "lucide-react";

/** Shared module selection for individual and batch processing. */
export function ModFeatureChoice({ title, detail, checked, locked = false, lockLabel, disabled = false, onChange }: {
  title: string;
  detail: string;
  checked: boolean;
  locked?: boolean;
  lockLabel?: string;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
}) {
  return <label className="mod-feature-choice" data-selected={checked} data-disabled={disabled || locked}>
    <input type="checkbox" checked={checked} disabled={disabled || locked}
      onChange={event => onChange(event.target.checked)} />
    <span><strong>{title}</strong><small>{detail}</small>
      {locked && <em><Check size={11} aria-hidden="true" />{lockLabel}</em>}
    </span>
  </label>;
}
