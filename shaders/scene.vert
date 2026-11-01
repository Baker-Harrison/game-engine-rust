#version 450
layout(location = 0) in vec4 pos;
layout(location = 1) in vec4 normal;
layout(location = 2) in vec4 color;
layout(push_constant) uniform U {
    mat4 view_proj;
    vec4 eye;
    vec4 sky;
    vec4 sun;
    vec4 params;
} u;
layout(location = 0) out vec3 world;
layout(location = 1) out vec3 nrm;
layout(location = 2) out vec4 col;
layout(location = 3) out float depth;
layout(location = 4) out float kind;
void main() {
    vec4 p = u.view_proj * vec4(pos.xyz, 1.0);
    if (normal.w > 1.5) {
        p = vec4(pos.x, -pos.y, pos.z, 1.0);
    }
    gl_Position = p;
    world = pos.xyz;
    nrm = normal.xyz;
    col = color;
    depth = p.w;
    kind = normal.w;
}
