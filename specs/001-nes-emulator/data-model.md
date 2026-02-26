# Data Model: NES Emulator

**Branch**: `001-nes-emulator` | **Date**: 2026-02-25

## Entity Diagram

```
┌─────────────┐     loads      ┌────────────────┐
│  ROM File   │───────────────▶│   Cartridge    │
│  (iNES)     │                │                │
└─────────────┘                │ - header       │
                               │ - prg_rom      │
                               │ - chr_rom      │
                               │ - mapper       │
                               │ - sram         │
                               └───────┬────────┘
                                       │ plugs into
                                       ▼
                               ┌────────────────┐
                               │      Bus       │
                               │ (memory map)   │
                               └──┬──┬──┬──┬────┘
                                  │  │  │  │
                    ┌─────────────┘  │  │  └─────────────┐
                    ▼                ▼  ▼                 ▼
             ┌──────────┐   ┌──────────┐  ┌──────────┐  ┌──────────┐
             │   CPU    │   │   PPU    │  │   APU    │  │   RAM    │
             │  (6502)  │   │ (2C02)   │  │ (2A03)   │  │  (2KB)   │
             └──────────┘   └──────────┘  └──────────┘  └──────────┘
                    │              │             │
                    ▼              ▼             ▼
             ┌──────────┐   ┌──────────┐  ┌──────────┐
             │  Input   │   │  Frame   │  │  Audio   │
             │Controller│   │  Buffer  │  │  Buffer  │
             └──────────┘   └──────────┘  └──────────┘
                                │               │
                                ▼               ▼
                          ┌──────────┐   ┌──────────┐
                          │  Canvas  │   │ AudioWklt│
                          │  (2D)   │   │ (WebAudio)│
                          └──────────┘   └──────────┘
```

## Entities

### Cartridge

Parsed from an iNES ROM file. Immutable after loading (except SRAM).

| Field      | Type          | Description                                |
|------------|---------------|--------------------------------------------|
| header     | iNesHeader    | 16-byte parsed header (mapper, mirroring)  |
| prg_rom    | Vec\<u8\>     | Program ROM banks (16KB each)              |
| chr_rom    | Vec\<u8\>     | Character ROM banks (8KB each)             |
| sram       | [u8; 8192]    | Battery-backed save RAM (if present)       |
| mapper     | Box\<dyn Mapper\> | Active mapper implementation            |

**Validation rules**:
- Header must start with bytes `NES\x1A`
- PRG-ROM size must match header declaration
- CHR-ROM size must match header declaration (0 = uses CHR-RAM)
- Mapper number must be in supported set {0, 1, 2, 3, 4}

**iNesHeader fields**:
- prg_rom_banks: u8 (count of 16KB banks)
- chr_rom_banks: u8 (count of 8KB banks)
- mapper_number: u8
- mirroring: Horizontal | Vertical | FourScreen
- has_battery_ram: bool
- has_trainer: bool

### CPU (MOS 6502)

Cycle-accurate 6502 processor state.

| Field        | Type   | Description                              |
|--------------|--------|------------------------------------------|
| a            | u8     | Accumulator register                     |
| x            | u8     | X index register                         |
| y            | u8     | Y index register                         |
| sp           | u8     | Stack pointer (offset from $0100)        |
| pc           | u16    | Program counter                          |
| status       | Flags  | Status register (N, V, -, B, D, I, Z, C) |
| cycles       | u64    | Total elapsed cycles                     |
| stall_cycles | u16    | Remaining stall cycles (DMA, etc.)       |

**Status flags** (bitflags):
- C (bit 0): Carry
- Z (bit 1): Zero
- I (bit 2): Interrupt disable
- D (bit 3): Decimal mode (not used on NES but flag exists)
- B (bit 4): Break command (virtual, push-only)
- U (bit 5): Unused (always 1)
- V (bit 6): Overflow
- N (bit 7): Negative

**State transitions**:
- RESET → PC loaded from $FFFC-$FFFD, SP -= 3, I flag set
- NMI → push PC + status, PC from $FFFA-$FFFB, I flag set
- IRQ → (if I flag clear) push PC + status, PC from $FFFE-$FFFF
- BRK → push PC+2 + status with B=1, PC from $FFFE-$FFFF

### PPU (2C02)

Picture Processing Unit state. Produces 256x240 pixel frames.

| Field          | Type          | Description                           |
|----------------|---------------|---------------------------------------|
| vram           | [u8; 2048]    | 2KB video RAM (nametables)            |
| oam            | [u8; 256]     | Object Attribute Memory (64 sprites)  |
| palette        | [u8; 32]      | Palette RAM                           |
| ctrl           | PpuCtrl       | Control register ($2000)              |
| mask           | PpuMask       | Mask register ($2001)                 |
| status         | PpuStatus     | Status register ($2002)               |
| oam_addr       | u8            | OAM address ($2003)                   |
| v              | u16           | Current VRAM address (15-bit)         |
| t              | u16           | Temporary VRAM address (15-bit)       |
| fine_x         | u8            | Fine X scroll (3-bit)                 |
| write_latch    | bool          | First/second write toggle             |
| data_buffer    | u8            | PPU data read buffer                  |
| scanline       | u16           | Current scanline (0-261)              |
| dot            | u16           | Current dot/cycle (0-340)             |
| frame_count    | u64           | Total frames rendered                 |
| frame_buffer   | [u8; 245760]  | RGBA pixel output (256x240x4)         |
| nmi_pending    | bool          | NMI interrupt to CPU                  |

