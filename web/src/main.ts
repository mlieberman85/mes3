/**
 * NES Emulator -- main entry point.
 *
 * Initializes the WASM module, wires up rendering, input, and UI,
 * then drives the emulation frame loop at NTSC timing (~60.0988 Hz).
 */

import init, { Emulator } from '../pkg/nes_wasm.js';
import { initRenderer, renderFrame, captureScreenshot } from './renderer.js';
import { InputHandler, type ButtonId } from './input.js';
import {
  initUi,
  handleRomLoadResult,
  showError,
  setStatus,
  setFps,
  setRomLoaded,
  updateSaveStateList,
  updateSlotIndicators,
  toggleSaveStatePanel,
} from './ui.js';
import { initAudio, sendSamples, suspend, resume, isInitialized } from './audio.js';
import {
  initStorage,
  computeRomHash,
  saveState as storageSaveState,
  loadState as storageLoadState,
  loadSlot,
  getOccupiedSlots,
  listStates,
  deleteState,
  renameState,
  saveSram,
  loadSram,
} from './storage.js';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** NTSC frame period in milliseconds (~60.0988 Hz). */
const FRAME_PERIOD_MS = 1000 / 60.0988;

/** Number of frames over which to average FPS. */
const FPS_SAMPLE_WINDOW = 60;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

let emulator: Emulator | null = null;
let wasmMemory: WebAssembly.Memory | null = null;
let running = false;
let muted = false;
let rafId = 0;

// Frame-timing state
let lastFrameTime = 0;
let frameTimes: number[] = [];

// ---------------------------------------------------------------------------
// Frame loop
// ---------------------------------------------------------------------------

function frameLoop(now: number): void {
  rafId = requestAnimationFrame(frameLoop);

  if (!emulator || !wasmMemory || !running) return;

  const elapsed = now - lastFrameTime;
  if (elapsed < FRAME_PERIOD_MS * 0.95) return;

  // Accumulator-based timing: run enough frames to stay in sync with
  // real time.  NES runs at 60.0988 Hz but rAF fires at the display
  // refresh rate (~60 Hz), so we occasionally need to run 2 frames in
  // a single callback to keep the audio buffer fed.
  let framesToRun = Math.floor(elapsed / FRAME_PERIOD_MS);
  if (framesToRun > 3) {
    // Cap catch-up to avoid spiral-of-death after long pauses
    framesToRun = 3;
    lastFrameTime = now - FRAME_PERIOD_MS;
  }
  lastFrameTime += framesToRun * FRAME_PERIOD_MS;

  for (let f = 0; f < framesToRun; f++) {
    const ok = emulator.run_frame();
    if (!ok) return;

    // Send audio for every emulated frame to keep the buffer fed
    if (isInitialized() && !muted) {
      const samples = emulator.audio_buffer();
      if (samples.length > 0) {
        sendSamples(samples);
      }
    }
  }

  // Render only the last frame (no wasted GPU work)
  const ptr = emulator.frame_buffer_ptr();
  renderFrame(wasmMemory, ptr);

  // Poll gamepad (must happen every frame)
  inputHandler?.pollGamepad();

  // FPS calculation (rolling average)
  frameTimes.push(now);
  if (frameTimes.length > FPS_SAMPLE_WINDOW) {
    frameTimes.shift();
  }
  if (frameTimes.length >= 2) {
    const span = frameTimes[frameTimes.length - 1] - frameTimes[0];
    const fps = ((frameTimes.length - 1) / span) * 1000;
    setFps(fps);
  }
}

function startLoop(): void {
  if (running) return;
  running = true;
  lastFrameTime = performance.now();
  frameTimes = [];
  rafId = requestAnimationFrame(frameLoop);
}

function stopLoop(): void {
  running = false;
  if (rafId) {
    cancelAnimationFrame(rafId);
    rafId = 0;
  }
}

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

let inputHandler: InputHandler | null = null;

function onButtonChange(button: number, pressed: boolean): void {
  emulator?.set_button_state(button, pressed);
}

/**
 * Handle keyboard shortcuts for save-state slots.
 * Shift+1-5: save to slot, F5-F9: save to slot
 * 1-5 (no modifier): load from slot, F6-F10 would conflict, so just 1-5
 */
function setupSlotShortcuts(): void {
  window.addEventListener('keydown', (e: KeyboardEvent) => {
    // Don't intercept if a rebind prompt is active or input is focused
    if (document.activeElement?.tagName === 'INPUT') return;

    const digit = e.code.match(/^Digit([1-5])$/)?.[1];
    if (!digit) return;

    const slot = parseInt(digit, 10);

    if (e.shiftKey) {
      e.preventDefault();
      saveToSlot(slot);
    } else if (!e.ctrlKey && !e.altKey && !e.metaKey) {
      // Only intercept bare digit keys when a ROM is loaded
      if (!emulator || !currentRomHash) return;
      e.preventDefault();
      activeSlot = slot;
      loadFromSlot(slot);
      refreshSlotIndicators();
    }
  });
}

