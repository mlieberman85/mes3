# WASM Interface Contract: NES Emulator

**Branch**: `001-nes-emulator` | **Date**: 2026-02-25
**Version**: 0.1.0

This document defines the public interface exported from the
`nes-wasm` crate via `wasm-bindgen`. The frontend TypeScript code
depends on this contract.

## Exported Types

### `Emulator`

Primary entry point. One instance per running game.

### `EmulatorInfo`

Read-only diagnostic snapshot returned by `get_info()`.

| Field          | Type   | Description                           |
|----------------|--------|---------------------------------------|
| version        | String | Crate version (SemVer)                |
| rom_loaded     | bool   | Whether a ROM is currently loaded     |
| mapper_number  | u8     | Active mapper number (0 if no ROM)    |
| cpu_cycles     | u64    | Total CPU cycles elapsed              |
| frame_count    | u64    | Total frames rendered                 |
| fps            | f64    | Current measured frames per second    |

### `Button` (enum)

NES controller button identifiers.

| Variant | Value | Description      |
|---------|-------|------------------|
| A       | 0     | A button         |
| B       | 1     | B button         |
| Select  | 2     | Select button    |
| Start   | 3     | Start button     |
| Up      | 4     | D-pad up         |
| Down    | 5     | D-pad down       |
| Left    | 6     | D-pad left       |
| Right   | 7     | D-pad right      |

### `RomLoadResult` (enum)

Result of attempting to load a ROM.

| Variant            | Description                                  |
|--------------------|----------------------------------------------|
| Ok                 | ROM loaded successfully                      |
| InvalidFormat      | Not a valid iNES file                        |
| UnsupportedMapper  | Mapper number not in supported set           |
| PalNotSupported    | ROM is PAL region (not supported)            |

## Exported Methods

### Lifecycle

| Method                   | Signature                                     | Description                                  |
|--------------------------|-----------------------------------------------|----------------------------------------------|
| `Emulator::new()`        | `() → Emulator`                               | Create a new emulator instance (no ROM)      |
| `Emulator::load_rom()`   | `(data: &[u8]) → RomLoadResult`               | Parse iNES ROM and initialize emulation      |
| `Emulator::reset()`      | `()`                                          | Reset CPU/PPU/APU to power-on state          |

### Emulation Loop

| Method                    | Signature                                    | Description                                  |
|---------------------------|----------------------------------------------|----------------------------------------------|
| `Emulator::run_frame()`   | `() → bool`                                  | Run one full NTSC frame. Returns false if no ROM loaded |
| `Emulator::frame_buffer_ptr()` | `() → *const u8`                        | Pointer to RGBA frame buffer in WASM memory (256x240x4 = 245,760 bytes) |
| `Emulator::audio_buffer()` | `() → Vec<f32>`                             | Consume audio samples generated this frame (~800 at 48kHz) |

### Input

| Method                          | Signature                            | Description                                  |
|---------------------------------|--------------------------------------|----------------------------------------------|
| `Emulator::set_button_state()`  | `(button: Button, pressed: bool)`    | Set a controller button pressed/released     |

### Save States

| Method                        | Signature                              | Description                                  |
|-------------------------------|----------------------------------------|----------------------------------------------|
| `Emulator::save_state()`      | `() → Vec<u8>`                         | Serialize complete emulator state to binary  |
| `Emulator::load_state()`      | `(data: &[u8]) → bool`                | Restore state from binary. Returns false on invalid data |

### Diagnostics

| Method                   | Signature                                     | Description                                  |
|--------------------------|-----------------------------------------------|----------------------------------------------|
| `Emulator::get_info()`   | `() → EmulatorInfo`                            | Return current diagnostic snapshot           |

## TypeScript Usage

```typescript
import init, { Emulator, Button, RomLoadResult } from '../pkg/nes_wasm';

await init();

const emu = Emulator.new();
const result = emu.load_rom(new Uint8Array(romArrayBuffer));

if (result === RomLoadResult.Ok) {
  // Main loop
  function frame() {
    emu.run_frame();

    // Render: read frame buffer from WASM memory
    const ptr = emu.frame_buffer_ptr();
    const pixels = new Uint8ClampedArray(
      memory.buffer, ptr, 256 * 240 * 4
    );
    const imageData = new ImageData(pixels, 256, 240);
    ctx.putImageData(imageData, 0, 0);

    // Audio: transfer samples to AudioWorklet
    const samples = emu.audio_buffer();
    audioWorklet.port.postMessage(samples, [samples.buffer]);

    requestAnimationFrame(frame);
  }
  requestAnimationFrame(frame);
}
```

## Versioning

This interface follows SemVer per the project constitution.
Breaking changes to method signatures, return types, or enum
variants require a MAJOR version bump. New methods or variants
are MINOR. Bug fixes to existing behavior are PATCH.

Current: **0.1.0** (pre-stable, breaking changes expected)