**Scanline state machine**:
- Scanlines 0-239: Visible — fetch tiles, evaluate sprites, render
- Scanline 240: Post-render — idle
- Scanline 241: VBlank begins — set VBlank flag, trigger NMI if enabled
- Scanlines 241-260: VBlank — CPU reads/writes PPU freely
- Scanline 261: Pre-render — clear flags, reload scroll registers

### APU (2A03)

Audio Processing Unit. Five channels running at CPU clock.

| Field           | Type          | Description                          |
|-----------------|---------------|--------------------------------------|
| pulse1          | PulseChannel  | First pulse/square wave channel      |
| pulse2          | PulseChannel  | Second pulse/square wave channel     |
| triangle        | TriChannel    | Triangle wave channel                |
| noise           | NoiseChannel  | Noise generator channel              |
| dmc             | DmcChannel    | Delta modulation channel             |
| frame_counter   | FrameCounter  | Frame counter (4/5 step sequencer)   |
| sample_buffer   | Vec\<f32\>    | Audio samples for current frame      |
| sample_rate     | u32           | Output sample rate (48000)           |

**Channel common fields** (PulseChannel example):
- enabled: bool
- length_counter: u8
- timer_period: u16
- timer_value: u16
- duty_cycle: u8
- envelope: Envelope (volume, divider, decay, loop, start)
- sweep: Sweep (enabled, period, negate, shift)

### Save State

Serialized emulator snapshot. Stored in IndexedDB.

| Field       | Type          | Description                             |
|-------------|---------------|-----------------------------------------|
| id          | String        | Unique key: `{game_hash}-{timestamp}`   |
| game_hash   | String        | SHA-256 of ROM file (identity)          |
| timestamp   | u64           | Unix timestamp (ms) at save time        |
| name        | Option\<String\> | User-provided label (optional)       |
| state_data  | Vec\<u8\>     | Serialized binary (CPU+PPU+APU+RAM+mapper) |
| screenshot  | Option\<Vec\<u8\>\> | PNG thumbnail at save time (optional) |

**Identity rule**: game_hash derived from full ROM content (excluding
header) ensures save states cannot be loaded for a different ROM.

### Input Configuration

User-customized control mappings. Persisted in browser storage.

| Field          | Type                | Description                       |
|----------------|---------------------|-----------------------------------|
| keyboard_map   | Map\<Key, Button\>  | Keyboard key → NES button         |
| gamepad_map    | Map\<u8, Button\>   | Gamepad button index → NES button |

**NES Button enum**: Up, Down, Left, Right, A, B, Start, Select

**Default keyboard mapping**:
- Arrow keys → D-pad
- Z → B, X → A
- Enter → Start, Shift → Select

### Mapper (trait)

Interface for cartridge memory mapping hardware.

| Method         | Signature                          | Description             |
|----------------|------------------------------------|-------------------------|
| read_prg       | (addr: u16) → u8                   | Read from PRG space     |
| write_prg      | (addr: u16, val: u8)               | Write to PRG space      |
| read_chr       | (addr: u16) → u8                   | Read from CHR space     |
| write_chr      | (addr: u16, val: u8)               | Write to CHR space      |
| mirroring      | () → Mirroring                     | Current nametable mirror|
| save_state     | () → Vec\<u8\>                     | Serialize mapper state  |
| load_state     | (data: &[u8])                      | Restore mapper state    |

**Implementations**: NROM (0), MMC1 (1), UxROM (2), CNROM (3), MMC3 (4)

## Memory Map

### CPU Address Space ($0000-$FFFF)

| Range           | Size  | Description                        |
|-----------------|-------|------------------------------------|
| $0000-$07FF     | 2KB   | Internal RAM                       |
| $0800-$1FFF     | —     | Mirrors of $0000-$07FF             |
| $2000-$2007     | 8     | PPU registers                      |
| $2008-$3FFF     | —     | Mirrors of $2000-$2007             |
| $4000-$4017     | 24    | APU and I/O registers              |
| $4018-$401F     | 8     | Normally disabled APU/test          |
| $4020-$5FFF     | —     | Cartridge expansion (mapper-dependent) |
| $6000-$7FFF     | 8KB   | Cartridge SRAM (battery-backed)    |
| $8000-$FFFF     | 32KB  | Cartridge PRG-ROM (mapper-dependent) |

### PPU Address Space ($0000-$3FFF)

| Range           | Size  | Description                        |
|-----------------|-------|------------------------------------|
| $0000-$0FFF     | 4KB   | Pattern table 0 (CHR-ROM/RAM)      |
| $1000-$1FFF     | 4KB   | Pattern table 1 (CHR-ROM/RAM)      |
| $2000-$23FF     | 1KB   | Nametable 0                        |
| $2400-$27FF     | 1KB   | Nametable 1                        |
| $2800-$2BFF     | 1KB   | Nametable 2 (mirror, depends)      |
| $2C00-$2FFF     | 1KB   | Nametable 3 (mirror, depends)      |
| $3000-$3EFF     | —     | Mirrors of $2000-$2EFF             |
| $3F00-$3F1F     | 32    | Palette RAM                        |
| $3F20-$3FFF     | —     | Mirrors of palette                 |
