---
description: "Task list for NES Emulator implementation"
---

# Tasks: NES Emulator

**Input**: Design documents from `/specs/001-nes-emulator/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/wasm-interface.md

**Tests**: Included — constitution Principle I (Test-First) is NON-NEGOTIABLE.

**Organization**: Tasks grouped by user story (P1 → P2 → P3) for independent delivery.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: US1, US2, US3 (maps to spec.md user stories)
- All paths relative to repository root

## Path Conventions

- **Rust workspace**: `crates/nes-core/`, `crates/nes-wasm/`, `crates/nes-cli/`
- **Frontend**: `web/src/`
- **Tests**: `crates/nes-core/tests/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Cargo workspace, crate scaffolding, frontend project initialization

- [x] T001 Create Cargo workspace root with members config in Cargo.toml
- [x] T002 [P] Create nes-core crate (no_std + alloc, bitflags dep) in crates/nes-core/Cargo.toml + crates/nes-core/src/lib.rs
- [x] T003 [P] Create nes-wasm crate (wasm-bindgen dep, depends on nes-core) in crates/nes-wasm/Cargo.toml + crates/nes-wasm/src/lib.rs
- [x] T004 [P] Create nes-cli crate (depends on nes-core) in crates/nes-cli/Cargo.toml + crates/nes-cli/src/main.rs
- [x] T005 [P] Initialize web frontend with Vite + TypeScript + vite-plugin-wasm in web/package.json + web/vite.config.ts + web/index.html
- [x] T006 [P] Add WASM build script (build:wasm npm script running cargo build + wasm-bindgen) in web/package.json
- [x] T007 [P] Configure .gitignore for web/pkg/, target/, node_modules/ in .gitignore

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: CPU core, memory bus, cartridge loading — MUST complete before ANY user story

**CRITICAL**: No user story work can begin until this phase is complete

- [x] T008 Implement CPU struct with registers (A, X, Y, SP, PC) and bitflags status register in crates/nes-core/src/cpu/mod.rs
- [x] T009 Implement 13 addressing modes (immediate, zero page, zero page X/Y, absolute, absolute X/Y, indirect, indexed indirect, indirect indexed, relative, accumulator, implied) in crates/nes-core/src/cpu/addressing.rs
- [x] T010 [P] Implement iNES ROM parser with header validation, PRG/CHR extraction, mapper number detection, and PAL rejection in crates/nes-core/src/cartridge.rs
- [x] T011 [P] Define Mapper trait (read_prg, write_prg, read_chr, write_chr, mirroring, save_state, load_state) and factory function in crates/nes-core/src/mappers/mod.rs
- [x] T012 Implement NROM mapper (mapper 0, no bank switching) in crates/nes-core/src/mappers/nrom.rs
- [x] T013 Implement memory bus with CPU address space decoding ($0000-$FFFF) including RAM, PPU register mirrors, APU/IO, and cartridge space in crates/nes-core/src/bus.rs
- [x] T014 Implement Nes top-level struct connecting CPU, bus, and cartridge with run_frame() catch-up loop (~29781 CPU cycles per frame, PPU 3:1) in crates/nes-core/src/lib.rs

**Checkpoint**: Foundation ready — `cargo test` compiles, bus address decoding works, NROM ROMs parse successfully

---

## Phase 3: User Story 1 — Load and Play a Game (Priority: P1) MVP

**Goal**: User loads a .nes ROM in the browser, sees correct graphics, controls the game with keyboard/gamepad at 60fps NTSC

**Independent Test**: Load nestest.nes and verify all 256 opcodes pass with correct cycle counts. Load a simple NROM game and confirm it is playable.

### Tests for User Story 1

> **Constitution Principle I**: Write tests FIRST, ensure they FAIL before implementation

- [x] T015 [P] [US1] Write CPU opcode tests covering all addressing modes with expected cycle counts in crates/nes-core/tests/cpu_tests.rs
- [x] T016 [P] [US1] Write nestest.nes ROM validation test (load ROM, run to completion, compare log against reference) in crates/nes-core/tests/nestest.rs
- [x] T017 [P] [US1] Write PPU tests for scanline timing, VBlank NMI, sprite-zero hit, and background rendering in crates/nes-core/tests/ppu_tests.rs

### Implementation for User Story 1

**CPU (6502)**:

- [x] T018 [US1] Implement all 151 official 6502 opcodes with cycle-accurate timing and page-crossing penalties in crates/nes-core/src/cpu/opcodes.rs
- [x] T019 [US1] Implement all 105 undocumented 6502 opcodes (LAX, SAX, DCP, ISB, SLO, RLA, SRE, RRA, etc.) in crates/nes-core/src/cpu/opcodes.rs
- [x] T020 [US1] Implement CPU interrupt handling (NMI, IRQ, BRK, RESET vectors) with correct cycle costs in crates/nes-core/src/cpu/mod.rs

