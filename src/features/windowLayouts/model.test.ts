import type { LayoutMonitor, WindowLayout } from "../../store/types";
import { adaptLayout, moveSlot, presetSlots, resizeSlot, slotRect, validateLayout, type PresetResolution } from "./model";

function assert(value: unknown, message: string): asserts value { if (!value) throw new Error(message); }
const monitor = (width: number, height: number, id = "main", x = 0, primary = true): LayoutMonitor => ({
  id, name: id, primary, scale_factor: primary ? 1.5 : 1,
  bounds: { x, y: 0, width, height: height + 40 }, work_area: { x, y: 0, width, height },
});

for (const [width, height, profile] of [[1920, 1040, "1080p"], [2560, 1400, "2k"], [3840, 2120, "4k"]] as const) {
  const displays = [monitor(width, height), monitor(1920, 1040, "left", -1920, false)];
  for (const count of [2, 3, 4, 5, 6, 7, 8]) {
    const windows = presetSlots(displays, count, profile as PresetResolution);
    assert(windows.length === count, `Preset ${profile}/${count} has the requested count`);
    assert(windows.every(slot => slot.monitor_id === "main"), "Presets only use the primary display");
    assert(windows.every(slot => slot.width >= 1280 && slot.height >= 720), "Every supported preset keeps playable window dimensions");
    assert(Math.abs(windows[0].x * 2 + windows[0].width - width) <= 1 && Math.abs(windows[0].y * 2 + windows[0].height - height) <= 1, "Primary always stays centered");
    assert(validateLayout({ id: "test", name: "test", monitors: displays, windows }) === null, "Every preset stays within the physical work area");
  }
}

const monitors = [monitor(2560, 1400), monitor(1920, 1040, "left", -1920, false)];
const layout: WindowLayout = { id: "desk", name: "Desk", monitors,
  windows: [{ monitor_id: "main", x: 500, y: 200, width: 1280, height: 720 }, { monitor_id: "main", x: 0, y: 0, width: 1280, height: 720 }] };
const snapped = moveSlot(layout, 0, { x: 1275, y: 4, width: 1280, height: 720 }, 8);
assert(snapped.x === 1280 && snapped.y === 0, "Snap aligns window edges and work-area edges");
const precise = moveSlot(layout, 0, { x: 1275, y: 4, width: 1280, height: 720 }, 0);
assert(precise.x === 1275 && precise.y === 4, "Bypassing snap preserves exact input");
const moved = moveSlot(layout, 0, { x: -1800, y: 900, width: 1280, height: 720 }, 0);
assert(moved.monitor_id === "left" && moved.y === 320 && slotRect(moved, monitors).x === -1800, "Negative desktop coordinates move to the correct display and stay inside");
const resized = resizeSlot(layout, 0, layout.windows[0], 2000, 2000, "nw");
assert(resized.width === 800 && resized.height === 600, "All resize edges enforce the minimum");
assert(resized.x + resized.width === 1780 && resized.y + resized.height === 920, "Resizing a leading edge keeps the opposite corner fixed");
const recovered = adaptLayout({ ...layout, windows: [{ monitor_id: "left", x: 320, y: 160, width: 1280, height: 720 }] }, [monitor(1024, 768)]);
assert(recovered.windows[0].monitor_id === "main" && recovered.windows[0].x === 0 && recovered.windows[0].y === 24, "Disconnected displays recover a centered window on the primary");
assert(validateLayout(recovered) === null, "Recovered layout remains valid");
