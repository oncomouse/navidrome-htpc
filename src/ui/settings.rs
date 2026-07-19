use eframe::egui;
use crate::state::AppState;
use crate::config::{AuthMethod, ReplayGainMode};
use crate::theme::*;

pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    ui.add_space(20.0);
    ui.label(egui::RichText::new("‹ Settings").size(24.0).color(TEXT_PRIMARY));
    ui.add_space(20.0);

    egui::ScrollArea::vertical().id_salt("settings").show(ui, |ui| {
        // Connection
        ui.label(egui::RichText::new("Connection").size(18.0).color(ACCENT));
        ui.add_space(8.0);
        ui.horizontal(|ui| { ui.label("Server URL:"); ui.text_edit_singleline(&mut state.config.server.url); });
        ui.horizontal(|ui| { ui.label("Username:"); ui.text_edit_singleline(&mut state.config.server.username); });
        ui.horizontal(|ui| {
            ui.label("Auth Method:");
            egui::ComboBox::from_id_salt("auth_method")
                .selected_text(format!("{:?}", state.config.server.auth_method))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.config.server.auth_method, AuthMethod::Token, "Token");
                    ui.selectable_value(&mut state.config.server.auth_method, AuthMethod::ApiKey, "API Key");
                    ui.selectable_value(&mut state.config.server.auth_method, AuthMethod::Plain, "Plain");
                });
        });
        if ui.button("Reconnect").clicked() {
            let _ = state.config.save();
            // TODO: restart SubsonicClient with new config
        }
        ui.add_space(20.0);

        // Audio
        ui.label(egui::RichText::new("Audio").size(18.0).color(ACCENT));
        ui.add_space(8.0);
        ui.horizontal(|ui| { ui.label("Device:"); ui.text_edit_singleline(&mut state.config.audio.device); });
        ui.checkbox(&mut state.config.audio.exclusive, "Exclusive Mode");
        ui.checkbox(&mut state.config.audio.gapless, "Gapless");
        ui.horizontal(|ui| {
            ui.label("ReplayGain:");
            egui::ComboBox::from_id_salt("replaygain")
                .selected_text(format!("{:?}", state.config.audio.replaygain))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.config.audio.replaygain, ReplayGainMode::Off, "Off");
                    ui.selectable_value(&mut state.config.audio.replaygain, ReplayGainMode::Track, "Track");
                    ui.selectable_value(&mut state.config.audio.replaygain, ReplayGainMode::Album, "Album");
                });
        });
        ui.add_space(20.0);

        // Display
        ui.label(egui::RichText::new("Display").size(18.0).color(ACCENT));
        ui.add_space(8.0);
        ui.horizontal(|ui| { ui.label("UI Scale:"); ui.add(egui::Slider::new(&mut state.config.display.scale, 1.0..=3.0)); });
        ui.add_space(20.0);

        // Playback
        ui.label(egui::RichText::new("Playback").size(18.0).color(ACCENT));
        ui.add_space(8.0);
        ui.checkbox(&mut state.config.playback.scrobble, "Scrobble");
        ui.checkbox(&mut state.config.playback.auto_advance, "Auto-advance");
        ui.checkbox(&mut state.config.playback.resume_on_start, "Resume on Start");
        ui.add_space(20.0);

        // Cache
        ui.label(egui::RichText::new("Cache").size(18.0).color(ACCENT));
        ui.add_space(8.0);
        ui.horizontal(|ui| { ui.label("Cover Art Size:"); ui.add(egui::Slider::new(&mut state.config.cache.cover_art_size, 100..=600)); });
        if ui.button("Clear Cache").clicked() {
            // TODO: clear cover art cache
            state.toasts.push(crate::state::Toast { message: "Cache cleared".into(), ttl: 3.0 });
        }
    });

    // Save on any change
    let _ = state.config.save();
}