// ---------------------------------------------------------------------------
// Save / Load state (persistent via IndexedDB)
// ---------------------------------------------------------------------------

/** SHA-256 hex hash of the currently loaded ROM. */
let currentRomHash: string | null = null;

/** Whether the currently loaded ROM has battery-backed SRAM. */
let hasBatteryRam = false;

/**
 * Refresh the save-state list UI for the current ROM.
 */
async function refreshSaveStateList(): Promise<void> {
  if (!currentRomHash) return;
  try {
    const states = await listStates(currentRomHash);
    updateSaveStateList(states);
  } catch (err) {
    console.error('Failed to list save states:', err);
  }
}

/** Currently selected save slot (1-5). */
let activeSlot = 1;

/**
 * Save emulator state to IndexedDB with a canvas screenshot.
 */
async function onSaveState(): Promise<void> {
  await saveToSlot(activeSlot);
}

/**
 * Save emulator state to a specific numbered slot.
 */
async function saveToSlot(slot: number): Promise<void> {
  if (!emulator || !currentRomHash) return;

  try {
    const stateData = emulator.save_state();
    const canvas = document.getElementById('screen') as HTMLCanvasElement;
    let screenshot: Blob | null = null;
    try {
      screenshot = await captureScreenshot(canvas);
    } catch {
      // Screenshot capture is best-effort
    }

    await storageSaveState(currentRomHash, stateData, screenshot, undefined, slot);
    setStatus(`Saved to slot ${slot}`);
    await refreshSaveStateList();
    await refreshSlotIndicators();
  } catch (err) {
    const msg = err instanceof Error ? err.message : 'Unknown error';
    showError(`Failed to save state: ${msg}`);
  }
}

/**
 * Quick-load: load from the active slot.
 */
async function onLoadState(): Promise<void> {
  await loadFromSlot(activeSlot);
}

/**
 * Load emulator state from a specific numbered slot.
 */
async function loadFromSlot(slot: number): Promise<void> {
  if (!emulator || !currentRomHash) return;

  try {
    const data = await loadSlot(currentRomHash, slot);
    if (!data) {
      setStatus(`Slot ${slot} is empty`);
      return;
    }

    const ok = emulator.load_state(data.stateData);
    setStatus(ok ? `Loaded slot ${slot}` : 'Failed to load state');
  } catch (err) {
    const msg = err instanceof Error ? err.message : 'Unknown error';
    showError(`Failed to load state: ${msg}`);
  }
}

/**
 * Update the slot indicators in the UI to show which slots have data.
 */
async function refreshSlotIndicators(): Promise<void> {
  if (!currentRomHash) return;
  try {
    const occupied = await getOccupiedSlots(currentRomHash);
    updateSlotIndicators(occupied, activeSlot);
  } catch {
    // Non-fatal
  }
}

/**
 * Load a specific save state by ID (triggered from the list UI).
 */
async function onLoadStateById(id: number): Promise<void> {
  if (!emulator) return;

  try {
    const data = await storageLoadState(id);
    if (!data) {
      setStatus('Save state not found');
      return;
    }

    const ok = emulator.load_state(data.stateData);
    setStatus(ok ? `Loaded: ${data.name}` : 'Failed to load state');
  } catch (err) {
    const msg = err instanceof Error ? err.message : 'Unknown error';
    showError(`Failed to load state: ${msg}`);
  }
}

/**
 * Delete a save state and refresh the list.
 */
async function onDeleteState(id: number): Promise<void> {
  try {
    await deleteState(id);
    setStatus('Save state deleted');
    await refreshSaveStateList();
  } catch (err) {
    const msg = err instanceof Error ? err.message : 'Unknown error';
    showError(`Failed to delete state: ${msg}`);
  }
}

/**
 * Rename a save state and refresh the list.
 */
async function onRenameState(id: number, newName: string): Promise<void> {
  try {
    await renameState(id, newName);
    await refreshSaveStateList();
  } catch (err) {
    const msg = err instanceof Error ? err.message : 'Unknown error';
    showError(`Failed to rename state: ${msg}`);
  }
}

/**
 * Persist battery-backed SRAM to IndexedDB (if applicable).
 */
async function autoSaveSram(): Promise<void> {
  if (!emulator || !currentRomHash || !hasBatteryRam) return;

  try {
    const stateData = emulator.save_state();
    // SRAM is part of the full emulator state; we store the full state
    // under the SRAM key so we can restore battery RAM on next load.
    await saveSram(currentRomHash, stateData);
  } catch (err) {
    console.error('Failed to auto-save SRAM:', err);
  }
}

