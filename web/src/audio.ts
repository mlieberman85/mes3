/**
 * Audio subsystem -- AudioWorklet setup and per-frame sample transfer.
 *
 * Handles the "user gesture required" restriction by deferring AudioContext
 * creation until the first explicit user interaction (ROM load, button click).
 */

let audioCtx: AudioContext | null = null;
let workletNode: AudioWorkletNode | null = null;
let initialized = false;

/**
 * Initialize the audio subsystem.
 *
 * Creates an AudioContext at 48 kHz, loads the worklet module, and connects
 * the AudioWorkletNode to the audio destination. This must be called from
 * a user-gesture event handler to satisfy browser autoplay policies.
 *
 * Calling this after audio is already initialized is a no-op.
 */
export async function initAudio(): Promise<void> {
  if (initialized) return;

  audioCtx = new AudioContext({ sampleRate: 48_000 });

  // The worklet source lives in public/ as plain JS so it works both in dev
  // (served as-is by Vite) and in production (copied to dist/ root).
  // import.meta.env.BASE_URL is '/' in dev and '/mes3/' in production.
  const workletUrl = import.meta.env.BASE_URL + 'audio-worklet.js';
  await audioCtx.audioWorklet.addModule(workletUrl);

  workletNode = new AudioWorkletNode(audioCtx, 'nes-audio-processor');
  workletNode.connect(audioCtx.destination);

  // Pre-fill the ring buffer with ~2048 samples of silence to provide
  // headroom before the first real samples arrive, preventing startup clicks.
  const prefill = new Float32Array(2048);
  workletNode.port.postMessage(prefill, [prefill.buffer]);

  initialized = true;
}

/**
 * Post a buffer of audio samples to the worklet for playback.
 *
 * The samples are transferred (zero-copy) rather than copied, so the caller
 * must not reuse the buffer after this call.
 */
export function sendSamples(samples: Float32Array): void {
  if (!workletNode) return;
  workletNode.port.postMessage(samples, [samples.buffer]);
}

/** Suspend (mute) the audio output. */
export async function suspend(): Promise<void> {
  await audioCtx?.suspend();
}

/** Resume (unmute) the audio output. */
export async function resume(): Promise<void> {
  await audioCtx?.resume();
}

/** Returns true once `initAudio()` has completed successfully. */
export function isInitialized(): boolean {
  return initialized;
}
