#version 450

layout(location=0) in vec4 a_position;
layout(location=1) in vec4 a_color;
layout(location=2) in vec4 a_normal;

layout(location=0) out vec4 v_position;
layout(location=1) out vec4 v_color;
layout(location=2) out vec3 v_normal;

layout(set=0, binding=0) uniform Projection {
    mat4 view_proj;
};

void main() {
    v_position = a_position;
    v_color = a_color;

    // correct the normal so that it's always pointing towards the camera,
    // i.e. the normal under transformation by the view_proj that ends up with
    // smaller z
    vec4 positive_normal = view_proj * a_normal;
    vec4 negative_normal = view_proj * vec4(-a_normal.xyz, a_normal.w);
    if (positive_normal.z < negative_normal.z) {
        v_normal = a_normal.xyz;
    } else {
        v_normal = -a_normal.xyz;
    }

    gl_Position = view_proj * a_position;
}
