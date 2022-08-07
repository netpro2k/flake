use std::{collections::HashMap, process, time::Instant};

use miniquad::*;

#[repr(C)]
struct Vec2 {
    x: f32,
    y: f32,
}
#[repr(C)]
struct Vertex {
    pos: Vec2,
    uv: Vec2,
}

const KEY_TOGGLE_PLAY: KeyCode = KeyCode::P;
const KEY_PLAY_BACKWARD: KeyCode = KeyCode::H;
const KEY_STEP_DEBUG: KeyCode = KeyCode::J;
const KEY_UNDO_STEP_DEBUG: KeyCode = KeyCode::K;
const KEY_GO_FASTER: KeyCode = KeyCode::Equal;
const KEY_GO_SLOWER: KeyCode = KeyCode::Minus;
const KEY_GO_NORMAL: KeyCode = KeyCode::Key0;
const KEY_TERMINATE: KeyCode = KeyCode::Semicolon;

struct Debugger {
    is_enabled: bool,
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

    pub fn consume_key(self: &mut Self, keycode: KeyCode) -> bool {
        let result = *self.consumable_keys.get(&keycode).unwrap_or(&false);
        self.consumable_keys.insert(keycode, false);
        result
    }

    pub fn key_down(self: &mut Self, keycode: KeyCode) -> bool {
        *self.keyboard.get(&keycode).unwrap_or(&false)
    }
}

struct Stage {
    pipeline: Pipeline,
    bindings: Bindings,
    chip: Chip8,
    size: (i32, i32),
    debugger: Debugger,
}

mod chip8;
use chip8::Chip8;

impl Stage {
    pub fn new(ctx: &mut Context, filename: &str) -> Stage {
        let mut chip = Chip8::new();
        chip.execution_speed = 0.20;
        // chip.load("roms/test_opcode.ch8")
        //     .expect("Failed to load file");
        chip.load(filename).expect("Failed to load file");

        #[rustfmt::skip]
        let vertices: [Vertex; 4] = [
            Vertex { pos : Vec2 { x: -1.0, y: -1.0 }, uv: Vec2 { x: 0., y: 0. } },
            Vertex { pos : Vec2 { x:  1.0, y: -1.0 }, uv: Vec2 { x: 1., y: 0. } },
            Vertex { pos : Vec2 { x:  1.0, y:  1.0 }, uv: Vec2 { x: 1., y: 1. } },
            Vertex { pos : Vec2 { x: -1.0, y:  1.0}, uv: Vec2 { x: 0., y: 1. } },
        ];
        let vertex_buffer = Buffer::immutable(ctx, BufferType::VertexBuffer, &vertices);

        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
        let index_buffer = Buffer::immutable(ctx, BufferType::IndexBuffer, &indices);

        let pixels: [u8; 64 * 32] = [255; 64 * 32];
        let texture = Texture::from_data_and_format(
            ctx,
            &pixels,
            TextureParams {
                format: TextureFormat::Alpha,
                wrap: TextureWrap::Clamp,
                filter: FilterMode::Nearest,
                width: 64,
                height: 32,
            },
        );

        let bindings = Bindings {
            index_buffer,
            vertex_buffers: vec![vertex_buffer],
            images: vec![texture],
        };

        let shader = Shader::new(ctx, shader::VERTEX, shader::FRAGMENT, shader::meta()).unwrap();

        let pipeline = Pipeline::new(
            ctx,
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("pos", VertexFormat::Float2),
                VertexAttribute::new("uv", VertexFormat::Float2),
            ],
            shader,
        );

        Stage {
            pipeline,
            bindings,
            chip,
            size: (1200, 600),
            debugger: Debugger::new(),
        }
    }
}

fn keycode_to_index(keycode: KeyCode) -> Option<usize> {
    match keycode {
        KeyCode::Key1 => Some(1),
        KeyCode::Key2 => Some(2),
        KeyCode::Key3 => Some(3),
        KeyCode::Key4 => Some(0xc),
        KeyCode::Q => Some(4),
        KeyCode::W => Some(5),
        KeyCode::E => Some(6),
        KeyCode::R => Some(0xd),
        KeyCode::A => Some(7),
        KeyCode::S => Some(8),
        KeyCode::D => Some(9),
        KeyCode::F => Some(0xe),
        KeyCode::Z => Some(0xa),
        KeyCode::X => Some(0),
        KeyCode::C => Some(0xb),
        KeyCode::V => Some(0xf),
        _ => None,
    }
}

