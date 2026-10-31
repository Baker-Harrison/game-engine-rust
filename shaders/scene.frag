#version 450
layout(push_constant) uniform U {
    mat4 view_proj;
    vec4 eye;
    vec4 sky;
    vec4 sun;
    vec4 params;
} u;
layout(location = 0) in vec3 world;
layout(location = 1) in vec3 nrm;
layout(location = 2) in vec3 col;
layout(location = 3) in float depth;
layout(location = 0) out vec4 out_color;
void main() {
    vec3 n = normalize(nrm);
    if (dot(n, u.eye.xyz - world) < 0.0) n = -n;
    float shade = u.sun.w + (1.0 - u.sun.w) * max(dot(n, u.sun.xyz), 0.0);
    float t = clamp(depth / u.params.x, 0.0, 1.0);
    t = t * t;
    out_color = vec4(mix(col * shade, u.sky.xyz, t), 1.0);
}
