#version 450

/*
Specialization constants
*/
layout(constant_id = 0) const int canvas_size_x = 1;
layout(constant_id = 1) const int canvas_size_y = 1;
layout(constant_id = 2) const uint empty_matter = 1;
layout(local_size_x_id = 3, local_size_y_id = 4, local_size_z = 1) in;

/*
Buffers
*/
layout(set = 0, binding = 0) restrict buffer MatterInBuffer { uint matter_in[]; };
layout(set = 0, binding = 1) restrict writeonly buffer MatterOutBuffer { uint matter_out[]; };
layout(set = 0, binding = 2, rgba8) restrict uniform writeonly image2D canvas_img;

/*
Utility functions to be used in the various kernels:
*/

ivec2 get_current_sim_pos() {
    return ivec2(gl_GlobalInvocationID.xy);
}

int get_index(ivec2 pos) {
    return pos.y * canvas_size_x + pos.x;
}

bool is_inside_sim_canvas(ivec2 pos) {
    return pos.x >= 0 && pos.x < canvas_size_x &&
    pos.y >= 0 && pos.y < canvas_size_y;
}

uint read_matter(ivec2 pos) {
    return matter_in[get_index(pos)];
}

void write_matter(ivec2 pos, uint matter) {
    matter_out[get_index(pos)] = matter;
}

void write_image_color(ivec2 pos, vec4 color) {
    imageStore(canvas_img, pos, color);
}

vec4 matter_color_to_vec4(uint color) {
    return  vec4(float((color >> uint(24)) & uint(255)) / 255.0,
                float((color >> uint(16)) & uint(255)) / 255.0,
                float((color >> uint(8)) & uint(255)) / 255.0,
                1.0);
}

// 0-1 linear  from  0-255 sRGB
vec3 linear_from_srgb(vec3 srgb) {
    bvec3 cutoff = lessThan(srgb, vec3(10.31475));
    vec3 lower = srgb / vec3(3294.6);
    vec3 higher = pow((srgb + vec3(14.025)) / vec3(269.025), vec3(2.4));
    return mix(higher, lower, cutoff);
}

vec4 linear_from_srgba(vec4 srgba) {
    return vec4(linear_from_srgb(srgba.rgb * 255.0), srgba.a);
}

void write_color_to_image(ivec2 pos) {
    uint matter = read_matter(pos);
    // Our swapchain is in SRGB color space (default by bevy_vulkano). The system tries to interpret our canvas image as such. But our canvas image is
    // UNORM (only way to ImageStore), thus we need to convert the colors to linear space. We are assuming that images
    // Are already in SRGB color space. When we render, the linear gets interpreted as SRGB.
    write_image_color(pos, linear_from_srgba(matter_color_to_vec4(matter)));
}

void main() {
    write_color_to_image(get_current_sim_pos());
}
