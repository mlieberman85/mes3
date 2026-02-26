/**
 * IndexedDB storage module for NES save states and battery-backed SRAM.
 *
 * Uses the `idb` library for a promise-based IndexedDB API.
 */

import { openDB, type IDBPDatabase } from 'idb';

// ---------------------------------------------------------------------------
// Database schema
// ---------------------------------------------------------------------------

const DB_NAME = 'nes-emulator';
const DB_VERSION = 1;

const STORE_SAVE_STATES = 'save-states';
const STORE_SRAM = 'sram';

/** Shape of a persisted save-state record. */
interface SaveStateRecord {
  romHash: string;
  name: string;
  timestamp: number;
  stateData: Uint8Array;
  screenshot: Blob | null;
  /** Optional numbered slot (1-based). Null/undefined = unslotted save. */
  slot?: number;
}

/** Shape returned by listStates (lightweight -- no binary payload). */
export interface SaveStateSummary {
  id: number;
  name: string;
  timestamp: number;
  hasScreenshot: boolean;
  slot?: number;
}

/** Shape returned by loadState. */
export interface SaveStateData {
  stateData: Uint8Array;
  name: string;
  timestamp: number;
  screenshot: Blob | null;
}

// ---------------------------------------------------------------------------
// Database singleton
// ---------------------------------------------------------------------------

let db: IDBPDatabase | null = null;

/**
 * Open (or upgrade) the IndexedDB database. Safe to call multiple times --
 * subsequent calls return the cached connection.
 */
export async function initStorage(): Promise<void> {
  if (db) return;

  db = await openDB(DB_NAME, DB_VERSION, {
    upgrade(database) {
      // Save-state store: auto-increment key, indexed by romHash and timestamp.
      if (!database.objectStoreNames.contains(STORE_SAVE_STATES)) {
        const stateStore = database.createObjectStore(STORE_SAVE_STATES, {
          keyPath: 'id',
          autoIncrement: true,
        });
        stateStore.createIndex('romHash', 'romHash', { unique: false });
        stateStore.createIndex('timestamp', 'timestamp', { unique: false });
      }

      // SRAM store: keyed by romHash (one entry per ROM).
      if (!database.objectStoreNames.contains(STORE_SRAM)) {
        database.createObjectStore(STORE_SRAM, { keyPath: 'romHash' });
      }
    },
  });
}

// ---------------------------------------------------------------------------
// Internal helper -- ensure DB is open
// ---------------------------------------------------------------------------

function getDb(): IDBPDatabase {
  if (!db) {
    throw new Error('Storage not initialized. Call initStorage() first.');
  }
  return db;
}

// ---------------------------------------------------------------------------
// Save state CRUD
// ---------------------------------------------------------------------------

/**
 * Persist an emulator save state with metadata.
 *
 * @returns The auto-generated record ID.
 */
export async function saveState(
  romHash: string,
  stateData: Uint8Array,
  screenshot: Blob | null,
  name?: string,
  slot?: number,
): Promise<number> {
  const database = getDb();

  // If saving to a numbered slot, overwrite the existing slot entry.
  if (slot !== undefined) {
    const existing = await findSlotRecord(romHash, slot);
    if (existing !== null) {
      const updated: SaveStateRecord & { id: number } = {
        ...existing,
        name: name ?? `Slot ${slot}`,
        timestamp: Date.now(),
        stateData,
        screenshot,
        slot,
      };
      await database.put(STORE_SAVE_STATES, updated);
      return existing.id;
    }
  }

  const record: SaveStateRecord = {
    romHash,
    name: name ?? (slot !== undefined ? `Slot ${slot}` : `Save ${new Date().toLocaleString()}`),
    timestamp: Date.now(),
    stateData,
    screenshot,
    slot,
  };

  try {
    const id = (await database.add(STORE_SAVE_STATES, record)) as number;
    return id;
  } catch (err: unknown) {
    if (isQuotaExceededError(err)) {
      throw new Error(
        'Storage is full. Delete some save states to free space.',
      );
    }
    throw err;
  }
}

/**
 * Load the save state in a specific slot for a ROM.
 */
export async function loadSlot(
  romHash: string,
  slot: number,
): Promise<SaveStateData | null> {
  const record = await findSlotRecord(romHash, slot);
  if (!record) return null;
  return {
    stateData: record.stateData,
    name: record.name,
    timestamp: record.timestamp,
    screenshot: record.screenshot,
  };
}

/**
 * Return which slots (1-based) have data for a given ROM.
 */
export async function getOccupiedSlots(romHash: string): Promise<Set<number>> {
  const states = await listStates(romHash);
  const slots = new Set<number>();
  for (const s of states) {
    if (s.slot !== undefined) slots.add(s.slot);
  }
  return slots;
}

