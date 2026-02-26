use crate::bus::Bus;
use crate::cpu::addressing::{
    absolute, absolute_x, absolute_y, immediate, indexed_indirect, indirect, indirect_indexed,
    relative, zero_page, zero_page_x, zero_page_y,
};
use crate::cpu::{Cpu, CpuFlags};

// ---------------------------------------------------------------------------
// Helper: branch -- all conditional branches share the same logic.
// Returns total cycles consumed (2 not taken, 3 taken, 4 taken + page cross).
// ---------------------------------------------------------------------------
#[inline]
fn branch(cpu: &mut Cpu, bus: &Bus, condition: bool) -> u8 {
    let r = relative(cpu, bus);
    cpu.pc = cpu.pc.wrapping_add(2);
    if condition {
        let extra = if r.page_cross { 2 } else { 1 };
        cpu.pc = r.addr;
        2 + extra
    } else {
        2
    }
}

// ---------------------------------------------------------------------------
// Helper: compare -- CMP / CPX / CPY logic
// ---------------------------------------------------------------------------
#[inline]
fn compare(cpu: &mut Cpu, reg: u8, val: u8) {
    let result = reg.wrapping_sub(val);
    cpu.status.set(CpuFlags::CARRY, reg >= val);
    cpu.set_zn(result);
}

