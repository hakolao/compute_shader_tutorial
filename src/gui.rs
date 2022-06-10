use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_vulkano::{
    egui_winit_vulkano::{egui, egui::Ui},
    VulkanoWindows,
};

use crate::{
    camera::OrthographicCamera,
    matter::MatterId,
    utils::{cursor_to_world, MousePos},
    DynamicSettings,
};

fn sized_text(ui: &mut Ui, text: impl Into<String>, size: f32) {
    ui.label(egui::RichText::new(text).size(size));
}

pub fn user_interface(
    vulkano_windows: Res<VulkanoWindows>,
    diagnostics: Res<Diagnostics>,
    windows: Res<Windows>,
    camera: Res<OrthographicCamera>,
    mut settings: ResMut<DynamicSettings>,
) {
    let ctx = vulkano_windows
        .get_primary_window_renderer()
        .unwrap()
        .gui_context();
    egui::Area::new("fps")
        .fixed_pos(egui::pos2(10.0, 10.0))
        .show(&ctx, |ui| {
            let size = 30.0;
            if let Some(diag) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
                if let Some(avg) = diag.average() {
                    sized_text(ui, format!("FPS: {:.2}", avg), size);
                }
            }
            ui.heading("Settings");
            ui.add(egui::Slider::new(&mut settings.brush_radius, 1..=20).text("Brush Size"));
            ui.add(egui::Slider::new(&mut settings.move_steps, 1..=5).text("Move Steps"));
            // Selectable matter
            let matters = [MatterId::Empty, MatterId::Sand];
            egui::ComboBox::from_label("Matter")
                .selected_text(format!("{:?}", settings.draw_matter))
                .show_ui(ui, |ui| {
                    for matter in matters {
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
        });
    }
}
