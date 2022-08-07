use core::fmt;
use std::{
    fs::File,
    io::{stdin, Read},
    time::{Duration, Instant},
};

pub struct Chip8 {
    memory: [u8; 4096],
    pub display: [u8; 64 * 32],
    v: [u8; 16],
    pc: usize,
    st: u8,
    dt: u8,
    i: u16,
    stack: Vec<usize>,
    mode: Modes,
    pub keys: [bool; 16],

    next_tick: Instant,
    next_timers_tick: Instant,

    sound_playing: bool,
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
                .map(|b| if b != 0 { "■" } else { " " })
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
    CLS,                          // CLS — 00E0
    RET,                          // RET — 00EE
    JMP(usize),                   // JMP — 1NNN
    CALL(usize),                  // CALL NNN — 2NNN
    SeVxNn(usize, u8),            // SE VX, NN — 3XNN
    SneVxNn(usize, u8),           // SNE VX, NN — 4XNN
    SeVxVy(usize, usize),         // SE VX, VY — 5XY0
    LdVxNn(usize, u8),            // LD VX, NN — 6XNN
    AddVxNn(usize, u8),           // ADD VX, NN — 7XNN
    LdVxVy(usize, usize),         // LD VX, VY — 8XY0
    OrVxVy(usize, usize),         // OR VX, VY — 8XY1
    AndVxVy(usize, usize),        // AND VX, VY — 8XY2
    XorVxVy(usize, usize),        // XOR VX, VY — 8XY3
    AddVxVy(usize, usize),        // ADD VX, VY — 8XY4
    SubVxVy(usize, usize),        // SUB VX, VY — 8XY5
    ShrVxVy(usize, usize),        // SHR VX {, VY} — 8XY6
    SubnVxVy(usize, usize),       // SUBN VX, VY — 8XY7
    ShlVxVy(usize, usize),        // SHL VX {, VY} — 8XYE
    SneVxVy(usize, usize),        // SNE VX, VY — 9XY0
    LdINn(u16),                   // LD I, NNN — ANNN
    JmpV0Nnn(usize),              // JMP V0, NNN — BNNN
    RndVxNn(usize, u8),           // RND VX, NN – CXNN
    DrawVxVyN(usize, usize, u16), // DRW VX, VY, N — DXYN
    SkpVx(usize),                 // SKP VX — EX9E
    SknpVx(usize),                // SKNP VX — EXA1
    LdVxDt(usize),                // LD VX, DT — FX07
    LdVxK(usize),                 // LD VX, K — FX0A
    LdDtVx(usize),                // LD DT, VX — FX15
    LdStVx(usize),                // LD ST, VX — FX18
    AddIVx(usize),                // ADD I, VX — FX1E
    LdFVx(usize),                 // LD F, VX — FX29
    LdBVx(usize),                 // LD B, VX — FX33
    LdIVx(usize),                 // LD [I], VX — FX55
    LdVxI(usize),                 // LD VX, [I] — FX65
}

impl TryFrom<u16> for OpCodes {
    type Error = String;

