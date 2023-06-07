extern crate core;

use std::ops::{Index, IndexMut};

const HEX_SPRITES: [[u8; 5]; 0x10] = [
    [0xF0, 0x90, 0x90, 0x90, 0xF0], // 0
    [0x20, 0x60, 0x20, 0x20, 0x70], // 1
    [0xF0, 0x10, 0xF0, 0x80, 0xF0], // 2
    [0xF0, 0x10, 0xF0, 0x10, 0xF0], // 3
    [0x90, 0x90, 0xF0, 0x10, 0x10], // 4
    [0xF0, 0x80, 0xF0, 0x10, 0xF0], // 5
    [0xF0, 0x80, 0xF0, 0x90, 0xF0], // 6
    [0xF0, 0x10, 0x20, 0x40, 0x40], // 7
    [0xF0, 0x90, 0xF0, 0x90, 0xF0], // 8
    [0xF0, 0x90, 0xF0, 0x10, 0xF0], // 9
    [0xF0, 0x90, 0xF0, 0x90, 0x90], // A
    [0xE0, 0x90, 0xE0, 0x90, 0xE0], // B
    [0xF0, 0x80, 0x80, 0x80, 0xF0], // C
    [0xE0, 0x90, 0x90, 0x90, 0xE0], // D
    [0xF0, 0x80, 0xF0, 0x80, 0xF0], // E
    [0xF0, 0x80, 0xF0, 0x80, 0x80], // F
];

#[derive(Debug)]
pub enum Error {
    UndefinedOp(u16),
    PoppedEmptyStack,
}

// Quirk values concretely define ambiguous behavior in the C8 spec
#[derive(Copy, Clone)]
#[allow(dead_code)]
enum Quirk {
    VfReset(bool),
    Memory(bool),
    DispWait(bool),
    Clipping(bool),
    Shifting(bool),
    Jumping(bool),
}

struct Registers([u8; 0x10]);

impl Index<u8> for Registers {
    type Output = u8;

    fn index(&self, index: u8) -> &Self::Output {
        &self.0[usize::from(index)]
    }
}

impl IndexMut<u8> for Registers {
    fn index_mut(&mut self, index: u8) -> &mut Self::Output {
        &mut self.0[usize::from(index)]
    }
}

impl Registers {
    fn new() -> Self {
        Self([0; 0x10])
    }
}

pub struct ChipState {
    // CHIP-8 Memory
    mem: [u8; 0x1000],
    // Register values
    reg: Registers,
    // I register, used to store memory addresses
    i: u16,
    // Program counter
    pc: u16,
    // Runtime stack
    stack: Vec<u16>,
    // Framebuffer
    fbuf: [u64; 0x20],
    // User input keys
    pub keys: u16,
    // Delay timer
    d_tim: u8,
    // Sound timer
    s_tim: u8,
    // Seed for RNG operation
    rng_seed: u64,
}

impl ChipState {
    pub fn new(seed: u64) -> Self {
        let mut mem = [0; 0x1000];
        // move hex sprites into memory
        for (i, s) in HEX_SPRITES.iter().enumerate() {
            mem[i * 5..i * 5 + s.len()].copy_from_slice(s)
        }

        Self {
            mem,
            reg: Registers::new(),
            i: 0,
            pc: 0,
            stack: Vec::with_capacity(0x10),
            fbuf: [0; 0x20],
            keys: 0,
            d_tim: 0,
            s_tim: 0,
            rng_seed: seed,
        }
    }

    pub fn load(&mut self, rom: &[u8]) {
        let rom_start = 0x200;
        self.mem[rom_start as usize..rom_start as usize + rom.len()].copy_from_slice(rom);
        self.pc = rom_start;
    }

    pub fn tick(&mut self) -> Result<(), Error> {
        let instr = ((self.mem[self.pc as usize] as u16) << 8)
            | self.mem[self.pc as usize + 1] as u16;

        self.eval(instr)
    }

    pub fn get_fbuf(&self) -> &[u64] {
        &self.fbuf
    }

    pub fn press_key(&mut self, key: u8) {
        self.keys |= 1 << key;
    }

