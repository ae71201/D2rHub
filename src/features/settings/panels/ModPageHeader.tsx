import type { ReactNode } from "react";
import "./modLibrary.css";

export function ModPageHeader({ title, description, children }: {
  title: string;
  description: string;
  children?: ReactNode;
}) {
  return <header className="mod-page-header">
    <div><h2>{title}</h2><p>{description}</p></div>
    {children && <div className="mod-page-header-actions">{children}</div>}
  </header>;
}
