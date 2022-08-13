#version 100
attribute vec2 pos;
attribute vec2 uv;
uniform mat4 proj;
varying lowp vec2 texcoord;
void main() {
    gl_Position = proj * vec4(pos, 0, 1);
    texcoord = uv;
}
