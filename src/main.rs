#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use crossbeam_queue::SegQueue;
use cyan_fleet_control::message_handler::Message;
use cyan_fleet_control::message_handler::MessageHandler;

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> eframe::Result<()> {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let rt = tokio::runtime::Runtime::new().expect("Unable to create Runtime");

    // Enter the runtime so that `tokio::spawn` is available immediately.
    let _enter = rt.enter();

    let state = std::sync::Arc::new(std::sync::Mutex::new(cyan_fleet_control::AppData::new()));

    let poll_state = state.clone();

    std::thread::spawn(move || {
        let queue = SegQueue::<Message>::new();
        for message in <Message as strum::IntoEnumIterator>::iter() {
            queue.push(message);
        }

        rt.block_on(async {
            loop {
                let m = queue.pop().unwrap();
                queue.push(m);
                MessageHandler.handle_message(&m, poll_state.clone()).await;
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        })
    });

    let native_options = eframe::NativeOptions::default();

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
