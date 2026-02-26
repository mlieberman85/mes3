# Research: NES Emulator

**Branch**: `001-nes-emulator` | **Date**: 2026-02-25

## R1: Rust WASM Toolchain

**Decision**: `cargo build --target wasm32-unknown-unknown` + `wasm-bindgen` CLI.

**Rationale**: wasm-pack was archived in July 2025 with the rustwasm
working group. wasm-bindgen itself remains actively maintained. The
direct pipeline (`cargo build` → `wasm-bindgen` → `wasm-opt`) gives
full control over build flags and output.

**Build pipeline**:
```bash
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --out-dir pkg --target web \
  target/wasm32-unknown-unknown/release/nes_wasm.wasm
wasm-opt -O3 pkg/nes_wasm_bg.wasm -o pkg/nes_wasm_bg.wasm
```

**Alternatives considered**:
- wasm-pack: Archived, no longer maintained.
- Emscripten (wasm32-unknown-emscripten): Adds unnecessary C runtime.
  Not needed for pure Rust.

## R2: Rendering — Canvas 2D vs WebGL

**Decision**: Canvas 2D with `putImageData`.

**Rationale**: At 256x240 pixels (~240KB RGBA per frame), `putImageData`
is trivially fast at 60fps on modern hardware. All known Rust+WASM NES
emulators (nes-rust, rustynes, runes) use Canvas 2D. WebGL would add
shader/texture boilerplate for zero measurable benefit.

**Implementation**: Allocate a `Uint8ClampedArray` backed by WASM linear
memory. Write RGBA pixels directly from Rust. Create `ImageData` and
call `putImageData`. CSS `image-rendering: pixelated` handles integer
scaling.

**Alternatives considered**:
- WebGL: Significant boilerplate (shaders, textures, quad rendering)
  with no benefit at this resolution. Reserved for future CRT filter
  effects if ever needed.

## R3: Audio — Web Audio API

**Decision**: `AudioWorklet` with `postMessage` transferring
`Float32Array` buffers. SharedArrayBuffer upgrade optional.

**Rationale**: `ScriptProcessorNode` is deprecated and runs on the main
thread. AudioWorklet runs on a dedicated audio thread. Start with
message-passing (`postMessage` with transferable `Float32Array`) which
avoids SharedArrayBuffer's cross-origin isolation header requirements.
At 48kHz with a 2048-sample buffer (~42ms latency), message-passing
overhead is negligible.

**NES audio**: APU generates samples at CPU clock (~1.789 MHz), which
are downsampled to 48kHz. Samples collected during each frame (~800
samples at 48kHz / 60fps) are posted to the AudioWorklet.

**Alternatives considered**:
- SharedArrayBuffer ring buffer: Lower latency, zero-copy. Requires
  `Cross-Origin-Opener-Policy` and `Cross-Origin-Embedder-Policy`
  headers. Breaks GitHub Pages and some hosting. Upgrade path if needed.
- ScriptProcessorNode: Deprecated. Do not use.

## R4: Frontend Build Tooling

**Decision**: Vite with `vite-plugin-wasm`.

**Rationale**: Vite is the standard for modern frontend projects. Native
ESM dev server, fast production builds, TypeScript out of the box. The
`vite-plugin-wasm` package (~1KB) enables direct `.wasm` imports.

**Workflow**: A `build:wasm` npm script runs the cargo/wasm-bindgen
pipeline, outputting to `pkg/`. Vite imports from `pkg/`. `cargo-watch`
or `watchexec` can trigger Rust rebuilds on `.rs` changes during dev.

**Alternatives considered**:
- Webpack: Heavier, slower, more complex configuration. No advantage.
- No bundler (raw HTML + ES modules): Works but lacks dev server,
  HMR, TypeScript compilation. Too manual.

## R5: Cycle-Accurate Main Loop

**Decision**: Catch-up model. CPU executes one instruction, PPU catches
up by 3x that many cycles.

**NTSC timing constants**:
- Master clock: 21.477272 MHz
- CPU clock: 1.789773 MHz (master / 12)
- PPU clock: 5.369318 MHz (master / 4, = 3x CPU)
- PPU dots per scanline: 341
- Scanlines per frame: 262 (240 visible + 22 vblank/pre-render)
- PPU cycles per frame: 89,342 (89,341 on odd frames)
- CPU cycles per frame: ~29,780.67
- Frame rate: 60.0988 Hz

**Loop structure**:
```
per requestAnimationFrame:
  track real elapsed time to avoid drift (NES ≠ exactly 60Hz)
  while cpu_cycles_this_frame < 29781:
    cycles = cpu.step()           // 1 instruction: 1-7 cycles
    ppu.step(cycles * 3)          // PPU catches up
    apu.step(cycles)              // APU runs at CPU clock
    cpu_cycles_this_frame += cycles
```

Fractional cycles handled by alternating 29780/29781 or accumulating
a fractional counter. Odd-frame dot skip handled in PPU.

**Alternatives considered**:
- True master-clock stepping (advance 1 master cycle at a time): 3x
  slower for no practical benefit. No NES game depends on sub-instruction
  PPU timing.
- Run full frame then render: Breaks cycle accuracy. PPU register
  reads/writes mid-frame would not synchronize.

## R6: Save State Storage

**Decision**: IndexedDB via `idb` library (Jake Archibald).

**Rationale**: IndexedDB natively stores `ArrayBuffer` / `Uint8Array`
without serialization. Storage quota is 1GB+ per origin. The `idb`
wrapper (~1.2KB brotli) replaces verbose IDBRequest callbacks with
promises. NES save states are 50-100KB each — storage is effectively
unlimited.

**Schema**: Object store keyed by `{gameHash}-{timestamp}`. Fields:
game hash, timestamp, optional user name, state binary, optional
screenshot thumbnail.

**Alternatives considered**:
- localStorage: 5MB limit, strings only, synchronous. Not viable.
- Cache API: Designed for HTTP responses. Wrong abstraction.
- Raw IndexedDB: Functional but extremely verbose. idb adds <2KB.

## R7: Core Architecture — no_std

**Decision**: `no_std` + `alloc` for the emulation core. Only external
crate: `bitflags`.

**Rationale**: NES is fixed hardware. The core needs only fixed-size
arrays (2KB CPU RAM, 2KB VRAM, 256B OAM, 32B palette) plus `Vec<u8>`
for variable-size ROM data. `bitflags` provides zero-overhead flag
types for CPU status register and PPU registers. Multiple existing Rust
NES emulators (runes, tetanes) confirm this approach.

**Crate structure**:
```
nes-core/    # no_std + alloc, pure emulation
nes-wasm/    # wasm-bindgen frontend bridge
nes-cli/     # native runner for testing/debugging
```

nes-core has two callers (nes-wasm, nes-cli), satisfying the
constitution's Simplicity principle (abstractions need ≥2 callers).

**Alternatives considered**:
- Full std: Unnecessary. No file I/O, networking, or system calls in
  the emulation core.
- Additional crates (log, serde): Not needed in core. Logging is a
  platform concern (handled in nes-wasm/nes-cli). Serialization for
  save states can use a simple custom binary format.
