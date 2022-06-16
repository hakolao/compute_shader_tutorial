#version 450

#include "includes.glsl"

void fall_empty(ivec2 pos) {
    Matter current = read_matter(pos);
    Matter up = get_neighbor(pos, UP);
    Matter down = get_neighbor(pos, DOWN);
    Matter m = current;
    if (!is_at_border_top(pos) && falls_on_empty(up, current)) {
        m = up;
    } else if (!is_at_border_bottom(pos) && falls_on_empty(current, down)) {
        m = down;
    }
    write_matter(pos, m);
}

void main() {
    fall_empty(get_current_sim_pos());
}