/** UI controller -- wires up DOM buttons and overlays. */

// RomLoadResult values matching the WASM enum
const ROM_LOAD_OK = 0;
const ROM_LOAD_INVALID_FORMAT = 1;
const ROM_LOAD_UNSUPPORTED_MAPPER = 2;
const ROM_LOAD_PAL_NOT_SUPPORTED = 3;

/**
 * Human-readable error messages for each ROM load failure code.
 */
function romLoadMessage(result: number): string {
  switch (result) {
    case ROM_LOAD_INVALID_FORMAT:
      return 'Invalid ROM format. Please load a valid .nes file (iNES or NES 2.0).';
    case ROM_LOAD_UNSUPPORTED_MAPPER:
      return 'This ROM uses an unsupported mapper. Only common mappers (0, 1, 2, 3, 4) are currently supported.';
    case ROM_LOAD_PAL_NOT_SUPPORTED:
      return 'PAL ROMs are not supported. Please use an NTSC ROM.';
    default:
      return `Unknown ROM load error (code ${result}).`;
  }
}

/** Callback signatures the UI needs from the outside. */
export interface UiCallbacks {
  /** Called when the user selects a ROM file. Receives raw bytes. */
  onRomSelected: (data: Uint8Array) => void | Promise<void>;
  /** Called when the user clicks Reset. */
  onReset: () => void;
  /** Called when the user toggles mute. Returns new muted state. */
  onMuteToggle: () => boolean;
  /** Called when the user clicks Save State. */
  onSaveState: () => void;
  /** Called when the user clicks Load State (quick-load most recent). */
  onLoadState: () => void;
  /** Called when the user clicks Load on a specific save-state entry. */
  onLoadStateById?: (id: number) => void;
  /** Called when the user clicks Delete on a specific save-state entry. */
  onDeleteState?: (id: number) => void;
  /** Called when the user renames a save-state entry. */
  onRenameState?: (id: number, newName: string) => void;
}

// Cached DOM element references
let statusText: HTMLElement;
let fpsCounter: HTMLElement;
let errorOverlay: HTMLElement;
let errorMessage: HTMLElement;
let settingsPanel: HTMLElement;
let saveStatePanel: HTMLElement;
let saveStateList: HTMLElement;
let btnReset: HTMLButtonElement;
let btnSaveState: HTMLButtonElement;
let btnLoadState: HTMLButtonElement;
let btnMute: HTMLButtonElement;

/** Stored callbacks for save-state list interaction. */
let saveStateCallbacks: Pick<
  UiCallbacks,
  'onLoadStateById' | 'onDeleteState' | 'onRenameState'
> = {};

/**
 * Initialize all UI event wiring. Call once at startup.
 */
export function initUi(callbacks: UiCallbacks): void {
  // Resolve DOM references
  statusText = document.getElementById('status-text')!;
  fpsCounter = document.getElementById('fps-counter')!;
  errorOverlay = document.getElementById('error-overlay')!;
  errorMessage = document.getElementById('error-message')!;
  settingsPanel = document.getElementById('settings-panel')!;
  saveStatePanel = document.getElementById('save-state-panel')!;
  saveStateList = document.getElementById('save-state-list')!;

  // Preserve per-entry callbacks
  saveStateCallbacks = {
    onLoadStateById: callbacks.onLoadStateById,
    onDeleteState: callbacks.onDeleteState,
    onRenameState: callbacks.onRenameState,
  };

  const btnLoadRom = document.getElementById('btn-load-rom') as HTMLButtonElement;
  btnReset = document.getElementById('btn-reset') as HTMLButtonElement;
  btnMute = document.getElementById('btn-mute') as HTMLButtonElement;
  btnSaveState = document.getElementById('btn-save-state') as HTMLButtonElement;
  btnLoadState = document.getElementById('btn-load-state') as HTMLButtonElement;
  const btnSettings = document.getElementById('btn-settings') as HTMLButtonElement;
  const btnDismissError = document.getElementById('btn-dismiss-error') as HTMLButtonElement;
  const romInput = document.getElementById('rom-input') as HTMLInputElement;

  // Load ROM flow: button click triggers hidden file input
  btnLoadRom.addEventListener('click', () => romInput.click());

  romInput.addEventListener('change', () => {
    const file = romInput.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = () => {
      const data = new Uint8Array(reader.result as ArrayBuffer);
      callbacks.onRomSelected(data);
    };
    reader.onerror = () => {
      showError('Failed to read the ROM file.');
    };
    reader.readAsArrayBuffer(file);

    // Reset so the same file can be re-loaded
    romInput.value = '';
  });

  btnReset.addEventListener('click', () => callbacks.onReset());

  btnMute.addEventListener('click', () => {
    const muted = callbacks.onMuteToggle();
    btnMute.textContent = muted ? 'Unmute' : 'Mute';
  });

  btnSaveState.addEventListener('click', () => callbacks.onSaveState());
  btnLoadState.addEventListener('click', () => callbacks.onLoadState());

  btnSettings.addEventListener('click', () => {
    settingsPanel.classList.toggle('hidden');
  });

  btnDismissError.addEventListener('click', () => dismissError());
}

