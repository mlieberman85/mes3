# Implementation Plan: NES Emulator

**Branch**: `001-nes-emulator` | **Date**: 2026-02-25 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/001-nes-emulator/spec.md`

## Summary

Build a cycle-accurate NES emulator in Rust compiled to WebAssembly,
with a browser-based frontend for playing games. The emulator covers
the full MOS 6502 instruction set (all 256 opcodes including
undocumented), cycle-level CPU/PPU synchronization, all five APU
audio channels, and the five most common memory mappers (covering
~80% of the NES library). The frontend provides ROM loading,
configurable keyboard/gamepad input, audio playback, and unlimited
persistent save states.

## Technical Context

**Language/Version**: Rust (latest stable) targeting `wasm32-unknown-unknown`
**Primary Dependencies**:
- `wasm-bindgen` — Rust/JS interop for WASM exports
- `bitflags` — Zero-overhead flag types for CPU/PPU registers
- `vite` + `vite-plugin-wasm` — Frontend build tooling
- `idb` — IndexedDB wrapper for save state persistence (~1.2KB)
**Storage**: IndexedDB (save states as binary blobs, unlimited per ROM)
**Testing**: `cargo test` (native), `wasm-bindgen-test` (WASM),
Vite test runner (frontend)
**Target Platform**: Modern browsers (Chrome, Firefox, Safari latest
stable) via WebAssembly
**Project Type**: WASM library + web frontend
**Performance Goals**: 60.0988 fps (NTSC), <3 frame input latency
(<50ms), <2s ROM load-to-first-frame
**Constraints**: Cycle-accurate CPU/PPU synchronization, `no_std` +
`alloc` for core emulation crate, Canvas 2D rendering, AudioWorklet
for audio output
**Scale/Scope**: Single user, single ROM at a time, five mappers
(0-4), NTSC only

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Evidence |
|-----------|--------|----------|
| I. Test-First | PASS | CPU opcode tests written before implementation. nestest.nes validation as acceptance gate. PPU/APU tests per component. Frontend tests for I/O. |
| II. Simplicity | PASS | 3 crates justified: nes-core (2 callers: wasm + cli), nes-wasm (JS bridge), nes-cli (native test runner). Canvas 2D over WebGL (simpler, sufficient). Vanilla TypeScript frontend (no framework). Single external crate in core (bitflags). |
| III. Security-by-Default | PASS | ROM file bytes validated (iNES header check) before processing. WASM sandbox provides memory isolation by default. Minimal dependencies (bitflags only in core). No secrets or server-side components. |
| IV. Observability | PASS | `EmulatorInfo` diagnostic struct exposed via WASM (version, cycle count, frame count, fps). Structured error types (RomLoadResult enum). CPU/PPU state queryable for debugging. |
| V. Semantic Versioning | PASS | WASM interface contract versioned at 0.1.0 (pre-stable). Contract document defines breaking vs non-breaking changes. Version exposed via `EmulatorInfo.version`. |

**Post-Phase 1 re-check**: All principles remain satisfied. The
data model uses fixed-size arrays matching NES hardware (simplicity).
The Mapper trait has exactly 2 callers (bus read/write paths).
The WASM contract is documented with explicit versioning rules.

## Project Structure

### Documentation (this feature)

```text
specs/001-nes-emulator/
├── plan.md              # This file
├── spec.md              # Feature specification
├── research.md          # Phase 0: technology research
├── data-model.md        # Phase 1: entity definitions + memory maps
├── quickstart.md        # Phase 1: build/run/test instructions
├── contracts/
│   └── wasm-interface.md # Phase 1: WASM export contract
└── tasks.md             # Phase 2: task breakdown (next step)
```

### Source Code (repository root)

```text
Cargo.toml                 # Workspace root (members: crates/*)
crates/
├── nes-core/              # no_std + alloc, pure NES emulation
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs         # Public API: Nes struct, run_frame()
│   │   ├── cpu/
│   │   │   ├── mod.rs     # CPU struct, step(), interrupt handling
│   │   │   ├── opcodes.rs # 256-entry dispatch table + execution
│   │   │   └── addressing.rs # 13 addressing modes
│   │   ├── ppu/
│   │   │   ├── mod.rs     # PPU struct, step(), scanline state machine
│   │   │   └── registers.rs # $2000-$2007 register handlers
│   │   ├── apu/
│   │   │   ├── mod.rs     # APU struct, step(), frame counter
│   │   │   ├── pulse.rs   # Pulse channel (x2)
│   │   │   ├── triangle.rs # Triangle channel
│   │   │   ├── noise.rs   # Noise channel
│   │   │   └── dmc.rs     # Delta modulation channel
│   │   ├── bus.rs         # Memory bus: address decoding, read/write
│   │   ├── cartridge.rs   # iNES parsing, ROM validation
│   │   └── mappers/
│   │       ├── mod.rs     # Mapper trait + factory function
│   │       ├── nrom.rs    # Mapper 0: no bank switching
│   │       ├── mmc1.rs    # Mapper 1: serial shift register
│   │       ├── uxrom.rs   # Mapper 2: PRG bank switching
│   │       ├── cnrom.rs   # Mapper 3: CHR bank switching
│   │       └── mmc3.rs    # Mapper 4: scanline counter + IRQ
│   └── tests/
│       ├── cpu_tests.rs        # Per-opcode tests with cycle validation
│       ├── ppu_tests.rs        # PPU rendering and timing tests
│       ├── apu_tests.rs        # APU channel unit tests
│       ├── save_state_tests.rs # Save state round-trip tests
│       └── nestest.rs          # nestest.nes ROM validation
├── nes-wasm/              # wasm-bindgen exports
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs         # Emulator wrapper, Button enum, exports
└── nes-cli/               # Native runner (testing/debugging)
    ├── Cargo.toml
    └── src/
        └── main.rs        # ROM arg, headless or SDL2/minifb display

web/                       # Frontend (TypeScript + Vite)
├── package.json
├── vite.config.ts
├── index.html
├── src/
│   ├── main.ts            # Entry: WASM init, frame loop setup
│   ├── renderer.ts        # Canvas 2D putImageData rendering
│   ├── audio.ts           # AudioWorklet setup + sample transfer
│   ├── audio-worklet.ts   # AudioWorklet processor (ring buffer)
│   ├── input.ts           # Keyboard + Gamepad API mapping
│   ├── storage.ts         # IndexedDB save state CRUD via idb
│   └── ui.ts              # ROM picker, settings, save state UI
├── pkg/                   # WASM build output (gitignored)
└── styles.css
```

**Structure Decision**: Rust workspace with 3 crates + web frontend.
The `nes-core` crate is the emulation engine (`no_std` + `alloc`),
consumed by both `nes-wasm` (browser) and `nes-cli` (native). This
separation satisfies the constitution's Simplicity principle: each
crate has a clear single responsibility, and the core abstraction
has two distinct callers.

## Complexity Tracking

> No constitution violations. All design choices satisfy all five
> principles. No entries needed.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| (none)    | —          | —                                   |
