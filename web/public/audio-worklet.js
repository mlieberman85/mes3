/**
 * AudioWorklet processor for NES audio playback.
 *
 * Runs in a dedicated audio rendering thread. Receives sample data from the
 * main thread via MessagePort and feeds it to the audio output through a ring
 * buffer, providing glitch-free continuous playback.
 *
 * This file is loaded as a standalone worklet module -- it must not import
 * anything from the rest of the project.
 */

/** @type {number} Ring buffer capacity in samples. */
const RING_BUFFER_SIZE = 8192;

class NesAudioProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    /** @type {Float32Array} */
    this.buffer = new Float32Array(RING_BUFFER_SIZE);
    /** @type {number} */
    this.readIndex = 0;
    /** @type {number} */
    this.writeIndex = 0;
    /** @type {number} */
    this.count = 0;
    /** @type {number} Last valid sample for hold-on-underrun. */
    this.lastSample = 0.0;

    this.port.onmessage = (event) => {
      const samples = event.data;
      for (let i = 0; i < samples.length; i++) {
        if (this.count >= RING_BUFFER_SIZE) {
          // Buffer full -- drop oldest samples to keep latency bounded.
          this.readIndex = (this.readIndex + 1) % RING_BUFFER_SIZE;
          this.count--;
        }
        this.buffer[this.writeIndex] = samples[i];
        this.writeIndex = (this.writeIndex + 1) % RING_BUFFER_SIZE;
        this.count++;
      }
    };
  }

  process(_inputs, outputs, _parameters) {
    const output = outputs[0];
    if (!output || output.length === 0) return true;

    const channel = output[0];
    for (let i = 0; i < channel.length; i++) {
      if (this.count > 0) {
        this.lastSample = this.buffer[this.readIndex];
        channel[i] = this.lastSample;
        this.readIndex = (this.readIndex + 1) % RING_BUFFER_SIZE;
        this.count--;
      } else {
        // Underrun -- hold last sample to avoid clicks.
        channel[i] = this.lastSample;
      }
    }

    // Copy mono to all additional channels (typically just stereo).
    for (let ch = 1; ch < output.length; ch++) {
      output[ch].set(channel);
    }

    return true;
  }
}

registerProcessor('nes-audio-processor', NesAudioProcessor);
