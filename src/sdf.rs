use std::{collections::HashMap, fs::File, io::BufReader, path::Path, rc::Rc};

use glam::{Mat4, Quat, Vec3};
use image::{EncodableLayout, RgbaImage};
use miniquad::*;

pub struct SDFText {
    bindings: Bindings,
    font: Rc<SDFFont>,
    pub model: Mat4,
}

pub struct SDFFont {
    pipeline: Pipeline,
    glyphs: HashMap<char, GlyphInfo>,
    texture: Texture,
}

use glam::Vec2;

#[repr(C)]
#[derive(Default, Copy, Clone)]
struct Vertex {
    pos: Vec2,
    uv: Vec2,
}

struct GlyphInfo {
    size: Vec2,
    uv: Vec2,
    uv_size: Vec2,
    x_advance: f32,
    offset: Vec2,
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
#[serde(rename_all = "camelCase")]
struct BMFontJSONCommon {
    line_height: f32,
    base: f32,
}

#[derive(Serialize, Deserialize, Debug)]
struct BMFontJSON {
    pages: Vec<String>,
    chars: Vec<BMFontJSONGlyphInfo>,
    common: BMFontJSONCommon,
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
                    x_advance: info.xadvance,
                    offset: Vec2 {
                        x: info.xoffset,
                        y: data.common.base - (size.y + info.yoffset),
                    },
                },
            )
        })
        .collect();

    Ok((sdf_texture.into_rgba8(), map))
}

#[rustfmt::skip]
fn make_quad(info: &GlyphInfo, buf :&mut [Vertex], offset : Vec2) {
    buf[0] = Vertex { pos : Vec2::splat(0.0) + offset,                uv: Vec2 { x: info.uv.x, y: info.uv.y + info.uv_size.y } };
    buf[1] = Vertex { pos : Vec2 { x:  info.size.x, y: 0. } + offset, uv:  info.uv + info.uv_size  };
    buf[2] = Vertex { pos : info.size + offset,                       uv: Vec2 { x: info.uv.x + info.uv_size.x, y: info.uv.y } };
    buf[3] = Vertex { pos : Vec2 { x: 0., y:  info.size.y} + offset,  uv: info.uv };
}

fn make_mesh(glyphs: &HashMap<char, GlyphInfo>, text: &str) -> (Vec<Vertex>, Vec<u16>) {
    let num_chars = text.chars().count();
    let mut vertices = vec![Default::default(); num_chars * 4];

    let mut x_offset = 0.0;
    text.chars().enumerate().for_each(|(i, c)| {
        let info = glyphs.get(&c).unwrap();
        make_quad(
            info,
            &mut vertices[i * 4..i * 4 + 4],
            Vec2 { x: x_offset, y: 0. } + info.offset,
        );
        x_offset += info.x_advance;
    });

    let mut indices = vec![0; num_chars * 6];

    indices.chunks_exact_mut(6).enumerate().for_each(|(i, v)| {
        let o: u16 = 4 * i as u16;
        v.copy_from_slice(&[0, 1, 2, 0, 2, 3].map(|n| n + o));
    });

    (vertices, indices)
}

impl SDFFont {
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

        SDFFont {
            pipeline,
            glyphs,
            texture,
        }
    }
}

impl SDFText {
    pub fn new(ctx: &mut GraphicsContext, font: Rc<SDFFont>, text: &str) -> SDFText {
        let (vertices, indices) = make_mesh(&font.glyphs, text);

        let bindings = Bindings {
            index_buffer: Buffer::immutable(ctx, BufferType::IndexBuffer, &indices),
            vertex_buffers: vec![Buffer::immutable(ctx, BufferType::VertexBuffer, &vertices)],
            images: vec![font.texture],
        };

        let model = Mat4::from_scale_rotation_translation(
            Vec3::splat(1.0),
            Quat::from_rotation_z(0.0),
            Vec3::new(50.0, 50.0, 0.0),
        );

        SDFText {
            bindings,
            font,
            model,
        }
    }
    pub fn update_text(&mut self, ctx: &mut Context, text: String) {
        let (vertices, indices) = make_mesh(&self.font.glyphs, &text);
        self.bindings.index_buffer = Buffer::immutable(ctx, BufferType::IndexBuffer, &indices);
        self.bindings.vertex_buffers =
            vec![Buffer::immutable(ctx, BufferType::VertexBuffer, &vertices)];
    }

    pub fn draw(&self, ctx: &mut Context, projection: Mat4, view: Mat4) {
        ctx.apply_pipeline(&self.font.pipeline);
        ctx.apply_bindings(&self.bindings);
        ctx.apply_uniforms(&shader::Uniforms {
            model: self.model,
            view,
            projection,
        });
        ctx.draw(0, self.bindings.index_buffer.size() as i32, 1);
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
                uniforms: vec![
                    UniformDesc::new("model", UniformType::Mat4),
                    UniformDesc::new("view", UniformType::Mat4),
                    UniformDesc::new("projection", UniformType::Mat4),
                ],
            },
        }
    }

    #[repr(C)]
    pub struct Uniforms {
        pub model: glam::Mat4,
        pub view: glam::Mat4,
        pub projection: glam::Mat4,
    }
}
