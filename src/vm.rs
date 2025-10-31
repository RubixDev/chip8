use std::{
    env,
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};

use evdev::InputEvent;
use image::{DynamicImage, Rgb, RgbImage};
use rand::Rng;
use rodio::{OutputStream, Sink};
use viuer::Config;

use crate::{audio::SquareWave, input};

const FONT_SET: [u8; 80] = [
    0xf0, 0x90, 0x90, 0x90, 0xf0, 0x20, 0x60, 0x20, 0x20, 0x70, 0xf0, 0x10, 0xf0, 0x80, 0xf0, 0xf0,
    0x10, 0xf0, 0x10, 0xf0, 0x90, 0x90, 0xf0, 0x10, 0x10, 0xf0, 0x80, 0xf0, 0x10, 0xf0, 0xf0, 0x80,
    0xf0, 0x90, 0xf0, 0xf0, 0x10, 0x20, 0x40, 0x40, 0xf0, 0x90, 0xf0, 0x90, 0xf0, 0xf0, 0x90, 0xf0,
    0x10, 0xf0, 0xf0, 0x90, 0xf0, 0x90, 0x90, 0xe0, 0x90, 0xe0, 0x90, 0xe0, 0xf0, 0x80, 0x80, 0x80,
    0xf0, 0xe0, 0x90, 0x90, 0x90, 0xe0, 0xf0, 0x80, 0xf0, 0x80, 0xf0, 0xf0, 0x80, 0xf0, 0x80, 0x80,
];

pub struct Vm {
    memory: [u8; 0x1000],
    pc: usize,
    v: [u8; 16],
    i: usize,
    stack: Vec<usize>,
    screen: [[bool; 64]; 32],
    delay_timer: u8,
    sound_timer: u8,
    sound_start: Instant,
    viuer_conf: Config,
    input_channel: Receiver<InputEvent>,
    pressed_keys: [bool; 16],
}

