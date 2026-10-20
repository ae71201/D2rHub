import { useId } from "react";
import type { CollaborationMode } from "./types";
import "./roomCollaborationMode.css";

export function roomModeLabel(mode: CollaborationMode | undefined, en: boolean) {
  return ({ default: en ? "Default" : "默认", host: en ? "Shared host" : "拼房主机", follower: en ? "Shared follower" : "拼房跟随" })[mode ?? "default"];
}

export function RoomCollaborationMode({ value = "default", onChange, disabled, en = false, compact = false }: {
  value?: CollaborationMode; onChange: (value: CollaborationMode) => void; disabled?: boolean; en?: boolean; compact?: boolean;
}) {
  const name = useId();
  const descriptions = en
    ? { default: "Original create and follow workflow; no chat signals.", host: "Send a signal before creating. Incoming signals are ignored.", follower: "Read the primary account’s chat; all local accounts join together." }
    : { default: "原版创建与跟房，不发送或识别聊天信号。", host: "创建前发送信号，不监听聊天跟房。", follower: "监听本机主号聊天，主号和小号一起加入。" };
  return <fieldset className="room-collaboration" data-compact={compact} disabled={disabled}>
    <legend>{en ? "Room collaboration" : "拼房模式"}</legend>
    <div className="room-collaboration-options">
      {(["default", "host", "follower"] as const).map(mode => <label key={mode} data-selected={value === mode}>
        <input type="radio" name={name} value={mode} checked={value === mode} onChange={() => onChange(mode)} />
        <strong>{roomModeLabel(mode, en)}</strong>
        <small>{descriptions[mode]}</small>
      </label>)}
    </div>
    <p>{compact ? `${descriptions[value]} ${en ? "Applying a switch cancels the previous workflow." : "应用切换会取消旧流程。"}` : value === "follower"
      ? (en ? "Both room shortcuts join instead of creating. Keep the room rule synchronized with the host." : "本机不创建房间：两个房间快捷键均执行加入。请与拼房主机同步房间规则。")
      : (en ? "One mode at a time. Applying a mode change cancels the previous workflow and pending signals." : "三种模式互斥。应用切换时会取消旧流程和待执行信号。")}</p>
  </fieldset>;
}
