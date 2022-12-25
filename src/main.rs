#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Fire-Emblem Level Analyzer",
        native_options,
        Box::new(|cc| Box::new(fe_levels_gui::FeLevelGui::new(cc)))
    );
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
            Box::new(|cc| Box::new(fe_levels_gui::FeLevelGui::new(cc)))
        )
        .await
        .expect("failed to start eframe");
    });
}

#[cfg(target_arch = "wasm32")]
static RN_JESUS_IV : std::sync::Mutex<[u8; 12]> = std::sync::Mutex::new([0x24; 12]);

#[cfg(target_arch = "wasm32")]
fn rn_jesus(buffer : &mut [u8]) -> Result<(), getrandom::Error> {
    let mut guard = RN_JESUS_IV
        .lock()
        .map_err(|_| getrandom::Error::WEB_GET_RANDOM_VALUES)?;
    let copy = guard.clone();
    let mut instance =
        <chacha20::ChaCha20 as chacha20::cipher::KeyIvInit>::new(&[0x42; 32].into(), &copy.into());

    let all_zeros = vec![0u8; buffer.len()];

    chacha20::cipher::StreamCipher::apply_keystream_b2b(&mut instance, &all_zeros, buffer)
        .map_err(|_| getrandom::Error::WEB_GET_RANDOM_VALUES)?;

    let iv_zeros = [0u8; 12];

    chacha20::cipher::StreamCipher::apply_keystream_b2b(&mut instance, &iv_zeros, guard.as_mut())
        .map_err(|_| getrandom::Error::WEB_GET_RANDOM_VALUES)?;

    Ok(())
}

#[cfg(target_arch = "wasm32")]
getrandom::register_custom_getrandom!(rn_jesus);
