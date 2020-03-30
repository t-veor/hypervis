#version 450

layout(location=0) in vec4 a_position;
layout(location=1) in vec4 a_color;

layout(location=0) out vec4 v_color;

layout(set=0, binding=0) uniform Projection3 {
    mat4 view_proj3;
};

layout(set=0, binding=1) uniform Projection4 {
    mat4 view_proj4;
    vec4 translation;
};

layout(set=1, binding=0) uniform Transform {
    vec4 displacement;
    mat4 transform_matrix;
};

void main() {
    v_color = a_color;
    vec4 position4 = displacement + transform_matrix * a_position;
    vec4 position3 = view_proj4 * position4 + translation;
    gl_Position = view_proj3 * position3;
}