// ---------------------------------------------------------------------------
// ROM loading
// ---------------------------------------------------------------------------

async function onRomSelected(data: Uint8Array): Promise<void> {
  if (!emulator) return;

  // Auto-save SRAM for the previously loaded ROM before switching
  await autoSaveSram();

  stopLoop();

  // Initialize audio on the first user-initiated ROM load. This satisfies the
  // browser requirement that AudioContext creation happens inside a user gesture.
  if (!isInitialized()) {
    try {
      await initAudio();
    } catch (err) {
      console.warn('Audio initialization failed -- continuing without audio:', err);
    }
  }

  const result: number = emulator.load_rom(data);
  if (!handleRomLoadResult(result)) {
    return;
  }

  // Compute ROM hash for save-state association
  try {
    currentRomHash = await computeRomHash(data);
  } catch (err) {
    console.error('Failed to compute ROM hash:', err);
    currentRomHash = null;
  }

  // Detect battery-backed SRAM.
  // The iNES header byte at offset 6 has bit 1 set if battery RAM is present.
  hasBatteryRam = data.length > 6 ? (data[6] & 0x02) !== 0 : false;

  // Reset first, then restore SRAM (reset would wipe restored state).
  emulator.reset();

  // Attempt to restore battery-backed SRAM from a previous session.
  // load_state is called *after* reset so the restored SRAM is not cleared.
  let sramRestored = false;
  if (hasBatteryRam && currentRomHash) {
    try {
      const sramState = await loadSram(currentRomHash);
      if (sramState) {
        emulator.load_state(sramState);
        sramRestored = true;
      }
    } catch (err) {
      console.warn('Failed to restore SRAM:', err);
    }
  }

  setRomLoaded(true);
  setStatus(sramRestored ? 'ROM loaded -- SRAM restored' : 'ROM loaded -- running');

  // Refresh save-state list and slot indicators for this ROM
  await refreshSaveStateList();
  await refreshSlotIndicators();
  toggleSaveStatePanel(false);

  startLoop();
}

// ---------------------------------------------------------------------------
// Mute toggle
// ---------------------------------------------------------------------------

function onMuteToggle(): boolean {
  muted = !muted;
  if (muted) {
    suspend();
  } else {
    resume();
  }
  return muted;
}

// ---------------------------------------------------------------------------
// Visibility change -- pause when tab is hidden
// ---------------------------------------------------------------------------

function onVisibilityChange(): void {
  if (document.hidden) {
    stopLoop();
  } else if (emulator) {
    // Only resume if we were running before
    startLoop();
  }
}

// ---------------------------------------------------------------------------
// Bootstrap
// ---------------------------------------------------------------------------

async function main(): Promise<void> {
  // Wire up the UI first so buttons work and errors can be displayed
  // even if WASM loading fails.
  initUi({
    onRomSelected,
    onReset: () => {
      emulator?.reset();
      setStatus('Emulator reset');
    },
    onMuteToggle,
    onSaveState,
    onLoadState,
    onLoadStateById,
    onDeleteState,
    onRenameState,
    onSlotSelect: (slot: number) => {
      activeSlot = slot;
      refreshSlotIndicators();
    },
    getKeyMappings: () => inputHandler?.getKeyMappings() ?? {},
    setKeyMapping: (code, button, oldCode) => {
      if (!inputHandler) return;
      const mappings = inputHandler.getKeyMappings();
      delete mappings[oldCode];
      mappings[code] = button as ButtonId;
      inputHandler.setKeyMappings(mappings);
      inputHandler.saveConfig();
    },
  });

  // Pause/resume on tab visibility
  document.addEventListener('visibilitychange', onVisibilityChange);

  // Auto-save SRAM on page unload
  window.addEventListener('beforeunload', () => {
    autoSaveSram();
  });

  // Load WASM module
  try {
    const wasm = await init();
    wasmMemory = wasm.memory;
  } catch (err) {
    showError(
      'Failed to load the WASM emulator module. Make sure the WASM has been built (npm run build:wasm).',
    );
    console.error('WASM init error:', err);
    return;
  }

  // Canvas renderer
  const canvas = document.getElementById('screen') as HTMLCanvasElement;
  initRenderer(canvas);

  // Emulator instance
  emulator = new Emulator();

  // Input
  inputHandler = new InputHandler(onButtonChange);
  inputHandler.loadConfig();
  inputHandler.start();

  // Initialize IndexedDB storage
  try {
    await initStorage();
  } catch (err) {
    console.error('Storage initialization failed:', err);
  }

  // Save-state slot keyboard shortcuts (Shift+1-5 save, 1-5 load)
  setupSlotShortcuts();

  setStatus('Ready -- load a ROM to start');
}

main().catch((err) => {
  console.error('Unhandled error during initialization:', err);
});
