use core::fmt;
use std::{fs::File, io::Read};

pub struct Chip8 {
    memory: [u8; 4096],
    pub display: [u8; 64 * 32],
    v: [u8; 16],
    pc: usize,
    st: u8,
    i: u16,
    stack: Vec<usize>,
    mode: Modes,
    pub keys: [bool; 16],
}

impl fmt::Debug for Chip8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "
PC: {:#06x}
 I: {:#06x}
 V: {}
 DISPLAY:
{}
",
            &self.pc,
            &self.i,
            &self.v.map(|v| format!("{:#06x}", v)).join(","),
            &self
                .display
                .map(|b| if b == 255 { "■" } else { " " })
                .chunks(64)
                .map(|line| line.join("") + "\n")
                .collect::<String>()
        ))
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Modes {
    Chip8,
    // Chip48,
    // SuperChip,
}

#[derive(Debug)]
enum OpCodes {
    NOOP,
    CLS,
    RET,
    JMP(usize),
    CALL(usize),
    SE(usize, u8),
    SNE(usize, u8),
    SEV(usize, usize),
    LdVxNn(usize, u8),
    ADD(usize, u8),
    LDV(usize, usize),
    ORV(usize, usize),
    ANDV(usize, usize),
    XORV(usize, usize),
    ADDV(usize, usize),
    SUBV(usize, usize),
    SHRV(usize, usize),
    SUBNV(usize, usize),
    SHLV(usize, usize),
    SNEV(usize, usize),
    LDI(u16),
    // JmpVoNn(u16),
    RndVxNn(usize, u8),
    DRAW(usize, usize, u16),
    // SkpVx,
    SknpVx(usize),
    // LdVxDt,
    LdVxK(usize),
    // LdDtVx,
    LdStVx(usize),
    // AddIVx,
    LdFVx(usize),
    LDBV(usize),
    // LdVVx,
    LDIV(usize),
    LDVI(usize),
}

impl TryFrom<u16> for OpCodes {
    type Error = String;

    fn try_from(v: u16) -> Result<Self, Self::Error> {
        match v {
            0x0000 => Ok(OpCodes::NOOP),
            0x00EE => Ok(OpCodes::RET),
            0x00E0 => Ok(OpCodes::CLS),
            v if v & 0xF000 == 0x1000 => Ok(OpCodes::JMP((v & 0x0FFF) as usize)),
            v if v & 0xF000 == 0x2000 => Ok(OpCodes::CALL((v & 0x0FFF) as usize)),
            v if v & 0xF000 == 0x6000 => Ok(OpCodes::LdVxNn(
                ((v & 0x0F00) >> 8) as usize,
                (v & 0x00FF) as u8,
            )),
            v if v & 0xF000 == 0x7000 => Ok(OpCodes::ADD(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                (v & 0x00FF) as u8,
            )),
            v if v & 0xF000 == 0xA000 => Ok(OpCodes::LDI(v & 0x0FFF)),
            v if v & 0xF000 == 0xC000 => Ok(OpCodes::RndVxNn(
                ((v & 0x0F00) >> 8) as usize,
                (v & 0x00FF) as u8,
            )),
            v if v & 0xF000 == 0xD000 => Ok(OpCodes::DRAW(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                ((v & 0x00F0) >> 4).try_into().unwrap(),
                v & 0x000F,
            )),
            v if v & 0xF0FF == 0xE0A1 => Ok(OpCodes::SknpVx(((v & 0x0F00) >> 8) as usize)),

            v if v & 0xF000 == 0x3000 => Ok(OpCodes::SE(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                (v & 0x00FF) as u8,
            )),
            v if v & 0xF000 == 0x4000 => Ok(OpCodes::SNE(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                (v & 0x00FF) as u8,
            )),
            v if v & 0xF000 == 0x5000 => Ok(OpCodes::SEV(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                ((v & 0x00F0) >> 4).try_into().unwrap(),
            )),

            v if v & 0xF00F == 0x8000 => Ok(OpCodes::LDV(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                ((v & 0x00F0) >> 4).try_into().unwrap(),
            )),
            v if v & 0xF00F == 0x8001 => Ok(OpCodes::ORV(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                ((v & 0x00F0) >> 4).try_into().unwrap(),
            )),
            v if v & 0xF00F == 0x8002 => Ok(OpCodes::ANDV(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                ((v & 0x00F0) >> 4).try_into().unwrap(),
            )),
            v if v & 0xF00F == 0x8003 => Ok(OpCodes::XORV(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                ((v & 0x00F0) >> 4).try_into().unwrap(),
            )),
            v if v & 0xF00F == 0x8004 => Ok(OpCodes::ADDV(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                ((v & 0x00F0) >> 4).try_into().unwrap(),
            )),
            v if v & 0xF00F == 0x8005 => Ok(OpCodes::SUBV(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                ((v & 0x00F0) >> 4).try_into().unwrap(),
            )),
            v if v & 0xF00F == 0x8006 => Ok(OpCodes::SHRV(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                ((v & 0x00F0) >> 4).try_into().unwrap(),
            )),
            v if v & 0xF00F == 0x8007 => Ok(OpCodes::SUBNV(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                ((v & 0x00F0) >> 4).try_into().unwrap(),
            )),
            v if v & 0xF00F == 0x800E => Ok(OpCodes::SHLV(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                ((v & 0x00F0) >> 4).try_into().unwrap(),
            )),
            v if v & 0xF00F == 0x9000 => Ok(OpCodes::SNEV(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                ((v & 0x00F0) >> 4).try_into().unwrap(),
            )),
            v if v & 0xF0FF == 0xF055 => Ok(OpCodes::LDIV(((v & 0x0F00) >> 8).try_into().unwrap())),
            v if v & 0xF0FF == 0xF065 => Ok(OpCodes::LDVI(((v & 0x0F00) >> 8).try_into().unwrap())),
            v if v & 0xF0FF == 0xF00A => Ok(OpCodes::LdVxK(((v & 0x0F00) >> 8) as usize)),
            v if v & 0xF0FF == 0xF018 => Ok(OpCodes::LdStVx(((v & 0x0F00) >> 8) as usize)),
            v if v & 0xF0FF == 0xF029 => Ok(OpCodes::LdFVx(((v & 0x0F00) >> 8) as usize)),
            v if v & 0xF0FF == 0xF033 => Ok(OpCodes::LDBV(((v & 0x0F00) >> 8) as usize)),

            _ => Err(format!("Op code not implemented for {:#06x}", v)),
        }
    }
}

