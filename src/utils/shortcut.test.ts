import { normalizeShortcut } from "./shortcut";

function assertEqual(actual: string, expected: string, message: string) {
  if (actual !== expected) {
    throw new Error(`FAIL: ${message}\nExpected: "${expected}"\nActual:   "${actual}"`);
  }
  console.log(`PASS: ${message}`);
}

assertEqual(normalizeShortcut("ctrl+1"), "Ctrl+1", "simple ctrl+1");
assertEqual(normalizeShortcut("Alt+Ctrl+f"), "Ctrl+Alt+F", "modifier order and uppercase key");
assertEqual(normalizeShortcut("shift+alt+ctrl+s"), "Ctrl+Alt+Shift+S", "all modifiers without win");
assertEqual(normalizeShortcut("Ctrl+space"), "Ctrl+Space", "space key handling");
assertEqual(normalizeShortcut("Ctrl+Num+"), "Ctrl+Num+", "numpad add keeps an unambiguous key token");
assertEqual(normalizeShortcut(""), "", "empty string");
console.log("All shortcut normalization tests passed successfully!");
