#version 450

#include "includes.glsl"

void draw_matter(ivec2 pos, ivec2 draw_pos, float radius, Matter matter) {
    int y_start = draw_pos.y - int(radius);
    int y_end = draw_pos.y + int(radius);
    int x_start = draw_pos.x - int(radius);
    int x_end = draw_pos.x + int(radius);
    if (pos.x >= x_start && pos.x <= x_end && pos.y >= y_start && pos.y <= y_end) {
        vec2 diff = vec2(pos) - vec2(draw_pos);
        float dist = length(diff);
        if (round(dist) <= radius) {
            // We write to matter input
            write_matter_input(pos, matter);
        }
    }
}

void main() {
    draw_matter(
        get_current_sim_pos(),
        push_constants.draw_pos,
        push_constants.draw_radius,
        new_matter(push_constants.draw_matter)
    );
}