# Quickstart: NES Emulator

**Branch**: `001-nes-emulator` | **Date**: 2026-02-25

## Prerequisites

- Rust toolchain (latest stable) with `wasm32-unknown-unknown` target
- Node.js 18+ and npm
- `wasm-bindgen-cli` (must match `wasm-bindgen` crate version)
- `wasm-opt` (from binaryen, optional — for production builds)

## Setup

```bash
# 1. Install Rust WASM target
rustup target add wasm32-unknown-unknown

# 2. Install wasm-bindgen CLI
cargo install wasm-bindgen-cli

# 3. Install frontend dependencies
cd web && npm install && cd ..
```

## Build

```bash
# Build WASM (debug)
cargo build --target wasm32-unknown-unknown -p nes-wasm

# Generate JS bindings
wasm-bindgen --out-dir web/pkg --target web \
  target/wasm32-unknown-unknown/debug/nes_wasm.wasm

# Build WASM (release, optimized)
cargo build --release --target wasm32-unknown-unknown -p nes-wasm
wasm-bindgen --out-dir web/pkg --target web \
  target/wasm32-unknown-unknown/release/nes_wasm.wasm
wasm-opt -O3 web/pkg/nes_wasm_bg.wasm -o web/pkg/nes_wasm_bg.wasm
```

## Run (Development)

```bash
# Start Vite dev server (serves frontend + WASM)
cd web && npm run dev
```

Open `http://localhost:5173` in the browser. Click "Load ROM" and
select a `.nes` file.

## Run (Native CLI)

```bash
# Run a ROM natively (for testing/debugging)
cargo run -p nes-cli -- path/to/rom.nes
```

## Test

```bash
# Run all Rust tests (native)
cargo test

# Run CPU tests specifically
cargo test -p nes-core --test cpu_tests

# Run nestest validation
cargo test -p nes-core --test nestest

# Run WASM tests (headless browser)
# Requires wasm-pack or wasm-bindgen-test-runner
cargo test --target wasm32-unknown-unknown -p nes-wasm

# Run frontend tests
cd web && npm test

# Lint
cargo clippy -- -D warnings
cargo fmt --check
cd web && npm run lint
```

## Project Structure

```
Cargo.toml                 # Workspace root
crates/
├── nes-core/              # Pure Rust NES emulation (no_std + alloc)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs         # Public API: Nes struct
│   │   ├── cpu/           # 6502 CPU (all 256 opcodes)
│   │   │   ├── mod.rs
│   │   │   ├── opcodes.rs # Opcode dispatch table
│   │   │   └── addressing.rs # Addressing modes
│   │   ├── ppu/           # Picture Processing Unit
│   │   │   ├── mod.rs
│   │   │   └── registers.rs
│   │   ├── apu/           # Audio Processing Unit
│   │   │   ├── mod.rs
│   │   │   ├── pulse.rs
│   │   │   ├── triangle.rs
│   │   │   ├── noise.rs
│   │   │   └── dmc.rs
│   │   ├── bus.rs         # Memory bus / address decoding
│   │   ├── cartridge.rs   # ROM loading, iNES parsing
│   │   └── mappers/       # Mapper implementations
│   │       ├── mod.rs     # Mapper trait + factory
│   │       ├── nrom.rs    # Mapper 0
│   │       ├── mmc1.rs    # Mapper 1
│   │       ├── uxrom.rs   # Mapper 2
│   │       ├── cnrom.rs   # Mapper 3
│   │       └── mmc3.rs    # Mapper 4
│   └── tests/
│       ├── cpu_tests.rs        # Per-opcode cycle-accurate tests
│       ├── ppu_tests.rs        # PPU rendering tests
│       ├── apu_tests.rs        # APU channel unit tests
│       ├── save_state_tests.rs # Save state round-trip tests
│       └── nestest.rs          # nestest.nes validation
├── nes-wasm/              # WASM bindings (wasm-bindgen)
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs         # Thin wrapper: Emulator struct + exports
└── nes-cli/               # Native CLI runner
    ├── Cargo.toml
    └── src/
        └── main.rs        # ROM loading, SDL2/minifb display

web/                       # Frontend (TypeScript + Vite)
├── package.json
├── vite.config.ts
├── index.html
├── src/
│   ├── main.ts            # Entry point, WASM init
│   ├── renderer.ts        # Canvas 2D rendering
│   ├── audio.ts           # AudioWorklet setup
│   ├── audio-worklet.ts   # AudioWorklet processor
│   ├── input.ts           # Keyboard + Gamepad handling
│   ├── storage.ts         # IndexedDB save state persistence
│   └── ui.ts              # Controls, settings, save state browser
├── pkg/                   # WASM build output (gitignored)
└── styles.css
```

## Key Workflows

### Load and Play a ROM
1. User clicks "Load ROM" → file picker opens
2. Frontend reads file as `ArrayBuffer`
3. Passes bytes to `Emulator::load_rom()`
4. If `RomLoadResult::Ok`, start `requestAnimationFrame` loop
5. Each frame: `run_frame()` → read frame buffer → `putImageData`

### Save / Load State
1. User triggers save → `Emulator::save_state()` returns `Vec<u8>`
2. Frontend stores in IndexedDB with metadata (hash, timestamp, name)
3. User browses saves → frontend queries IndexedDB by game hash
4. User loads save → frontend reads binary → `Emulator::load_state()`

### Input Configuration
1. Default mappings loaded on first run
2. User opens settings → remaps keys/buttons
3. Mappings persisted to localStorage (small JSON)
4. On input event: lookup mapping → `Emulator::set_button_state()`
