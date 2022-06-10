mod ca_pipeline;
mod camera;
mod gui;
mod matter;
mod quad_pipeline;
mod render;
mod utils;
mod vertex;

use bevy::{
    core::FixedTimestep,
    input::{mouse::MouseWheel, system::exit_on_esc_system},
    prelude::*,
    window::WindowMode,
};
use bevy_vulkano::{VulkanoContext, VulkanoWindows, VulkanoWinitConfig, VulkanoWinitPlugin};

use crate::{
    ca_pipeline::CAPipeline,
    camera::OrthographicCamera,
    gui::user_interface,
    matter::MatterId,
    render::FillScreenRenderPass,
    utils::{cursor_to_world, get_canvas_line, MousePos},
};

pub const WIDTH: f32 = 1024.0;
pub const HEIGHT: f32 = 1024.0;
pub const KERNEL_SIZE_X: u32 = 16;
pub const KERNEL_SIZE_Y: u32 = 16;
pub const CANVAS_SIZE_X: u32 = WIDTH as u32;
pub const CANVAS_SIZE_Y: u32 = HEIGHT as u32;
pub const SIM_FPS: f64 = 60.0;
pub const CLEAR_COLOR: [f32; 4] = [1.0; 4];
pub const CAMERA_MOVE_SPEED: f32 = 200.0;

pub struct DynamicSettings {
    pub brush_radius: u32,
    pub move_steps: u32,
    pub draw_matter: MatterId,
    pub is_paused: bool,
}

impl Default for DynamicSettings {
    fn default() -> Self {
        Self {
            brush_radius: 4,
            move_steps: 1,
            draw_matter: MatterId::Sand,
            is_paused: false,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct PreviousMousePos(pub Option<MousePos>);

#[derive(Debug, Copy, Clone)]
pub struct CurrentMousePos(pub Option<MousePos>);

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
        .add_plugin(bevy::diagnostic::DiagnosticsPlugin)
        .add_plugin(bevy::diagnostic::FrameTimeDiagnosticsPlugin)
        .add_plugin(bevy::input::InputPlugin)
        .add_plugin(VulkanoWinitPlugin)
        .add_startup_system(setup)
        .add_system(exit_on_esc_system)
        .add_system(input_actions)
        .add_system(update_camera)
        .add_system(update_mouse)
        .add_system(draw_matter)
        .add_system(user_interface)
        // Simulate only SIM_FPS times per second
        .add_system_set_to_stage(
            CoreStage::Update,
            SystemSet::new()
                .with_run_criteria(FixedTimestep::steps_per_second(SIM_FPS))
                .with_system(simulate),
        )
        // Render after update
        .add_system_to_stage(CoreStage::PostUpdate, render)
        .run();
}

/// Creates our simulation & render pipelines
fn setup(
    mut commands: Commands,
    vulkano_windows: Res<VulkanoWindows>,
    vulkano_context: Res<VulkanoContext>,
) {
    let primary_window_renderer = vulkano_windows.get_primary_window_renderer().unwrap();
    // Create our render pass
    let fill_screen = FillScreenRenderPass::new(
        vulkano_context.graphics_queue(),
        primary_window_renderer.swapchain_format(),
    );

    // Use same queue for compute
    let sim_pipeline = CAPipeline::new(vulkano_context.compute_queue());
    // Create simple orthographic camera
    let camera = OrthographicCamera::default();
    // Insert resources
    commands.insert_resource(fill_screen);
    commands.insert_resource(sim_pipeline);
    commands.insert_resource(camera);
    commands.insert_resource(DynamicSettings::default());
    commands.insert_resource(PreviousMousePos(None));
    commands.insert_resource(CurrentMousePos(None));
}

fn draw_matter(
    mut sim_pipeline: ResMut<CAPipeline>,
    prev: Res<PreviousMousePos>,
    current: Res<CurrentMousePos>,
    settings: Res<DynamicSettings>,
    mouse_button_input: Res<Input<MouseButton>>,
) {
    if let Some(current) = current.0 {
        if mouse_button_input.pressed(MouseButton::Left) {
            let line = get_canvas_line(prev.0, current);
            sim_pipeline.draw_matter(&line, settings.brush_radius as f32, settings.draw_matter);
        }
    }
}

fn simulate(mut sim_pipeline: ResMut<CAPipeline>, settings: Res<DynamicSettings>) {
    sim_pipeline.step(settings.move_steps, settings.is_paused);
}

fn render(
    mut vulkano_windows: ResMut<VulkanoWindows>,
    mut fill_screen: ResMut<FillScreenRenderPass>,
    sim_pipeline: Res<CAPipeline>,
    camera: Res<OrthographicCamera>,
) {
    let window_renderer = vulkano_windows.get_primary_window_renderer_mut().unwrap();
    // Start frame
    let before = match window_renderer.start_frame() {
        Err(e) => {
            bevy::log::error!("Failed to start frame: {}", e);
            return;
        }
        Ok(f) => f,
    };

    let canvas_image = sim_pipeline.color_image();

    // Render
    let final_image = window_renderer.final_image();
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
    let after_gui = window_renderer
        .gui()
        .draw_on_image(after_images, final_image);

    // Finish Frame
    window_renderer.finish_frame(after_gui);
}

fn update_camera(windows: Res<Windows>, mut camera: ResMut<OrthographicCamera>) {
    let window = windows.get_primary().unwrap();
    camera.update(window.width(), window.height());
}

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

fn input_actions(
    time: Res<Time>,
    mut camera: ResMut<OrthographicCamera>,
    keyboard_input: Res<Input<KeyCode>>,
    mut mouse_input_events: EventReader<MouseWheel>,
    mut settings: ResMut<DynamicSettings>,
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

    // Pause
    if keyboard_input.just_pressed(KeyCode::Space) {
        settings.is_paused = !settings.is_paused;
    }
}
