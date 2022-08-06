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

struct Stage {
    pipeline: Pipeline,
    bindings: Bindings,
    chip: Chip8,
    size: (i32, i32),
}

mod chip8;
use chip8::Chip8;

impl Stage {
    pub fn new(ctx: &mut Context) -> Stage {
        let mut chip = Chip8::new();
        chip.load("roms/test_opcode.ch8")
            .expect("Failed to load file");

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
        }
    }
}

impl EventHandler for Stage {
    fn update(&mut self, ctx: &mut Context) {
        self.chip.tick();
        self.bindings.images[0].update(ctx, &self.chip.display)
    }

    fn resize_event(&mut self, _ctx: &mut Context, width: f32, height: f32) {
        self.size = (width as i32, height as i32);
    }

    fn draw(&mut self, ctx: &mut Context) {
        // let t = date::now();

        ctx.begin_default_pass(Default::default());

        let (width, height) = self.size;
        if width > 2 * height {
            ctx.apply_viewport((width - height * 2) / 2, 0, height * 2, height);
        } else {
            ctx.apply_viewport(0, (height - width / 2) / 2, width, width / 2);
        }
        // ctx.apply_viewport(0, 0, width, height);
        ctx.apply_pipeline(&self.pipeline);
        ctx.apply_bindings(&self.bindings);
        // for i in 0..10 {
        //     let t = t + i as f64 * 0.3;

        //     ctx.apply_uniforms(&shader::Uniforms {
        //         offset: (t.sin() as f32 * 0.5, (t * 3.).cos() as f32 * 0.5),
        //     });
        ctx.draw(0, 6, 1);
        // }
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

    #[repr(C)]
    pub struct Uniforms {
        pub offset: (f32, f32),
    }
}

fn main() {
    miniquad::start(
        conf::Conf {
            window_title: "Flake".to_string(),
            window_width: 1200,
            window_height: 600,
            ..Default::default()
        },
        |mut ctx| Box::new(Stage::new(&mut ctx)),
    );
}
