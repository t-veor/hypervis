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

vec2 poisson_disk[4] = {
  { -0.94201624, -0.39906216},
  {  0.94558609, -0.76890725},
  {-0.094184101, -0.92938870},
  {  0.34495938,  0.29387760},
};

float sigmoid(float x) {
    if (x >= 1.0) return 1.0;
    else if (x <= -1.0) return 0.0;
    else return 0.5 + x * (1.0 - abs(x) * 0.5);
}

float fetch_shadow(vec4 pos, float theta) {
    if (pos.w <= 0.0) {
        return 1.0;
    }
    float bias = 0.0005 * tan(theta);
    bias = clamp(bias, 0, 0.001);

    vec3 light_local = vec3(
        pos.xy / pos.w * 0.5 + 0.5,
        pos.z / pos.w - bias
    );

    float result = 0.0;
    for (int i = 0; i < 4; i++) {
        vec3 modified_local = vec3(
            light_local.xy + poisson_disk[i] / 700.0,
            light_local.z
        );
        result += texture(sampler2DShadow(t_shadow, s_shadow), modified_local);
    }
    return sigmoid(result / 2.0 - 1.0);
}

void main() {
    vec3 ambient = vec3(0.2, 0.2, 0.2);

    vec3 light_dir = normalize(light_pos.xyz - v_position.xyz);
    float theta = acos(dot(v_normal, light_dir));
    float diffuse = max(0.0, dot(v_normal, light_dir));
    float shadow = fetch_shadow(light_proj * v_position, theta);
    vec3 color = ambient + shadow * diffuse * light_color.xyz * (1 - ambient);

    f_color = vec4(color, 1.0) * v_color;
}