    pub fn release_key(&mut self, key: u8) {
        self.keys &= !(1 << key);
    }

    fn key_is_pressed(&self, key: u8) -> bool {
        (self.keys >> key) & 1 == 1
    }
    // Xorshift, see https://en.wikipedia.org/wiki/Xorshift
    fn rand(&mut self) -> u8 {
        self.rng_seed ^= self.rng_seed << 13;
        self.rng_seed ^= self.rng_seed >> 17;
        self.rng_seed ^= self.rng_seed << 5;

        (self.rng_seed & 0xFF) as u8
    }

    fn eval(&mut self, instr: u16) -> Result<(), Error> {
        let [upper, lower ] = instr.to_be_bytes();

        match upper & 0xF0 {
            0x00 => match instr {
                // Clear the framebuffer
                0x00E0 => {
                    self.fbuf = [0; 0x20];
                    self.pc += 2;
                }
                // Return from a subroutine
                0x00EE => {
                    self.pc = match self.stack.pop() {
                        Some(a) => a,
                        None => return Err(Error::PoppedEmptyStack),
                    };
                }
                _ => return Err(Error::UndefinedOp(instr))
            }
            // Jump to location
            0x10 => {
                self.pc = instr & 0x0FFF;
            }
            // Call subroutine
            0x20 => {
                self.stack.push(self.pc + 2);
                self.pc = instr & 0x0FFF;
            }
            // Skip next instruction if ...
            0x30 => {
                if self.reg[upper & 0x0F] == lower {
                    self.pc += 2;
                }
                self.pc += 2;
            }
            0x40 => {
                if self.reg[upper & 0x0F] != lower {
                    self.pc += 2;
                }
                self.pc += 2;
            }
            0x50 => {
                match lower & 0x0F {
                    0x00 => {
                        if self.reg[upper & 0x0F] == self.reg[(lower & 0xF0) >> 4] {
                            self.pc += 2;
                        }
                        self.pc += 2;
                    }
                    _ => {
                        return Err(Error::UndefinedOp(instr));
                    }
                }
            }
            0x60 => {
                self.reg[upper & 0x0F] = lower;
                self.pc += 2;
            }
            0x70 => {
                let (res, _) = self.reg[upper & 0x0F].overflowing_add(lower);
                self.reg[upper & 0x0F] = res;
                self.pc += 2;
            }
            0x80 => {
                let x = upper & 0x0F;
                let y = (lower & 0xF0) >> 4;
                match lower & 0x0F {
                    0x00 => self.reg[x] = self.reg[y],
                    0x01 => {
                        self.reg[x] |= self.reg[y];
                        self.reg[0xF] = 0;
                    }
                    0x02 => {
                        self.reg[x] &= self.reg[y];
                        self.reg[0xF] = 0;
                    }
                    0x03 => {
                        self.reg[x] ^= self.reg[y];
                        self.reg[0xF] = 0;
                    }
                    0x04 => {
                        let (sum, overflow) = self.reg[x].overflowing_add(self.reg[y]);
                        self.reg[x] = sum;
                        self.reg[0xF] = if overflow { 1 } else { 0 };
                    }
                    0x05 => {
                        let orig_x = self.reg[x];
                        let (diff, _) = self.reg[x].overflowing_sub(self.reg[y]);
                        self.reg[x] = diff;
                        self.reg[0xF] = if orig_x > self.reg[y] { 1 } else { 0 };
                    }
                    0x06 => {
                        let orig = self.reg[y];
                        let (shr, _) = orig.overflowing_shr(1);
                        self.reg[x] = shr;
                        self.reg[0xF] = orig & 0x1;
                    }
                    0x07 => {
                        let (diff, _) = self.reg[y].overflowing_sub(self.reg[x]);
                        self.reg[x] = diff;
                        self.reg[0xF] = if self.reg[y] > self.reg[x] { 1 } else { 0 };
                    }
                    0x0E => {
                        let orig = self.reg[y];
                        let (shl, _) = orig.overflowing_shl(1);
                        self.reg[x] = shl;
                        self.reg[0xF] = (orig & 0x80) >> 7;
                    }
                    _ => {
                        return Err(Error::UndefinedOp(instr));
                    }
                }
                self.pc += 2;
            }
            0x90 => {
                if instr & 0xF != 0x0 {
                    return Err(Error::UndefinedOp(instr));
                }

                if self.reg[upper & 0x0F] != self.reg[(lower & 0xF0) >> 4] {
                    self.pc += 2;
                }
                self.pc += 2;
            }
            0xA0 => {
                self.i = instr & 0x0FFF;
                self.pc += 2;
            }
            0xB0 => {
                self.pc = (instr & 0x0FFF) + u16::from(self.reg[0x0]);
            }
            0xC0 => {
                self.reg[upper & 0xF] = self.rand() & lower;
                self.pc += 2;
            }
            // Draw routine
            0xD0 => {
                let x = self.reg[upper & 0x0F] % 64;
                let mut y = self.reg[(lower & 0xF0) >> 4] % 32;
                self.reg[0xF] = 0;
                let n = u16::from(lower & 0x0F);

                for i in self.i..self.i + n {
                    let byte = self.mem[i as usize];
                    let line = u64::from(byte) << 56 >> x;
                    let compare = self.fbuf[y as usize] | line;
                    self.fbuf[y as usize] ^= line;
                    self.reg[0xF] = if self.fbuf[y as usize] != compare {
                        1
                    } else {
                        0
                    };
                    y += 1;
                    if y > 31 {
                        break;
                    }
                }
                self.pc += 2;
            }
            0xE0 => {
                let key = self.reg[upper & 0x0F];
                match lower {
                    0x9E => {
                        if self.key_is_pressed(key) {
                            self.pc += 2;
                        }
                    }
                    0xA1 => {
                        if !self.key_is_pressed(key) {
                            self.pc += 2;
                        }
                    }
                    _ => {
                        return Err(Error::UndefinedOp(instr));
                    }
                }
                self.pc += 2;
            }
            0xF0 => {
                match lower {
                    0x07 => {
                        self.reg[upper & 0xF] = self.d_tim;
                        self.pc += 2;
                    }
                    // Halt until key down
                    0x0A => {
                        for i in 0..=0xF {
                            if self.key_is_pressed(i) {
                                self.reg[upper & 0x0F] = i;
                                self.pc += 2;
                                break;
                            }
                        }
                    }
                    // Set delay timer to value of Vx
                    0x15 => {
                        self.d_tim = self.reg[0x0F & upper];
                        self.pc += 2;
                    }
                    // Set sound timer to value of Vx
                    0x18 => {
                        self.s_tim = self.reg[0x0F & upper];
                        self.pc += 2;
                    }
                    // Add value of Vx to I
                    0x1E => {
                        self.i += u16::from(self.reg[0x0F & upper]);
                        self.pc += 2;
                    }
                    0x29 => {
                        self.i = u16::from(self.reg[0x0F & upper]) * 5;
                        self.pc += 2;
                    }
                    0x33 => {
                        let reg_value = self.reg[upper & 0x0F];
                        self.mem[usize::from(self.i)] = reg_value / 100;
                        self.mem[usize::from(self.i) + 1] = (reg_value % 100) / 10;
                        self.mem[usize::from(self.i) + 2] = reg_value % 10;
                        self.pc += 2;
                    }
                    0x55 => {
                        for x in 0..=(upper & 0x0F) {
                            self.mem[usize::from(self.i) + usize::from(x)] = self.reg[x];
                        }
                        self.i += 1;
                        self.pc += 2;
                    }
                    0x65 => {
                        for x in 0..=(upper & 0x0F) {
                            self.reg[x] = self.mem[usize::from(self.i) + usize::from(x)];
                        }
                        self.i += 1;
                        self.pc += 2;
                    }
                    _ => {
                        return Err(Error::UndefinedOp(instr));
                    }
                }
            }

            _ => return Err(Error::UndefinedOp(instr))
        }

        self.d_tim = self.d_tim.saturating_sub(1);
        self.s_tim = self.s_tim.saturating_sub(1);

        Ok(())
    }
}