impl Chip8 {
    pub fn new() -> Self {
        Chip8 {
            memory: [0; 4096],
            v: [0; 16],
            pc: 0x200,
            st: 0,
            i: 0,
            display: [0; 64 * 32],
            stack: vec![],
            mode: Modes::Chip8,
            keys: [false; 16],
        }
    }

    pub fn load(&mut self, filename: &str) -> Result<(), std::io::Error> {
        self.memory.fill(0);

        self.memory[0..16 * 5].copy_from_slice(&[
            0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
            0x20, 0x60, 0x20, 0x20, 0x70, // 1
            0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
            0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
            0x90, 0x90, 0xF0, 0x10, 0x10, // 4
            0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
            0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
            0xF0, 0x10, 0x20, 0x40, 0x40, // 7
            0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
            0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
            0xF0, 0x90, 0xF0, 0x90, 0x90, // A
            0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
            0xF0, 0x80, 0x80, 0x80, 0xF0, // C
            0xE0, 0x90, 0x90, 0x90, 0xE0, // D
            0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
            0xF0, 0x80, 0xF0, 0x80, 0x80, // F
        ]);
        let mut file = File::open(filename)?;
        file.read(&mut self.memory[0x200..])
            .expect("Failed to read file");
        Ok(())
    }

    pub fn tick(&mut self) {
        let next_instruction: u16 =
            u16::from_be_bytes(self.memory[self.pc..self.pc + 2].try_into().unwrap());
        self.pc += 2;
        if self.st > 0 {
            self.st -= 1; // TODO Do this at 60Hz
        }

        let op = OpCodes::try_from(next_instruction).unwrap();
        // println!("{:#06x}: {:?}", next_instruction, op);
        println!("{:?}", self.keys);

        match op {
            OpCodes::CLS => {
                self.display.fill(0);
            }
            OpCodes::LDI(n) => {
                self.i = n;
            }
            OpCodes::RndVxNn(x, n) => {
                self.v[x] = n & rand::random::<u8>();
            }
            OpCodes::LdVxNn(x, n) => {
                self.v[x] = n;
            }
            OpCodes::DRAW(vx, vy, n) => {
                self.v[0xf] = 0;
                let x = (self.v[vx] as usize) % 64; // wrap
                let y = (self.v[vy] as usize) % 32; // wrap
                for dy in 0..n as usize {
                    if y + dy >= 32 {
                        break; // clip
                    }
                    let line: u8 = self.memory[self.i as usize + dy];
                    for dx in 0..8 as usize {
                        if x + dx >= 64 {
                            break; // clip
                        }
                        let loc = x + dx + (y + dy) * 64;
                        let cur = self.display[loc];
                        self.display[loc] ^= if ((0b10000000 >> dx) & line) != 0 {
                            255
                        } else {
                            0
                        };
                        if cur == 255 && self.display[loc] == 0 {
                            self.v[0xf] = 1;
                        }
                    }
                }
            }

            OpCodes::SknpVx(x) => {
                if !self.keys[self.v[x] as usize] {
                    self.pc += 2;
                }
            }
            OpCodes::ADD(x, n) => {
                self.v[x] = self.v[x].wrapping_add(n);
            }
            OpCodes::JMP(n) => {
                let target = n.try_into().unwrap();
                // if target == self.pc - 2 {
                //     println!("Infinite jump... quitting..");
                //     return;
                // }
                self.pc = target;
            }
            OpCodes::NOOP => {}
            OpCodes::SE(x, n) => {
                if self.v[x] == n {
                    self.pc += 2;
                }
            }
            OpCodes::SNE(x, n) => {
                if self.v[x] != n {
                    self.pc += 2;
                }
            }
            OpCodes::SEV(x, y) => {
                if self.v[x] == self.v[y] {
                    self.pc += 2;
                }
            }
            OpCodes::SNEV(x, y) => {
                if self.v[x] != self.v[y] {
                    self.pc += 2;
                }
            }
            OpCodes::CALL(n) => {
                self.stack.push(self.pc);
                self.pc = n;
            }
            OpCodes::RET => self.pc = self.stack.pop().unwrap(),
            OpCodes::LDV(x, y) => {
                self.v[x] = self.v[y];
            }
            OpCodes::ORV(x, y) => {
                self.v[x] |= self.v[y];
            }
            OpCodes::ANDV(x, y) => {
                self.v[x] &= self.v[y];
            }
            OpCodes::XORV(x, y) => {
                self.v[x] ^= self.v[y];
            }
            OpCodes::ADDV(x, y) => {
                let (result, did_overflow) = self.v[x].overflowing_add(self.v[y]);
                self.v[x] = result;
                self.v[0xf] = if did_overflow { 1 } else { 0 };
            }
            OpCodes::SUBV(x, y) => {
                let (result, did_overflow) = self.v[x].overflowing_sub(self.v[y]);
                self.v[x] = result;
                self.v[0xf] = if did_overflow { 1 } else { 0 };
            }
            OpCodes::SUBNV(x, y) => {
                let (result, did_overflow) = self.v[y].overflowing_sub(self.v[x]);
                self.v[x] = result;
                self.v[0xf] = if did_overflow { 1 } else { 0 };
            }
            OpCodes::SHRV(x, y) => {
                if self.mode == Modes::Chip8 {
                    self.v[x] = self.v[y];
                }
                self.v[0xf] = self.v[x] & 0x01;
                self.v[x] = self.v[x] >> 1;
            }
            OpCodes::SHLV(x, y) => {
                if self.mode == Modes::Chip8 {
                    self.v[x] = self.v[y];
                }
                self.v[0xf] = self.v[x] & 0x80;
                self.v[x] = self.v[x] << 1;
            }
            OpCodes::LDIV(x) => {
                for dx in 0..x + 1 {
                    self.memory[(self.i as usize) + dx] = self.v[dx];
                }
            }
            OpCodes::LDVI(x) => {
                for dx in 0..x + 1 {
                    self.v[dx] = self.memory[(self.i as usize) + dx];
                }
            }
            OpCodes::LdVxK(x) => {
                if let Some(key) = self.keys.iter().position(|&b| b) {
                    self.v[x] = key as u8;
                } else {
                    self.pc -= 2;
                }
            }
            OpCodes::LdStVx(x) => {
                // buzz!
                self.st = self.v[x];
            }
            OpCodes::LdFVx(x) => {
                self.i = (self.v[x] * 0x5) as u16;
            }
            OpCodes::LDBV(x) => {
                self.memory[(self.i as usize)] = self.v[x] / 100;
                self.memory[(self.i as usize) + 1] = (self.v[x] / 10) % 10;
                self.memory[(self.i as usize) + 2] = self.v[x] % 10;
            }
        }
    }
}
