#version 450

#include "includes.glsl"

vec4 matter_color_to_vec4(uint color) {
    return vec4(float((color >> uint(16)) & uint(255)) / 255.0,
    float((color >> uint(8)) & uint(255)) / 255.0,
    float(color & uint(255)) / 255.0,
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
    Matter matter = read_matter(pos);
    // Our swapchain is in SRGB color space (default by bevy_vulkano). The system tries to interpret our canvas image as such. But our canvas image is
    // UNORM (only way to ImageStore), thus we need to convert the colors to linear space. We are assuming that images
    // Are already in SRGB color space. When we render, the linear gets interpreted as SRGB.
    write_image_color(pos, linear_from_srgba(matter_color_to_vec4(matter.color)));
}

void main() {
    write_color_to_image(get_current_sim_pos());
}