**PPU (2C02)**:

- [x] T021 [US1] Implement PPU struct with VRAM, OAM, palette, internal registers (v, t, fine_x, write_latch), and 245760-byte RGBA frame buffer in crates/nes-core/src/ppu/mod.rs
- [x] T022 [US1] Implement PPU register handlers for $2000-$2007 (PPUCTRL, PPUMASK, PPUSTATUS, OAMADDR, OAMDATA, PPUSCROLL, PPUADDR, PPUDATA) in crates/nes-core/src/ppu/registers.rs
- [x] T023 [US1] Implement PPU background tile fetching (nametable reads, pattern table lookups, attribute table) and pixel rendering in crates/nes-core/src/ppu/mod.rs
- [x] T024 [US1] Implement PPU sprite evaluation (8x8 + 8x16 modes), sprite rendering with priority, and sprite-zero hit detection in crates/nes-core/src/ppu/mod.rs
- [x] T025 [US1] Implement PPU scrolling (fine X/Y scroll, coarse scroll increment, nametable switching, loopy register behavior) in crates/nes-core/src/ppu/mod.rs
- [x] T026 [US1] Implement PPU scanline state machine (visible 0-239, post-render 240, VBlank 241-260, pre-render 261) with NMI generation and odd-frame skip in crates/nes-core/src/ppu/mod.rs
- [x] T027 [US1] Wire PPU into bus for CPU register reads/writes ($2000-$2007 mirrors) and OAM DMA ($4014) in crates/nes-core/src/bus.rs

**Mappers**:

- [x] T028 [P] [US1] Implement MMC1 mapper (mapper 1, serial shift register, PRG/CHR bank switching) in crates/nes-core/src/mappers/mmc1.rs
- [x] T029 [P] [US1] Implement UxROM mapper (mapper 2, PRG bank switching) in crates/nes-core/src/mappers/uxrom.rs
- [x] T030 [P] [US1] Implement CNROM mapper (mapper 3, CHR bank switching) in crates/nes-core/src/mappers/cnrom.rs
- [x] T031 [P] [US1] Implement MMC3 mapper (mapper 4, PRG/CHR bank switching, scanline counter, IRQ) in crates/nes-core/src/mappers/mmc3.rs

**WASM Bindings**:

- [x] T032 [US1] Implement WASM Emulator struct with new(), load_rom() → RomLoadResult, and reset() in crates/nes-wasm/src/lib.rs
- [x] T033 [US1] Implement WASM run_frame() → bool and frame_buffer_ptr() → *const u8 exports in crates/nes-wasm/src/lib.rs
- [x] T034 [US1] Implement WASM Button enum, set_button_state(), EmulatorInfo, and get_info() exports in crates/nes-wasm/src/lib.rs

**Frontend**:

- [x] T035 [P] [US1] Implement Canvas 2D renderer (read WASM frame buffer via Uint8ClampedArray, putImageData, pixelated scaling with 8:7 PAR) in web/src/renderer.ts
- [x] T036 [P] [US1] Implement HTML layout with responsive canvas, ROM picker button, and controls panel in web/index.html + web/styles.css
- [x] T037 [US1] Implement keyboard input handler with default NES mappings (arrows=dpad, Z=B, X=A, Enter=Start, Shift=Select) in web/src/input.ts
- [x] T038 [US1] Implement gamepad input handler via Gamepad API with auto-detection and default button mapping in web/src/input.ts
- [x] T039 [US1] Implement input configuration persistence (localStorage JSON) and remapping UI in web/src/input.ts + web/src/ui.ts
- [x] T040 [US1] Implement ROM loading UI (file picker, ArrayBuffer read, call load_rom, display errors for invalid/unsupported/PAL ROMs) in web/src/ui.ts
- [x] T041 [US1] Implement main entry point with WASM init, requestAnimationFrame loop, frame timing compensation for 60.0988Hz drift in web/src/main.ts
- [x] T042 [US1] Implement tab focus/blur detection to pause and resume emulation in web/src/main.ts
- [x] T043 [US1] Implement nes-cli native runner with ROM arg and headless execution mode for test automation in crates/nes-cli/src/main.rs
- [x] T044 [US1] Run nestest.nes validation end-to-end and verify SC-001 (all 256 opcodes pass with correct cycle counts)

**Checkpoint**: User Story 1 fully functional — ROMs load, games display correctly, keyboard/gamepad input works, 60fps maintained

---

## Phase 4: User Story 2 — Audio Playback (Priority: P2)

**Goal**: All five NES audio channels produce correct sound through the browser, synchronized with gameplay

**Independent Test**: Load a music-focused test ROM, verify all 5 channels produce output, confirm no desync with video

### Tests for User Story 2

