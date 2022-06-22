#version 450

#include "includes.glsl"

void main() {
    ivec2 pos = get_current_sim_pos();
    if (pos == push_constants.query_pos) {
        write_query_matter(read_matter(pos));
    }
}