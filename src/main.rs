use std::{env, fs};

use chip8::Vm;

fn main() {
    let filename = env::args().nth(1).unwrap();
    let speed = env::args().nth(2).unwrap().parse::<u64>().unwrap();

    let mut rom = [0; 0x1000];
    let mut buf = fs::read(filename).unwrap();
    buf.resize(0x1000 - 0x200, 0);
    rom[0x200..].copy_from_slice(&buf);

    Vm::run(rom, speed);
}