/**
 * Display a ROM load result. Shows an error overlay on failure.
 * Returns true if the ROM loaded successfully.
 */
export function handleRomLoadResult(result: number): boolean {
  if (result === ROM_LOAD_OK) {
    return true;
  }
  showError(romLoadMessage(result));
  return false;
}

/** Show the error overlay with the given message. */
export function showError(message: string): void {
  errorMessage.textContent = message;
  errorOverlay.classList.remove('hidden');
}

/** Dismiss the error overlay. */
export function dismissError(): void {
  errorOverlay.classList.add('hidden');
}

/** Update the status bar text. */
export function setStatus(text: string): void {
  statusText.textContent = text;
}

/** Update the FPS counter display. */
export function setFps(fps: number): void {
  fpsCounter.textContent = `${fps.toFixed(1)} FPS`;
}

/**
 * Enable or disable ROM-dependent buttons (Reset, Save/Load State).
 */
export function setRomLoaded(loaded: boolean): void {
  btnReset.disabled = !loaded;
  btnSaveState.disabled = !loaded;
  btnLoadState.disabled = !loaded;
}

/** Show or hide the save-state list panel. */
export function toggleSaveStatePanel(forceShow?: boolean): void {
  if (forceShow !== undefined) {
    saveStatePanel.classList.toggle('hidden', !forceShow);
  } else {
    saveStatePanel.classList.toggle('hidden');
  }
}

/**
 * Update the save-state list display with the provided state summaries.
 *
 * Each entry shows the name, a human-readable timestamp, and
 * Load / Rename / Delete action buttons.
 */
export function updateSaveStateList(
  states: Array<{ id: number; name: string; timestamp: number }>,
): void {
  saveStateList.innerHTML = '';

  if (states.length === 0) {
    const empty = document.createElement('p');
    empty.textContent = 'No save states yet.';
    empty.className = 'save-state-empty';
    saveStateList.appendChild(empty);
    return;
  }

  for (const state of states) {
    const row = document.createElement('div');
    row.className = 'save-state-row';
    row.dataset.stateId = String(state.id);

    const info = document.createElement('span');
    info.className = 'save-state-info';
    info.textContent = `${state.name}  —  ${new Date(state.timestamp).toLocaleString()}`;
    row.appendChild(info);

    const actions = document.createElement('span');
    actions.className = 'save-state-actions';

    const btnLoad = document.createElement('button');
    btnLoad.textContent = 'Load';
    btnLoad.addEventListener('click', () => {
      saveStateCallbacks.onLoadStateById?.(state.id);
    });
    actions.appendChild(btnLoad);

    const btnRename = document.createElement('button');
    btnRename.textContent = 'Rename';
    btnRename.addEventListener('click', () => {
      const newName = prompt('Enter new name:', state.name);
      if (newName !== null && newName.trim() !== '') {
        saveStateCallbacks.onRenameState?.(state.id, newName.trim());
      }
    });
    actions.appendChild(btnRename);

    const btnDelete = document.createElement('button');
    btnDelete.textContent = 'Delete';
    btnDelete.addEventListener('click', () => {
      saveStateCallbacks.onDeleteState?.(state.id);
    });
    actions.appendChild(btnDelete);

    row.appendChild(actions);
    saveStateList.appendChild(row);
  }
}
