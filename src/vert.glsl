#version 100
attribute vec2 pos;
attribute vec2 uv;
uniform mat4 model;
uniform mat4 view;
uniform mat4 projection;
varying lowp vec2 texcoord;
void main() {
    gl_Position = projection * view * model * vec4(pos, 0, 1);
    texcoord = uv;
}
