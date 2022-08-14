#version 100
attribute vec2 pos;
attribute vec2 uv;
uniform mat4 projection;
uniform mat4 model;
varying lowp vec2 texcoord;
void main() {
    gl_Position = projection * model * vec4(pos, 0, 1);
    texcoord = uv;
}
