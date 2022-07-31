use core::fmt;
use std::{
    fs::File,
    io::{stdin, BufReader, Read},
};

struct Chip8 {
    memory: [u8; 4096],
    display: [bool; 64 * 32],
    v: [u8; 16],
    pc: usize,
    i: u16,
    stack: Vec<usize>,
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

#[derive(Debug)]
enum OpCodes {
    NOOP,
    CLS,
    JMP(usize),
    CALL(usize),
    RET,
    LD(usize, u8),
    ADD(usize, u8),
    LDI(u16),
    DRAW(usize, usize, u16),
    SE(usize, u8),
    SNE(usize, u8),
    SEV(usize, usize),
    SNEV(usize, usize),
}

impl TryFrom<u16> for OpCodes {
    type Error = ();

    fn try_from(v: u16) -> Result<Self, Self::Error> {
        match v {
            0x0000 => Ok(OpCodes::NOOP),
            0x00EE => Ok(OpCodes::RET),
            0x00E0 => Ok(OpCodes::CLS),
            v if v & 0xF000 == 0x1000 => Ok(OpCodes::JMP((v & 0x0FFF) as usize)),
            v if v & 0xF000 == 0x2000 => Ok(OpCodes::CALL((v & 0x0FFF) as usize)),
            v if v & 0xF000 == 0x6000 => Ok(OpCodes::LD(
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
            v if v & 0xF000 == 0x9000 => Ok(OpCodes::SNEV(
                ((v & 0x0F00) >> 8).try_into().unwrap(),
                ((v & 0x00F0) >> 4).try_into().unwrap(),
            )),

            _ => Err(()),
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
    };

    let mut file = File::open("roms/test_opcode.ch8")?;
    file.read(&mut c.memory[0x200..])
        .expect("Failed to read file");

    loop {
        let next_instruction: u16 =
            u16::from_be_bytes(c.memory[c.pc..c.pc + 2].try_into().unwrap());
        c.pc += 2;

        print!("{:#06x} ", next_instruction);

        // match next_instruction.try_into().unwrap() {
        match OpCodes::try_from(next_instruction).unwrap() {
            OpCodes::CLS => {
                println!("CLS");
                c.display.fill(false);
            }
            OpCodes::LDI(n) => {
                println!("LDI {:#06x}", n);
                c.i = n;
            }
            OpCodes::LD(x, n) => {
                println!("LD {:#06x} {:#06x}", x, n);
                c.v[x] = n;
            }
            OpCodes::DRAW(vx, vy, n) => {
                println!("DRAW {:#06x} {:#06x} {:#06x}", vx, vy, n);
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
                println!("ADD {:#06x} {:#06x}", x, n);
                c.v[x] = c.v[x].wrapping_add(n);
            }
            OpCodes::JMP(n) => {
                println!("JMP {:#06x}", n);
                c.pc = n.try_into().unwrap();
            }
            OpCodes::NOOP => {
                println!("NOOP");
            }
            OpCodes::SE(x, n) => {
                println!("SE {:#06x} {:#06x}", x, n);
                if c.v[x] == n {
                    c.pc += 2;
                }
            }
            OpCodes::SNE(x, n) => {
                println!("SNE {:#06x} {:#06x}", x, n);
                if c.v[x] != n {
                    c.pc += 2;
                }
            }
            OpCodes::SEV(x, y) => {
                println!("SNV {:#06x} {:#06x}", x, y);
                if c.v[x] == c.v[y] {
                    c.pc += 2;
                }
            }
            OpCodes::SNEV(x, y) => {
                println!("SNEV {:#06x} {:#06x}", x, y);
                if c.v[x] != c.v[y] {
                    c.pc += 2;
                }
            }
            OpCodes::CALL(n) => {
                println!("JMP {:#06x}", n);
                c.stack.push(c.pc);
                c.pc = n;
            }
            OpCodes::RET => {
                println!("RET");
                c.pc = c.stack.pop().unwrap()
            }
        }
        println!("{:?}", c);

        stdin().read(&mut [0]).unwrap();
    }

    Ok(())
}
