use crate::{Chip8, Stage};
use miniquad::Context;
use miniquad::KeyCode;
use std::{collections::HashMap, process, time::Instant};

pub const KEY_TOGGLE_PLAY: KeyCode = KeyCode::P;
pub const KEY_PLAY_BACKWARD: KeyCode = KeyCode::H;
pub const KEY_STEP_DEBUG: KeyCode = KeyCode::J;
pub const KEY_UNDO_STEP_DEBUG: KeyCode = KeyCode::K;
pub const KEY_GO_FASTER: KeyCode = KeyCode::Equal;
pub const KEY_GO_SLOWER: KeyCode = KeyCode::Minus;
pub const KEY_GO_NORMAL: KeyCode = KeyCode::Key0;
pub const KEY_TERMINATE: KeyCode = KeyCode::Semicolon;

pub struct Debugger {
    pub is_enabled: bool,
    is_playing: bool,
    keyboard: HashMap<KeyCode, bool>,
    consumable_keys: HashMap<KeyCode, bool>,
    states: Vec<Chip8>,
}

impl Debugger {
    pub fn new() -> Debugger {
        Debugger {
            is_enabled: true,
            is_playing: false,
            keyboard: HashMap::new(),
            consumable_keys: HashMap::new(),
            states: vec![],
        }
    }
    pub fn consume_key(&mut self, keycode: KeyCode) -> bool {
        let result = *self.consumable_keys.get(&keycode).unwrap_or(&false);
        self.consumable_keys.insert(keycode, false);
        result
    }
    pub fn is_key_down(&mut self, keycode: KeyCode) -> bool {
        *self.keyboard.get(&keycode).unwrap_or(&false)
    }
    pub fn key_down_event(&mut self, keycode: KeyCode) {
        self.keyboard.insert(keycode, true);
        self.consumable_keys.insert(keycode, true);
    }
    pub fn key_up_event(&mut self, keycode: KeyCode) {
        self.keyboard.insert(keycode, false);
        self.consumable_keys.insert(keycode, false);
    }
}

pub fn update(stage: &mut Stage, ctx: &mut Context) {
    if !stage.debugger.is_enabled {
        stage.chip.step_with_time();
        stage.bindings.images[0].update(ctx, &stage.chip.display);
        return;
    }
    if stage.debugger.consume_key(KEY_TERMINATE) {
        process::exit(0);
    }
    if stage.debugger.consume_key(KEY_GO_FASTER) {
        stage.chip.execution_speed += 0.1;
        println!("Faster! {}", stage.chip.execution_speed);
    }
    if stage.debugger.consume_key(KEY_GO_SLOWER) {
        stage.chip.execution_speed = 0.1;
        println!("Slower! {}", stage.chip.execution_speed);
    }
    if stage.debugger.consume_key(KEY_GO_NORMAL) {
        stage.chip.execution_speed = 1.0;
        println!("Normal! {}", stage.chip.execution_speed);
    }
    if stage.debugger.consume_key(KEY_TOGGLE_PLAY) {
        stage.debugger.is_playing = !stage.debugger.is_playing;
        if stage.debugger.is_playing {
            // Reset timers so that we don't immediately jump ahead
            stage.chip.next_tick = Instant::now();
            stage.chip.next_timers_tick = Instant::now();
            // TODO: There is a more correct way to resume,
            //       by getting the duration between the two timers.
        }
    }
    if stage.debugger.is_playing {
        stage.debugger.states.push(stage.chip.clone());
        stage.chip.step_with_time(); // Note: We don't close sub-step states here
    } else {
        if stage.debugger.consume_key(KEY_STEP_DEBUG) {
            stage.debugger.states.push(stage.chip.clone());
            println!("{:?}", stage.debugger.states.last().unwrap());
            stage.chip.step_debug();
            println!(
                "
----------------------------------------------------------
Changes:
{}
----------------------------------------------------------",
                Chip8::compare(stage.debugger.states.last().unwrap(), &stage.chip)
            );
        }
        if stage.debugger.is_key_down(KEY_PLAY_BACKWARD) {
            if let Some(prev) = stage.debugger.states.pop() {
                stage.chip.clone_from(&prev);
            }
        }
        if stage.debugger.consume_key(KEY_UNDO_STEP_DEBUG) {
            if let Some(prev) = stage.debugger.states.pop() {
                stage.chip.clone_from(&prev);
                println!("{:?}", stage.chip);
            }
        }
    }
    stage.bindings.images[0].update(ctx, &stage.chip.display);
}