- [x] T045 [P] [US2] Write APU channel unit tests (pulse duty cycles, triangle sequencer, noise LFSR, DMC sample playback, frame counter timing) in crates/nes-core/tests/apu_tests.rs

### Implementation for User Story 2

**APU Core**:

- [x] T046 [US2] Implement APU struct with frame counter (4-step and 5-step sequencer modes) in crates/nes-core/src/apu/mod.rs
- [x] T047 [P] [US2] Implement pulse channel (duty cycle, envelope, sweep, length counter, timer) shared by pulse1 + pulse2 in crates/nes-core/src/apu/pulse.rs
- [x] T048 [P] [US2] Implement triangle channel (linear counter, length counter, sequencer) in crates/nes-core/src/apu/triangle.rs
- [x] T049 [P] [US2] Implement noise channel (LFSR, mode flag, envelope, length counter) in crates/nes-core/src/apu/noise.rs
- [x] T050 [P] [US2] Implement DMC channel (delta modulation, sample address/length, IRQ, memory reader) in crates/nes-core/src/apu/dmc.rs
- [x] T051 [US2] Implement APU register handlers ($4000-$4013, $4015 status, $4017 frame counter) and channel mixer in crates/nes-core/src/apu/mod.rs
- [x] T052 [US2] Implement audio sample output with downsampling from CPU clock (~1.789MHz) to 48kHz in crates/nes-core/src/apu/mod.rs
- [x] T053 [US2] Wire APU into memory bus ($4000-$4017 registers) and main loop (apu.step() called per CPU cycle) in crates/nes-core/src/bus.rs + crates/nes-core/src/lib.rs

**WASM + Frontend**:

- [x] T054 [US2] Add audio_buffer() → Vec<f32> WASM export to consume per-frame audio samples in crates/nes-wasm/src/lib.rs
- [x] T055 [US2] Implement AudioWorklet processor with ring buffer for continuous audio playback in web/src/audio-worklet.ts
- [x] T056 [US2] Implement AudioWorklet setup (AudioContext, addModule, connect) and per-frame sample transfer via postMessage in web/src/audio.ts
- [x] T057 [US2] Implement mute/unmute toggle in UI and integrate audio start/stop into main emulation loop in web/src/main.ts + web/src/ui.ts

**Checkpoint**: Audio plays correctly for all 5 channels, mute/unmute works, no desync with video

---

## Phase 5: User Story 3 — Emulation State Management (Priority: P3)

**Goal**: Users can save unlimited snapshots of emulator state, browse them, and restore any save across browser sessions

**Independent Test**: Save state during gameplay, close tab, reopen, load state, verify exact restoration of CPU/PPU/APU state

### Tests for User Story 3

- [x] T058 [P] [US3] Write save state serialization round-trip tests (save → load → verify identical CPU/PPU/APU/mapper state) in crates/nes-core/tests/save_state_tests.rs

### Implementation for User Story 3

**Core Serialization**:

