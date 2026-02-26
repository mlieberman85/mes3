# Feature Specification: NES Emulator

**Feature Branch**: `001-nes-emulator`
**Created**: 2026-02-25
**Status**: Draft
**Input**: User description: "I want to build an NES emulator that is accurate to the various specs like 6502 including undocumented/unsupported instructions"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Load and Play a Game (Priority: P1)

A user opens the emulator in their browser, selects a ROM file from
their local machine, and sees the game running on screen with correct
graphics. They control the game using keyboard input mapped to the
standard NES controller layout (D-pad, A, B, Start, Select). The
emulation runs at the original NTSC frame rate with accurate CPU
behavior including all 256 opcodes (official and undocumented).

**Why this priority**: This is the core value proposition. Without
the ability to load, display, and interact with a game, nothing
else matters.

**Independent Test**: Load a well-known test ROM (e.g., nestest.nes)
and verify it runs to completion with correct output. Load a simple
commercial game and confirm it is playable with correct visuals and
responsive controls.

**Acceptance Scenarios**:

1. **Given** the emulator is open in a browser, **When** the user
   selects a valid .nes ROM file, **Then** the game begins running
   and displays the title screen within 2 seconds.
2. **Given** a game is running, **When** the user presses a mapped
   keyboard key, **Then** the corresponding NES controller input is
   registered within the same frame.
3. **Given** a game is running, **When** the CPU encounters an
   undocumented opcode (e.g., LAX, SAX, DCP), **Then** the
   emulator executes it according to known hardware behavior
   instead of crashing or halting.
4. **Given** a game is running, **When** 60 seconds of gameplay
   elapse, **Then** the emulator maintains a consistent frame rate
   matching the original NTSC timing (≈60.0988 fps) without
   visible stuttering.

---

### User Story 2 - Audio Playback (Priority: P2)

While playing a game, the user hears the original soundtrack and
sound effects through their browser's audio output. All five NES
audio channels (two pulse, triangle, noise, DMC) produce sound
that is recognizably faithful to the original hardware.

**Why this priority**: Audio is essential to the complete NES
experience but the emulator is still usable (and testable) without
it. Decoupling audio from the MVP allows faster iteration on the
core CPU/PPU loop.

**Independent Test**: Load a ROM known for its audio (e.g., a
music-focused test ROM or a game with a recognizable soundtrack)
and verify all five channels produce output. Compare waveform
characteristics against a reference recording.

**Acceptance Scenarios**:

1. **Given** a game is running with audio enabled, **When** the
   game triggers a sound effect, **Then** the sound is audible
   within the same frame it was triggered.
2. **Given** a game is running, **When** the user mutes and
   unmutes audio, **Then** audio stops and resumes without
   desynchronizing from the gameplay.
3. **Given** a game uses the DMC (delta modulation) channel,
   **When** that channel plays a sample, **Then** the sample
   is reproduced without distortion or popping artifacts.

---

### User Story 3 - Emulation State Management (Priority: P3)

A user can save the complete emulator state at any point during
gameplay and restore it later, allowing them to resume exactly
where they left off. Save states persist across browser sessions.

**Why this priority**: Save states are a quality-of-life feature
that depends on a fully working emulation core. They add
significant user value but are not required for basic emulation.

**Independent Test**: Save state during gameplay, close the browser
tab, reopen the emulator, load the save state, and verify gameplay
resumes at the exact point of the save with identical CPU, PPU,
and APU state.

**Acceptance Scenarios**:

1. **Given** a game is running, **When** the user triggers a save
   state, **Then** the complete emulator state is persisted within
   1 second and the user receives visual confirmation.
2. **Given** a save state exists, **When** the user loads it,
   **Then** the emulator restores to the exact saved state within
   1 second, including display, audio position, and controller
   state.
3. **Given** no save state exists for the current ROM, **When**
   the user attempts to load a state, **Then** the emulator
   displays a clear message indicating no save is available
   without interrupting any running game.

---

### Edge Cases

- What happens when the user loads an invalid or corrupted ROM file?
  The emulator MUST display a clear error message and remain usable.
- What happens when a ROM uses an unsupported mapper?
  The emulator MUST inform the user which mapper is required and
  that it is not yet supported, rather than silently failing.
- What happens when the browser tab loses focus?
  The emulator MUST pause emulation to avoid wasting resources
  and resume when focus returns.
- What happens when the user loads a new ROM while a game is running?
  The emulator MUST immediately stop the current game and load the
  new ROM without any confirmation prompt. Unsaved state is discarded.
- What happens when the user loads a PAL-region ROM?
  The emulator MUST detect the PAL region flag in the ROM header
  and display a message indicating that PAL ROMs are not supported
  in this release. The emulator MUST NOT attempt to run the ROM
  at incorrect timing.

## Clarifications

### Session 2026-02-25

- Q: What CPU/PPU timing accuracy model should the emulator use? → A: Cycle-accurate (CPU and PPU advance in lockstep, one cycle at a time)
- Q: Should keyboard mappings be fixed or configurable, and should gamepad be supported? → A: Configurable keyboard with sensible defaults plus game controller (gamepad) input support
- Q: How many save state slots should be available per ROM? → A: Unlimited (user can create as many as browser storage allows)
- Q: What happens when the user loads a new ROM while a game is running? → A: Immediate swap (stop current game and load new ROM instantly, no confirmation)

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST load ROM files in the iNES (.nes) format
  and parse header, PRG-ROM, and CHR-ROM banks correctly.
