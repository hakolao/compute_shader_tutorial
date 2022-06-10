use bevy::{
    math::{const_mat4, Mat4, Vec2},
    prelude::Transform,
};

// c1r1: y flipped for vulkan
#[rustfmt::skip]
pub const OPENGL_TO_VULKAN_MATRIX: Mat4 = const_mat4!([
    1.0, 0.0, 0.0, 0.0,
    0.0, -1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
]);

const Z_POS: f32 = -10.0;

/// An orthographic camera that cannot move :)
#[derive(Debug, Copy, Clone)]
pub struct OrthographicCamera {
    pub pos: Vec2,
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    pub near: f32,
    pub far: f32,
    pub scale: f32,
}

impl OrthographicCamera {
    pub fn update(&mut self, width: f32, height: f32) {
        let half_width = width / 2.0;
        let half_height = height / 2.0;
        self.left = -half_width;
        self.right = half_width;
        self.top = half_height;
        self.bottom = -half_height;
    }

    pub fn world_to_screen(&self) -> Mat4 {
        (OPENGL_TO_VULKAN_MATRIX
            * Mat4::orthographic_rh(
                self.left * self.scale,
                self.right * self.scale,
                self.bottom * self.scale,
                self.top * self.scale,
                self.near,
                self.far,
            ))
            * Transform::from_translation(self.pos.extend(Z_POS)).compute_matrix()
    }
}

impl Default for OrthographicCamera {
    fn default() -> Self {
        OrthographicCamera {
            pos: Vec2::new(0.0, 0.0),
            left: -1.0,
            right: 1.0,
            bottom: -1.0,
            top: 1.0,
            near: 0.0,
            far: 1000.0,
            scale: 1.0,
        }
    }
}
