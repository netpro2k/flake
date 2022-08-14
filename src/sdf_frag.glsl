#version 100
precision lowp float;
varying lowp vec2 texcoord;
uniform sampler2D tex;


float cubicPulse( float c, float w, float x )
{
    if( x>c+w ) return 0.0;
    x = abs(x - c);
    if( x>w ) return 0.0;
    x /= w;
    return 1.0 - x*x*(3.0-2.0*x);
}

void main() {
    float d = texture2D(tex, vec2(texcoord.x, texcoord.y)).a;
    float c = smoothstep(0.4,0.6, d);
    vec3 ch = vec3(c,c,c);
    float outline = cubicPulse(0.5, 0.1, d);
    gl_FragColor.rgb = mix(ch, vec3(1.,0.,0.), outline);
}
