/** Canvas 2D renderer for NES frame output. */

const NES_WIDTH = 256;
const NES_HEIGHT = 240;
const FRAME_BYTES = NES_WIDTH * NES_HEIGHT * 4;

let ctx: CanvasRenderingContext2D | null = null;
let imageData: ImageData | null = null;

/**
 * Initialize the 2D rendering context on the given canvas.
 * Configures nearest-neighbor scaling for crisp pixel art.
 */
export function initRenderer(canvas: HTMLCanvasElement): void {
  const context = canvas.getContext('2d', { alpha: false });
  if (!context) {
    throw new Error('Failed to get 2D rendering context');
  }
  context.imageSmoothingEnabled = false;
  ctx = context;
  imageData = ctx.createImageData(NES_WIDTH, NES_HEIGHT);
}

/**
 * Render a single frame by reading RGBA pixel data from WASM linear memory.
 *
 * @param wasmMemory - The WebAssembly.Memory backing the emulator
 * @param frameBufferPtr - Pointer (byte offset) into wasmMemory where the
 *   256x240x4 RGBA frame buffer begins
 */
export function renderFrame(
  wasmMemory: WebAssembly.Memory,
  frameBufferPtr: number,
): void {
  if (!ctx || !imageData) return;

  const pixels = new Uint8ClampedArray(
    wasmMemory.buffer,
    frameBufferPtr,
    FRAME_BYTES,
  );
  imageData.data.set(pixels);
  ctx.putImageData(imageData, 0, 0);
}

/**
 * Capture the current canvas contents as a PNG Blob.
 *
 * Useful for generating thumbnails alongside save states.
 */
export function captureScreenshot(
  canvas: HTMLCanvasElement,
): Promise<Blob> {
  return new Promise((resolve, reject) => {
    canvas.toBlob((blob) => {
      if (blob) {
        resolve(blob);
      } else {
        reject(new Error('Failed to capture screenshot from canvas'));
      }
    }, 'image/png');
  });
}