    fn try_from(v: u16) -> Result<Self, Self::Error> {
        match v {
            0x0000 => Ok(OpCodes::NOOP),
            0x00EE => Ok(OpCodes::RET),
            0x00E0 => Ok(OpCodes::CLS),
            v if v & 0xF000 == 0x1000 => Ok(OpCodes::JMP((v & 0x0FFF) as usize)),
            v if v & 0xF000 == 0xB000 => Ok(OpCodes::JmpV0Nnn((v & 0x0FFF) as usize)),
            v if v & 0xF000 == 0x2000 => Ok(OpCodes::CALL((v & 0x0FFF) as usize)),
            v if v & 0xF000 == 0x6000 => Ok(OpCodes::LdVxNn(
                ((v & 0x0F00) >> 8) as usize,
                (v & 0x00FF) as u8,
            )),
            v if v & 0xF000 == 0x7000 => Ok(OpCodes::AddVxNn(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                (v & 0x00FF) as u8,
            )),
            v if v & 0xF000 == 0xA000 => Ok(OpCodes::LdINn(v & 0x0FFF)),
            v if v & 0xF000 == 0xC000 => Ok(OpCodes::RndVxNn(
                ((v & 0x0F00) >> 8) as usize,
                (v & 0x00FF) as u8,
            )),
            v if v & 0xF000 == 0xD000 => Ok(OpCodes::DrawVxVyN(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                ((v & 0x00F0) >> 4).try_into().unwrap(),
                v & 0x000F,
            )),
            v if v & 0xF0FF == 0xE09E => Ok(OpCodes::SkpVx(((v & 0x0F00) >> 8) as usize)),
            v if v & 0xF0FF == 0xE0A1 => Ok(OpCodes::SknpVx(((v & 0x0F00) >> 8) as usize)),

            v if v & 0xF000 == 0x3000 => Ok(OpCodes::SeVxNn(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                (v & 0x00FF) as u8,
            )),
            v if v & 0xF000 == 0x4000 => Ok(OpCodes::SneVxNn(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                (v & 0x00FF) as u8,
            )),
            v if v & 0xF000 == 0x5000 => Ok(OpCodes::SeVxVy(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                ((v & 0x00F0) >> 4).try_into().unwrap(),
            )),

            v if v & 0xF00F == 0x8000 => Ok(OpCodes::LdVxVy(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                ((v & 0x00F0) >> 4).try_into().unwrap(),
            )),
            v if v & 0xF00F == 0x8001 => Ok(OpCodes::OrVxVy(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                ((v & 0x00F0) >> 4).try_into().unwrap(),
            )),
            v if v & 0xF00F == 0x8002 => Ok(OpCodes::AndVxVy(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                ((v & 0x00F0) >> 4).try_into().unwrap(),
            )),
            v if v & 0xF00F == 0x8003 => Ok(OpCodes::XorVxVy(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                ((v & 0x00F0) >> 4).try_into().unwrap(),
            )),
            v if v & 0xF00F == 0x8004 => Ok(OpCodes::AddVxVy(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                ((v & 0x00F0) >> 4).try_into().unwrap(),
            )),
            v if v & 0xF00F == 0x8005 => Ok(OpCodes::SubVxVy(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                ((v & 0x00F0) >> 4).try_into().unwrap(),
            )),
            v if v & 0xF00F == 0x8006 => Ok(OpCodes::ShrVxVy(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                ((v & 0x00F0) >> 4).try_into().unwrap(),
            )),
            v if v & 0xF00F == 0x8007 => Ok(OpCodes::SubnVxVy(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                ((v & 0x00F0) >> 4).try_into().unwrap(),
            )),
            v if v & 0xF00F == 0x800E => Ok(OpCodes::ShlVxVy(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                ((v & 0x00F0) >> 4).try_into().unwrap(),
            )),
            v if v & 0xF00F == 0x9000 => Ok(OpCodes::SneVxVy(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                ((v & 0x00F0) >> 4).try_into().unwrap(),
            )),
            v if v & 0xF0FF == 0xF015 => Ok(OpCodes::LdDtVx(((v & 0x0F00) >> 8) as usize)),
            v if v & 0xF0FF == 0xF055 => {
                Ok(OpCodes::LdIVx(((v & 0x0F00) >> 8).try_into().unwrap()))
            }
            v if v & 0xF0FF == 0xF065 => {
                Ok(OpCodes::LdVxI(((v & 0x0F00) >> 8).try_into().unwrap()))
            }
            v if v & 0xF0FF == 0xF007 => Ok(OpCodes::LdVxDt(((v & 0x0F00) >> 8) as usize)),
            v if v & 0xF0FF == 0xF00A => Ok(OpCodes::LdVxK(((v & 0x0F00) >> 8) as usize)),
            v if v & 0xF0FF == 0xF018 => Ok(OpCodes::LdStVx(((v & 0x0F00) >> 8) as usize)),
            v if v & 0xF0FF == 0xF029 => Ok(OpCodes::LdFVx(((v & 0x0F00) >> 8) as usize)),
            v if v & 0xF0FF == 0xF033 => Ok(OpCodes::LdBVx(((v & 0x0F00) >> 8) as usize)),
            v if v & 0xF0FF == 0xF01E => Ok(OpCodes::AddIVx(((v & 0x0F00) >> 8) as usize)),

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
            dt: 0,
            i: 0,
            display: [0; 64 * 32],
            stack: vec![],
            mode: Modes::Chip8,
            keys: [false; 16],
            next_tick: Instant::now(),
            next_timers_tick: Instant::now(),
            sound_playing: false,
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

    pub fn step(&mut self, t: Instant) {
        loop {
            if t < self.next_tick && t < self.next_timers_tick {
                return;
            }

            if self.next_timers_tick < self.next_tick {
                if self.st > 0 {
                    self.st -= 1;
                }
                if self.dt > 0 {
                    self.dt -= 1;
                }
                self.next_timers_tick += Duration::from_secs_f32(1.0 / 60.0);
            } else {
                self.tick();
                self.next_tick += Duration::from_secs_f32(1.0 / 700.0);
            }
            if self.st > 0 && !self.sound_playing {
                // TODO
                // play sound
                self.sound_playing = true;
                println!("Start sound");
            } else if self.st == 0 && self.sound_playing {
                println!("stop sound");
                self.sound_playing = false;
                // stop sound
            }
        }
    }

    pub fn tick(&mut self) {
        let next_instruction: u16 =
            u16::from_be_bytes(self.memory[self.pc..self.pc + 2].try_into().unwrap());
        self.pc += 2;

        let op = OpCodes::try_from(next_instruction).unwrap();
        println!("{:#06x}: {:?}", next_instruction, op);
        // println!("{:?}", self);

        match op {
            OpCodes::CLS => {
                self.display.fill(0);
            }
            OpCodes::LdINn(n) => {
                self.i = n;
            }
            OpCodes::RndVxNn(x, n) => {
                self.v[x] = n & rand::random::<u8>();
            }
            OpCodes::LdVxNn(x, n) => {
                self.v[x] = n;
            }
            OpCodes::DrawVxVyN(vx, vy, n) => {
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
                        if ((0b10000000 >> dx) & line) != 0 {
                            self.display[loc] ^= 255;
                        }
                        if cur == 255 && self.display[loc] == 0 {
                            self.v[0xf] = 1;
                        }
                    }
                }
            }

            OpCodes::SkpVx(x) => {
                if self.keys[self.v[x] as usize] {
                    self.pc += 2;
                }
            }
            OpCodes::SknpVx(x) => {
                if !self.keys[self.v[x] as usize] {
                    self.pc += 2;
                }
            }
            OpCodes::AddVxNn(x, n) => {
                self.v[x] = self.v[x].wrapping_add(n);
            }
            OpCodes::JMP(n) => {
                self.pc = n;
            }
            OpCodes::JmpV0Nnn(n) => {
                self.pc = n + self.v[0] as usize;
            }
            OpCodes::NOOP => {}
            OpCodes::SeVxNn(x, n) => {
                if self.v[x] == n {
                    self.pc += 2;
                }
            }
            OpCodes::SneVxNn(x, n) => {
                if self.v[x] != n {
                    self.pc += 2;
                }
            }
            OpCodes::SeVxVy(x, y) => {
                if self.v[x] == self.v[y] {
                    self.pc += 2;
                }
            }
            OpCodes::SneVxVy(x, y) => {
                if self.v[x] != self.v[y] {
                    self.pc += 2;
                }
            }
            OpCodes::CALL(n) => {
                self.stack.push(self.pc);
                self.pc = n;
            }
            OpCodes::RET => self.pc = self.stack.pop().unwrap(),
            OpCodes::LdVxVy(x, y) => {
                self.v[x] = self.v[y];
            }
            OpCodes::OrVxVy(x, y) => {
                self.v[x] |= self.v[y];
            }
            OpCodes::AndVxVy(x, y) => {
                self.v[x] &= self.v[y];
            }
            OpCodes::XorVxVy(x, y) => {
                self.v[x] ^= self.v[y];
            }
            OpCodes::AddVxVy(x, y) => {
                let (result, did_overflow) = self.v[x].overflowing_add(self.v[y]);
                self.v[x] = result;
                self.v[0xf] = if did_overflow { 1 } else { 0 };
            }
            OpCodes::SubVxVy(x, y) => {
                let (result, did_overflow) = self.v[x].overflowing_sub(self.v[y]);
                self.v[x] = result;
                self.v[0xf] = if did_overflow { 1 } else { 0 };
            }
            OpCodes::SubnVxVy(x, y) => {
                let (result, did_overflow) = self.v[y].overflowing_sub(self.v[x]);
                self.v[x] = result;
                self.v[0xf] = if did_overflow { 1 } else { 0 };
            }
            OpCodes::ShrVxVy(x, y) => {
                if self.mode == Modes::Chip8 {
                    self.v[x] = self.v[y];
                }
                self.v[0xf] = self.v[x] & 0x01;
                self.v[x] = self.v[x] >> 1;
            }
            OpCodes::ShlVxVy(x, y) => {
                if self.mode == Modes::Chip8 {
                    self.v[x] = self.v[y];
                }
                self.v[0xf] = self.v[x] & 0x80;
                self.v[x] = self.v[x] << 1;
            }
            OpCodes::LdIVx(x) => {
                for dx in 0..x + 1 {
                    self.memory[(self.i as usize) + dx] = self.v[dx];
                }
            }
            OpCodes::LdVxI(x) => {
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
                self.st = self.v[x];
            }
            OpCodes::LdDtVx(x) => {
                self.dt = self.v[x];
            }
            OpCodes::LdVxDt(x) => {
                self.v[x] = self.dt;
            }
            OpCodes::LdFVx(x) => {
                self.i = (self.v[x] * 0x5) as u16;
            }
            OpCodes::AddIVx(x) => {
                self.i = self.i + self.v[x] as u16;
            }
            OpCodes::LdBVx(x) => {
                self.memory[(self.i as usize)] = self.v[x] / 100;
                self.memory[(self.i as usize) + 1] = (self.v[x] / 10) % 10;
                self.memory[(self.i as usize) + 2] = self.v[x] % 10;
            }
        }
    }
}
