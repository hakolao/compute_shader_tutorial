#version 450
layout(location=0) in vec2 position;
layout(location=1) in vec2 tex_coords;
layout(location=2) in vec4 color;

layout(push_constant) uniform PushConstants {
    mat4 world_to_screen;
    vec2 scale;
} push_constants;


layout(location = 0) out vec2 v_tex_coords;
layout(location = 1) out vec4 v_color;

void main() {
    gl_Position =  push_constants.world_to_screen * vec4(position * push_constants.scale, 0.0, 1.0);
    v_tex_coords = tex_coords;
    v_color = color;
}