impl Vm {
    pub fn run(mut rom: [u8; 0x1000], speed: u64) -> Self {
        // force viuer to use unicode blocks
        env::set_var("TERM", "xterm-256color");
        let viuer_conf = Config {
            use_kitty: false,
            use_iterm: false,
            // restore_cursor: true,
            // y: 1,
            ..Default::default()
        };

        // clear the screen
        println!("\x1b[2J");

        let (sender, input_channel) = mpsc::channel();
        input::start_listeners(sender);

        rom[0..80].copy_from_slice(&FONT_SET);
        let mut vm = Self {
            memory: rom,
            pc: 0x200,
            v: [0; 16],
            i: 0,
            stack: vec![],
            screen: [[false; 64]; 32],
            delay_timer: 0,
            sound_timer: 0,
            sound_start: Instant::now(),
            viuer_conf,
            input_channel,
            pressed_keys: [false; 16],
        };

        let (_stream, handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&handle).unwrap();
        sink.append(SquareWave::new(240.));
        sink.set_volume(0.4);
        sink.pause();

        let mut print_offset = 0;
        loop {
            print_offset += 1;
            print_offset %= 30;
            if vm.sound_timer > 0 {
                if sink.is_paused() {
                    sink.play();
                }
            } else if !sink.is_paused()
                // play sounds for at least 50 milliseconds
                && Instant::now().duration_since(vm.sound_start).as_millis() > 50
            {
                sink.pause();
            }
            thread::sleep(Duration::from_millis(speed));
            vm.delay_timer = vm.delay_timer.saturating_sub(1);
            vm.sound_timer = vm.sound_timer.saturating_sub(1);
            vm.show_screen();

            let (hi, lo) = (vm.memory[vm.pc], vm.memory[vm.pc + 1]);
            print!("\x1b[{print_offset}B\x1b[K{hi:02x}{lo:02x}: ");
            match (hi, lo) {
                (0x00, 0xe0) => {
                    println!("clear screen");
                    vm.screen = [[false; 64]; 32];
                }
                (0x00, 0xee) => {
                    println!("return");
                    match vm.stack.pop() {
                        Some(addr) => vm.pc = addr,
                        None => break,
                    }
                }
                (0x10..=0x1f, _) => {
                    let addr = lo as usize | ((hi as usize) & 0x0f) << 8;
                    println!("goto: {addr} ({hi:02x}{lo:02x}) ({hi}) ({lo})");
                    vm.pc = addr;
                    continue;
                }
                (0x20..=0x2f, _) => {
                    let addr = lo as usize | ((hi as usize) & 0x0f) << 8;
                    println!("call: {addr}");
                    vm.stack.push(vm.pc);
                    vm.pc = addr;
                    continue;
                }
                (0x30..=0x3f, _) => {
                    let vx = vm.v[(hi & 0x0f) as usize];
                    let nn = lo;
                    println!("skip if {vx} == {nn}");
                    if vx == nn {
                        vm.pc += 2;
                    }
                }
                (0x40..=0x4f, _) => {
                    let vx = vm.v[(hi & 0x0f) as usize];
                    let nn = lo;
                    println!("skip if {vx} != {nn}");
                    if vx != nn {
                        vm.pc += 2;
                    }
                }
                (0x50..=0x5f, _) if lo & 0x0f == 0 => {
                    let vx = vm.v[(hi & 0x0f) as usize];
                    let vy = vm.v[((lo & 0xf0) >> 4) as usize];
                    println!("skip if {vx} == {vy}");
                    if vx == vy {
                        vm.pc += 2;
                    }
                }
                (0x60..=0x6f, _) => {
                    let vx = (hi & 0x0f) as usize;
                    let nn = lo;
                    println!("v{vx:x} = {nn}");
                    vm.v[vx] = nn;
                }
                (0x70..=0x7f, _) => {
                    let vx = (hi & 0x0f) as usize;
                    let nn = lo;
                    println!("v{vx:x} += {nn}");
                    vm.v[vx] = vm.v[vx].wrapping_add(nn);
                }
                (0x80..=0x8f, _) => {
                    let vx = (hi & 0x0f) as usize;
                    let vy = ((lo & 0xf0) >> 4) as usize;
                    match lo & 0x0f {
                        0x00 => {
                            println!("v{vx:x} = v{vy:x}");
                            vm.v[vx] = vm.v[vy];
                        }
                        0x01 => {
                            println!("v{vx:x} |= v{vy:x}");
                            vm.v[vx] |= vm.v[vy];
                        }
                        0x02 => {
                            println!("v{vx:x} &= v{vy:x}");
                            vm.v[vx] &= vm.v[vy];
                        }
                        0x03 => {
                            println!("v{vx:x} ^= v{vy:x}");
                            vm.v[vx] ^= vm.v[vy];
                        }
                        0x04 => {
                            println!("v{vx:x} += v{vy:x}");
                            let (new_vx, carry) = vm.v[vx].overflowing_add(vm.v[vy]);
                            vm.v[vx] = new_vx;
                            vm.v[0xf] = carry as u8;
                        }
                        0x05 => {
                            println!("v{vx:x} -= v{vy:x} ({} -= {})", vm.v[vx], vm.v[vy]);
                            vm.v[0xf] = (vm.v[vx] > vm.v[vy]) as u8;
                            vm.v[vx] = vm.v[vx].wrapping_sub(vm.v[vy]);
                        }
                        0x06 => {
                            println!("v{vx:x} >>= 1");
                            vm.v[0xf] = vm.v[vx] & 1;
                            vm.v[vx] >>= 1;
                        }
                        0x07 => {
                            println!("v{vx:x} = v{vy:x} - v{vx:x}");
                            vm.v[0xf] = (vm.v[vy] > vm.v[vx]) as u8;
                            vm.v[vx] = vm.v[vy].wrapping_sub(vm.v[vx]);
                        }
                        0x0e => {
                            println!("v{vx:x} <<= 1");
                            vm.v[0xf] = (vm.v[vx] >> 7) & 1;
                            vm.v[vx] <<= 1;
                        }
                        _ => {
                            panic!("illegal instruction: {hi:02x}{lo:02x}")
                        }
                    }
                }
                (0x90..=0x9f, _) if lo & 0x0f == 0 => {
                    let vx = vm.v[(hi & 0x0f) as usize];
                    let vy = vm.v[((lo & 0xf0) >> 4) as usize];
                    println!("skip if {vx} != {vy}");
                    if vx != vy {
                        vm.pc += 2;
                    }
                }
                (0xa0..=0xaf, _) => {
                    let addr = lo as usize | ((hi as usize) & 0x0f) << 8;
                    println!("I = {addr}");
                    vm.i = addr;
                }
                (0xb0..=0xbf, _) => {
                    let addr = lo as usize | ((hi as usize) & 0x0f) << 8;
                    println!("PC = v0 + {addr}");
                    vm.pc = addr.wrapping_add(vm.v[0x0] as usize);
                    continue;
                }
                (0xc0..=0xcf, _) => {
                    let vx = (hi & 0x0f) as usize;
                    let nn = lo;
                    println!("v{vx:x} == rand() & {nn}");
                    let rand = rand::thread_rng().gen::<u8>();
                    vm.v[vx] = rand & nn;
                }
                (0xd0..=0xdf, _) => {
                    let vx = vm.v[(hi & 0x0f) as usize] as usize;
                    let vy = vm.v[((lo & 0xf0) >> 4) as usize] as usize;
                    let n = lo as usize & 0x0f;
                    println!("draw({vx}, {vy}, {n})");

                    // TODO: collisions are broken?
                    vm.v[0xf] = 0;
                    for row in 0..n {
                        for column in 0..8 {
                            let pixel = &mut vm.screen[(row + vy) % 32][(column + vx) % 64];
                            let new_pixel = (vm.memory[vm.i + row] >> (7 - column)) & 1;
                            vm.v[0xf] |= *pixel as u8 & new_pixel;
                            *pixel ^= new_pixel != 0;
                        }
                    }
                }
                (0xe0..=0xef, 0x9e) => {
                    let vx = vm.v[(hi & 0x0f) as usize] as usize;
                    println!("skip if key {vx} pressed");
                    vm.fetch_key_events();
                    if vm.pressed_keys[vx] {
                        vm.pc += 2;
                    }
                }
                (0xe0..=0xef, 0xa1) => {
                    let vx = vm.v[(hi & 0x0f) as usize] as usize;
                    println!("skip if key {vx} not pressed");
                    vm.fetch_key_events();
                    if !vm.pressed_keys[vx] {
                        vm.pc += 2;
                    }
                }
                (0xf0..=0xff, 0x07) => {
                    let vx = (hi & 0x0f) as usize;
                    println!("v{vx:x} = get_delay() = {}", vm.delay_timer);
                    vm.v[vx] = vm.delay_timer;
                }
                (0xf0..=0xff, 0x0a) => {
                    let vx = (hi & 0x0f) as usize;
                    println!("v{vx:x} = get_key()");
                    vm.v[vx] = vm.wait_for_key();
                }
                (0xf0..=0xff, 0x15) => {
                    let vx = vm.v[(hi & 0x0f) as usize];
                    println!("delay_timer = {vx}");
                    vm.delay_timer = vx;
                }
                (0xf0..=0xff, 0x18) => {
                    let vx = vm.v[(hi & 0x0f) as usize];
                    vm.sound_start = Instant::now();
                    vm.sound_timer = vx;
                }
                (0xf0..=0xff, 0x1e) => {
                    let vx = vm.v[(hi & 0x0f) as usize];
                    println!("I += {vx}");
                    vm.i = vm.i.wrapping_add(vx as usize);
                }
                (0xf0..=0xff, 0x29) => {
                    let vx = vm.v[(hi & 0x0f) as usize] as usize;
                    println!("I = sprite_addr[{vx}]");
                    vm.i = vx * 5;
                }
                (0xf0..=0xff, 0x33) => {
                    let mut vx = vm.v[(hi & 0x0f) as usize];
                    println!("store BCD of {vx}");

                    for offset in (0..3).rev() {
                        vm.memory[vm.i + offset] = vx % 10;
                        vx /= 10;
                    }
                }
                (0xf0..=0xff, 0x55) => {
                    let vx = (hi & 0x0f) as usize;
                    println!("reg_dump({vx}, {})", vm.i);
                    for v in 0..=vx {
                        vm.memory[vm.i + v] = vm.v[v];
                    }
                }
                (0xf0..=0xff, 0x65) => {
                    let vx = (hi & 0x0f) as usize;
                    println!("reg_load({vx}, {})", vm.i);
                    for v in 0..=vx {
                        vm.v[v] = vm.memory[vm.i + v];
                    }
                }
                _ => {
                    panic!("illegal instruction: {hi:02x}{lo:02x}")
                }
            }
            vm.pc += 2;
        }
        vm
    }

