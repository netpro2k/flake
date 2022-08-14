use core::fmt;
use std::{
    fs::File,
    io::Read,
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

    pub execution_speed: f32,
    pub next_tick: Instant,
    pub next_timers_tick: Instant,

    sound_playing: bool,
}

impl Chip8 {
    pub fn compare(a: &Chip8, b: &Chip8) -> String {
        let mut s = vec![];

        a.memory
            .iter()
            .enumerate()
            .zip(b.memory.iter())
            .filter(|((_index, x), y)| x != y)
            .for_each(|((index, x), y)| {
                s.push(format!("Memory {:#06x}: {:#06x} → {:#06x}", index, x, y))
            });

        a.display
            .iter()
            .enumerate()
            .zip(b.display.iter())
            .filter(|((_index, x), y)| x != y)
            .for_each(|((index, x), y)| {
                s.push(format!("Display {:#06x}: {:#06x} → {:#06x}", index, x, y))
            });

        a.v.iter()
            .enumerate()
            .zip(b.v.iter())
            .filter(|((_index, x), y)| x != y)
            .for_each(|((index, x), y)| {
                s.push(format!("V {:#06x}: {:#06x} → {:#06x}", index, x, y))
            });

        if a.pc != b.pc {
            s.push(format!("PC: {:#06x} → {:#06x}", a.pc, b.pc));
        }

        if a.st != b.st {
            s.push(format!("ST: {:#06x} → {:#06x}", a.st, b.st));
        }

        if a.dt != b.dt {
            s.push(format!("DT: {:#06x} → {:#06x}", a.dt, b.dt));
        }

        if a.i != b.i {
            s.push(format!(" I: {:#06x} → {:#06x}", a.i, b.i));
        }

        // a.stack
        //     .iter()
        //     .enumerate()
        //     .zip(b.stack.iter())
        //     .filter(|((_index, x), y)| x != y)
        //     .for_each(|((index, x), y)| {
        //         s.push(format!("Stack {:#06x}: {:#06x} → {:#06x}", index, x, y))
        //     });

        // stack

        if a.mode != b.mode {
            s.push(format!(" mode: {:?} → {:?}", a.mode, b.mode));
        }

        // keys

        if a.next_tick != b.next_tick {
            s.push(format!(" tick: {:?} → {:?}", a.next_tick, b.next_tick));
        }

        if a.next_timers_tick != b.next_timers_tick {
            s.push(format!(
                "timers: {:?} → {:?}",
                a.next_timers_tick, b.next_timers_tick
            ));
        }

        if a.sound_playing != b.sound_playing {
            s.push(format!(
                "sound_playing: {:?} → {:?}",
                a.sound_playing, b.sound_playing
            ));
        }

        s.join("\n")
    }
}

impl std::clone::Clone for Chip8 {
    fn clone(&self) -> Self {
        let mut chip8 = Chip8::new();
        chip8.clone_from(self);
        chip8
    }

    fn clone_from(&mut self, source: &Self) {
        self.memory.copy_from_slice(&source.memory);
        self.display.copy_from_slice(&source.display);
        self.v.copy_from_slice(&source.v);
        self.pc = source.pc;
        self.st = source.st;
        self.dt = source.dt;
        self.i = source.i;
        self.stack = source.stack.clone();
        self.mode = source.mode;
        self.keys.copy_from_slice(&source.keys);
        self.execution_speed = source.execution_speed;
        self.next_tick = source.next_tick;
        self.next_timers_tick = source.next_timers_tick;
        self.sound_playing = source.sound_playing;
    }
}

impl fmt::Debug for Chip8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "




