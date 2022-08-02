mod ca_simulator;
mod camera;
mod gui;
mod quad_pipeline;
mod render;
mod utils;
mod vertex;

use bevy::{
    input::mouse::MouseWheel,
    prelude::*,
    window::{close_on_esc, WindowMode},
};
use bevy_vulkano::{BevyVulkanoWindows, VulkanoWinitConfig, VulkanoWinitPlugin};

use crate::{
    ca_simulator::CASimulator,
    camera::OrthographicCamera,
    gui::user_interface,
    render::FillScreenRenderPass,
    utils::{cursor_to_world, get_canvas_line, MousePos},
};

pub const WIDTH: f32 = 1920.0;
pub const HEIGHT: f32 = 1080.0;
pub const CANVAS_SIZE_X: u32 = 512;
pub const CANVAS_SIZE_Y: u32 = 512;
pub const LOCAL_SIZE_X: u32 = 32;
pub const LOCAL_SIZE_Y: u32 = 32;
pub const NUM_WORK_GROUPS_X: u32 = CANVAS_SIZE_X / LOCAL_SIZE_X;
pub const NUM_WORK_GROUPS_Y: u32 = CANVAS_SIZE_Y / LOCAL_SIZE_Y;
pub const CLEAR_COLOR: [f32; 4] = [1.0; 4];
pub const CAMERA_MOVE_SPEED: f32 = 200.0;

pub struct DynamicSettings {
    pub brush_radius: f32,
    pub draw_matter: u32,
}

impl Default for DynamicSettings {
    fn default() -> Self {
        Self {
            brush_radius: 4.0,
            draw_matter: 0xff0000ff,
        }
    }
}

fn main() {
    App::new()
        .insert_non_send_resource(VulkanoWinitConfig::default())
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
        .add_plugin(bevy::time::TimePlugin)
        .add_plugin(bevy::diagnostic::LogDiagnosticsPlugin::default())
        .add_plugin(bevy::diagnostic::DiagnosticsPlugin)
        .add_plugin(bevy::diagnostic::FrameTimeDiagnosticsPlugin)
        .add_plugin(bevy::input::InputPlugin)
        .add_plugin(VulkanoWinitPlugin)
        .add_startup_system(setup)
        .add_system(close_on_esc)
        .add_system(input_actions)
        .add_system(update_camera)
        .add_system(update_mouse)
        .add_system(draw_matter)
        .add_system(simulate)
        // Gui
        .add_system(user_interface)
        // Render after update
        .add_system_to_stage(CoreStage::PostUpdate, render)
        .run();
}

/// Creates our simulation & render pipelines
fn setup(mut commands: Commands, vulkano_windows: NonSend<BevyVulkanoWindows>) {
    let (primary_window_renderer, _gui) = vulkano_windows.get_primary_window_renderer().unwrap();
    // Create our render pass
    let fill_screen = FillScreenRenderPass::new(
        primary_window_renderer.graphics_queue(),
        primary_window_renderer.swapchain_format(),
    );
    let simulator = CASimulator::new(primary_window_renderer.compute_queue());

    // Create simple orthographic camera
    let mut camera = OrthographicCamera::default();
    // Zoom camera to fit vertical pixels
    camera.zoom_to_fit_vertical_pixels(CANVAS_SIZE_Y, HEIGHT as u32);
    // Insert resources
    commands.insert_resource(fill_screen);
    commands.insert_resource(camera);
    commands.insert_resource(simulator);
    commands.insert_resource(PreviousMousePos(None));
    commands.insert_resource(CurrentMousePos(None));
    commands.insert_resource(DynamicSettings::default());
}

/// Step simulation
fn simulate(mut sim_pipeline: ResMut<CASimulator>) {
    sim_pipeline.step();
}

/// Render the simulation
fn render(
    mut vulkano_windows: NonSendMut<BevyVulkanoWindows>,
    mut fill_screen: ResMut<FillScreenRenderPass>,
    camera: Res<OrthographicCamera>,
    simulator: Res<CASimulator>,
) {
    let (window_renderer, gui) = vulkano_windows.get_primary_window_renderer_mut().unwrap();
    // Start frame
    let before = match window_renderer.acquire() {
        Err(e) => {
            bevy::log::error!("Failed to start frame: {}", e);
            return;
        }
        Ok(f) => f,
    };

    let canvas_image = simulator.color_image();

    // Render
    let final_image = window_renderer.swapchain_image_view();
    let after_images = fill_screen.draw(
        before,
        *camera,
        canvas_image,
        final_image.clone(),
        CLEAR_COLOR,
        false,
        true,
    );
    // Draw gui
    let after_gui = gui.draw_on_image(after_images, final_image);
    // Finish Frame
    window_renderer.present(after_gui, true);
}

/// Update camera (if window is resized)
fn update_camera(windows: Res<Windows>, mut camera: ResMut<OrthographicCamera>) {
    let window = windows.get_primary().unwrap();
    camera.update(window.width(), window.height());
}

/// Input actions for camera movement, zoom and pausing
fn input_actions(
    time: Res<Time>,
    mut camera: ResMut<OrthographicCamera>,
    keyboard_input: Res<Input<KeyCode>>,
    mut mouse_input_events: EventReader<MouseWheel>,
) {
    // Move camera with arrows & WASD
    let up = keyboard_input.pressed(KeyCode::W) || keyboard_input.pressed(KeyCode::Up);
    let down = keyboard_input.pressed(KeyCode::S) || keyboard_input.pressed(KeyCode::Down);
    let left = keyboard_input.pressed(KeyCode::A) || keyboard_input.pressed(KeyCode::Left);
    let right = keyboard_input.pressed(KeyCode::D) || keyboard_input.pressed(KeyCode::Right);

    let x_axis = -(right as i8) + left as i8;
    let y_axis = -(up as i8) + down as i8;

    let mut move_delta = Vec2::new(x_axis as f32, y_axis as f32);
    if move_delta != Vec2::ZERO {
        move_delta /= move_delta.length();
        camera.pos += move_delta * time.delta_seconds() * CAMERA_MOVE_SPEED;
    }

    // Zoom camera with mouse scroll
    for e in mouse_input_events.iter() {
        if e.y < 0.0 {
            camera.scale *= 1.05;
        } else {
            camera.scale *= 1.0 / 1.05;
        }
    }
}

/// Draw matter to our grid
fn draw_matter(
    mut simulator: ResMut<CASimulator>,
    prev: Res<PreviousMousePos>,
    current: Res<CurrentMousePos>,
    mouse_button_input: Res<Input<MouseButton>>,
    settings: Res<DynamicSettings>,
) {
    if let Some(current) = current.0 {
        if mouse_button_input.pressed(MouseButton::Left) {
            let line = get_canvas_line(prev.0, current);
            simulator.draw_matter(&line, settings.brush_radius, settings.draw_matter);
        }
    }
}

/// Mouse position from last frame
#[derive(Debug, Copy, Clone)]
pub struct PreviousMousePos(pub Option<MousePos>);

/// Mouse position now
#[derive(Debug, Copy, Clone)]
pub struct CurrentMousePos(pub Option<MousePos>);

/// Update mouse position
fn update_mouse(
    windows: Res<Windows>,
    mut _prev: ResMut<PreviousMousePos>,
    mut _current: ResMut<CurrentMousePos>,
    camera: Res<OrthographicCamera>,
) {
    _prev.0 = _current.0;
    let primary = windows.get_primary().unwrap();
    if primary.cursor_position().is_some() {
        _current.0 = Some(MousePos {
            world: cursor_to_world(primary, camera.pos, camera.scale),
        });
    }
}
