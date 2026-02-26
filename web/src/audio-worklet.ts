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

/** Ring buffer capacity in samples. */
const RING_BUFFER_SIZE = 8192;

class NesAudioProcessor extends AudioWorkletProcessor {
  private readonly buffer: Float32Array;
  private readIndex: number;
  private writeIndex: number;
  private count: number;

  constructor() {
    super();
    this.buffer = new Float32Array(RING_BUFFER_SIZE);
    this.readIndex = 0;
    this.writeIndex = 0;
    this.count = 0;

    this.port.onmessage = (event: MessageEvent<Float32Array>) => {
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

  process(
    _inputs: Float32Array[][],
    outputs: Float32Array[][],
    _parameters: Record<string, Float32Array>,
  ): boolean {
    const output = outputs[0];
    if (!output || output.length === 0) return true;

    const channel = output[0];
    for (let i = 0; i < channel.length; i++) {
      if (this.count > 0) {
        channel[i] = this.buffer[this.readIndex];
        this.readIndex = (this.readIndex + 1) % RING_BUFFER_SIZE;
        this.count--;
      } else {
        // Underrun -- output silence.
        channel[i] = 0;
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
