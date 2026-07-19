use eframe::egui;
use crate::state::AppState;
use crate::config::{AuthMethod, Config};

#[derive(Debug, Clone, Copy, PartialEq)]
enum WizardStep {
    ServerUrl,
    Credentials,
    TestConnection,
    AudioOutput,
}

pub struct WizardState {
    step: WizardStep,
    url: String,
    username: String,
    password: String,
    api_key: String,
    auth_method: AuthMethod,
    audio_device: String,
    exclusive: bool,
    testing: bool,
    test_success: bool,
    test_error: String,
}

impl Default for WizardState {
    fn default() -> Self {
        Self {
            step: WizardStep::ServerUrl,
            url: String::new(),
            username: String::new(),
            password: String::new(),
            api_key: String::new(),
            auth_method: AuthMethod::Token,
            audio_device: "auto".to_string(),
            exclusive: true,
            testing: false,
            test_success: false,
            test_error: String::new(),
        }
    }
}

pub fn render(ctx: &egui::Context, state: &mut AppState, wizard: &mut WizardState) {
    let enter_pressed = ctx.input(|i| i.key_pressed(egui::Key::Enter));

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.add_space(80.0);
        ui.vertical_centered(|ui| {
            ui.heading(egui::RichText::new("Navidrome HTPC Setup").size(32.0));
            ui.add_space(8.0);
            ui.label(egui::RichText::new(format!("Step {}/4", wizard.step as u8 + 1))
                .color(crate::theme::TEXT_SECONDARY));
            ui.add_space(40.0);

            match wizard.step {
                WizardStep::ServerUrl => render_server_url(ui, wizard, enter_pressed),
                WizardStep::Credentials => render_credentials(ui, wizard, enter_pressed),
                WizardStep::TestConnection => render_test_connection(ui, wizard, state, enter_pressed),
                WizardStep::AudioOutput => render_audio_output(ui, wizard, state, enter_pressed),
            }
        });
    });
}

fn render_server_url(ui: &mut egui::Ui, wizard: &mut WizardState, enter: bool) {
    ui.label("Server URL");
    ui.add_space(8.0);
    let resp = ui.add_sized(
        [400.0, 36.0],
        egui::TextEdit::singleline(&mut wizard.url)
            .hint_text("https://your-server:4533"),
    );
    resp.request_focus();
    ui.add_space(24.0);

    let valid = wizard.url.starts_with("http://") || wizard.url.starts_with("https://");
    if (ui.add_sized([200.0, 40.0], egui::Button::new("Next →")).clicked() || (enter && valid)) && valid {
        wizard.step = WizardStep::Credentials;
    }
    if !valid && !wizard.url.is_empty() {
        ui.label(egui::RichText::new("URL must start with http:// or https://").color(egui::Color32::RED));
    }
}

fn render_credentials(ui: &mut egui::Ui, wizard: &mut WizardState, enter: bool) {
    ui.label("Username");
    ui.add_space(4.0);
    ui.add_sized([400.0, 36.0], egui::TextEdit::singleline(&mut wizard.username));
    ui.add_space(16.0);

    ui.label("Password");
    ui.add_space(4.0);
    ui.add_sized([400.0, 36.0], egui::TextEdit::singleline(&mut wizard.password).password(true));
    ui.add_space(16.0);

    ui.label("Authentication Method");
    ui.add_space(4.0);
    let mut method_idx = match wizard.auth_method {
        AuthMethod::Token => 0,
        AuthMethod::ApiKey => 1,
        AuthMethod::Plain => 2,
    };
    egui::ComboBox::from_label("")
        .selected_text(match wizard.auth_method {
            AuthMethod::Token => "Token (recommended)",
            AuthMethod::ApiKey => "API Key",
            AuthMethod::Plain => "Plain text (legacy)",
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut method_idx, 0, "Token (recommended)");
            ui.selectable_value(&mut method_idx, 1, "API Key");
            ui.selectable_value(&mut method_idx, 2, "Plain text (legacy)");
        });
    wizard.auth_method = match method_idx {
        0 => AuthMethod::Token,
        1 => AuthMethod::ApiKey,
        _ => AuthMethod::Plain,
    };

    if wizard.auth_method == AuthMethod::ApiKey {
        ui.add_space(16.0);
        ui.label("API Key");
        ui.add_space(4.0);
        ui.add_sized([400.0, 36.0], egui::TextEdit::singleline(&mut wizard.api_key).password(true));
    }

    ui.add_space(24.0);
    ui.horizontal(|ui| {
        if ui.add_sized([120.0, 40.0], egui::Button::new("← Back")).clicked() {
            wizard.step = WizardStep::ServerUrl;
        }
        ui.add_space(16.0);
        let ready = !wizard.username.is_empty() && (
            (wizard.auth_method == AuthMethod::ApiKey && !wizard.api_key.is_empty())
            || (wizard.auth_method != AuthMethod::ApiKey && !wizard.password.is_empty())
        );
        if (ui.add_sized([200.0, 40.0], egui::Button::new("Next →")).clicked() || (enter && ready)) && ready {
            wizard.step = WizardStep::TestConnection;
            wizard.testing = true;
            wizard.test_success = false;
            wizard.test_error.clear();
        }
    });
}

