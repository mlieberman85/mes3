pub mod addressing;
pub mod opcodes;

use bitflags::bitflags;

use crate::bus::Bus;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CpuFlags: u8 {
        const CARRY     = 0b0000_0001;
        const ZERO      = 0b0000_0010;
        const IRQ_DISABLE = 0b0000_0100;
        const DECIMAL   = 0b0000_1000;
        const BREAK     = 0b0001_0000;
        const UNUSED    = 0b0010_0000;
        const OVERFLOW  = 0b0100_0000;
        const NEGATIVE  = 0b1000_0000;
    }
}

#[derive(Debug, Clone)]
pub struct Cpu {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,
    pub pc: u16,
    pub status: CpuFlags,
    pub cycles: u64,
    pub stall_cycles: u16,
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            sp: 0xFD,
            pc: 0,
            status: CpuFlags::UNUSED | CpuFlags::IRQ_DISABLE,
            cycles: 0,
            stall_cycles: 0,
        }
    }

    /// Reset CPU to power-on state, loading PC from reset vector.
    pub fn reset(&mut self, bus: &mut Bus) {
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.sp = 0xFD;
        self.status = CpuFlags::UNUSED | CpuFlags::IRQ_DISABLE;
        self.pc = self.read_u16(bus, 0xFFFC);
        self.cycles = 7; // Reset takes 7 cycles
        self.stall_cycles = 0;
    }

    /// Execute one instruction. Returns the number of CPU cycles consumed.
    pub fn step(&mut self, bus: &mut Bus) -> u8 {
        if self.stall_cycles > 0 {
            self.stall_cycles -= 1;
            self.cycles += 1;
            return 1;
        }

        let opcode = self.read(bus, self.pc);
        let cycles = opcodes::execute(self, bus, opcode);
        self.cycles += cycles as u64;
        cycles
    }

    /// Trigger a non-maskable interrupt.
    pub fn nmi(&mut self, bus: &mut Bus) {
        self.push_u16(bus, self.pc);
        let flags = (self.status | CpuFlags::UNUSED) - CpuFlags::BREAK;
        self.push(bus, flags.bits());
        self.status.insert(CpuFlags::IRQ_DISABLE);
        self.pc = self.read_u16(bus, 0xFFFA);
        self.cycles += 7;
    }

    /// Trigger an interrupt request (only if IRQ not disabled).
    pub fn irq(&mut self, bus: &mut Bus) {
        if !self.status.contains(CpuFlags::IRQ_DISABLE) {
            self.push_u16(bus, self.pc);
            let flags = (self.status | CpuFlags::UNUSED) - CpuFlags::BREAK;
            self.push(bus, flags.bits());
            self.status.insert(CpuFlags::IRQ_DISABLE);
            self.pc = self.read_u16(bus, 0xFFFE);
            self.cycles += 7;
        }
    }

    /// Check if IRQ is disabled.
    pub fn irq_disabled(&self) -> bool {
        self.status.contains(CpuFlags::IRQ_DISABLE)
    }

    // -- Memory access helpers --

    pub fn read(&self, bus: &Bus, addr: u16) -> u8 {
        bus.read(addr)
    }

    pub fn read_u16(&self, bus: &Bus, addr: u16) -> u16 {
        let lo = bus.read(addr) as u16;
        let hi = bus.read(addr.wrapping_add(1)) as u16;
        (hi << 8) | lo
    }

    pub fn write(&self, bus: &mut Bus, addr: u16, val: u8) {
        bus.write(addr, val);
    }

    // -- Stack helpers --

    pub fn push(&mut self, bus: &mut Bus, val: u8) {
        bus.write(0x0100 | self.sp as u16, val);
        self.sp = self.sp.wrapping_sub(1);
    }

    pub fn push_u16(&mut self, bus: &mut Bus, val: u16) {
        self.push(bus, (val >> 8) as u8);
        self.push(bus, (val & 0xFF) as u8);
    }

    pub fn pull(&mut self, bus: &Bus) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        bus.read(0x0100 | self.sp as u16)
    }

    pub fn pull_u16(&mut self, bus: &Bus) -> u16 {
        let lo = self.pull(bus) as u16;
        let hi = self.pull(bus) as u16;
        (hi << 8) | lo
    }

    // -- Flag helpers --

    pub fn set_zn(&mut self, val: u8) {
        self.status.set(CpuFlags::ZERO, val == 0);
        self.status.set(CpuFlags::NEGATIVE, val & 0x80 != 0);
    }

    /// Save CPU state to a byte buffer.
    pub fn save_state(&self, buf: &mut alloc::vec::Vec<u8>) {
        buf.push(self.a);
        buf.push(self.x);
        buf.push(self.y);
        buf.push(self.sp);
        buf.extend_from_slice(&self.pc.to_le_bytes());
        buf.push(self.status.bits());
        buf.extend_from_slice(&self.cycles.to_le_bytes());
        buf.extend_from_slice(&self.stall_cycles.to_le_bytes());
    }

    /// Load CPU state from a byte buffer. Returns false on invalid data.
    pub fn load_state(&mut self, data: &[u8], cursor: &mut usize) -> bool {
        if *cursor + 15 > data.len() {
            return false;
        }
        self.a = data[*cursor];
        *cursor += 1;
        self.x = data[*cursor];
        *cursor += 1;
        self.y = data[*cursor];
        *cursor += 1;
        self.sp = data[*cursor];
        *cursor += 1;
        self.pc = u16::from_le_bytes([data[*cursor], data[*cursor + 1]]);
        *cursor += 2;
        self.status = CpuFlags::from_bits_truncate(data[*cursor]);
        *cursor += 1;
        self.cycles = u64::from_le_bytes(data[*cursor..*cursor + 8].try_into().unwrap());
        *cursor += 8;
        // stall_cycles omitted from older states — treat as 0 if missing
        if *cursor + 2 <= data.len() {
            self.stall_cycles = u16::from_le_bytes([data[*cursor], data[*cursor + 1]]);
            *cursor += 2;
        }
        true
    }
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}
