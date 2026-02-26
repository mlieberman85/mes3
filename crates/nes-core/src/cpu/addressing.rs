use crate::bus::Bus;
use crate::cpu::Cpu;

/// Addressing mode result: the resolved address and whether a page boundary was crossed.
#[derive(Debug, Clone, Copy)]
pub struct AddrResult {
    pub addr: u16,
    pub page_cross: bool,
}

impl AddrResult {
    pub fn new(addr: u16, page_cross: bool) -> Self {
        Self { addr, page_cross }
    }
}

/// Implied / Accumulator — no memory address needed.
pub fn implied() -> AddrResult {
    AddrResult::new(0, false)
}

/// Immediate — operand is next byte (PC+1).
pub fn immediate(cpu: &Cpu) -> AddrResult {
    AddrResult::new(cpu.pc.wrapping_add(1), false)
}

/// Zero Page — operand at zero page address.
pub fn zero_page(cpu: &Cpu, bus: &Bus) -> AddrResult {
    let addr = bus.read(cpu.pc.wrapping_add(1)) as u16;
    AddrResult::new(addr, false)
}

/// Zero Page,X — zero page address + X register (wraps within page).
pub fn zero_page_x(cpu: &Cpu, bus: &Bus) -> AddrResult {
    let base = bus.read(cpu.pc.wrapping_add(1));
    let addr = base.wrapping_add(cpu.x) as u16;
    AddrResult::new(addr, false)
}

/// Zero Page,Y — zero page address + Y register (wraps within page).
pub fn zero_page_y(cpu: &Cpu, bus: &Bus) -> AddrResult {
    let base = bus.read(cpu.pc.wrapping_add(1));
    let addr = base.wrapping_add(cpu.y) as u16;
    AddrResult::new(addr, false)
}

/// Absolute — full 16-bit address.
pub fn absolute(cpu: &Cpu, bus: &Bus) -> AddrResult {
    let lo = bus.read(cpu.pc.wrapping_add(1)) as u16;
    let hi = bus.read(cpu.pc.wrapping_add(2)) as u16;
    AddrResult::new((hi << 8) | lo, false)
}

/// Absolute,X — absolute address + X register.
pub fn absolute_x(cpu: &Cpu, bus: &Bus) -> AddrResult {
    let lo = bus.read(cpu.pc.wrapping_add(1)) as u16;
    let hi = bus.read(cpu.pc.wrapping_add(2)) as u16;
    let base = (hi << 8) | lo;
    let addr = base.wrapping_add(cpu.x as u16);
    let page_cross = (base & 0xFF00) != (addr & 0xFF00);
    AddrResult::new(addr, page_cross)
}

/// Absolute,Y — absolute address + Y register.
pub fn absolute_y(cpu: &Cpu, bus: &Bus) -> AddrResult {
    let lo = bus.read(cpu.pc.wrapping_add(1)) as u16;
    let hi = bus.read(cpu.pc.wrapping_add(2)) as u16;
    let base = (hi << 8) | lo;
    let addr = base.wrapping_add(cpu.y as u16);
    let page_cross = (base & 0xFF00) != (addr & 0xFF00);
    AddrResult::new(addr, page_cross)
}

/// Indirect — JMP only. 16-bit pointer to the actual address.
/// Note: hardware bug — wraps within page on boundary.
pub fn indirect(cpu: &Cpu, bus: &Bus) -> AddrResult {
    let ptr_lo = bus.read(cpu.pc.wrapping_add(1)) as u16;
    let ptr_hi = bus.read(cpu.pc.wrapping_add(2)) as u16;
    let ptr = (ptr_hi << 8) | ptr_lo;

    // Hardware bug: if ptr is $xxFF, high byte comes from $xx00
    let lo = bus.read(ptr) as u16;
    let hi_addr = if ptr & 0x00FF == 0x00FF {
        ptr & 0xFF00 // wrap within page
    } else {
        ptr.wrapping_add(1)
    };
    let hi = bus.read(hi_addr) as u16;
    AddrResult::new((hi << 8) | lo, false)
}

/// Indexed Indirect (X) — ($nn,X). Pointer in zero page.
pub fn indexed_indirect(cpu: &Cpu, bus: &Bus) -> AddrResult {
    let base = bus.read(cpu.pc.wrapping_add(1));
    let ptr = base.wrapping_add(cpu.x);
    let lo = bus.read(ptr as u16) as u16;
    let hi = bus.read(ptr.wrapping_add(1) as u16) as u16;
    AddrResult::new((hi << 8) | lo, false)
}

/// Indirect Indexed (Y) — ($nn),Y. Pointer in zero page, then add Y.
pub fn indirect_indexed(cpu: &Cpu, bus: &Bus) -> AddrResult {
    let ptr = bus.read(cpu.pc.wrapping_add(1));
    let lo = bus.read(ptr as u16) as u16;
    let hi = bus.read(ptr.wrapping_add(1) as u16) as u16;
    let base = (hi << 8) | lo;
    let addr = base.wrapping_add(cpu.y as u16);
    let page_cross = (base & 0xFF00) != (addr & 0xFF00);
    AddrResult::new(addr, page_cross)
}

/// Relative — signed 8-bit offset from PC+2 (for branches).
pub fn relative(cpu: &Cpu, bus: &Bus) -> AddrResult {
    let offset = bus.read(cpu.pc.wrapping_add(1)) as i8;
    let base = cpu.pc.wrapping_add(2);
    let addr = base.wrapping_add(offset as u16);
    let page_cross = (base & 0xFF00) != (addr & 0xFF00);
    AddrResult::new(addr, page_cross)
}
