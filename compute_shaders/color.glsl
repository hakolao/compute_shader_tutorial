#version 450

#include "includes.glsl"

void write_color_to_image(ivec2 pos) {
    Matter matter = read_matter(pos);
    write_image_color(pos, matter_color_to_vec4(matter.color));
}

void main() {
    write_color_to_image(get_current_sim_pos());
}