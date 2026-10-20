import { useId } from "react";

export function RoomToolProcessingOptions({ en, enabled, disabled, onChange }: {
  en: boolean;
  enabled: boolean;
  disabled?: boolean;
  onChange: (enabled: boolean) => void;
}) {
  const descriptionId = useId();
  return <div className="mod-feature-options">
    <label>
      <input type="checkbox" checked={enabled} disabled={disabled} aria-describedby={descriptionId}
        onChange={event => onChange(event.target.checked)} />
      <span>{en ? "Default to Hell when creating a game" : "创建房间默认地狱"}</span>
    </label>
    <p id={descriptionId}>{en
      ? "Automatically select Hell when the in-game create form opens. Turn off to choose the difficulty yourself."
      : "打开局内创建表单时自动选择地狱；关闭后手动选择难度。"}</p>
  </div>;
}
