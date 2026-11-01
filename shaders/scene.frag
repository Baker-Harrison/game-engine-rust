#version 450
layout(push_constant) uniform U {
    mat4 view_proj;
    vec4 eye;
    vec4 sky;
    vec4 sun;
    vec4 params;
} u;
layout(set = 0, binding = 0) uniform sampler2DArray albedo_tex;
layout(set = 0, binding = 1) uniform sampler2DArray normal_tex;
layout(set = 0, binding = 2) uniform sampler2DArray packed_tex;
layout(location = 0) in vec3 world;
layout(location = 1) in vec3 nrm;
layout(location = 2) in vec4 col;
layout(location = 3) in float depth;
layout(location = 4) in float kind;
layout(location = 5) in vec2 uv;
layout(location = 6) flat in float layer;
layout(location = 7) flat in float mode;
layout(location = 0) out vec4 out_color;
void main() {
    if (kind > 1.5) {
        out_color = col;
        return;
    }
    vec3 n = normalize(nrm);
    vec3 to_eye = normalize(u.eye.xyz - world);
    if (dot(n, to_eye) < 0.0) n = -n;
    vec3 base = col.xyz;
    float roughness = 0.85;
    float metallic = 0.0;
    float occlusion = 1.0;
    float alpha = 1.0;
    bool textured = layer > 0.5 && kind < 0.5;
    if (textured) {
        float slice = layer - 1.0;
        vec3 dp1 = dFdx(world);
        vec3 dp2 = dFdy(world);
        vec2 duv1 = dFdx(uv);
        vec2 duv2 = dFdy(uv);
        vec3 dp2perp = cross(dp2, n);
        vec3 dp1perp = cross(n, dp1);
        vec3 tangent = dp2perp * duv1.x + dp1perp * duv2.x;
        vec3 bitangent = dp2perp * duv1.y + dp1perp * duv2.y;
        float inverse_scale = inversesqrt(max(max(dot(tangent, tangent), dot(bitangent, bitangent)), 1e-20));
        vec3 texel_normal = texture(normal_tex, vec3(uv, slice)).xyz * 2.0 - 1.0;
        vec3 perturbed = tangent * inverse_scale * texel_normal.x + bitangent * inverse_scale * texel_normal.y + n * texel_normal.z;
        if (dot(perturbed, perturbed) > 1e-8) n = normalize(perturbed);
        vec4 albedo = texture(albedo_tex, vec3(uv, slice));
        base = albedo.xyz;
        if (mode > 0.5 && mode < 1.5 && albedo.a < 0.5) discard;
        if (mode > 1.5) alpha = albedo.a;
        vec3 surface = texture(packed_tex, vec3(uv, slice)).xyz;
        roughness = max(surface.x, 0.04);
        metallic = surface.y;
        occlusion = surface.z;
    }
    float ambient = u.sun.w;
    float facing = max(dot(n, u.sun.xyz), 0.0);
    float shade = ambient * occlusion + (1.0 - ambient) * facing;
    if (kind > 0.5) shade = 1.0;
    vec3 lit = base * shade;
    if (textured) {
        vec3 half_vector = normalize(u.sun.xyz + to_eye);
        float r4 = max(roughness * roughness * roughness * roughness, 0.0004);
        float sharpness = min(2.0 / r4 - 2.0, 2000.0);
        float highlight = pow(max(dot(n, half_vector), 0.0), sharpness);
        float strength = highlight * facing * (1.0 - roughness) * 0.6;
        vec3 tint = mix(vec3(1.0), base, metallic);
        lit = base * (1.0 - metallic * 0.8) * shade + tint * strength;
    }
    float t = clamp(depth / u.params.x, 0.0, 1.0);
    t = t * t;
    out_color = vec4(mix(lit, u.sky.xyz, t), alpha);
}