- [x] T059 [US3] Implement save/load state binary serialization for CPU (registers, status, cycles) in crates/nes-core/src/cpu/mod.rs
- [x] T060 [P] [US3] Implement save/load state binary serialization for PPU (VRAM, OAM, palette, registers, scanline position) in crates/nes-core/src/ppu/mod.rs
- [x] T061 [P] [US3] Implement save/load state binary serialization for APU (all channel state, frame counter, sample position) in crates/nes-core/src/apu/mod.rs
- [x] T062 [US3] Implement save/load state for bus (2KB RAM) and all mapper implementations (bank registers, shift registers, counters) in crates/nes-core/src/bus.rs + crates/nes-core/src/mappers/*.rs
- [x] T063 [US3] Implement Nes-level save_state() → Vec<u8> and load_state(&[u8]) → bool composing all component states in crates/nes-core/src/lib.rs

**WASM + Frontend**:

- [x] T064 [US3] Add save_state() → Vec<u8> and load_state(&[u8]) → bool WASM exports in crates/nes-wasm/src/lib.rs
- [x] T065 [US3] Implement IndexedDB storage module (open DB, save-states object store, CRUD by game hash) using idb in web/src/storage.ts
- [x] T066 [US3] Implement save state browser UI (list saves for current ROM, load, delete, rename, timestamp display) in web/src/ui.ts
- [x] T067 [US3] Implement screenshot capture from canvas for save state thumbnails in web/src/renderer.ts
- [x] T068 [US3] Implement storage-full detection and user notification (suggest deleting older states) in web/src/storage.ts + web/src/ui.ts
- [x] T069 [US3] Integrate save/load triggers into main UI (buttons + keyboard shortcuts) and wire to WASM + IndexedDB in web/src/main.ts + web/src/ui.ts
- [x] T069a [US3] Implement battery-backed SRAM persistence to IndexedDB (auto-save on ROM unload, auto-load on ROM load for cartridges with has_battery_ram flag) in web/src/storage.ts + crates/nes-wasm/src/lib.rs

**Checkpoint**: Save states persist across sessions, unlimited slots per ROM, UI shows list with thumbnails, round-trip produces identical gameplay

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Performance tuning, cross-browser validation, final quality gates

- [x] T070 [P] Performance profiling: frame timing drift compensation (60.0988Hz vs display refresh), input latency measurement (verify <3 frames per SC-004), and frame drop rate monitoring in web/src/main.ts
- [ ] T071 [P] Cross-browser testing validation (Chrome, Firefox, Safari) against SC-008 acceptance criteria
- [ ] T072 [P] Verify quickstart.md build/run/test instructions match actual project in specs/001-nes-emulator/quickstart.md
- [x] T073 Code cleanup, resolve all cargo clippy warnings, cargo fmt check, frontend lint
- [x] T074 [P] Add WASM build optimization (wasm-opt -O3) to build:wasm script in web/package.json
- [x] T075 Run full acceptance validation against success criteria SC-001 through SC-008

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion — BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Foundational — core emulation + display
- **User Story 2 (Phase 4)**: Depends on Foundational + T053 wires APU into main loop (can start APU core in parallel with late US1 frontend tasks)
- **User Story 3 (Phase 5)**: Depends on Foundational + at least US1 complete (needs working emulation to have meaningful state to save)
- **Polish (Phase 6)**: Depends on all desired user stories being complete

### User Story Dependencies

- **US1 (P1)**: Can start after Foundational (Phase 2) — No dependencies on other stories
- **US2 (P2)**: APU core (T046-T053) can start after Foundational. Frontend audio (T054-T057) needs WASM bindings from US1.
- **US3 (P3)**: Serialization (T059-T063) can start after Foundational. Frontend save state UI (T065-T069) needs working emulation from US1.

### Within Each User Story

- Tests MUST be written and FAIL before implementation (Principle I)
- CPU before PPU (PPU depends on CPU cycle timing)
- PPU core before bus wiring (register handlers needed first)
- Mappers are independent of each other (all parallel)
- Rust core before WASM bindings before frontend
- Frontend modules are largely parallel (renderer, input, UI)
- Main entry point (main.ts) integrates everything last

### Parallel Opportunities

**Setup phase**: T002-T007 all parallel after T001
**Foundational**: T010 + T011 parallel; T008 → T009 sequential
**US1 tests**: T015 + T016 + T017 all parallel
**US1 mappers**: T028 + T029 + T030 + T031 all parallel
**US1 frontend**: T035 + T036 parallel
**US2 channels**: T047 + T048 + T049 + T050 all parallel
**US3 serialization**: T060 + T061 parallel

---

## Parallel Examples

### User Story 1 — Mappers (all independent, different files)

```
Task: "Implement MMC1 mapper in crates/nes-core/src/mappers/mmc1.rs"
Task: "Implement UxROM mapper in crates/nes-core/src/mappers/uxrom.rs"
Task: "Implement CNROM mapper in crates/nes-core/src/mappers/cnrom.rs"
Task: "Implement MMC3 mapper in crates/nes-core/src/mappers/mmc3.rs"
```

### User Story 2 — APU Channels (all independent, different files)

```
Task: "Implement pulse channel in crates/nes-core/src/apu/pulse.rs"
Task: "Implement triangle channel in crates/nes-core/src/apu/triangle.rs"
Task: "Implement noise channel in crates/nes-core/src/apu/noise.rs"
Task: "Implement DMC channel in crates/nes-core/src/apu/dmc.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL — blocks all stories)
3. Complete Phase 3: User Story 1 (CPU + PPU + mappers + frontend)
4. **STOP and VALIDATE**: Run nestest.nes, load an NROM game, verify playable at 60fps
5. Deploy/demo if ready — this is a working NES emulator

### Incremental Delivery

1. Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → **MVP: playable emulator**
3. Add User Story 2 → Test independently → **Audio-enabled emulator**
4. Add User Story 3 → Test independently → **Full-featured emulator with save states**
5. Each story adds value without breaking previous stories

### Partial Parallelism

With US1 nearing completion:
- APU core work (T046-T053) can begin once Foundational is done
- Save state serialization (T059-T063) can begin once CPU/PPU structs exist
- Frontend tasks for US2/US3 wait until US1 WASM bindings are stable

---

## Notes

- [P] tasks = different files, no dependencies on incomplete tasks
- [Story] label maps task to specific user story for traceability
- Constitution Principle I (Test-First): All test tasks MUST be completed and FAIL before their corresponding implementation tasks
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Total estimated CPU cycles per frame: ~29781 (alternating for fractional accuracy)
- nestest.nes is the acceptance gate for CPU correctness (SC-001)