/** Find the record for a specific ROM+slot combination. */
async function findSlotRecord(
  romHash: string,
  slot: number,
): Promise<(SaveStateRecord & { id: number }) | null> {
  const database = getDb();
  const tx = database.transaction(STORE_SAVE_STATES, 'readonly');
  const index = tx.store.index('romHash');
  let cursor = await index.openCursor(romHash);
  while (cursor) {
    const rec = cursor.value as SaveStateRecord & { id: number };
    if (rec.slot === slot) return rec;
    cursor = await cursor.continue();
  }
  return null;
}

/**
 * Load a save state by its auto-increment ID.
 *
 * @returns The save-state payload, or `null` if not found.
 */
export async function loadState(id: number): Promise<SaveStateData | null> {
  const database = getDb();
  const record: SaveStateRecord | undefined = await database.get(
    STORE_SAVE_STATES,
    id,
  );

  if (!record) return null;

  return {
    stateData: record.stateData,
    name: record.name,
    timestamp: record.timestamp,
    screenshot: record.screenshot,
  };
}

/**
 * List all save states for a given ROM hash (lightweight summaries only).
 */
export async function listStates(romHash: string): Promise<SaveStateSummary[]> {
  const database = getDb();
  const tx = database.transaction(STORE_SAVE_STATES, 'readonly');
  const index = tx.store.index('romHash');

  const results: SaveStateSummary[] = [];

  let cursor = await index.openCursor(romHash);
  while (cursor) {
    const rec = cursor.value as SaveStateRecord & { id: number };
    results.push({
      id: rec.id,
      name: rec.name,
      timestamp: rec.timestamp,
      hasScreenshot: rec.screenshot !== null,
      slot: rec.slot,
    });
    cursor = await cursor.continue();
  }

  // Most recent first
  results.sort((a, b) => b.timestamp - a.timestamp);
  return results;
}

/** Delete a save state by ID. */
export async function deleteState(id: number): Promise<void> {
  const database = getDb();
  await database.delete(STORE_SAVE_STATES, id);
}

/** Rename a save state. */
export async function renameState(id: number, newName: string): Promise<void> {
  const database = getDb();
  const record: SaveStateRecord | undefined = await database.get(
    STORE_SAVE_STATES,
    id,
  );
  if (!record) return;

  record.name = newName;
  await database.put(STORE_SAVE_STATES, record);
}

// ---------------------------------------------------------------------------
// SRAM (battery-backed save RAM)
// ---------------------------------------------------------------------------

/** Persist battery-backed SRAM for a ROM. */
export async function saveSram(
  romHash: string,
  sramData: Uint8Array,
): Promise<void> {
  const database = getDb();
  try {
    await database.put(STORE_SRAM, { romHash, sramData });
  } catch (err: unknown) {
    if (isQuotaExceededError(err)) {
      throw new Error(
        'Storage is full. Delete some save states to free space for SRAM.',
      );
    }
    throw err;
  }
}

/** Load battery-backed SRAM for a ROM. Returns null if none saved. */
export async function loadSram(
  romHash: string,
): Promise<Uint8Array | null> {
  const database = getDb();
  const record = await database.get(STORE_SRAM, romHash);
  return record ? (record.sramData as Uint8Array) : null;
}

// ---------------------------------------------------------------------------
// Storage usage estimate
// ---------------------------------------------------------------------------

/** Estimate how much storage is in use (via the Storage API). */
export async function getStorageUsage(): Promise<{
  usage: number;
  quota: number;
} | null> {
  if (!navigator.storage?.estimate) return null;

  const estimate = await navigator.storage.estimate();
  return {
    usage: estimate.usage ?? 0,
    quota: estimate.quota ?? 0,
  };
}

// ---------------------------------------------------------------------------
// ROM hashing
// ---------------------------------------------------------------------------

/**
 * Compute a SHA-256 hex digest of the given ROM data.
 *
 * Uses the Web Crypto SubtleCrypto API (available in all modern browsers).
 */
export async function computeRomHash(romData: Uint8Array): Promise<string> {
  const hashBuffer = await crypto.subtle.digest('SHA-256', romData as unknown as BufferSource);
  const hashArray = new Uint8Array(hashBuffer);
  return Array.from(hashArray)
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

// ---------------------------------------------------------------------------
// Error helpers
// ---------------------------------------------------------------------------

function isQuotaExceededError(err: unknown): boolean {
  if (err instanceof DOMException) {
    // "QuotaExceededError" is the standard name; some browsers also use code 22.
    return err.name === 'QuotaExceededError' || err.code === 22;
  }
  return false;
}
