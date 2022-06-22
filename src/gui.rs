use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_vulkano::{
    egui_winit_vulkano::{egui, egui::Ui},
    VulkanoWindows,
};
use strum::IntoEnumIterator;

use crate::{
    ca_simulator::CASimulator,
    camera::OrthographicCamera,
    cursor_to_world,
    matter::MatterId,
    timer::{RenderTimer, SimTimer},
    DynamicSettings, MousePos, CANVAS_SIZE_X, CANVAS_SIZE_Y,
};

/// Give our text a custom size
fn sized_text(ui: &mut Ui, text: impl Into<String>, size: f32) {
    ui.label(egui::RichText::new(text).size(size));
}

/// System to generate user interface with egui
pub fn user_interface(
    vulkano_windows: Res<VulkanoWindows>,
    diagnostics: Res<Diagnostics>,
    windows: Res<Windows>,
    camera: Res<OrthographicCamera>,
    mut settings: ResMut<DynamicSettings>,
    sim_timer: Res<SimTimer>,
    render_timer: Res<RenderTimer>,
    mut simulator: ResMut<CASimulator>,
) {
    let ctx = vulkano_windows
        .get_primary_window_renderer()
        .unwrap()
        .gui_context();
    egui::Area::new("fps")
        .fixed_pos(egui::pos2(10.0, 10.0))
        .show(&ctx, |ui| {
            let size = 15.0;
            ui.heading("Info");
            if let Some(diag) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
                if let Some(avg) = diag.average() {
                    sized_text(ui, format!("FPS: {:.2}", avg), size);
                }
            }
            sized_text(
                ui,
                format!("Grid size: ({},{})", CANVAS_SIZE_X, CANVAS_SIZE_Y),
                size,
            );
            sized_text(
                ui,
                format!(
                    "Sim Time: {:.2} ms, {}",
                    sim_timer.0.time_average_ms(),
                    if settings.is_paused {
                        "Paused"
                    } else {
                        "Playing"
                    }
                ),
                size,
            );
            sized_text(
                ui,
                format!("Render Time: {:.2} ms", render_timer.0.time_average_ms()),
                size,
            );
            ui.heading("Settings");
            ui.add(egui::Slider::new(&mut settings.brush_radius, 0.5..=20.0).text("Brush Size"));
            ui.add(egui::Slider::new(&mut settings.move_steps, 1..=5).text("Move Steps"));
            // Selectable matter
            egui::ComboBox::from_label("Matter")
                .selected_text(format!("{:?}", settings.draw_matter))
                .show_ui(ui, |ui| {
                    for matter in MatterId::iter() {
                        ui.selectable_value(
                            &mut settings.draw_matter,
                            matter,
                            format!("{:?}", matter),
                        );
                    }
                });
        });
    let primary = windows.get_primary().unwrap();
    if primary.cursor_position().is_some() {
        let world_pos = cursor_to_world(primary, camera.pos, camera.scale);
        let sim_pos = MousePos::new(world_pos).canvas_pos();
        egui::containers::show_tooltip_at_pointer(&ctx, egui::Id::new("Hover tooltip"), |ui| {
            ui.label(format!("World: [{:.2}, {:.2}]", world_pos.x, world_pos.y));
            ui.label(format!("Sim: [{:.2}, {:.2}]", sim_pos.x, sim_pos.y));
            if let Some(matter) = simulator.query_matter(sim_pos.as_ivec2()) {
                ui.label(format!("Matter: {:?}", matter));
            }
        });
    }
}
