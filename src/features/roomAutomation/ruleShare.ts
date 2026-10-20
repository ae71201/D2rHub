import type { RoomAutomationConfig } from "./types";
export type SharedRoomRule = Pick<RoomAutomationConfig, "name_prefix" | "password" | "next_sequence" | "sequence_width">;
export function copyRoomRule(config: RoomAutomationConfig): string {
  return JSON.stringify({ format: "d2rhub-room-rule", version: 1, name_prefix: config.name_prefix,
    password: config.password, next_sequence: config.next_sequence, sequence_width: config.sequence_width }, null, 2);
}
export function parseRoomRule(text: string): SharedRoomRule {
  if (text.length > 4096) throw new Error("规则内容过长");
  const value = JSON.parse(text);
  const hasControls = (text: string) => /[\u0000-\u001f\u007f-\u009f\u2028\u2029]/.test(text);
  if (!value || value.format !== "d2rhub-room-rule" || value.version !== 1
    || typeof value.name_prefix !== "string" || (!value.name_prefix.trim() || hasControls(value.name_prefix))
    || typeof value.password !== "string" || hasControls(value.password) || value.password.length > 15
    || !Number.isInteger(value.next_sequence) || value.next_sequence < 0 || value.next_sequence >= 4294967295
    || !Number.isInteger(value.sequence_width) || value.sequence_width < 1 || value.sequence_width > 6
    || (value.name_prefix + String(value.next_sequence).padStart(value.sequence_width, "0")).length > 15) {
    throw new Error("房间规则格式或范围无效");
  }
  return { name_prefix: value.name_prefix, password: value.password,
    next_sequence: value.next_sequence, sequence_width: value.sequence_width };
}
