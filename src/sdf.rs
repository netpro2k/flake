use std::{collections::HashMap, fs::File, io::BufReader, path::Path};

use glam::Mat4;
use image::{EncodableLayout, RgbaImage};
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

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct BMFontJSONGlyphInfo {
    id: u32,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    xoffset: f32,
    yoffset: f32,
    xadvance: f32,
    page: u8,
    chnl: u8,
}

#[derive(Serialize, Deserialize, Debug)]
struct BMFontJSON {
    pages: Vec<String>,
    chars: Vec<BMFontJSONGlyphInfo>,
}

#[derive(Debug)]
enum FontLoadError {
    IO(std::io::Error),
    Parse(serde_json::Error),
    Image(image::ImageError),
}
impl From<std::io::Error> for FontLoadError {
    fn from(error: std::io::Error) -> Self {
        FontLoadError::IO(error)
    }
}
impl From<serde_json::Error> for FontLoadError {
    fn from(error: serde_json::Error) -> Self {
        FontLoadError::Parse(error)
    }
}
impl From<image::ImageError> for FontLoadError {
    fn from(error: image::ImageError) -> Self {
        FontLoadError::Image(error)
    }
}

fn load_font(filename: &str) -> Result<(RgbaImage, HashMap<char, GlyphInfo>), FontLoadError> {
    let reader = BufReader::new(File::open(filename)?);
    let data: BMFontJSON = serde_json::from_reader(reader)?;
    let path = match Path::new(filename).parent() {
        Some(parent) => parent.join(data.pages[0].clone()),
        None => {
            return Result::Err(
                std::io::Error::new(std::io::ErrorKind::NotFound, "filename must be a file").into(),
            )
        }
    };

    let sdf_texture = image::open(path)?;

    let texture_size = Vec2 {
        x: sdf_texture.width() as f32,
        y: sdf_texture.height() as f32,
    };

    let map = data
        .chars
        .iter()
        .map(|info| {
            let size = Vec2 {
                x: info.width,
                y: info.height,
            };
            (
                // TODO nice way to return an error result from this?
                char::from_u32(info.id).expect("Invalid character id"),
                GlyphInfo {
                    size,
                    uv: Vec2 {
                        x: info.x / texture_size.x,
                        y: info.y / texture_size.y,
                    },
                    uv_size: size / texture_size,
                },
            )
        })
        .collect();

    Ok((sdf_texture.into_rgba8(), map))
}

#[rustfmt::skip]
fn make_quad(info: &GlyphInfo) -> [Vertex; 4] {
    [
        Vertex { pos : Vec2::splat(0.0),                uv: Vec2 { x: info.uv.x, y: info.uv.y + info.uv_size.y } },
        Vertex { pos : Vec2 { x:  info.size.x, y: 0. }, uv:  info.uv + info.uv_size  },
        Vertex { pos : info.size,                       uv: Vec2 { x: info.uv.x + info.uv_size.x, y: info.uv.y } },
        Vertex { pos : Vec2 { x: 0., y:  info.size.y},  uv: info.uv },
    ]
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

        let (sdf_texture, glyphs) =
            load_font("./assets/roboto-bold.json").expect("failed to load font");

        let vertices = make_quad(glyphs.get(&'b').unwrap());
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
