#version 450

layout(location=0) in vec4 a_position;

layout(set=0, binding=0) uniform Light {
    mat4 light_proj;
    vec4 light_pos;
    vec4 light_color;
};

void main() {
    gl_Position = light_proj * a_position;
}
