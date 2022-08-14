use glam::Mat4;
use miniquad::*;

pub struct SDFText {
    pipeline: Pipeline,
    bindings: Bindings,
}

use glam::Vec2;

#[repr(C)]
struct Vertex {
    pos: Vec2,
    uv: Vec2,
}

struct GlyphInfo {
    size: Vec2,
    uv: Vec2,
    uv_size: Vec2,
}

fn make_quad(info: GlyphInfo) {
    #[rustfmt::skip]
    let vertices: [Vertex; 4] = [
        Vertex { pos : Vec2::splat(0.0),                uv: Vec2 { x: info.uv.x, y: info.uv.y + info.uv_size.y } },
        Vertex { pos : Vec2 { x:  info.size.x, y: 0. }, uv:  info.uv + info.uv_size  },
        Vertex { pos : info.size,                       uv: Vec2 { x: info.uv.x + info.uv_size.x, y: info.uv.y } },
        Vertex { pos : Vec2 { x: 0., y:  info.size.y},  uv: info.uv },
    ];
}

impl SDFText {
    pub fn new(ctx: &mut Context) -> Self {
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

        let sdf_texture = image::open("./assets/roboto-bold.png").unwrap();

        let char_pos = Vec2 { x: 212.0, y: 216.0 };
        let char_size = Vec2 { x: 59.0, y: 62.0 };
        let texture_size = Vec2 {
            x: sdf_texture.width() as f32,
            y: sdf_texture.height() as f32,
        };

        let uv_pos = char_pos / texture_size;
        let uv_size = char_size / texture_size;
        dbg!(uv_size);

        #[rustfmt::skip]
        let vertices: [Vertex; 4] = [
            Vertex { pos : Vec2::splat(0.0), uv: Vec2 { x: uv_pos.x, y: uv_pos.y + uv_size.y } },
            Vertex { pos : Vec2 { x:  char_size.x, y: 0. }, uv:  uv_pos + uv_size  },
            Vertex { pos : char_size, uv: Vec2 { x: uv_pos.x + uv_size.x, y: uv_pos.y } },
            Vertex { pos : Vec2 { x: 0., y:  char_size.y}, uv: uv_pos },
        ];
        let vertex_buffer = Buffer::immutable(ctx, BufferType::VertexBuffer, &vertices);

        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
        let index_buffer = Buffer::immutable(ctx, BufferType::IndexBuffer, &indices);

        let texture = Texture::from_data_and_format(
            ctx,
            sdf_texture.as_bytes(),
            TextureParams {
                format: TextureFormat::RGBA8,
                wrap: TextureWrap::Clamp,
                filter: FilterMode::Linear,
                width: sdf_texture.width(),
                height: sdf_texture.height(),
            },
        );

        let bindings = Bindings {
            index_buffer,
            vertex_buffers: vec![vertex_buffer],
            images: vec![texture],
        };

        SDFText { pipeline, bindings }
    }

    pub fn draw(&self, ctx: &mut Context, proj: Mat4) {
        ctx.apply_pipeline(&self.pipeline);
        ctx.apply_bindings(&self.bindings);
        ctx.apply_uniforms(&shader::Uniforms { proj });
        ctx.draw(0, 6, 1);
    }
}

mod shader {
    use miniquad::*;

    pub const VERTEX: &str = include_str!("sdf_vert.glsl");
    pub const FRAGMENT: &str = include_str!("sdf_frag.glsl");

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