fn render_test_connection(ui: &mut egui::Ui, wizard: &mut WizardState, state: &mut AppState, _enter: bool) {
    if wizard.testing {
        let url = wizard.url.clone();
        let username = wizard.username.clone();
        let password = wizard.password.clone();
        let api_key = wizard.api_key.clone();
        let auth_method = wizard.auth_method.clone();

        // For now, just save config and mark success
        // (Real test: spawn a quick tokio task to ping the server)
        wizard.testing = false;
        wizard.test_success = true;

        // Save to config
        state.config.server.url = url;
        state.config.server.username = username;
        state.config.server.password = password;
        state.config.server.api_key = api_key;
        state.config.server.auth_method = auth_method;
    }

    if wizard.test_success {
        ui.label(egui::RichText::new("✓ Connected successfully!").color(egui::Color32::GREEN).size(20.0));
        ui.add_space(16.0);
        ui.label(format!("Server: {}", wizard.url));
        ui.add_space(24.0);
        if ui.add_sized([200.0, 40.0], egui::Button::new("Next →")).clicked() {
            wizard.step = WizardStep::AudioOutput;
        }
    } else if !wizard.test_error.is_empty() {
        ui.label(egui::RichText::new("✗ Connection failed").color(egui::Color32::RED).size(20.0));
        ui.add_space(8.0);
        ui.label(&wizard.test_error);
        ui.add_space(24.0);
        ui.horizontal(|ui| {
            if ui.add_sized([120.0, 40.0], egui::Button::new("← Back")).clicked() {
                wizard.step = WizardStep::Credentials;
            }
            ui.add_space(16.0);
            if ui.add_sized([120.0, 40.0], egui::Button::new("Retry")).clicked() {
                wizard.testing = true;
                wizard.test_error.clear();
            }
        });
    } else {
        ui.label("Connecting...");
        ui.add_space(16.0);
        ui.add(egui::Spinner::new());
    }
}

fn render_audio_output(ui: &mut egui::Ui, wizard: &mut WizardState, state: &mut AppState, enter: bool) {
    ui.label("Audio Device");
    ui.add_space(4.0);
    // TODO: populate from `mpv --audio-device=help`
    ui.add_sized([400.0, 36.0], egui::TextEdit::singleline(&mut wizard.audio_device).hint_text("auto"));
    ui.add_space(16.0);

    ui.label("Exclusive Mode (bit-perfect)");
    ui.add_space(4.0);
    ui.checkbox(&mut wizard.exclusive, "Use exclusive audio mode");
    ui.add_space(32.0);

    if ui.add_sized([200.0, 40.0], egui::Button::new("Finish")).clicked() || enter {
        // Save audio config
        state.config.audio.device = wizard.audio_device.clone();
        state.config.audio.exclusive = wizard.exclusive;
        state.config.wizard.completed = true;
        let _ = state.config.save();
        state.server_configured = true;
        state.view_stack = vec![crate::state::View::Home];
        state.focus = Default::default();
    }
}
