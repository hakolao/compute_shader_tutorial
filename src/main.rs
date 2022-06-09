mod camera;
mod quad_pipeline;
mod render;
mod utils;
mod vertex;

use std::sync::Arc;

use bevy::{input::system::exit_on_esc_system, prelude::*, window::WindowMode};
use bevy_vulkano::{
    texture_from_file, VulkanoWindows, VulkanoWinitConfig, VulkanoWinitPlugin, DEFAULT_IMAGE_FORMAT,
};
use vulkano::image::ImageViewAbstract;

use crate::{camera::OrthographicCamera, render::FillScreenRenderPass};

pub const WIDTH: f32 = 1024.0;
pub const HEIGHT: f32 = 1024.0;

fn main() {
    App::new()
        .insert_resource(VulkanoWinitConfig::default())
        .insert_resource(WindowDescriptor {
            width: WIDTH,
            height: HEIGHT,
            title: "Cellular Automata".to_string(),
            present_mode: bevy::window::PresentMode::Immediate,
            resizable: true,
            mode: WindowMode::Windowed,
            ..WindowDescriptor::default()
        })
        // Add needed plugins
        .add_plugin(bevy::core::CorePlugin)
        .add_plugin(bevy::log::LogPlugin)
        .add_plugin(bevy::input::InputPlugin)
        .add_plugin(VulkanoWinitPlugin)
        .add_startup_system(setup)
        .add_system(exit_on_esc_system)
        .add_system(render_system)
        .run();
}

pub struct TreeImage(pub Arc<dyn ImageViewAbstract + Send + Sync + 'static>);

/// Creates our simulation & render pipelines
fn setup(mut commands: Commands, vulkano_windows: Res<VulkanoWindows>) {
    let primary_window_renderer = vulkano_windows.get_primary_window_renderer().unwrap();
    let gfx_queue = primary_window_renderer.graphics_queue();
    // Create our render pass
    let fill_screen = FillScreenRenderPass::new(
        gfx_queue.clone(),
        primary_window_renderer.swapchain_format(),
    );
    // Create a tree image to test pipeline
    let tree_image = texture_from_file(
        gfx_queue,
        include_bytes!("../assets/tree.png"),
        DEFAULT_IMAGE_FORMAT,
    )
    .unwrap();
    // Create simple orthographic camera
    let camera = OrthographicCamera::default();
    // Insert resources
    commands.insert_resource(fill_screen);
    commands.insert_resource(TreeImage(tree_image));
    commands.insert_resource(camera);
}

fn render_system(
    windows: Res<Windows>,
    mut vulkano_windows: ResMut<VulkanoWindows>,
    mut fill_screen: ResMut<FillScreenRenderPass>,
    tree_image: Res<TreeImage>,
    mut camera: ResMut<OrthographicCamera>,
) {
    let primary_window_renderer = vulkano_windows.get_primary_window_renderer_mut().unwrap();
    // Start frame
    let before = match primary_window_renderer.start_frame() {
        Err(e) => {
            bevy::log::error!("Failed to start frame: {}", e);
            return;
        }
        Ok(f) => f,
    };

    let color_image = tree_image.0.clone();
    let final_image = primary_window_renderer.final_image();
    // Update camera
    let window = windows.get_primary().unwrap();
    camera.update(window.width(), window.height());
    // Render
    let after_render =
        fill_screen.render_image_to_screen(before, *camera, color_image, final_image);

    // Finish Frame
    primary_window_renderer.finish_frame(after_render);
}
