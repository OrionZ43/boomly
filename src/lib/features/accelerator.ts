// Участок D — KeyboardEvent → строка сочетания для global-shortcut (`Ctrl+Shift+F1`).
// Берём e.code (физическая клавиша), чтобы раскладка RU/EN не влияла.

const MOD_CODES = new Set([
  "ControlLeft", "ControlRight", "ShiftLeft", "ShiftRight", "AltLeft", "AltRight", "MetaLeft", "MetaRight",
]);

const NAMED: Record<string, string> = {
  Space: "Space", Enter: "Enter", Tab: "Tab", Backspace: "Backspace", Delete: "Delete", Insert: "Insert",
  Home: "Home", End: "End", PageUp: "PageUp", PageDown: "PageDown",
  ArrowUp: "Up", ArrowDown: "Down", ArrowLeft: "Left", ArrowRight: "Right",
  Minus: "-", Equal: "=", BracketLeft: "[", BracketRight: "]", Backslash: "\\", Semicolon: ";",
  Quote: "'", Comma: ",", Period: ".", Slash: "/", Backquote: "`",
  NumpadAdd: "NumAdd", NumpadSubtract: "NumSubtract", NumpadMultiply: "NumMultiply",
  NumpadDivide: "NumDivide", NumpadDecimal: "NumDecimal", NumpadEnter: "NumEnter",
  Pause: "Pause", ScrollLock: "ScrollLock", PrintScreen: "PrintScreen",
};

export function keyName(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit\d$/.test(code)) return code.slice(5);
  if (/^F\d{1,2}$/.test(code)) return code;
  if (/^Numpad\d$/.test(code)) return `Num${code.slice(6)}`;
  return NAMED[code] ?? null;
}

export type Capture =
  | { kind: "done"; accelerator: string }
  | { kind: "partial"; preview: string; needMod?: boolean } // только модификаторы или клавиша без Ctrl/Alt
  | { kind: "cancel" }
  | { kind: "clear" }
  | { kind: "ignore" };

export function captureKey(e: KeyboardEvent): Capture {
  const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Super"].filter(
    Boolean,
  ) as string[];
  if (!mods.length && e.code === "Escape") return { kind: "cancel" };
  if (!mods.length && (e.code === "Backspace" || e.code === "Delete")) return { kind: "clear" };
  if (MOD_CODES.has(e.code)) return { kind: "partial", preview: mods.join("+") };
  const key = keyName(e.code);
  if (!key) return { kind: "ignore" };
  // Без модификатора (или с одним Shift) разрешаем только F-клавиши и цифровой блок —
  // иначе глобальный хоткей перехватит обычный набор текста (Shift+A = «A») в играх и Discord.
  const onlyShift = mods.length === 1 && mods[0] === "Shift";
  if ((!mods.length || onlyShift) && !/^(F\d{1,2}|Num.+|Pause|ScrollLock|Insert)$/.test(key)) return { kind: "partial", preview: [...mods, key].join("+"), needMod: true };
  return { kind: "done", accelerator: [...mods, key].join("+") };
}
