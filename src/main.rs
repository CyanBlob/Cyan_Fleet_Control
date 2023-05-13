#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> eframe::Result<()> {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();
    
    let rt = tokio::runtime::Runtime::new().expect("Unable to create Runtime");

    // Enter the runtime so that `tokio::spawn` is available immediately.
    let _enter = rt.enter();

    // Execute the runtime in its own thread.
    // The future doesn't have to do anything. In this example, it just sleeps forever.
    std::thread::spawn(move || {
        rt.block_on(async {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            }
        })
    });

    let native_options = eframe::NativeOptions::default();

    let state = std::sync::Arc::new(std::sync::Mutex::new(cyan_fleet_control::AppData::new()));
    
    eframe::run_native(
        "eframe template",
        native_options,
        Box::new(|cc| Box::new(cyan_fleet_control::AppState::new(cc, state))),
        //Box::new(|cc| Box::new(Arc::new(Mutex::new(cyan_fleet_control::AppState::new(cc))))),
    )
}

// when compiling to web using trunk.
#[cfg(target_arch = "wasm32")]
fn main() {
    // Make sure panics are logged using `console.error`.
    console_error_panic_hook::set_once();

    // Redirect tracing to console.log and friends:
    tracing_wasm::set_as_global_default();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::start_web(
            "the_canvas_id", // hardcode it
            web_options,
            Box::new(|cc| Box::new(cyan_fleet_control::TemplateApp::new(cc))),
        )
        .await
        .expect("failed to start eframe");
    });
}
