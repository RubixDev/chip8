# Terminal based CHIP-8 Emulator

This emulator was mostly written in a single day a couple of years ago. It's not
the best out there, but should theoretically support everything.

## Usage

The binary takes two arguments:

1. The path to the ROM file to run
2. The emulation speed as a number of milliseconds to wait between each CPU
   cycle. Lower numbers mean faster speeds

```bash
cargo run --release -- <path/to/rom.ch8> <speed>
```

### Input

Input is handled via `evdev`, which requires the user to have read access to the
files in `/dev/input`, usually by being in the `input` group.

The 4x4 keypad is composed of the following keys:

```
+---+---+---+---+
| 1 | 2 | 3 | 4 |
+---+---+---+---+
| Q | W | E | R |
+---+---+---+---+
| A | S | D | F |
+---+---+---+---+
| Z | X | C | V |
+---+---+---+---+
```
