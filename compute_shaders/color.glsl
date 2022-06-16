#version 450

#include "includes.glsl"

vec4 matter_color_to_vec4(uint color) {
    return  vec4(float((color >> uint(16)) & uint(255)) / 255.0,
                float((color >> uint(8)) & uint(255)) / 255.0,
                float(color & uint(255)) / 255.0,
                1.0);
}

void write_color_to_image(ivec2 pos) {
    Matter matter = read_matter(pos);
    write_image_color(pos, matter_color_to_vec4(matter.color));
}

void main() {
    write_color_to_image(get_current_sim_pos());
}