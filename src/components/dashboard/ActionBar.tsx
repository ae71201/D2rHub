import React from "react";

export function ActionBar({ children, disabled = false }: { children: React.ReactNode; disabled?: boolean }) {
  return (
    <fieldset disabled={disabled} className="dashboard-actionbar flex items-center gap-3 px-5 py-2.5 shrink-0">
      {children}
    </fieldset>
  );
}
