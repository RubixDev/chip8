use std::{sync::mpsc::Sender, thread};

use evdev::{InputEvent, Key, InputEventKind};

const REQUIRED_KEYS: &[Key] = &[
    Key::KEY_1,
    Key::KEY_2,
    Key::KEY_3,
    Key::KEY_4,
    Key::KEY_Q,
    Key::KEY_W,
    Key::KEY_E,
    Key::KEY_R,
    Key::KEY_A,
    Key::KEY_S,
    Key::KEY_D,
    Key::KEY_F,
    Key::KEY_Z,
    Key::KEY_X,
    Key::KEY_C,
    Key::KEY_V,
];

pub fn start_listeners(channel: Sender<InputEvent>) {
    // search for devices with support for all needed keys
    let devices = evdev::enumerate()
        .map(|d| d.1)
        .filter(|dev| {
            dev.supported_keys().map_or(false, |keys| {
                REQUIRED_KEYS.iter().all(|key| keys.contains(*key))
            })
        })
        .collect::<Vec<_>>();

    // spawn one thread for each device
    for mut device in devices {
        let channel = channel.clone();
        thread::spawn(move || loop {
            for event in device.fetch_events().unwrap() {
                channel.send(event).unwrap();
            }
        });
    }
}

pub fn key_idx(event: &InputEvent) -> Option<usize> {
    match event.kind() {
        InputEventKind::Key(Key::KEY_1) => Some(0x1),
        InputEventKind::Key(Key::KEY_2) => Some(0x2),
        InputEventKind::Key(Key::KEY_3) => Some(0x3),
        InputEventKind::Key(Key::KEY_4) => Some(0xc),

        InputEventKind::Key(Key::KEY_Q) => Some(0x4),
        InputEventKind::Key(Key::KEY_W) => Some(0x5),
        InputEventKind::Key(Key::KEY_E) => Some(0x6),
        InputEventKind::Key(Key::KEY_R) => Some(0xd),

        InputEventKind::Key(Key::KEY_A) => Some(0x7),
        InputEventKind::Key(Key::KEY_S) => Some(0x8),
        InputEventKind::Key(Key::KEY_D) => Some(0x9),
        InputEventKind::Key(Key::KEY_F) => Some(0xe),

        InputEventKind::Key(Key::KEY_Z) => Some(0xa),
        InputEventKind::Key(Key::KEY_X) => Some(0x0),
        InputEventKind::Key(Key::KEY_C) => Some(0xb),
        InputEventKind::Key(Key::KEY_V) => Some(0xf),

        _ => None,
    }
}
