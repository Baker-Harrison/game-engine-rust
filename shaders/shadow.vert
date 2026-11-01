#version 450
layout(location = 0) in vec4 pos;
layout(push_constant) uniform L {
    mat4 light_view_proj;
} l;
void main() {
    gl_Position = l.light_view_proj * vec4(pos.xyz, 1.0);
}
