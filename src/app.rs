use std::sync::mpsc::{Receiver, Sender};

use log::debug;

use crate::{
    authenticate::AuthMessage,
    channels::{AsyncRequestBridge, AsyncRequestBridgeState},
    service::AsyncServiceMessage,
};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    // Example stuff:
    label: String,
    instance: String,

    #[serde(skip)] // This how you opt-out of serialization of a field
    value: f32,

    #[serde(skip)] // This how you opt-out of serialization of a field
    async_bridge: Option<AsyncRequestBridge<AsyncServiceMessage, AsyncAppState>>, // #[serde(skip)]
                                                                                  // tx: Sender<AsyncMessage>,

                                                                                  // #[serde(skip)]
                                                                                  // rx: Receiver<AsyncMessage>,
}

pub struct AsyncAppState {
    auth_bridge: Option<AsyncRequestBridge<AuthMessage, AuthUiState>>,
    counter: u32,
}

pub struct AuthUiState {
    signed_in: bool,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            instance: "".to_owned(),
            value: 2.7,
            async_bridge: None,
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        async_bridge: AsyncRequestBridge<AsyncServiceMessage, AsyncAppState>,
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
            if br.pump_messages() {
                ctx.request_repaint();
            }
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
                                prev_state: _,
                                handler: _,
                            } => "Awaiting".to_string(),

                            crate::channels::AsyncRequestBridgeState::Updating =>
                                "Updating".to_string(),
                            crate::channels::AsyncRequestBridgeState::Complete(AsyncAppState {
                                counter: n,
                                ..
                            }) => format!("Complete: {}", &n),
                            crate::channels::AsyncRequestBridgeState::Error(e) =>
                                format!("Error: {}", e),
                        }
                    ));
                    if ui.button("Async invoke").clicked() {
                        async_bridge.send(
                            AsyncServiceMessage::Echo(1),
                            Box::new(|m, prev_state| match (m, prev_state) {
                                (AsyncServiceMessage::Echo(n), Some(mut prev_app_state)) => {
                                    prev_app_state.counter += n;
                                    prev_app_state
                                }
                                (AsyncServiceMessage::Echo(n), None) => AsyncAppState {
                                    auth_bridge: None,
                                    counter: n,
                                },
                                _ => panic!("can't handle this response."),
                            }),
                        );
                    }
                });
                ui.horizontal(|ui| {
                    // Is there an authenticated service existing?
                    if let AsyncRequestBridgeState::Complete(AsyncAppState {
                        auth_bridge: Some(auth_ui_bridge),
                        ..
                    }) = &mut async_bridge.state
                    {
                        // It exists, so there is a mastodon instance
                        match &auth_ui_bridge.state {
                            crate::channels::AsyncRequestBridgeState::Init => {
                                // In init state, show a button that will continue the login when
                                // clicked.
                                if ui
                                    .button(format!("Continue logging in to {}", &self.instance))
                                    .clicked()
                                {
                                    auth_ui_bridge.send(
                                        AuthMessage::Initialize(self.instance.clone()),
                                        Box::new(|m, prev_state| match (m, prev_state) {
                                            (AuthMessage::MastodonData(md), Some(p)) => {
                                                AuthUiState { signed_in: true }
                                            }
                                            (AuthMessage::MastodonData(md), None) => {
                                                AuthUiState { signed_in: true }
                                            }
                                            _ => panic!("can't handle this response."),
                                        }),
                                    );
                                }
                            }
                            crate::channels::AsyncRequestBridgeState::Awaiting {
                                response,
                                prev_state,
                                handler,
                            } => {
                                ui.label("Logging in...");
                            }
                            crate::channels::AsyncRequestBridgeState::Updating => {
                                ui.label("updating");
                            }
                            crate::channels::AsyncRequestBridgeState::Complete(auth_state) => {
                                ui.label(format!("complete. signed in: {}", auth_state.signed_in));
                            }
                            crate::channels::AsyncRequestBridgeState::Error(e) => {
                                ui.label(e);
                            }
                        }
                    } else {
                        // There's no backend yet. Allow specifying and connecting to one
                        ui.label("Instance:");
                        ui.text_edit_singleline(&mut self.instance);
                        if ui.button("Log in").clicked() {
                            debug!("Send start auth message");
                            async_bridge.send(
                                AsyncServiceMessage::StartAuth,
                                Box::new(|m, prev_state| {
                                    debug!("Received reply for start auth service");
                                    match (m, prev_state) {
                                        // Upon receiving a channel for our attempted login, update the
                                        // state. If there is state already, add it to that
                                        (AsyncServiceMessage::AuthChannel(ac), Some(mut p)) => {
                                            p.auth_bridge = Some(AsyncRequestBridge::<
                                                AuthMessage,
                                                AuthUiState,
                                            >::new(
                                                ac
                                            ));

                                            p
                                        }
                                        // Or create async app state
                                        (AsyncServiceMessage::AuthChannel(ac), None) => {
                                            AsyncAppState {
                                                auth_bridge: Some(AsyncRequestBridge::<
                                                    AuthMessage,
                                                    AuthUiState,
                                                >::new(
                                                    ac
                                                )),
                                                counter: 0,
                                            }
                                        }
                                        _ => panic!("can't handle this response."),
                                    }
                                }),
                            );
                        }
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