/// Execute a single opcode. Returns the cycle count consumed.
pub fn execute(cpu: &mut Cpu, bus: &mut Bus, opcode: u8) -> u8 {
    match opcode {
        // =================================================================
        // ADC -- Add with Carry
        // =================================================================
        0x69 => {
            // ADC Immediate
            let r = immediate(cpu);
            let val = cpu.read(bus, r.addr);
            adc(cpu, val);
            cpu.pc = cpu.pc.wrapping_add(2);
            2
        }
        0x65 => {
            // ADC Zero Page
            let r = zero_page(cpu, bus);
            let val = cpu.read(bus, r.addr);
            adc(cpu, val);
            cpu.pc = cpu.pc.wrapping_add(2);
            3
        }
        0x75 => {
            // ADC Zero Page,X
            let r = zero_page_x(cpu, bus);
            let val = cpu.read(bus, r.addr);
            adc(cpu, val);
            cpu.pc = cpu.pc.wrapping_add(2);
            4
        }
        0x6D => {
            // ADC Absolute
            let r = absolute(cpu, bus);
            let val = cpu.read(bus, r.addr);
            adc(cpu, val);
            cpu.pc = cpu.pc.wrapping_add(3);
            4
        }
        0x7D => {
            // ADC Absolute,X
            let r = absolute_x(cpu, bus);
            let val = cpu.read(bus, r.addr);
            adc(cpu, val);
            cpu.pc = cpu.pc.wrapping_add(3);
            4 + r.page_cross as u8
        }
        0x79 => {
            // ADC Absolute,Y
            let r = absolute_y(cpu, bus);
            let val = cpu.read(bus, r.addr);
            adc(cpu, val);
            cpu.pc = cpu.pc.wrapping_add(3);
            4 + r.page_cross as u8
        }
        0x61 => {
            // ADC (Indirect,X)
            let r = indexed_indirect(cpu, bus);
            let val = cpu.read(bus, r.addr);
            adc(cpu, val);
            cpu.pc = cpu.pc.wrapping_add(2);
            6
        }
        0x71 => {
            // ADC (Indirect),Y
            let r = indirect_indexed(cpu, bus);
            let val = cpu.read(bus, r.addr);
            adc(cpu, val);
            cpu.pc = cpu.pc.wrapping_add(2);
            5 + r.page_cross as u8
        }

        // =================================================================
        // AND -- Logical AND
        // =================================================================
        0x29 => {
            let r = immediate(cpu);
            let val = cpu.read(bus, r.addr);
            cpu.a &= val;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            2
        }
        0x25 => {
            let r = zero_page(cpu, bus);
            let val = cpu.read(bus, r.addr);
            cpu.a &= val;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            3
        }
        0x35 => {
            let r = zero_page_x(cpu, bus);
            let val = cpu.read(bus, r.addr);
            cpu.a &= val;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            4
        }
        0x2D => {
            let r = absolute(cpu, bus);
            let val = cpu.read(bus, r.addr);
            cpu.a &= val;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(3);
            4
        }
        0x3D => {
            let r = absolute_x(cpu, bus);
            let val = cpu.read(bus, r.addr);
            cpu.a &= val;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(3);
            4 + r.page_cross as u8
        }
        0x39 => {
            let r = absolute_y(cpu, bus);
            let val = cpu.read(bus, r.addr);
            cpu.a &= val;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(3);
            4 + r.page_cross as u8
        }
        0x21 => {
            let r = indexed_indirect(cpu, bus);
            let val = cpu.read(bus, r.addr);
            cpu.a &= val;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            6
        }
        0x31 => {
            let r = indirect_indexed(cpu, bus);
            let val = cpu.read(bus, r.addr);
            cpu.a &= val;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            5 + r.page_cross as u8
        }

        // =================================================================
        // ASL -- Arithmetic Shift Left
        // =================================================================
        0x0A => {
            // ASL Accumulator
            let old = cpu.a;
            cpu.a = old << 1;
            cpu.status.set(CpuFlags::CARRY, old & 0x80 != 0);
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(1);
            2
        }
        0x06 => {
            let r = zero_page(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let result = val << 1;
            cpu.status.set(CpuFlags::CARRY, val & 0x80 != 0);
            cpu.write(bus, r.addr, result);
            cpu.set_zn(result);
            cpu.pc = cpu.pc.wrapping_add(2);
            5
        }
        0x16 => {
            let r = zero_page_x(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let result = val << 1;
            cpu.status.set(CpuFlags::CARRY, val & 0x80 != 0);
            cpu.write(bus, r.addr, result);
            cpu.set_zn(result);
            cpu.pc = cpu.pc.wrapping_add(2);
            6
        }
        0x0E => {
            let r = absolute(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let result = val << 1;
            cpu.status.set(CpuFlags::CARRY, val & 0x80 != 0);
            cpu.write(bus, r.addr, result);
            cpu.set_zn(result);
            cpu.pc = cpu.pc.wrapping_add(3);
            6
        }
        0x1E => {
            let r = absolute_x(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let result = val << 1;
            cpu.status.set(CpuFlags::CARRY, val & 0x80 != 0);
            cpu.write(bus, r.addr, result);
            cpu.set_zn(result);
            cpu.pc = cpu.pc.wrapping_add(3);
            7
        }

        // =================================================================
        // Branch Instructions
        // =================================================================
        0x90 => branch(cpu, bus, !cpu.status.contains(CpuFlags::CARRY)), // BCC
        0xB0 => branch(cpu, bus, cpu.status.contains(CpuFlags::CARRY)),  // BCS
        0xF0 => branch(cpu, bus, cpu.status.contains(CpuFlags::ZERO)),   // BEQ
        0x30 => branch(cpu, bus, cpu.status.contains(CpuFlags::NEGATIVE)), // BMI
        0xD0 => branch(cpu, bus, !cpu.status.contains(CpuFlags::ZERO)),  // BNE
        0x10 => branch(cpu, bus, !cpu.status.contains(CpuFlags::NEGATIVE)), // BPL
        0x50 => branch(cpu, bus, !cpu.status.contains(CpuFlags::OVERFLOW)), // BVC
        0x70 => branch(cpu, bus, cpu.status.contains(CpuFlags::OVERFLOW)), // BVS

        // =================================================================
        // BIT -- Bit Test
        // =================================================================
        0x24 => {
            let r = zero_page(cpu, bus);
            let val = cpu.read(bus, r.addr);
            cpu.status.set(CpuFlags::ZERO, cpu.a & val == 0);
            cpu.status.set(CpuFlags::OVERFLOW, val & 0x40 != 0);
            cpu.status.set(CpuFlags::NEGATIVE, val & 0x80 != 0);
            cpu.pc = cpu.pc.wrapping_add(2);
            3
        }
        0x2C => {
            let r = absolute(cpu, bus);
            let val = cpu.read(bus, r.addr);
            cpu.status.set(CpuFlags::ZERO, cpu.a & val == 0);
            cpu.status.set(CpuFlags::OVERFLOW, val & 0x40 != 0);
            cpu.status.set(CpuFlags::NEGATIVE, val & 0x80 != 0);
            cpu.pc = cpu.pc.wrapping_add(3);
            4
        }

        // =================================================================
        // BRK -- Force Interrupt
        // =================================================================
        0x00 => {
            cpu.pc = cpu.pc.wrapping_add(2); // BRK skips the byte after it
            cpu.push_u16(bus, cpu.pc);
            let flags = cpu.status | CpuFlags::BREAK | CpuFlags::UNUSED;
            cpu.push(bus, flags.bits());
            cpu.status.insert(CpuFlags::IRQ_DISABLE);
            cpu.pc = cpu.read_u16(bus, 0xFFFE);
            7
        }

        // =================================================================
        // CLC / CLD / CLI / CLV -- Clear flags
        // =================================================================
        0x18 => {
            cpu.status.remove(CpuFlags::CARRY);
            cpu.pc = cpu.pc.wrapping_add(1);
            2
        }
        0xD8 => {
            cpu.status.remove(CpuFlags::DECIMAL);
            cpu.pc = cpu.pc.wrapping_add(1);
            2
        }
        0x58 => {
            cpu.status.remove(CpuFlags::IRQ_DISABLE);
            cpu.pc = cpu.pc.wrapping_add(1);
            2
        }
        0xB8 => {
            cpu.status.remove(CpuFlags::OVERFLOW);
            cpu.pc = cpu.pc.wrapping_add(1);
            2
        }

        // =================================================================
        // CMP -- Compare Accumulator
        // =================================================================
        0xC9 => {
            let r = immediate(cpu);
            let val = cpu.read(bus, r.addr);
            compare(cpu, cpu.a, val);
            cpu.pc = cpu.pc.wrapping_add(2);
            2
        }
        0xC5 => {
            let r = zero_page(cpu, bus);
            let val = cpu.read(bus, r.addr);
            compare(cpu, cpu.a, val);
            cpu.pc = cpu.pc.wrapping_add(2);
            3
        }
        0xD5 => {
            let r = zero_page_x(cpu, bus);
            let val = cpu.read(bus, r.addr);
            compare(cpu, cpu.a, val);
            cpu.pc = cpu.pc.wrapping_add(2);
            4
        }
        0xCD => {
            let r = absolute(cpu, bus);
            let val = cpu.read(bus, r.addr);
            compare(cpu, cpu.a, val);
            cpu.pc = cpu.pc.wrapping_add(3);
            4
        }
        0xDD => {
            let r = absolute_x(cpu, bus);
            let val = cpu.read(bus, r.addr);
            compare(cpu, cpu.a, val);
            cpu.pc = cpu.pc.wrapping_add(3);
            4 + r.page_cross as u8
        }
        0xD9 => {
            let r = absolute_y(cpu, bus);
            let val = cpu.read(bus, r.addr);
            compare(cpu, cpu.a, val);
            cpu.pc = cpu.pc.wrapping_add(3);
            4 + r.page_cross as u8
        }
        0xC1 => {
            let r = indexed_indirect(cpu, bus);
            let val = cpu.read(bus, r.addr);
            compare(cpu, cpu.a, val);
            cpu.pc = cpu.pc.wrapping_add(2);
            6
        }
        0xD1 => {
            let r = indirect_indexed(cpu, bus);
            let val = cpu.read(bus, r.addr);
            compare(cpu, cpu.a, val);
            cpu.pc = cpu.pc.wrapping_add(2);
            5 + r.page_cross as u8
        }

        // =================================================================
        // CPX -- Compare X Register
        // =================================================================
        0xE0 => {
            let r = immediate(cpu);
            let val = cpu.read(bus, r.addr);
            compare(cpu, cpu.x, val);
            cpu.pc = cpu.pc.wrapping_add(2);
            2
        }
        0xE4 => {
            let r = zero_page(cpu, bus);
            let val = cpu.read(bus, r.addr);
            compare(cpu, cpu.x, val);
            cpu.pc = cpu.pc.wrapping_add(2);
            3
        }
        0xEC => {
            let r = absolute(cpu, bus);
            let val = cpu.read(bus, r.addr);
            compare(cpu, cpu.x, val);
            cpu.pc = cpu.pc.wrapping_add(3);
            4
        }

        // =================================================================
        // CPY -- Compare Y Register
        // =================================================================
        0xC0 => {
            let r = immediate(cpu);
            let val = cpu.read(bus, r.addr);
            compare(cpu, cpu.y, val);
            cpu.pc = cpu.pc.wrapping_add(2);
            2
        }
        0xC4 => {
            let r = zero_page(cpu, bus);
            let val = cpu.read(bus, r.addr);
            compare(cpu, cpu.y, val);
            cpu.pc = cpu.pc.wrapping_add(2);
            3
        }
        0xCC => {
            let r = absolute(cpu, bus);
            let val = cpu.read(bus, r.addr);
            compare(cpu, cpu.y, val);
            cpu.pc = cpu.pc.wrapping_add(3);
            4
        }

        // =================================================================
        // DEC -- Decrement Memory
        // =================================================================
        0xC6 => {
            let r = zero_page(cpu, bus);
            let val = cpu.read(bus, r.addr).wrapping_sub(1);
            cpu.write(bus, r.addr, val);
            cpu.set_zn(val);
            cpu.pc = cpu.pc.wrapping_add(2);
            5
        }
        0xD6 => {
            let r = zero_page_x(cpu, bus);
            let val = cpu.read(bus, r.addr).wrapping_sub(1);
            cpu.write(bus, r.addr, val);
            cpu.set_zn(val);
            cpu.pc = cpu.pc.wrapping_add(2);
            6
        }
        0xCE => {
            let r = absolute(cpu, bus);
            let val = cpu.read(bus, r.addr).wrapping_sub(1);
            cpu.write(bus, r.addr, val);
            cpu.set_zn(val);
            cpu.pc = cpu.pc.wrapping_add(3);
            6
        }
        0xDE => {
            let r = absolute_x(cpu, bus);
            let val = cpu.read(bus, r.addr).wrapping_sub(1);
            cpu.write(bus, r.addr, val);
            cpu.set_zn(val);
            cpu.pc = cpu.pc.wrapping_add(3);
            7
        }

        // =================================================================
        // DEX / DEY -- Decrement X / Y Register
        // =================================================================
        0xCA => {
            cpu.x = cpu.x.wrapping_sub(1);
            cpu.set_zn(cpu.x);
            cpu.pc = cpu.pc.wrapping_add(1);
            2
        }
        0x88 => {
            cpu.y = cpu.y.wrapping_sub(1);
            cpu.set_zn(cpu.y);
            cpu.pc = cpu.pc.wrapping_add(1);
            2
        }

        // =================================================================
        // EOR -- Exclusive OR
        // =================================================================
        0x49 => {
            let r = immediate(cpu);
            let val = cpu.read(bus, r.addr);
            cpu.a ^= val;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            2
        }
        0x45 => {
            let r = zero_page(cpu, bus);
            let val = cpu.read(bus, r.addr);
            cpu.a ^= val;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            3
        }
        0x55 => {
            let r = zero_page_x(cpu, bus);
            let val = cpu.read(bus, r.addr);
            cpu.a ^= val;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            4
        }
        0x4D => {
            let r = absolute(cpu, bus);
            let val = cpu.read(bus, r.addr);
            cpu.a ^= val;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(3);
            4
        }
        0x5D => {
            let r = absolute_x(cpu, bus);
            let val = cpu.read(bus, r.addr);
            cpu.a ^= val;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(3);
            4 + r.page_cross as u8
        }
        0x59 => {
            let r = absolute_y(cpu, bus);
            let val = cpu.read(bus, r.addr);
            cpu.a ^= val;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(3);
            4 + r.page_cross as u8
        }
        0x41 => {
            let r = indexed_indirect(cpu, bus);
            let val = cpu.read(bus, r.addr);
            cpu.a ^= val;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            6
        }
        0x51 => {
            let r = indirect_indexed(cpu, bus);
            let val = cpu.read(bus, r.addr);
            cpu.a ^= val;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            5 + r.page_cross as u8
        }

        // =================================================================
        // INC -- Increment Memory
        // =================================================================
        0xE6 => {
            let r = zero_page(cpu, bus);
            let val = cpu.read(bus, r.addr).wrapping_add(1);
            cpu.write(bus, r.addr, val);
            cpu.set_zn(val);
            cpu.pc = cpu.pc.wrapping_add(2);
            5
        }
        0xF6 => {
            let r = zero_page_x(cpu, bus);
            let val = cpu.read(bus, r.addr).wrapping_add(1);
            cpu.write(bus, r.addr, val);
            cpu.set_zn(val);
            cpu.pc = cpu.pc.wrapping_add(2);
            6
        }
        0xEE => {
            let r = absolute(cpu, bus);
            let val = cpu.read(bus, r.addr).wrapping_add(1);
            cpu.write(bus, r.addr, val);
            cpu.set_zn(val);
            cpu.pc = cpu.pc.wrapping_add(3);
            6
        }
        0xFE => {
            let r = absolute_x(cpu, bus);
            let val = cpu.read(bus, r.addr).wrapping_add(1);
            cpu.write(bus, r.addr, val);
            cpu.set_zn(val);
            cpu.pc = cpu.pc.wrapping_add(3);
            7
        }

        // =================================================================
        // INX / INY -- Increment X / Y Register
        // =================================================================
        0xE8 => {
            cpu.x = cpu.x.wrapping_add(1);
            cpu.set_zn(cpu.x);
            cpu.pc = cpu.pc.wrapping_add(1);
            2
        }
        0xC8 => {
            cpu.y = cpu.y.wrapping_add(1);
            cpu.set_zn(cpu.y);
            cpu.pc = cpu.pc.wrapping_add(1);
            2
        }

        // =================================================================
        // JMP -- Jump
        // =================================================================
        0x4C => {
            // JMP Absolute
            let r = absolute(cpu, bus);
            cpu.pc = r.addr;
            3
        }
        0x6C => {
            // JMP Indirect (with hardware page-wrap bug)
            let r = indirect(cpu, bus);
            cpu.pc = r.addr;
            5
        }

        // =================================================================
        // JSR -- Jump to Subroutine
        // =================================================================
        0x20 => {
            let r = absolute(cpu, bus);
            let ret = cpu.pc.wrapping_add(2); // push address of last byte of JSR
            cpu.push_u16(bus, ret);
            cpu.pc = r.addr;
            6
        }

        // =================================================================
        // LDA -- Load Accumulator
        // =================================================================
        0xA9 => {
            let r = immediate(cpu);
            cpu.a = cpu.read(bus, r.addr);
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            2
        }
        0xA5 => {
            let r = zero_page(cpu, bus);
            cpu.a = cpu.read(bus, r.addr);
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            3
        }
        0xB5 => {
            let r = zero_page_x(cpu, bus);
            cpu.a = cpu.read(bus, r.addr);
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            4
        }
        0xAD => {
            let r = absolute(cpu, bus);
            cpu.a = bus.read_mut(r.addr);
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(3);
            4
        }
        0xBD => {
            let r = absolute_x(cpu, bus);
            cpu.a = bus.read_mut(r.addr);
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(3);
            4 + r.page_cross as u8
        }
        0xB9 => {
            let r = absolute_y(cpu, bus);
            cpu.a = bus.read_mut(r.addr);
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(3);
            4 + r.page_cross as u8
        }
        0xA1 => {
            let r = indexed_indirect(cpu, bus);
            cpu.a = bus.read_mut(r.addr);
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            6
        }
        0xB1 => {
            let r = indirect_indexed(cpu, bus);
            cpu.a = bus.read_mut(r.addr);
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            5 + r.page_cross as u8
        }

        // =================================================================
        // LDX -- Load X Register
        // =================================================================
        0xA2 => {
            let r = immediate(cpu);
            cpu.x = cpu.read(bus, r.addr);
            cpu.set_zn(cpu.x);
            cpu.pc = cpu.pc.wrapping_add(2);
            2
        }
        0xA6 => {
            let r = zero_page(cpu, bus);
            cpu.x = cpu.read(bus, r.addr);
            cpu.set_zn(cpu.x);
            cpu.pc = cpu.pc.wrapping_add(2);
            3
        }
        0xB6 => {
            let r = zero_page_y(cpu, bus);
            cpu.x = cpu.read(bus, r.addr);
            cpu.set_zn(cpu.x);
            cpu.pc = cpu.pc.wrapping_add(2);
            4
        }
        0xAE => {
            let r = absolute(cpu, bus);
            cpu.x = bus.read_mut(r.addr);
            cpu.set_zn(cpu.x);
            cpu.pc = cpu.pc.wrapping_add(3);
            4
        }
        0xBE => {
            let r = absolute_y(cpu, bus);
            cpu.x = bus.read_mut(r.addr);
            cpu.set_zn(cpu.x);
            cpu.pc = cpu.pc.wrapping_add(3);
            4 + r.page_cross as u8
        }

        // =================================================================
        // LDY -- Load Y Register
        // =================================================================
        0xA0 => {
            let r = immediate(cpu);
            cpu.y = cpu.read(bus, r.addr);
            cpu.set_zn(cpu.y);
            cpu.pc = cpu.pc.wrapping_add(2);
            2
        }
        0xA4 => {
            let r = zero_page(cpu, bus);
            cpu.y = cpu.read(bus, r.addr);
            cpu.set_zn(cpu.y);
            cpu.pc = cpu.pc.wrapping_add(2);
            3
        }
        0xB4 => {
            let r = zero_page_x(cpu, bus);
            cpu.y = cpu.read(bus, r.addr);
            cpu.set_zn(cpu.y);
            cpu.pc = cpu.pc.wrapping_add(2);
            4
        }
        0xAC => {
            let r = absolute(cpu, bus);
            cpu.y = bus.read_mut(r.addr);
            cpu.set_zn(cpu.y);
            cpu.pc = cpu.pc.wrapping_add(3);
            4
        }
        0xBC => {
            let r = absolute_x(cpu, bus);
            cpu.y = bus.read_mut(r.addr);
            cpu.set_zn(cpu.y);
            cpu.pc = cpu.pc.wrapping_add(3);
            4 + r.page_cross as u8
        }

        // =================================================================
        // LSR -- Logical Shift Right
        // =================================================================
        0x4A => {
            // LSR Accumulator
            let old = cpu.a;
            cpu.a = old >> 1;
            cpu.status.set(CpuFlags::CARRY, old & 0x01 != 0);
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(1);
            2
        }
        0x46 => {
            let r = zero_page(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let result = val >> 1;
            cpu.status.set(CpuFlags::CARRY, val & 0x01 != 0);
            cpu.write(bus, r.addr, result);
            cpu.set_zn(result);
            cpu.pc = cpu.pc.wrapping_add(2);
            5
        }
        0x56 => {
            let r = zero_page_x(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let result = val >> 1;
            cpu.status.set(CpuFlags::CARRY, val & 0x01 != 0);
            cpu.write(bus, r.addr, result);
            cpu.set_zn(result);
            cpu.pc = cpu.pc.wrapping_add(2);
            6
        }
        0x4E => {
            let r = absolute(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let result = val >> 1;
            cpu.status.set(CpuFlags::CARRY, val & 0x01 != 0);
            cpu.write(bus, r.addr, result);
            cpu.set_zn(result);
            cpu.pc = cpu.pc.wrapping_add(3);
            6
        }
        0x5E => {
            let r = absolute_x(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let result = val >> 1;
            cpu.status.set(CpuFlags::CARRY, val & 0x01 != 0);
            cpu.write(bus, r.addr, result);
            cpu.set_zn(result);
            cpu.pc = cpu.pc.wrapping_add(3);
            7
        }

        // =================================================================
        // NOP -- No Operation (official)
        // =================================================================
        0xEA => {
            cpu.pc = cpu.pc.wrapping_add(1);
            2
        }

        // =================================================================
        // ORA -- Logical Inclusive OR
        // =================================================================
        0x09 => {
            let r = immediate(cpu);
            let val = cpu.read(bus, r.addr);
            cpu.a |= val;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            2
        }
        0x05 => {
            let r = zero_page(cpu, bus);
            let val = cpu.read(bus, r.addr);
            cpu.a |= val;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            3
        }
        0x15 => {
            let r = zero_page_x(cpu, bus);
            let val = cpu.read(bus, r.addr);
            cpu.a |= val;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            4
        }
        0x0D => {
            let r = absolute(cpu, bus);
            let val = cpu.read(bus, r.addr);
            cpu.a |= val;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(3);
            4
        }
        0x1D => {
            let r = absolute_x(cpu, bus);
            let val = cpu.read(bus, r.addr);
            cpu.a |= val;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(3);
            4 + r.page_cross as u8
        }
        0x19 => {
            let r = absolute_y(cpu, bus);
            let val = cpu.read(bus, r.addr);
            cpu.a |= val;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(3);
            4 + r.page_cross as u8
        }
        0x01 => {
            let r = indexed_indirect(cpu, bus);
            let val = cpu.read(bus, r.addr);
            cpu.a |= val;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            6
        }
        0x11 => {
            let r = indirect_indexed(cpu, bus);
            let val = cpu.read(bus, r.addr);
            cpu.a |= val;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            5 + r.page_cross as u8
        }

        // =================================================================
        // PHA / PHP / PLA / PLP -- Stack Operations
        // =================================================================
        0x48 => {
            // PHA
            cpu.push(bus, cpu.a);
            cpu.pc = cpu.pc.wrapping_add(1);
            3
        }
        0x08 => {
            // PHP -- push with BREAK and UNUSED set
            let flags = cpu.status | CpuFlags::BREAK | CpuFlags::UNUSED;
            cpu.push(bus, flags.bits());
            cpu.pc = cpu.pc.wrapping_add(1);
            3
        }
        0x68 => {
            // PLA
            cpu.a = cpu.pull(bus);
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(1);
            4
        }
        0x28 => {
            // PLP -- pull, ignore BREAK and UNUSED (keep them as they were)
            let val = cpu.pull(bus);
            cpu.status = CpuFlags::from_bits_truncate(val);
            cpu.status.remove(CpuFlags::BREAK);
            cpu.status.insert(CpuFlags::UNUSED);
            cpu.pc = cpu.pc.wrapping_add(1);
            4
        }

        // =================================================================
        // ROL -- Rotate Left
        // =================================================================
        0x2A => {
            // ROL Accumulator
            let old = cpu.a;
            let carry_in = cpu.status.contains(CpuFlags::CARRY) as u8;
            cpu.a = (old << 1) | carry_in;
            cpu.status.set(CpuFlags::CARRY, old & 0x80 != 0);
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(1);
            2
        }
        0x26 => {
            let r = zero_page(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let carry_in = cpu.status.contains(CpuFlags::CARRY) as u8;
            let result = (val << 1) | carry_in;
            cpu.status.set(CpuFlags::CARRY, val & 0x80 != 0);
            cpu.write(bus, r.addr, result);
            cpu.set_zn(result);
            cpu.pc = cpu.pc.wrapping_add(2);
            5
        }
        0x36 => {
            let r = zero_page_x(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let carry_in = cpu.status.contains(CpuFlags::CARRY) as u8;
            let result = (val << 1) | carry_in;
            cpu.status.set(CpuFlags::CARRY, val & 0x80 != 0);
            cpu.write(bus, r.addr, result);
            cpu.set_zn(result);
            cpu.pc = cpu.pc.wrapping_add(2);
            6
        }
        0x2E => {
            let r = absolute(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let carry_in = cpu.status.contains(CpuFlags::CARRY) as u8;
            let result = (val << 1) | carry_in;
            cpu.status.set(CpuFlags::CARRY, val & 0x80 != 0);
            cpu.write(bus, r.addr, result);
            cpu.set_zn(result);
            cpu.pc = cpu.pc.wrapping_add(3);
            6
        }
        0x3E => {
            let r = absolute_x(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let carry_in = cpu.status.contains(CpuFlags::CARRY) as u8;
            let result = (val << 1) | carry_in;
            cpu.status.set(CpuFlags::CARRY, val & 0x80 != 0);
            cpu.write(bus, r.addr, result);
            cpu.set_zn(result);
            cpu.pc = cpu.pc.wrapping_add(3);
            7
        }

        // =================================================================
        // ROR -- Rotate Right
        // =================================================================
        0x6A => {
            // ROR Accumulator
            let old = cpu.a;
            let carry_in = (cpu.status.contains(CpuFlags::CARRY) as u8) << 7;
            cpu.a = (old >> 1) | carry_in;
            cpu.status.set(CpuFlags::CARRY, old & 0x01 != 0);
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(1);
            2
        }
        0x66 => {
            let r = zero_page(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let carry_in = (cpu.status.contains(CpuFlags::CARRY) as u8) << 7;
            let result = (val >> 1) | carry_in;
            cpu.status.set(CpuFlags::CARRY, val & 0x01 != 0);
            cpu.write(bus, r.addr, result);
            cpu.set_zn(result);
            cpu.pc = cpu.pc.wrapping_add(2);
            5
        }
        0x76 => {
            let r = zero_page_x(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let carry_in = (cpu.status.contains(CpuFlags::CARRY) as u8) << 7;
            let result = (val >> 1) | carry_in;
            cpu.status.set(CpuFlags::CARRY, val & 0x01 != 0);
            cpu.write(bus, r.addr, result);
            cpu.set_zn(result);
            cpu.pc = cpu.pc.wrapping_add(2);
            6
        }
        0x6E => {
            let r = absolute(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let carry_in = (cpu.status.contains(CpuFlags::CARRY) as u8) << 7;
            let result = (val >> 1) | carry_in;
            cpu.status.set(CpuFlags::CARRY, val & 0x01 != 0);
            cpu.write(bus, r.addr, result);
            cpu.set_zn(result);
            cpu.pc = cpu.pc.wrapping_add(3);
            6
        }
        0x7E => {
            let r = absolute_x(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let carry_in = (cpu.status.contains(CpuFlags::CARRY) as u8) << 7;
            let result = (val >> 1) | carry_in;
            cpu.status.set(CpuFlags::CARRY, val & 0x01 != 0);
            cpu.write(bus, r.addr, result);
            cpu.set_zn(result);
            cpu.pc = cpu.pc.wrapping_add(3);
            7
        }

        // =================================================================
        // RTI -- Return from Interrupt
        // =================================================================
        0x40 => {
            let flags = cpu.pull(bus);
            cpu.status = CpuFlags::from_bits_truncate(flags);
            cpu.status.remove(CpuFlags::BREAK);
            cpu.status.insert(CpuFlags::UNUSED);
            cpu.pc = cpu.pull_u16(bus);
            6
        }

        // =================================================================
        // RTS -- Return from Subroutine
        // =================================================================
        0x60 => {
            cpu.pc = cpu.pull_u16(bus).wrapping_add(1);
            6
        }

        // =================================================================
        // SBC -- Subtract with Carry
        // =================================================================
        0xE9 => {
            let r = immediate(cpu);
            let val = cpu.read(bus, r.addr);
            sbc(cpu, val);
            cpu.pc = cpu.pc.wrapping_add(2);
            2
        }
        0xE5 => {
            let r = zero_page(cpu, bus);
            let val = cpu.read(bus, r.addr);
            sbc(cpu, val);
            cpu.pc = cpu.pc.wrapping_add(2);
            3
        }
        0xF5 => {
            let r = zero_page_x(cpu, bus);
            let val = cpu.read(bus, r.addr);
            sbc(cpu, val);
            cpu.pc = cpu.pc.wrapping_add(2);
            4
        }
        0xED => {
            let r = absolute(cpu, bus);
            let val = cpu.read(bus, r.addr);
            sbc(cpu, val);
            cpu.pc = cpu.pc.wrapping_add(3);
            4
        }
        0xFD => {
            let r = absolute_x(cpu, bus);
            let val = cpu.read(bus, r.addr);
            sbc(cpu, val);
            cpu.pc = cpu.pc.wrapping_add(3);
            4 + r.page_cross as u8
        }
        0xF9 => {
            let r = absolute_y(cpu, bus);
            let val = cpu.read(bus, r.addr);
            sbc(cpu, val);
            cpu.pc = cpu.pc.wrapping_add(3);
            4 + r.page_cross as u8
        }
        0xE1 => {
            let r = indexed_indirect(cpu, bus);
            let val = cpu.read(bus, r.addr);
            sbc(cpu, val);
            cpu.pc = cpu.pc.wrapping_add(2);
            6
        }
        0xF1 => {
            let r = indirect_indexed(cpu, bus);
            let val = cpu.read(bus, r.addr);
            sbc(cpu, val);
            cpu.pc = cpu.pc.wrapping_add(2);
            5 + r.page_cross as u8
        }

        // =================================================================
        // SEC / SED / SEI -- Set flags
        // =================================================================
        0x38 => {
            cpu.status.insert(CpuFlags::CARRY);
            cpu.pc = cpu.pc.wrapping_add(1);
            2
        }
        0xF8 => {
            cpu.status.insert(CpuFlags::DECIMAL);
            cpu.pc = cpu.pc.wrapping_add(1);
            2
        }
        0x78 => {
            cpu.status.insert(CpuFlags::IRQ_DISABLE);
            cpu.pc = cpu.pc.wrapping_add(1);
            2
        }

        // =================================================================
        // STA -- Store Accumulator
        // =================================================================
        0x85 => {
            let r = zero_page(cpu, bus);
            cpu.write(bus, r.addr, cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            3
        }
        0x95 => {
            let r = zero_page_x(cpu, bus);
            cpu.write(bus, r.addr, cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            4
        }
        0x8D => {
            let r = absolute(cpu, bus);
            cpu.write(bus, r.addr, cpu.a);
            cpu.pc = cpu.pc.wrapping_add(3);
            4
        }
        0x9D => {
            let r = absolute_x(cpu, bus);
            cpu.write(bus, r.addr, cpu.a);
            cpu.pc = cpu.pc.wrapping_add(3);
            5 // always 5 for stores
        }
        0x99 => {
            let r = absolute_y(cpu, bus);
            cpu.write(bus, r.addr, cpu.a);
            cpu.pc = cpu.pc.wrapping_add(3);
            5
        }
        0x81 => {
            let r = indexed_indirect(cpu, bus);
            cpu.write(bus, r.addr, cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            6
        }
        0x91 => {
            let r = indirect_indexed(cpu, bus);
            cpu.write(bus, r.addr, cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            6 // always 6 for stores
        }

        // =================================================================
        // STX -- Store X Register
        // =================================================================
        0x86 => {
            let r = zero_page(cpu, bus);
            cpu.write(bus, r.addr, cpu.x);
            cpu.pc = cpu.pc.wrapping_add(2);
            3
        }
        0x96 => {
            let r = zero_page_y(cpu, bus);
            cpu.write(bus, r.addr, cpu.x);
            cpu.pc = cpu.pc.wrapping_add(2);
            4
        }
        0x8E => {
            let r = absolute(cpu, bus);
            cpu.write(bus, r.addr, cpu.x);
            cpu.pc = cpu.pc.wrapping_add(3);
            4
        }

        // =================================================================
        // STY -- Store Y Register
        // =================================================================
        0x84 => {
            let r = zero_page(cpu, bus);
            cpu.write(bus, r.addr, cpu.y);
            cpu.pc = cpu.pc.wrapping_add(2);
            3
        }
        0x94 => {
            let r = zero_page_x(cpu, bus);
            cpu.write(bus, r.addr, cpu.y);
            cpu.pc = cpu.pc.wrapping_add(2);
            4
        }
        0x8C => {
            let r = absolute(cpu, bus);
            cpu.write(bus, r.addr, cpu.y);
            cpu.pc = cpu.pc.wrapping_add(3);
            4
        }

        // =================================================================
        // TAX / TAY / TSX / TXA / TXS / TYA -- Transfer Instructions
        // =================================================================
        0xAA => {
            cpu.x = cpu.a;
            cpu.set_zn(cpu.x);
            cpu.pc = cpu.pc.wrapping_add(1);
            2
        } // TAX
        0xA8 => {
            cpu.y = cpu.a;
            cpu.set_zn(cpu.y);
            cpu.pc = cpu.pc.wrapping_add(1);
            2
        } // TAY
        0xBA => {
            cpu.x = cpu.sp;
            cpu.set_zn(cpu.x);
            cpu.pc = cpu.pc.wrapping_add(1);
            2
        } // TSX
        0x8A => {
            cpu.a = cpu.x;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(1);
            2
        } // TXA
        0x9A => {
            cpu.sp = cpu.x;
            cpu.pc = cpu.pc.wrapping_add(1);
            2
        } // TXS (no flags)
        0x98 => {
            cpu.a = cpu.y;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(1);
            2
        } // TYA

        // =================================================================
        //
        //                UNDOCUMENTED / ILLEGAL OPCODES
        //
        // =================================================================

        // =================================================================
        // *SBC (EB) -- Identical to official SBC immediate
        // =================================================================
        0xEB => {
            let r = immediate(cpu);
            let val = cpu.read(bus, r.addr);
            sbc(cpu, val);
            cpu.pc = cpu.pc.wrapping_add(2);
            2
        }

        // =================================================================
        // *LAX -- Load A and X  (LDA + LDX combined)
        // =================================================================
        0xA7 => {
            // LAX Zero Page
            let r = zero_page(cpu, bus);
            let val = cpu.read(bus, r.addr);
            cpu.a = val;
            cpu.x = val;
            cpu.set_zn(val);
            cpu.pc = cpu.pc.wrapping_add(2);
            3
        }
        0xB7 => {
            // LAX Zero Page,Y
            let r = zero_page_y(cpu, bus);
            let val = cpu.read(bus, r.addr);
            cpu.a = val;
            cpu.x = val;
            cpu.set_zn(val);
            cpu.pc = cpu.pc.wrapping_add(2);
            4
        }
        0xAF => {
            // LAX Absolute
            let r = absolute(cpu, bus);
            let val = cpu.read(bus, r.addr);
            cpu.a = val;
            cpu.x = val;
            cpu.set_zn(val);
            cpu.pc = cpu.pc.wrapping_add(3);
            4
        }
        0xBF => {
            // LAX Absolute,Y
            let r = absolute_y(cpu, bus);
            let val = cpu.read(bus, r.addr);
            cpu.a = val;
            cpu.x = val;
            cpu.set_zn(val);
            cpu.pc = cpu.pc.wrapping_add(3);
            4 + r.page_cross as u8
        }
        0xA3 => {
            // LAX (Indirect,X)
            let r = indexed_indirect(cpu, bus);
            let val = cpu.read(bus, r.addr);
            cpu.a = val;
            cpu.x = val;
            cpu.set_zn(val);
            cpu.pc = cpu.pc.wrapping_add(2);
            6
        }
        0xB3 => {
            // LAX (Indirect),Y
            let r = indirect_indexed(cpu, bus);
            let val = cpu.read(bus, r.addr);
            cpu.a = val;
            cpu.x = val;
            cpu.set_zn(val);
            cpu.pc = cpu.pc.wrapping_add(2);
            5 + r.page_cross as u8
        }

        // =================================================================
        // *SAX -- Store A & X  (writes A AND X to memory)
        // =================================================================
        0x87 => {
            // SAX Zero Page
            let r = zero_page(cpu, bus);
            cpu.write(bus, r.addr, cpu.a & cpu.x);
            cpu.pc = cpu.pc.wrapping_add(2);
            3
        }
        0x97 => {
            // SAX Zero Page,Y
            let r = zero_page_y(cpu, bus);
            cpu.write(bus, r.addr, cpu.a & cpu.x);
            cpu.pc = cpu.pc.wrapping_add(2);
            4
        }
        0x8F => {
            // SAX Absolute
            let r = absolute(cpu, bus);
            cpu.write(bus, r.addr, cpu.a & cpu.x);
            cpu.pc = cpu.pc.wrapping_add(3);
            4
        }
        0x83 => {
            // SAX (Indirect,X)
            let r = indexed_indirect(cpu, bus);
            cpu.write(bus, r.addr, cpu.a & cpu.x);
            cpu.pc = cpu.pc.wrapping_add(2);
            6
        }

        // =================================================================
        // *DCP -- Decrement memory then Compare (DEC + CMP)
        // =================================================================
        0xC7 => {
            let r = zero_page(cpu, bus);
            let val = cpu.read(bus, r.addr).wrapping_sub(1);
            cpu.write(bus, r.addr, val);
            compare(cpu, cpu.a, val);
            cpu.pc = cpu.pc.wrapping_add(2);
            5
        }
        0xD7 => {
            let r = zero_page_x(cpu, bus);
            let val = cpu.read(bus, r.addr).wrapping_sub(1);
            cpu.write(bus, r.addr, val);
            compare(cpu, cpu.a, val);
            cpu.pc = cpu.pc.wrapping_add(2);
            6
        }
        0xCF => {
            let r = absolute(cpu, bus);
            let val = cpu.read(bus, r.addr).wrapping_sub(1);
            cpu.write(bus, r.addr, val);
            compare(cpu, cpu.a, val);
            cpu.pc = cpu.pc.wrapping_add(3);
            6
        }
        0xDF => {
            let r = absolute_x(cpu, bus);
            let val = cpu.read(bus, r.addr).wrapping_sub(1);
            cpu.write(bus, r.addr, val);
            compare(cpu, cpu.a, val);
            cpu.pc = cpu.pc.wrapping_add(3);
            7
        }
        0xDB => {
            let r = absolute_y(cpu, bus);
            let val = cpu.read(bus, r.addr).wrapping_sub(1);
            cpu.write(bus, r.addr, val);
            compare(cpu, cpu.a, val);
            cpu.pc = cpu.pc.wrapping_add(3);
            7
        }
        0xC3 => {
            let r = indexed_indirect(cpu, bus);
            let val = cpu.read(bus, r.addr).wrapping_sub(1);
            cpu.write(bus, r.addr, val);
            compare(cpu, cpu.a, val);
            cpu.pc = cpu.pc.wrapping_add(2);
            8
        }
        0xD3 => {
            let r = indirect_indexed(cpu, bus);
            let val = cpu.read(bus, r.addr).wrapping_sub(1);
            cpu.write(bus, r.addr, val);
            compare(cpu, cpu.a, val);
            cpu.pc = cpu.pc.wrapping_add(2);
            8
        }

        // =================================================================
        // *ISB / ISC -- Increment memory then SBC (INC + SBC)
        // =================================================================
        0xE7 => {
            let r = zero_page(cpu, bus);
            let val = cpu.read(bus, r.addr).wrapping_add(1);
            cpu.write(bus, r.addr, val);
            sbc(cpu, val);
            cpu.pc = cpu.pc.wrapping_add(2);
            5
        }
        0xF7 => {
            let r = zero_page_x(cpu, bus);
            let val = cpu.read(bus, r.addr).wrapping_add(1);
            cpu.write(bus, r.addr, val);
            sbc(cpu, val);
            cpu.pc = cpu.pc.wrapping_add(2);
            6
        }
        0xEF => {
            let r = absolute(cpu, bus);
            let val = cpu.read(bus, r.addr).wrapping_add(1);
            cpu.write(bus, r.addr, val);
            sbc(cpu, val);
            cpu.pc = cpu.pc.wrapping_add(3);
            6
        }
        0xFF => {
            let r = absolute_x(cpu, bus);
            let val = cpu.read(bus, r.addr).wrapping_add(1);
            cpu.write(bus, r.addr, val);
            sbc(cpu, val);
            cpu.pc = cpu.pc.wrapping_add(3);
            7
        }
        0xFB => {
            let r = absolute_y(cpu, bus);
            let val = cpu.read(bus, r.addr).wrapping_add(1);
            cpu.write(bus, r.addr, val);
            sbc(cpu, val);
            cpu.pc = cpu.pc.wrapping_add(3);
            7
        }
        0xE3 => {
            let r = indexed_indirect(cpu, bus);
            let val = cpu.read(bus, r.addr).wrapping_add(1);
            cpu.write(bus, r.addr, val);
            sbc(cpu, val);
            cpu.pc = cpu.pc.wrapping_add(2);
            8
        }
        0xF3 => {
            let r = indirect_indexed(cpu, bus);
            let val = cpu.read(bus, r.addr).wrapping_add(1);
            cpu.write(bus, r.addr, val);
            sbc(cpu, val);
            cpu.pc = cpu.pc.wrapping_add(2);
            8
        }

        // =================================================================
        // *SLO -- Shift Left then OR (ASL + ORA)
        // =================================================================
        0x07 => {
            let r = zero_page(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let result = val << 1;
            cpu.status.set(CpuFlags::CARRY, val & 0x80 != 0);
            cpu.write(bus, r.addr, result);
            cpu.a |= result;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            5
        }
        0x17 => {
            let r = zero_page_x(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let result = val << 1;
            cpu.status.set(CpuFlags::CARRY, val & 0x80 != 0);
            cpu.write(bus, r.addr, result);
            cpu.a |= result;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            6
        }
        0x0F => {
            let r = absolute(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let result = val << 1;
            cpu.status.set(CpuFlags::CARRY, val & 0x80 != 0);
            cpu.write(bus, r.addr, result);
            cpu.a |= result;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(3);
            6
        }
        0x1F => {
            let r = absolute_x(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let result = val << 1;
            cpu.status.set(CpuFlags::CARRY, val & 0x80 != 0);
            cpu.write(bus, r.addr, result);
            cpu.a |= result;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(3);
            7
        }
        0x1B => {
            let r = absolute_y(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let result = val << 1;
            cpu.status.set(CpuFlags::CARRY, val & 0x80 != 0);
            cpu.write(bus, r.addr, result);
            cpu.a |= result;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(3);
            7
        }
        0x03 => {
            let r = indexed_indirect(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let result = val << 1;
            cpu.status.set(CpuFlags::CARRY, val & 0x80 != 0);
            cpu.write(bus, r.addr, result);
            cpu.a |= result;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            8
        }
        0x13 => {
            let r = indirect_indexed(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let result = val << 1;
            cpu.status.set(CpuFlags::CARRY, val & 0x80 != 0);
            cpu.write(bus, r.addr, result);
            cpu.a |= result;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            8
        }

        // =================================================================
        // *RLA -- Rotate Left then AND (ROL + AND)
        // =================================================================
        0x27 => {
            let r = zero_page(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let carry_in = cpu.status.contains(CpuFlags::CARRY) as u8;
            let result = (val << 1) | carry_in;
            cpu.status.set(CpuFlags::CARRY, val & 0x80 != 0);
            cpu.write(bus, r.addr, result);
            cpu.a &= result;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            5
        }
        0x37 => {
            let r = zero_page_x(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let carry_in = cpu.status.contains(CpuFlags::CARRY) as u8;
            let result = (val << 1) | carry_in;
            cpu.status.set(CpuFlags::CARRY, val & 0x80 != 0);
            cpu.write(bus, r.addr, result);
            cpu.a &= result;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            6
        }
        0x2F => {
            let r = absolute(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let carry_in = cpu.status.contains(CpuFlags::CARRY) as u8;
            let result = (val << 1) | carry_in;
            cpu.status.set(CpuFlags::CARRY, val & 0x80 != 0);
            cpu.write(bus, r.addr, result);
            cpu.a &= result;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(3);
            6
        }
        0x3F => {
            let r = absolute_x(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let carry_in = cpu.status.contains(CpuFlags::CARRY) as u8;
            let result = (val << 1) | carry_in;
            cpu.status.set(CpuFlags::CARRY, val & 0x80 != 0);
            cpu.write(bus, r.addr, result);
            cpu.a &= result;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(3);
            7
        }
        0x3B => {
            let r = absolute_y(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let carry_in = cpu.status.contains(CpuFlags::CARRY) as u8;
            let result = (val << 1) | carry_in;
            cpu.status.set(CpuFlags::CARRY, val & 0x80 != 0);
            cpu.write(bus, r.addr, result);
            cpu.a &= result;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(3);
            7
        }
        0x23 => {
            let r = indexed_indirect(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let carry_in = cpu.status.contains(CpuFlags::CARRY) as u8;
            let result = (val << 1) | carry_in;
            cpu.status.set(CpuFlags::CARRY, val & 0x80 != 0);
            cpu.write(bus, r.addr, result);
            cpu.a &= result;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            8
        }
        0x33 => {
            let r = indirect_indexed(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let carry_in = cpu.status.contains(CpuFlags::CARRY) as u8;
            let result = (val << 1) | carry_in;
            cpu.status.set(CpuFlags::CARRY, val & 0x80 != 0);
            cpu.write(bus, r.addr, result);
            cpu.a &= result;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            8
        }

        // =================================================================
        // *SRE -- Shift Right then EOR (LSR + EOR)
        // =================================================================
        0x47 => {
            let r = zero_page(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let result = val >> 1;
            cpu.status.set(CpuFlags::CARRY, val & 0x01 != 0);
            cpu.write(bus, r.addr, result);
            cpu.a ^= result;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            5
        }
        0x57 => {
            let r = zero_page_x(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let result = val >> 1;
            cpu.status.set(CpuFlags::CARRY, val & 0x01 != 0);
            cpu.write(bus, r.addr, result);
            cpu.a ^= result;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            6
        }
        0x4F => {
            let r = absolute(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let result = val >> 1;
            cpu.status.set(CpuFlags::CARRY, val & 0x01 != 0);
            cpu.write(bus, r.addr, result);
            cpu.a ^= result;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(3);
            6
        }
        0x5F => {
            let r = absolute_x(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let result = val >> 1;
            cpu.status.set(CpuFlags::CARRY, val & 0x01 != 0);
            cpu.write(bus, r.addr, result);
            cpu.a ^= result;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(3);
            7
        }
        0x5B => {
            let r = absolute_y(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let result = val >> 1;
            cpu.status.set(CpuFlags::CARRY, val & 0x01 != 0);
            cpu.write(bus, r.addr, result);
            cpu.a ^= result;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(3);
            7
        }
        0x43 => {
            let r = indexed_indirect(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let result = val >> 1;
            cpu.status.set(CpuFlags::CARRY, val & 0x01 != 0);
            cpu.write(bus, r.addr, result);
            cpu.a ^= result;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            8
        }
        0x53 => {
            let r = indirect_indexed(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let result = val >> 1;
            cpu.status.set(CpuFlags::CARRY, val & 0x01 != 0);
            cpu.write(bus, r.addr, result);
            cpu.a ^= result;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            8
        }

        // =================================================================
        // *RRA -- Rotate Right then ADC (ROR + ADC)
        // =================================================================
        0x67 => {
            let r = zero_page(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let carry_in = (cpu.status.contains(CpuFlags::CARRY) as u8) << 7;
            let result = (val >> 1) | carry_in;
            cpu.status.set(CpuFlags::CARRY, val & 0x01 != 0);
            cpu.write(bus, r.addr, result);
            adc(cpu, result);
            cpu.pc = cpu.pc.wrapping_add(2);
            5
        }
        0x77 => {
            let r = zero_page_x(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let carry_in = (cpu.status.contains(CpuFlags::CARRY) as u8) << 7;
            let result = (val >> 1) | carry_in;
            cpu.status.set(CpuFlags::CARRY, val & 0x01 != 0);
            cpu.write(bus, r.addr, result);
            adc(cpu, result);
            cpu.pc = cpu.pc.wrapping_add(2);
            6
        }
        0x6F => {
            let r = absolute(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let carry_in = (cpu.status.contains(CpuFlags::CARRY) as u8) << 7;
            let result = (val >> 1) | carry_in;
            cpu.status.set(CpuFlags::CARRY, val & 0x01 != 0);
            cpu.write(bus, r.addr, result);
            adc(cpu, result);
            cpu.pc = cpu.pc.wrapping_add(3);
            6
        }
        0x7F => {
            let r = absolute_x(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let carry_in = (cpu.status.contains(CpuFlags::CARRY) as u8) << 7;
            let result = (val >> 1) | carry_in;
            cpu.status.set(CpuFlags::CARRY, val & 0x01 != 0);
            cpu.write(bus, r.addr, result);
            adc(cpu, result);
            cpu.pc = cpu.pc.wrapping_add(3);
            7
        }
        0x7B => {
            let r = absolute_y(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let carry_in = (cpu.status.contains(CpuFlags::CARRY) as u8) << 7;
            let result = (val >> 1) | carry_in;
            cpu.status.set(CpuFlags::CARRY, val & 0x01 != 0);
            cpu.write(bus, r.addr, result);
            adc(cpu, result);
            cpu.pc = cpu.pc.wrapping_add(3);
            7
        }
        0x63 => {
            let r = indexed_indirect(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let carry_in = (cpu.status.contains(CpuFlags::CARRY) as u8) << 7;
            let result = (val >> 1) | carry_in;
            cpu.status.set(CpuFlags::CARRY, val & 0x01 != 0);
            cpu.write(bus, r.addr, result);
            adc(cpu, result);
            cpu.pc = cpu.pc.wrapping_add(2);
            8
        }
        0x73 => {
            let r = indirect_indexed(cpu, bus);
            let val = cpu.read(bus, r.addr);
            let carry_in = (cpu.status.contains(CpuFlags::CARRY) as u8) << 7;
            let result = (val >> 1) | carry_in;
            cpu.status.set(CpuFlags::CARRY, val & 0x01 != 0);
            cpu.write(bus, r.addr, result);
            adc(cpu, result);
            cpu.pc = cpu.pc.wrapping_add(2);
            8
        }

        // =================================================================
        // *ANC / *AAC -- AND immediate, copy bit 7 to carry
        // =================================================================
        0x0B | 0x2B => {
            let r = immediate(cpu);
            let val = cpu.read(bus, r.addr);
            cpu.a &= val;
            cpu.set_zn(cpu.a);
            cpu.status.set(CpuFlags::CARRY, cpu.a & 0x80 != 0);
            cpu.pc = cpu.pc.wrapping_add(2);
            2
        }

        // =================================================================
        // *ALR / *ASR -- AND immediate then LSR accumulator
        // =================================================================
        0x4B => {
            let r = immediate(cpu);
            let val = cpu.read(bus, r.addr);
            cpu.a &= val;
            cpu.status.set(CpuFlags::CARRY, cpu.a & 0x01 != 0);
            cpu.a >>= 1;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            2
        }

        // =================================================================
        // *ARR -- AND immediate then ROR accumulator (with special flags)
        // =================================================================
        0x6B => {
            let r = immediate(cpu);
            let val = cpu.read(bus, r.addr);
            cpu.a &= val;
            let carry_in = (cpu.status.contains(CpuFlags::CARRY) as u8) << 7;
            cpu.a = (cpu.a >> 1) | carry_in;
            cpu.set_zn(cpu.a);
            // ARR has unique flag behavior
            let bit6 = (cpu.a >> 6) & 1;
            let bit5 = (cpu.a >> 5) & 1;
            cpu.status.set(CpuFlags::CARRY, bit6 != 0);
            cpu.status.set(CpuFlags::OVERFLOW, bit6 ^ bit5 != 0);
            cpu.pc = cpu.pc.wrapping_add(2);
            2
        }

        // =================================================================
        // *XAA / *ANE -- Unstable: (A | CONST) & X & imm -> A
        // CONST is typically 0xFF on most hardware
        // =================================================================
        0x8B => {
            let r = immediate(cpu);
            let val = cpu.read(bus, r.addr);
            // Most common behavior: A = (A | 0xFF) & X & imm = X & imm
            cpu.a = cpu.x & val;
            cpu.set_zn(cpu.a);
            cpu.pc = cpu.pc.wrapping_add(2);
            2
        }

        // =================================================================
        // *AHX / *SHA -- Store A & X & (high byte of addr + 1)
        // =================================================================
        0x9F => {
            // AHX Absolute,Y
            let r = absolute_y(cpu, bus);
            let hi = ((r.addr >> 8) as u8).wrapping_add(1);
            let val = cpu.a & cpu.x & hi;
            cpu.write(bus, r.addr, val);
            cpu.pc = cpu.pc.wrapping_add(3);
            5
        }
        0x93 => {
            // AHX (Indirect),Y
            let r = indirect_indexed(cpu, bus);
            let hi = ((r.addr >> 8) as u8).wrapping_add(1);
            let val = cpu.a & cpu.x & hi;
            cpu.write(bus, r.addr, val);
            cpu.pc = cpu.pc.wrapping_add(2);
            6
        }

        // =================================================================
        // *TAS / *SHS -- SP = A & X; store SP & (high byte + 1)
        // =================================================================
        0x9B => {
            let r = absolute_y(cpu, bus);
            cpu.sp = cpu.a & cpu.x;
            let hi = ((r.addr >> 8) as u8).wrapping_add(1);
            let val = cpu.sp & hi;
            cpu.write(bus, r.addr, val);
            cpu.pc = cpu.pc.wrapping_add(3);
            5
        }

        // =================================================================
        // *SHX / *SXA -- Store X & (high byte of addr + 1)
        // =================================================================
        0x9E => {
            let r = absolute_y(cpu, bus);
            let hi = ((r.addr >> 8) as u8).wrapping_add(1);
            let val = cpu.x & hi;
            cpu.write(bus, r.addr, val);
            cpu.pc = cpu.pc.wrapping_add(3);
            5
        }

        // =================================================================
        // *SHY / *SYA -- Store Y & (high byte of addr + 1)
        // =================================================================
        0x9C => {
            let r = absolute_x(cpu, bus);
            let hi = ((r.addr >> 8) as u8).wrapping_add(1);
            let val = cpu.y & hi;
            cpu.write(bus, r.addr, val);
            cpu.pc = cpu.pc.wrapping_add(3);
            5
        }

        // =================================================================
        // *LAS / *LAR -- A,X,SP = M & SP
        // =================================================================
        0xBB => {
            let r = absolute_y(cpu, bus);
            let val = cpu.read(bus, r.addr) & cpu.sp;
            cpu.a = val;
            cpu.x = val;
            cpu.sp = val;
            cpu.set_zn(val);
            cpu.pc = cpu.pc.wrapping_add(3);
            4 + r.page_cross as u8
        }

        // =================================================================
        // Unofficial NOPs (various sizes and cycle counts)
        // =================================================================

        // 1-byte NOPs (implied) -- 2 cycles
        0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xFA => {
            cpu.pc = cpu.pc.wrapping_add(1);
            2
        }

        // 2-byte NOPs (immediate / zero page) -- various cycles
        0x80 | 0x82 | 0x89 | 0xC2 | 0xE2 => {
            // *NOP immediate -- 2 bytes, 2 cycles
            cpu.pc = cpu.pc.wrapping_add(2);
            2
        }
        0x04 | 0x44 | 0x64 => {
            // *NOP zero page -- 2 bytes, 3 cycles
            cpu.pc = cpu.pc.wrapping_add(2);
            3
        }
        0x14 | 0x34 | 0x54 | 0x74 | 0xD4 | 0xF4 => {
            // *NOP zero page,X -- 2 bytes, 4 cycles
            cpu.pc = cpu.pc.wrapping_add(2);
            4
        }

        // 3-byte NOPs (absolute) -- 4 cycles
        0x0C => {
            // *NOP absolute -- 3 bytes, 4 cycles
            cpu.pc = cpu.pc.wrapping_add(3);
            4
        }
        0x1C | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC => {
            // *NOP absolute,X -- 3 bytes, 4+1 cycles (page cross)
            let r = absolute_x(cpu, bus);
            let _ = r; // read to trigger side effects if any
            cpu.pc = cpu.pc.wrapping_add(3);
            4 + r.page_cross as u8
        }

        // =================================================================
        // KIL / JAM / HLT -- Halt the processor (illegal, locks up CPU)
        // We implement as infinite loop (PC does not advance)
        // =================================================================
        0x02 | 0x12 | 0x22 | 0x32 | 0x42 | 0x52 | 0x62 | 0x72 | 0x92 | 0xB2 | 0xD2 | 0xF2 => {
            // KIL/JAM -- CPU halts. Return 2 but don't advance PC.
            // In practice this locks up the CPU.
            // We don't advance PC so the CPU will execute this forever.
            2
        }

        // =================================================================
        // *LAX immediate (AB) -- Unstable, behavior varies
        // Most common: A = X = (A | CONST) & imm
        // =================================================================
        0xAB => {
            let r = immediate(cpu);
            let val = cpu.read(bus, r.addr);
            // Common behavior: A = X = (A | 0xFF) & imm = imm
            cpu.a = val;
            cpu.x = val;
            cpu.set_zn(val);
            cpu.pc = cpu.pc.wrapping_add(2);
            2
        }

        // =================================================================
        // *SBX / *AXS -- X = (A & X) - imm (no borrow)
        // =================================================================
        0xCB => {
            let r = immediate(cpu);
            let val = cpu.read(bus, r.addr);
            let ax = cpu.a & cpu.x;
            let result = ax.wrapping_sub(val);
            cpu.status.set(CpuFlags::CARRY, ax >= val);
            cpu.x = result;
            cpu.set_zn(cpu.x);
            cpu.pc = cpu.pc.wrapping_add(2);
            2
        }
    }
}

// ---------------------------------------------------------------------------
// ADC helper -- Add with Carry (used by ADC and *RRA)
// ---------------------------------------------------------------------------
#[inline]
fn adc(cpu: &mut Cpu, val: u8) {
    let carry = cpu.status.contains(CpuFlags::CARRY) as u16;
    let sum = cpu.a as u16 + val as u16 + carry;
    let result = sum as u8;

    cpu.status.set(CpuFlags::CARRY, sum > 0xFF);
    // Overflow: set if sign of both inputs differs from sign of output
    cpu.status.set(
        CpuFlags::OVERFLOW,
        (cpu.a ^ result) & (val ^ result) & 0x80 != 0,
    );
    cpu.a = result;
    cpu.set_zn(cpu.a);
}

// ---------------------------------------------------------------------------
// SBC helper -- Subtract with Carry (used by SBC, *ISB)
// ---------------------------------------------------------------------------
#[inline]
fn sbc(cpu: &mut Cpu, val: u8) {
    // SBC is equivalent to ADC with the complement of the operand
    adc(cpu, !val);
}