- **FR-002**: System MUST emulate the MOS 6502 CPU with all 151
  official opcodes and all 105 undocumented opcodes, matching
  documented hardware behavior for each. Each opcode MUST consume
  the correct number of CPU cycles including page-crossing penalties.
- **FR-003**: System MUST emulate the NES PPU (picture processing
  unit) including background rendering, sprite rendering (8x8 and
  8x16 modes), scrolling, and sprite-zero hit detection. The CPU
  and PPU MUST advance in lockstep at cycle granularity to support
  games that rely on mid-scanline timing (e.g., raster effects,
  split-scroll tricks).
- **FR-004**: System MUST render the emulated display at the native
  NES resolution (256x240 pixels) scaled to fit the browser
  viewport using letterbox scaling (centered with black bars) while
  preserving the original 8:7 pixel aspect ratio. Scaling MUST use
  nearest-neighbor interpolation to maintain sharp pixels.
- **FR-005**: System MUST map keyboard inputs to the standard NES
  controller layout (D-pad, A, B, Start, Select) for one player.
  Default key bindings MUST be provided. Users MUST be able to
  remap keyboard bindings through the UI, and custom mappings
  MUST persist across sessions.
- **FR-005a**: System MUST support game controller (gamepad) input
  via the browser's Gamepad API. When a compatible controller is
  connected, the emulator MUST detect it and map its buttons to
  the NES controller layout automatically. Users MUST be able to
  remap gamepad bindings.
- **FR-006**: System MUST support the following memory mappers at
  minimum: NROM (mapper 0), MMC1 (mapper 1), UxROM (mapper 2),
  CNROM (mapper 3), and MMC3 (mapper 4). These five mappers cover
  approximately 80% of the licensed NES library.
- **FR-007**: System MUST emulate the NES APU with all five audio
  channels: two pulse-wave, one triangle-wave, one noise, and one
  delta modulation channel (DMC).
- **FR-008**: System MUST provide save-state functionality that
  captures and restores the full emulator state (CPU registers,
  RAM, PPU state, APU state, mapper state). Users MUST be able
  to create an unlimited number of save states per ROM, limited
  only by available browser storage.
- **FR-008a**: Each save state MUST be labeled with a timestamp
  and optional user-provided name. Users MUST be able to browse,
  load, and delete individual save states through the UI.
- **FR-008b**: When browser storage is insufficient for a new save
  state, the system MUST inform the user and suggest deleting
  older states rather than silently failing or overwriting.
- **FR-009**: System MUST persist save states in the browser across
  sessions so users can resume later.
- **FR-010**: System MUST provide a visible error message when a
  ROM fails to load, uses an unsupported mapper, or encounters an
  unrecoverable emulation error.
- **FR-011**: System MUST pause emulation when the browser tab
  loses focus and resume when it regains focus.

### Key Entities

- **ROM/Cartridge**: A loaded game image containing header metadata,
  program data (PRG-ROM), graphical data (CHR-ROM), and mapper
  configuration.
- **CPU State**: The complete state of the 6502 processor including
  registers (A, X, Y, SP, PC), status flags (N, V, B, D, I, Z, C),
  and cycle count.
- **PPU State**: Video processing state including VRAM, OAM (sprite
  memory), scroll position, rendering phase, and current scanline.
- **APU State**: Audio processing state including channel registers,
  frame counter, and sample buffer.
- **Save State**: A serialized snapshot of all emulator state
  (CPU, PPU, APU, RAM, mapper) at a specific point in time.
  Identified by ROM association, creation timestamp, and optional
  user-provided name. Unlimited per ROM.
- **Controller Input**: The current button state for one NES
  controller (8 buttons: Up, Down, Left, Right, A, B, Start,
  Select).
- **Input Configuration**: User-customized key/button mappings for
  keyboard and gamepad, persisted across sessions.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: The emulator passes the `nestest` CPU test ROM with
  zero instruction failures across all 256 opcodes and correct
  cycle counts for every instruction.
- **SC-002**: Games using supported mappers (0-4) display correct
  graphics with no visible rendering artifacts during normal
  gameplay (verified against reference screenshots).
- **SC-003**: The emulator maintains the target frame rate
  (≈60 fps for NTSC) with less than 5% frame drops during
  continuous gameplay on a mid-range device (2020-era laptop).
- **SC-004**: User input latency from key press to on-screen
  response is below 3 frames (≈50ms).
- **SC-005**: Audio output has no perceivable delay or
  desynchronization relative to on-screen events during normal
  gameplay.
- **SC-006**: Save states round-trip correctly: saving and
  immediately loading produces gameplay indistinguishable from
  uninterrupted play.
- **SC-007**: ROM load-to-first-frame time is under 2 seconds
  on a standard broadband connection.
- **SC-008**: The emulator functions correctly in the latest
  stable releases of Chrome, Firefox, and Safari.

### Assumptions

- The primary target is NTSC emulation only. PAL ROMs are
  rejected with a clear message. PAL support is a future
  enhancement.
- Only single-player (one controller) is required for the initial
  release. Two-player support can be added later.
- The emulator does not need to support NES 2.0 ROM format in the
  initial release; standard iNES is sufficient.
- Mapper support is limited to the five most common mappers
  (0, 1, 2, 3, 4). Additional mappers are future enhancements.
- The frontend framework choice will be determined during the
  planning phase, per the project constitution.
- Battery-backed save RAM (SRAM) for games with persistent saves
  (e.g., The Legend of Zelda) is handled as part of mapper
  emulation and persisted alongside save states.
