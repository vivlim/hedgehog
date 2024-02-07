use std::sync::mpsc::{Receiver, Sender};

use crate::{channels::AsyncRequestBridge, service::AsyncServiceMessage};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    // Example stuff:
    label: String,

    #[serde(skip)] // This how you opt-out of serialization of a field
    value: f32,

    #[serde(skip)] // This how you opt-out of serialization of a field
    async_bridge: Option<AsyncRequestBridge<AsyncServiceMessage, u32>>, // #[serde(skip)]
                                                                        // tx: Sender<AsyncMessage>,

                                                                        // #[serde(skip)]
                                                                        // rx: Receiver<AsyncMessage>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            value: 2.7,
            async_bridge: None,
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        async_bridge: AsyncRequestBridge<AsyncServiceMessage, u32>,
    ) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        let mut app: TemplateApp = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };
        app.async_bridge = Some(async_bridge);
        app
    }
}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        if let Some(br) = &mut self.async_bridge {
            br.pump_messages();
            // TODO: if a message was received, we should refresh the ui!
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("eframe template");

            if let Some(async_bridge) = &mut self.async_bridge {
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Async state: {:?}",
                        match &async_bridge.state {
                            crate::channels::AsyncRequestBridgeState::Init =>
                                "Init (not run yet)".to_string(),
                            crate::channels::AsyncRequestBridgeState::Awaiting {
                                response: _,
                                prev_state,
                                handler: _,
                            } => format!(
                                "Awaiting (last value: {})",
                                match prev_state {
                                    Some(n) => format!("{}", n),
                                    None => "None".to_string(),
                                }
                            ),
                            crate::channels::AsyncRequestBridgeState::Updating =>
                                "Updating".to_string(),
                            crate::channels::AsyncRequestBridgeState::Complete(n) =>
                                format!("Complete: {}", &n),
                            crate::channels::AsyncRequestBridgeState::Error(e) =>
                                format!("Error: {}", e),
                        }
                    ));
                    if ui.button("Async invoke").clicked() {
                        async_bridge.send(
                            AsyncServiceMessage::Echo(1),
                            Box::new(|m, prev_state| match (m, prev_state) {
                                (AsyncServiceMessage::Echo(n), Some(p)) => n + p,
                                (AsyncServiceMessage::Echo(n), None) => n,
                            }),
                        );
                    }
                });
            }

            ui.horizontal(|ui| {
                ui.label("Write something: ");
                ui.text_edit_singleline(&mut self.label);
            });

            ui.add(egui::Slider::new(&mut self.value, 0.0..=10.0).text("value"));
            if ui.button("Increment").clicked() {
                self.value += 1.0;
            }

            ui.separator();

            ui.add(egui::github_link_file!(
                "https://github.com/emilk/eframe_template/blob/master/",
                "Source code."
            ));

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });
        });
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });
}
