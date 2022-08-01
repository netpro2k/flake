use core::fmt;
use std::thread;
use std::time;
use std::{
    fs::File,
    io::{stdin, stdout, BufReader, Read, Write},
};

struct Chip8 {
    memory: [u8; 4096],
    display: [bool; 64 * 32],
    v: [u8; 16],
    pc: usize,
    i: u16,
    stack: Vec<usize>,
    mode: Modes,
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
                .map(|b| if b { "■" } else { " " })
                .chunks(64)
                .map(|line| line.join("") + "\n")
                .collect::<String>()
        ))
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Modes {
    Chip8,
    Chip48,
    SuperChip,
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
    // RndVxNn,
    DRAW(usize, usize, u16),
    // SkpVx,
    // SknpVx,
    // LdVxDt,
    LdVxK(usize),
    // LdDtVx,
    // LdStVx,
    // AddIVx,
    // LdFVx,
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
            v if v & 0xF000 == 0xD000 => Ok(OpCodes::DRAW(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                ((v & 0x00F0) >> 4).try_into().unwrap(),
                v & 0x000F,
            )),
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
            v if v & 0xF0FF == 0xF00A => {
                Ok(OpCodes::LdVxK(((v & 0x0F00) >> 8).try_into().unwrap()))
            }
            v if v & 0xF0FF == 0xF033 => Ok(OpCodes::LDBV(((v & 0x0F00) >> 8).try_into().unwrap())),

            _ => Err(format!("Op code not implemented for {:#06x}", v)),
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut c = Chip8 {
        memory: [0; 4096],
        v: [0; 16],
        pc: 0x200,
        i: 0,
        display: [false; 64 * 32],
        stack: vec![],
        mode: Modes::Chip8,
    };

    let mut file = File::open("roms/breakout.ch8")?;
    file.read(&mut c.memory[0x200..])
        .expect("Failed to read file");

    loop {
        let next_instruction: u16 =
            u16::from_be_bytes(c.memory[c.pc..c.pc + 2].try_into().unwrap());
        c.pc += 2;

        let op = OpCodes::try_from(next_instruction).unwrap();
        println!("{:#06x}: {:?}", next_instruction, op);
        match op {
            OpCodes::CLS => {
                c.display.fill(false);
            }
            OpCodes::LDI(n) => {
                c.i = n;
            }
            OpCodes::LdVxNn(x, n) => {
                c.v[x] = n;
            }
            OpCodes::DRAW(vx, vy, n) => {
                c.v[0xf] = 0;
                let x = c.v[vx] as usize;
                let y = c.v[vy] as usize;
                for dy in 0..n as usize {
                    let line: u8 = c.memory[c.i as usize + dy];
                    for dx in 0..8 as usize {
                        let loc = x + dx + (y + dy) * 64;
                        let cur = c.display[loc];
                        c.display[loc] ^= ((0b10000000 >> dx) & line) != 0;
                        if cur && !c.display[loc] {
                            c.v[0xf] = 1;
                        }
                    }
                }
            }
            OpCodes::ADD(x, n) => {
                c.v[x] = c.v[x].wrapping_add(n);
            }
            OpCodes::JMP(n) => {
                let target = n.try_into().unwrap();
                if target == c.pc - 2 {
                    println!("Infinite jump... quitting..");
                    break;
                }
                c.pc = target;
            }
            OpCodes::NOOP => {}
            OpCodes::SE(x, n) => {
                if c.v[x] == n {
                    c.pc += 2;
                }
            }
            OpCodes::SNE(x, n) => {
                if c.v[x] != n {
                    c.pc += 2;
                }
            }
            OpCodes::SEV(x, y) => {
                if c.v[x] == c.v[y] {
                    c.pc += 2;
                }
            }
            OpCodes::SNEV(x, y) => {
                if c.v[x] != c.v[y] {
                    c.pc += 2;
                }
            }
            OpCodes::CALL(n) => {
                c.stack.push(c.pc);
                c.pc = n;
            }
            OpCodes::RET => c.pc = c.stack.pop().unwrap(),
            OpCodes::LDV(x, y) => {
                c.v[x] = c.v[y];
            }
            OpCodes::ORV(x, y) => {
                c.v[x] |= c.v[y];
            }
            OpCodes::ANDV(x, y) => {
                c.v[x] &= c.v[y];
            }
            OpCodes::XORV(x, y) => {
                c.v[x] ^= c.v[y];
            }
            OpCodes::ADDV(x, y) => {
                let (result, did_overflow) = c.v[x].overflowing_add(c.v[y]);
                c.v[x] = result;
                c.v[0xf] = if did_overflow { 1 } else { 0 };
            }
            OpCodes::SUBV(x, y) => {
                let (result, did_overflow) = c.v[x].overflowing_sub(c.v[y]);
                c.v[x] = result;
                c.v[0xf] = if did_overflow { 1 } else { 0 };
            }
            OpCodes::SUBNV(x, y) => {
                let (result, did_overflow) = c.v[y].overflowing_sub(c.v[x]);
                c.v[x] = result;
                c.v[0xf] = if did_overflow { 1 } else { 0 };
            }
            OpCodes::SHRV(x, y) => {
                if c.mode == Modes::Chip8 {
                    c.v[x] = c.v[y];
                }
                c.v[0xf] = c.v[x] & 0x01;
                c.v[x] = c.v[x] >> 1;
            }
            OpCodes::SHLV(x, y) => {
                if c.mode == Modes::Chip8 {
                    c.v[x] = c.v[y];
                }
                c.v[0xf] = c.v[x] & 0x80;
                c.v[x] = c.v[x] << 1;
            }
            OpCodes::LDIV(x) => {
                for dx in 0..x + 1 {
                    c.memory[(c.i as usize) + dx] = c.v[dx];
                }
            }
            OpCodes::LDVI(x) => {
                for dx in 0..x + 1 {
                    c.v[dx] = c.memory[(c.i as usize) + dx];
                }
            }
            OpCodes::LdVxK(x) => {
                println!("TODO!");
            }
            OpCodes::LDBV(x) => {
                c.memory[(c.i as usize)] = c.v[x] / 100;
                c.memory[(c.i as usize) + 1] = (c.v[x] / 10) % 10;
                c.memory[(c.i as usize) + 2] = c.v[x] % 10;
            }
        }
        println!("{:?}", c);

        // stdin().read(&mut [0]).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    Ok(())
}