impl EventHandler for Stage {
    fn update(&mut self, ctx: &mut Context) {
        if !self.debugger.is_enabled {
            self.chip.step_with_time();
            self.bindings.images[0].update(ctx, &self.chip.display);
            return;
        }

        // DEBUGGER

        if self.debugger.consume_key(KEY_TERMINATE) {
            process::exit(0);
        }

        if self.debugger.consume_key(KEY_GO_FASTER) {
            self.chip.execution_speed += 0.1;
            println!("Faster! {}", self.chip.execution_speed);
        }

        if self.debugger.consume_key(KEY_GO_SLOWER) {
            self.chip.execution_speed = 0.1;
            println!("Slower! {}", self.chip.execution_speed);
        }

        if self.debugger.consume_key(KEY_GO_NORMAL) {
            self.chip.execution_speed = 1.0;
            println!("Normal! {}", self.chip.execution_speed);
        }

        if self.debugger.consume_key(KEY_TOGGLE_PLAY) {
            self.debugger.is_playing = !self.debugger.is_playing;
            if self.debugger.is_playing {
                // Reset timers so that we don't immediately jump ahead
                self.chip.next_tick = Instant::now();
                self.chip.next_timers_tick = Instant::now();
                // TODO: There is a more correct way to resume,
                //       by getting the duration between the two timers.
            }
        }

        if self.debugger.is_playing {
            self.debugger.states.push(self.chip.clone());
            self.chip.step_with_time(); // Note: We don't close sub-step states here
        } else {
            if self.debugger.consume_key(KEY_STEP_DEBUG) {
                self.debugger.states.push(self.chip.clone());
                println!("{:?}", self.debugger.states.last().unwrap());
                self.chip.step_debug();
            }

            if self.debugger.key_down(KEY_PLAY_BACKWARD) {
                if let Some(prev) = self.debugger.states.pop() {
                    self.chip.clone_from(&prev);
                }
            }

            if self.debugger.consume_key(KEY_UNDO_STEP_DEBUG) {
                if let Some(prev) = self.debugger.states.pop() {
                    self.chip.clone_from(&prev);
                    println!("{:?}", self.chip);
                }
            }
        }

        self.bindings.images[0].update(ctx, &self.chip.display);
    }

    fn resize_event(&mut self, _ctx: &mut Context, width: f32, height: f32) {
        self.size = (width as i32, height as i32);
    }

    fn key_down_event(
        &mut self,
        _ctx: &mut Context,
        keycode: KeyCode,
        _keymods: KeyMods,
        _repeat: bool,
    ) {
        if let Some(index) = keycode_to_index(keycode) {
            self.chip.keys[index] = true;
        }

        self.debugger.keyboard.insert(keycode, true);
        self.debugger.consumable_keys.insert(keycode, true);
    }

    fn key_up_event(&mut self, _ctx: &mut Context, keycode: KeyCode, _keymods: KeyMods) {
        if let Some(index) = keycode_to_index(keycode) {
            self.chip.keys[index] = false;
        }
        self.debugger.keyboard.insert(keycode, false);
        self.debugger.consumable_keys.insert(keycode, false);
    }

    fn draw(&mut self, ctx: &mut Context) {
        ctx.begin_default_pass(Default::default());

        let (width, height) = self.size;
        if width > 2 * height {
            ctx.apply_viewport((width - height * 2) / 2, 0, height * 2, height);
        } else {
            ctx.apply_viewport(0, (height - width / 2) / 2, width, width / 2);
        }
        ctx.apply_pipeline(&self.pipeline);
        ctx.apply_bindings(&self.bindings);
        ctx.draw(0, 6, 1);
        ctx.end_render_pass();

        ctx.commit_frame();
    }
}

mod shader {
    use miniquad::*;

    pub const VERTEX: &str = r#"#version 100
    attribute vec2 pos;
    attribute vec2 uv;
    uniform vec2 offset;
    varying lowp vec2 texcoord;
    void main() {
        gl_Position = vec4(pos + offset, 0, 1);
        texcoord = uv;
    }"#;

    pub const FRAGMENT: &str = r#"#version 100
    precision lowp float;
    varying lowp vec2 texcoord;
    uniform sampler2D tex;
    void main() {
        float c = texture2D(tex, vec2(texcoord.x, 1.0 - texcoord.y)).r;
        gl_FragColor = vec4(c, c, 0.5, 1.0);
    }"#;

    pub fn meta() -> ShaderMeta {
        ShaderMeta {
            images: vec!["tex".to_string()],
            uniforms: UniformBlockLayout {
                uniforms: vec![UniformDesc::new("offset", UniformType::Float2)],
            },
        }
    }

    // #[repr(C)]
    // pub struct Uniforms {
    //     pub offset: (f32, f32),
    // }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    miniquad::start(
        conf::Conf {
            window_title: "Flake".to_string(),
            window_width: 1200,
            window_height: 600,
            ..Default::default()
        },
        move |mut ctx| {
            if let Some(filename) = args.get(1) {
                Box::new(Stage::new(&mut ctx, filename))
            } else {
                Box::new(Stage::new(&mut ctx, "roms/breakout.ch8"))
            }
        },
    );
}