■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■
{}
■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■
       PC: {:#06x}
        I: {:#06x}
 V[index]: {}
V[values]: {}
    Stack: {}
       ST: {}
       DT: {}
",
            &self
                .display
                .map(|b| if b != 0 { "■" } else { " " })
                .chunks(64)
                .map(|line| line.join("") + "\n")
                .collect::<String>(),
            &self.pc,
            &self.i,
            (0..16)
                .map(|i: u32| format!("{:#06x}, ", i))
                .collect::<String>(),
            &self.v.map(|v| format!("{:#06x}", v)).join(", "),
            &self
                .stack
                .iter()
                .map(|addr| format!("{:#06x}, ", addr))
                .collect::<String>(),
            &self.st,
            &self.dt,
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Modes {
    Chip8,
    // Chip48,
    // SuperChip,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpCodes {
    Unkn(u16),
    Cls,                    // CLS — 00E0
    Ret,                    // RET — 00EE
    Jmp(usize),             // JMP — 1NNN
    Call(usize),            // CALL NNN — 2NNN
    SeVxNn(usize, u8),      // SE VX, NN — 3XNN
    SneVxNn(usize, u8),     // SNE VX, NN — 4XNN
    SeVxVy(usize, usize),   // SE VX, VY — 5XY0
    LdVxNn(usize, u8),      // LD VX, NN — 6XNN
    AddVxNn(usize, u8),     // ADD VX, NN — 7XNN
    LdVxVy(usize, usize),   // LD VX, VY — 8XY0
    OrVxVy(usize, usize),   // OR VX, VY — 8XY1
    AndVxVy(usize, usize),  // AND VX, VY — 8XY2
    XorVxVy(usize, usize),  // XOR VX, VY — 8XY3
    AddVxVy(usize, usize),  // ADD VX, VY — 8XY4
    SubVxVy(usize, usize),  // SUB VX, VY — 8XY5
    ShrVxVy(usize, usize),  // SHR VX {, VY} — 8XY6
    SubnVxVy(usize, usize), // SUBN VX, VY — 8XY7
    ShlVxVy(usize, usize),  // SHL VX {, VY} — 8XYE
    SneVxVy(usize, usize),  // SNE VX, VY — 9XY0
    LdINn(u16),             // LD I, NNN — ANNN
    JmpV0Nnn(usize),        // JMP V0, NNN — BNNN
    // JmpVxNnn(usize, usize),       // JMP V0, NNN — BNNN
    RndVxNn(usize, u8),             // RND VX, NN – CXNN
    DrawVxVyN(usize, usize, usize), // DRW VX, VY, N — DXYN
    SkpVx(usize),                   // SKP VX — EX9E
    SknpVx(usize),                  // SKNP VX — EXA1
    LdVxDt(usize),                  // LD VX, DT — FX07
    LdVxK(usize),                   // LD VX, K — FX0A
    LdDtVx(usize),                  // LD DT, VX — FX15
    LdStVx(usize),                  // LD ST, VX — FX18
    AddIVx(usize),                  // ADD I, VX — FX1E
    LdFVx(usize),                   // LD F, VX — FX29
    LdBVx(usize),                   // LD B, VX — FX33
    LdIVx(usize),                   // LD [I], VX — FX55
    LdVxI(usize),                   // LD VX, [I] — FX65
}

impl TryFrom<u16> for OpCodes {
    type Error = String;

    fn try_from(v: u16) -> Result<Self, Self::Error> {
        let nnn = (v & 0x0FFF) as usize;

        let _byte0 = ((v & 0xFF00) >> 8) as u8;
        let byte1 = (v & 0x00FF) as u8;

        let _nib0 = ((v & 0xF000) >> 12) as usize;
        let nib1 = ((v & 0x0F00) >> 8) as usize;
        let nib2 = ((v & 0x00F0) >> 4) as usize;
        let nib3 = (v & 0x000F) as usize;

        Ok(match v & 0xF000 {
            0x0000 => match v {
                0x00EE => OpCodes::Ret,
                0x00E0 => OpCodes::Cls,
                _ => OpCodes::Unkn(v),
            },
            0x1000 => OpCodes::Jmp(nnn),
            0x2000 => OpCodes::Call(nnn),
            0x3000 => OpCodes::SeVxNn(nib1, byte1),
            0x4000 => OpCodes::SneVxNn(nib1, byte1),
            0x5000 => OpCodes::SeVxVy(nib1, nib2),
            0x6000 => OpCodes::LdVxNn(nib1, byte1),
            0x7000 => OpCodes::AddVxNn(nib1, byte1),
            0x8000 => match v & 0xF00F {
                0x8000 => OpCodes::LdVxVy(nib1, nib2),
                0x8001 => OpCodes::OrVxVy(nib1, nib2),
                0x8002 => OpCodes::AndVxVy(nib1, nib2),
                0x8003 => OpCodes::XorVxVy(nib1, nib2),
                0x8004 => OpCodes::AddVxVy(nib1, nib2),
                0x8005 => OpCodes::SubVxVy(nib1, nib2),
                0x8006 => OpCodes::ShrVxVy(nib1, nib2),
                0x8007 => OpCodes::SubnVxVy(nib1, nib2),
                0x800E => OpCodes::ShlVxVy(nib1, nib2),
                _ => OpCodes::Unkn(v),
            },
            0x9000 => OpCodes::SneVxVy(nib1, nib2),
            0xA000 => OpCodes::LdINn(nnn as u16),
            0xB000 => OpCodes::JmpV0Nnn(nnn),
            // 0xB000 => OpCodes::JmpVxNnn(nib1, nnn),
            0xC000 => OpCodes::RndVxNn(nib1, byte1),
            0xD000 => OpCodes::DrawVxVyN(nib1, nib2, nib3),
            0xE000 => match v & 0xF0FF {
                0xE09E => OpCodes::SkpVx(nib1),
                0xE0A1 => OpCodes::SknpVx(nib1),
                _ => OpCodes::Unkn(v),
            },
            0xF000 => match v & 0xF0FF {
                0xF015 => OpCodes::LdDtVx(nib1),
                0xF055 => OpCodes::LdIVx(nib1),
                0xF065 => OpCodes::LdVxI(nib1),
                0xF007 => OpCodes::LdVxDt(nib1),
                0xF00A => OpCodes::LdVxK(nib1),
                0xF018 => OpCodes::LdStVx(nib1),
                0xF029 => OpCodes::LdFVx(nib1),
                0xF033 => OpCodes::LdBVx(nib1),
                0xF01E => OpCodes::AddIVx(nib1),
                _ => OpCodes::Unkn(v),
            },
            _ => OpCodes::Unkn(v),
        })
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
            execution_speed: 1.0,
        }
    }

    pub fn load(&mut self, filename: &str) -> Result<(), std::io::Error> {
        self.memory.fill(0);

        self.memory[0..(16 * 5)].copy_from_slice(&[
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
        let file_length = file.metadata().unwrap().len() as usize;
        file.read_exact(&mut self.memory[0x200..0x200 + file_length])
            .expect("Failed to read file");
        Ok(())
    }

    pub fn step_debug(&mut self) {
        if self.next_timers_tick < self.next_tick {
            if self.st > 0 {
                self.st -= 1;
            }
            if self.dt > 0 {
                self.dt -= 1;
            }
            self.next_timers_tick += Duration::from_secs_f32(1.0 / (60.0 * self.execution_speed));
        } else {
            self.tick();
            self.next_tick += Duration::from_secs_f32(1.0 / (700.0 * self.execution_speed));
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

    pub fn step_with_time(&mut self) {
        let t = Instant::now();
        while t > self.next_tick && t > self.next_timers_tick {
            self.step_debug();
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
            OpCodes::Unkn(c) => {
                panic!("Unknwon opcode {}", c);
            }
            OpCodes::Cls => {
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
                for dy in 0..n {
                    if (y + dy) >= 32 {
                        break; // clip
                    }
                    let line: u8 = self.memory[self.i as usize + dy];
                    for dx in 0..8usize {
                        if (x + dx) >= 64 {
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
            OpCodes::Jmp(n) => {
                self.pc = n;
            }
            // OpCodes::JmpVxNnn(x, n) => {
            //     self.pc = n + self.v[x] as usize;
            // }
            OpCodes::JmpV0Nnn(n) => {
                self.pc = n + self.v[0] as usize;
            }
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
            OpCodes::Call(n) => {
                self.stack.push(self.pc);
                self.pc = n;
            }
            OpCodes::Ret => self.pc = self.stack.pop().unwrap(),
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
                self.v[0xf] = self.v[x] & 1;
                self.v[x] >>= 1;
            }
            OpCodes::ShlVxVy(x, y) => {
                if self.mode == Modes::Chip8 {
                    self.v[x] = self.v[y];
                }
                self.v[0xf] = self.v[x] >> 7;
                self.v[x] <<= 1;
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
                self.i += self.v[x] as u16;
            }
            OpCodes::LdBVx(x) => {
                self.memory[(self.i as usize)] = self.v[x] / 100;
                self.memory[(self.i as usize) + 1] = (self.v[x] / 10) % 10;
                self.memory[(self.i as usize) + 2] = self.v[x] % 10;
            }
        }
    }
}
