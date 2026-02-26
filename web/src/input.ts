/** Input handler supporting keyboard and gamepad for NES controller. */

// NES button constants matching the WASM Button enum
export const Button = {
  A: 0,
  B: 1,
  SELECT: 2,
  START: 3,
  UP: 4,
  DOWN: 5,
  LEFT: 6,
  RIGHT: 7,
} as const;

export type ButtonId = (typeof Button)[keyof typeof Button];

/** Maps keyboard event.code values to NES button IDs. */
export type KeyMappings = Record<string, ButtonId>;

const STORAGE_KEY = 'nes-input-config';

const DEFAULT_KEY_MAPPINGS: KeyMappings = {
  ArrowUp: Button.UP,
  ArrowDown: Button.DOWN,
  ArrowLeft: Button.LEFT,
  ArrowRight: Button.RIGHT,
  KeyZ: Button.B,
  KeyX: Button.A,
  Enter: Button.START,
  ShiftRight: Button.SELECT,
};

/**
 * Standard gamepad button index to NES button mapping.
 * Based on the W3C "standard" gamepad layout.
 */
const GAMEPAD_BUTTON_MAP: Record<number, ButtonId> = {
  0: Button.B,       // Face bottom (B / Cross)
  1: Button.A,       // Face right  (A / Circle)
  2: Button.SELECT,  // Face left   (X / Square) -- alt select
  3: Button.START,   // Face top    (Y / Triangle) -- alt start
  8: Button.SELECT,  // Back / Select
  9: Button.START,   // Start
  12: Button.UP,     // D-pad Up
  13: Button.DOWN,   // D-pad Down
  14: Button.LEFT,   // D-pad Left
  15: Button.RIGHT,  // D-pad Right
};

/** Axis dead-zone threshold for analog sticks mapped to d-pad. */
const AXIS_DEADZONE = 0.5;

export type ButtonChangeCallback = (button: number, pressed: boolean) => void;

export class InputHandler {
  private onButtonChange: ButtonChangeCallback;
  private keyMappings: KeyMappings;
  private listening = false;

  /** Track which gamepad buttons are currently held to detect edges. */
  private gamepadState: Map<number, boolean> = new Map();

  private handleKeyDown: (e: KeyboardEvent) => void;
  private handleKeyUp: (e: KeyboardEvent) => void;

  constructor(onButtonChange: ButtonChangeCallback) {
    this.onButtonChange = onButtonChange;
    this.keyMappings = { ...DEFAULT_KEY_MAPPINGS };

    this.handleKeyDown = (e: KeyboardEvent) => {
      const btn = this.keyMappings[e.code];
      if (btn !== undefined) {
        e.preventDefault();
        this.onButtonChange(btn, true);
      }
    };

    this.handleKeyUp = (e: KeyboardEvent) => {
      const btn = this.keyMappings[e.code];
      if (btn !== undefined) {
        e.preventDefault();
        this.onButtonChange(btn, false);
      }
    };
  }

  /** Begin listening for keyboard and gamepad events. */
  start(): void {
    if (this.listening) return;
    this.listening = true;
    window.addEventListener('keydown', this.handleKeyDown);
    window.addEventListener('keyup', this.handleKeyUp);
  }

  /** Stop listening for keyboard and gamepad events. */
  stop(): void {
    if (!this.listening) return;
    this.listening = false;
    window.removeEventListener('keydown', this.handleKeyDown);
    window.removeEventListener('keyup', this.handleKeyUp);
    this.gamepadState.clear();
  }

  /**
   * Poll connected gamepads for button state changes.
   * Should be called once per frame inside the requestAnimationFrame loop.
   */
  pollGamepad(): void {
    const gamepads = navigator.getGamepads();
    if (!gamepads) return;

    for (const gp of gamepads) {
      if (!gp || !gp.connected) continue;

      // Digital buttons
      for (const [gpIdx, nesBtn] of Object.entries(GAMEPAD_BUTTON_MAP)) {
        const idx = Number(gpIdx);
        const pressed = gp.buttons[idx]?.pressed ?? false;
        const wasPressed = this.gamepadState.get(idx) ?? false;

        if (pressed !== wasPressed) {
          this.gamepadState.set(idx, pressed);
          this.onButtonChange(nesBtn, pressed);
        }
      }

      // Left stick mapped to d-pad (axes 0 = horizontal, 1 = vertical)
      if (gp.axes.length >= 2) {
        this.pollAxis(gp.axes[0], Button.LEFT, Button.RIGHT);
        this.pollAxis(gp.axes[1], Button.UP, Button.DOWN);
      }
    }
  }

  /** Map an axis value to two opposing buttons with edge detection. */
  private pollAxis(
    value: number,
    negButton: ButtonId,
    posButton: ButtonId,
  ): void {
    // Use axis index offset (100+) to avoid collision with button indices
    const negKey = 100 + negButton;
    const posKey = 100 + posButton;

    const negPressed = value < -AXIS_DEADZONE;
    const posPressed = value > AXIS_DEADZONE;

    const wasNeg = this.gamepadState.get(negKey) ?? false;
    const wasPos = this.gamepadState.get(posKey) ?? false;

    if (negPressed !== wasNeg) {
      this.gamepadState.set(negKey, negPressed);
      this.onButtonChange(negButton, negPressed);
    }
    if (posPressed !== wasPos) {
      this.gamepadState.set(posKey, posPressed);
      this.onButtonChange(posButton, posPressed);
    }
  }

  /** Return a copy of the current keyboard mappings. */
  getKeyMappings(): KeyMappings {
    return { ...this.keyMappings };
  }

  /** Replace the keyboard mappings. */
  setKeyMappings(mappings: KeyMappings): void {
    this.keyMappings = { ...mappings };
  }

  /** Persist current key mappings to localStorage. */
  saveConfig(): void {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(this.keyMappings));
    } catch {
      // Storage full or unavailable -- silently ignore
    }
  }

  /** Load key mappings from localStorage. Falls back to defaults if absent. */
  loadConfig(): void {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (raw) {
        const parsed: unknown = JSON.parse(raw);
        if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
          this.keyMappings = parsed as KeyMappings;
          return;
        }
      }
    } catch {
      // Corrupt data -- fall through to defaults
    }
    this.keyMappings = { ...DEFAULT_KEY_MAPPINGS };
  }
}
