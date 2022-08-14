mod chip8;
mod debugger;
mod sdf;
use chip8::Chip8;
use debugger::{update, Debugger};
use glam::{Mat4, Vec2};
use miniquad::*;
use sdf::SDFText;

#[repr(C)]
struct Vertex {
    pos: Vec2,
    uv: Vec2,
}

pub struct Stage {
    pipeline: Pipeline,
    bindings: Bindings,
    chip: Chip8,
    size: (i32, i32),
    debugger: Debugger,
    text_test: SDFText,
}

impl Stage {
    pub fn new(ctx: &mut Context, filename: &str) -> Stage {
        let mut chip = Chip8::new();
        chip.execution_speed = 1.0;
        // chip.load("roms/test_opcode.ch8")
        //     .expect("Failed to load file");
        chip.load(filename).expect("Failed to load file");

        let text = SDFText::new(ctx);

        #[rustfmt::skip]
        let vertices: [Vertex; 4] = [
            Vertex { pos : Vec2 { x: -1.0, y: 0. }, uv: Vec2 { x: 0., y: 0. } },
            Vertex { pos : Vec2 { x:  1.0, y: 0. }, uv: Vec2 { x: 1., y: 0. } },
            Vertex { pos : Vec2 { x:  1.0, y:  1.0 }, uv: Vec2 { x: 1., y: 1. } },
            Vertex { pos : Vec2 { x: -1.0, y:  1.0}, uv: Vec2 { x: 0., y: 1. } },
        ];
        let vertex_buffer = Buffer::immutable(ctx, BufferType::VertexBuffer, &vertices);

        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
        let index_buffer = Buffer::immutable(ctx, BufferType::IndexBuffer, &indices);

        let pixels: [u8; 64 * 32] = [0; 64 * 32];
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
            text_test: text,
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
        // return;
        if !self.debugger.is_enabled {
            self.chip.step_with_time();
            self.bindings.images[0].update(ctx, &self.chip.display);
            return;
        }
        return update(self, ctx);
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
        self.debugger.key_down_event(keycode);
    }

    fn key_up_event(&mut self, _ctx: &mut Context, keycode: KeyCode, _keymods: KeyMods) {
        if let Some(index) = keycode_to_index(keycode) {
            self.chip.keys[index] = false;
        }
        self.debugger.key_up_event(keycode);
    }

    fn draw(&mut self, ctx: &mut Context) {
        ctx.begin_default_pass(Default::default());

        let (width, height) = self.size;
        ctx.apply_viewport(0, 0, width, height);
        let width = width as f32;
        let height = height as f32;
        let mut left = -1.0;
        let mut right = 1.0;
        let mut bottom = -1.0;
        let top = 1.0; // Keep the quad at the top of the window, no matter what;
        if width > height {
            // Show more of the world (left/right)
            // Keeping things centered
            left *= width / height;
            right *= width / height;
        } else {
            // Show more of the world,
            // but only stretch downwards.
            bottom = top - (top - bottom) * (height / width);
        }

        let proj = Mat4::orthographic_rh_gl(left, right, bottom, top, -1.0, 1.0);
        let view = Mat4::from_translation(glam::Vec3 {
            x: 0.0,
            y: 0.0,
            z: -1.0,
        });
        ctx.apply_pipeline(&self.pipeline);
        ctx.apply_bindings(&self.bindings);
        ctx.apply_uniforms(&shader::Uniforms { proj: view * proj });
        ctx.draw(0, 6, 1);

        self.text_test.draw(ctx, proj);

        ctx.end_render_pass();

        ctx.commit_frame();
    }
}

mod shader {
    use miniquad::*;

    pub const VERTEX: &str = include_str!("vert.glsl");
    pub const FRAGMENT: &str = include_str!("frag.glsl");

    pub fn meta() -> ShaderMeta {
        ShaderMeta {
            images: vec!["tex".to_string()],
            uniforms: UniformBlockLayout {
                uniforms: vec![UniformDesc::new("proj", UniformType::Mat4)],
            },
        }
    }

    #[repr(C)]
    pub struct Uniforms {
        pub proj: glam::Mat4,
    }
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
        move |ctx| {
            if let Some(filename) = args.get(1) {
                Box::new(Stage::new(ctx, filename))
            } else {
                Box::new(Stage::new(ctx, "roms/breakout.ch8"))
            }
        },
    );
}
