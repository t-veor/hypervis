#version 450

layout(location=0) in vec4 v_position;
layout(location=1) in vec4 v_color;
layout(location=2) in vec3 v_normal;

layout(location=0) out vec4 f_color;

layout(set=0, binding=1) uniform Light {
    mat4 light_proj;
    vec4 light_pos;
    vec4 light_color;
};
layout(set=0, binding=2) uniform texture2D t_shadow;
layout(set=0, binding=3) uniform samplerShadow s_shadow;

float fetch_shadow(vec4 pos) {
    if (pos.w <= 0.0) {
        return 1.0;
    }
    vec3 light_local = vec3(
        pos.xy / pos.w * 0.5 + 0.5,
        pos.z / pos.w - 0.0001
    );
    return texture(sampler2DShadow(t_shadow, s_shadow), light_local);
}

void main() {
    vec3 ambient = vec3(0.2, 0.2, 0.2);

    float shadow = fetch_shadow(light_proj * v_position);
    vec3 color = ambient + shadow * light_color.xyz * (1 - ambient);

    f_color = vec4(color, 1.0) * v_color;
}
