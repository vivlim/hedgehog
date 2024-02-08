#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    use instant::Duration;

    use hedgehog::{
        channels::{AsyncRequestBridge, Spawner},
        service::{new_async_service_channels, start_async_service, AsyncServiceMessage},
    };
    use log::{debug, trace, warn};
    use tokio::runtime::Runtime;

    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let (ui_async_tx, ui_async_rx) = new_async_service_channels();
    let (svc_async_tx, svc_async_rx) = new_async_service_channels();

    let spawner = Spawner::new(async move {
        debug!("start async service");
        start_async_service(ui_async_tx, svc_async_rx).await;
        warn!("done async service.");
    });

    // ui_async_rx (notifications from svc to ui) is currently not hooked up.
    let ui_bridge = AsyncRequestBridge::<AsyncServiceMessage, u32>::new(svc_async_tx, spawner);

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0])
            .with_icon(
                // NOE: Adding an icon is optional
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
                    .unwrap(),
            ),
        ..Default::default()
    };
    eframe::run_native(
        "eframe template",
        native_options,
        Box::new(|cc| Box::new(hedgehog::TemplateApp::new(cc, ui_bridge))),
    )
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    use hedgehog::{
        channels::{AsyncRequestBridge, Spawner},
        service::{new_async_service_channels, start_async_service, AsyncServiceMessage},
    };
    use log::{debug, trace, warn};

    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    let (ui_async_tx, ui_async_rx) = new_async_service_channels();
    let (svc_async_tx, svc_async_rx) = new_async_service_channels();

    let spawner = Spawner::new(async move {
        debug!("start async service");
        start_async_service(ui_async_tx, svc_async_rx).await;
        warn!("done async service.");
    });

    // ui_async_rx (notifications from svc to ui) is currently not hooked up.
    let ui_bridge = AsyncRequestBridge::<AsyncServiceMessage, u32>::new(svc_async_tx, spawner);

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "the_canvas_id", // hardcode it
                web_options,
                Box::new(|cc| Box::new(hedgehog::TemplateApp::new(cc, ui_bridge))),
            )
            .await
            .expect("failed to start eframe");
    });
}
