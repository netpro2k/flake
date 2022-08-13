#version 100
precision lowp float;
varying lowp vec2 texcoord;
uniform sampler2D tex;
void main() {
    float c = texture2D(tex, vec2(texcoord.x, 1.0 - texcoord.y)).r;
    gl_FragColor = vec4(c, c, 0.5, 1.0);
}