    fn show_screen(&self) {
        let mut img = RgbImage::new(64, 32);
        for (y, row) in self.screen.iter().enumerate() {
            for (x, pixel) in row.iter().enumerate() {
                img.put_pixel(
                    x as u32,
                    y as u32,
                    if *pixel {
                        Rgb([0, 255, 0])
                    } else {
                        Rgb([0; 3])
                    },
                );
            }
        }
        viuer::print(&DynamicImage::from(img), &self.viuer_conf).unwrap();
    }

    fn fetch_key_events(&mut self) {
        for event in self.input_channel.try_iter() {
            if let Some(idx) = input::key_idx(&event) {
                if event.value() == 0 {
                    self.pressed_keys[idx] = false;
                } else {
                    self.pressed_keys[idx] = true;
                }
            }
        }
    }

    fn wait_for_key(&mut self) -> u8 {
        loop {
            let event = self.input_channel.recv().unwrap();
            if let Some(idx) = input::key_idx(&event) {
                if event.value() == 0 {
                    self.pressed_keys[idx] = false;
                } else {
                    self.pressed_keys[idx] = true;
                    break idx as u8;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(code: impl AsRef<[u16]>) {
        let mut rom = [0; 0x1000];
        for (index, instr) in code.as_ref().iter().enumerate() {
            rom[index * 2 + 0x200..index * 2 + 0x202].copy_from_slice(&instr.to_be_bytes());
        }

        eprintln!("------");
        for chunk in rom[0x200..0x202 + code.as_ref().len() * 2].chunks(2) {
            for byte in chunk {
                eprint!("{byte:02x}");
            }
            eprintln!();
        }
        eprintln!("------");

        Vm::run(rom, 0);
    }

    #[test]
    fn jumps() {
        let mut rom = [0; 0x1000];
        rom[0x200] = 0x61;
        rom[0x201] = 0xff;

        rom[0x202] = 0x62;
        rom[0x203] = 0xfe;

        rom[0x204] = 0x91;
        rom[0x205] = 0x20;

        rom[0x208] = 0x00;
        rom[0x209] = 0xee;

        Vm::run(rom, 10);
    }

    #[test]
    fn arithmetic() {
        run([
            0x1206, //
            0x60ff, // v0 = ff
            0x61fe, // v1 = fe
            0x9010, //
            0x00ee, //
        ]);
    }

    #[test]
    fn collision() {
        run([
            0x1204, // skip the sprite data
            0xffff, // sprite data
            0x6001, // v0 = 1
            0x6101, // v1 = 1
            0xa202, // I = $202
            0xd012, // draw the sprite
            0x3f00, // vf should be 0
            0x1fff, // (goto $fff if check failed)
            0xd012, // draw the sprite again
            0x4f00, // vf should be 1
            0x1fff, // (goto $fff if check failed)
            0x00ee, // return
        ])
    }
}
