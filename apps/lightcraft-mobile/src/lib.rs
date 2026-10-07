//! LightCraft mobile shell (spike): the desktop egui UI and engine, unmodified, in an iOS or
//! Android app. What works and what doesn't is in `README.md`; the porting assessment is in
//! `docs/porting-report.md`.
//!
//! The library lives in the app's sandbox: `Documents/LightCraft Library` on iOS (visible in the
//! Files app), `<internal data>/LightCraft Library` on Android. A new library gets the demo photos.
//! There are no pickers yet: every host service except export writing is `None`.
#![cfg(any(target_os = "ios", target_os = "android"))]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use lightcraft_engine::Session;
use lightcraft_ui_egui::{LightcraftApp, Services};

struct App(LightcraftApp);

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.0.logic(ctx);
    }
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        self.0.raw_input_hook(raw);
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.0.ui(ui);
    }
    fn on_exit(&mut self) {
        if let Err(e) = self.0.session.close_library() {
            log::error!("saving the library failed: {e}");
        }
    }
}

fn services() -> Services {
    Services {
        // atomic (temp file + sync + rename), inside the sandbox
        write_shared: Some(std::sync::Arc::new(lightcraft_engine::export::write_file)),
        write: Some(Box::new(lightcraft_engine::export::write_file)),
        png: Some(Box::new(|img: &lightcraft_raster::Rgba8| {
            lightcraft_codecs::encode_png(&lightcraft_codecs::EncodeImage::rgba8(img), &lightcraft_codecs::EncodeMeta::default()).unwrap_or_default()
        })),
        ..Default::default()
    }
}

/// Logs to stderr (Xcode / `simctl launch --console-pty`). Android sends stderr nowhere: see README.
struct StderrLog;

impl log::Log for StderrLog {
    fn enabled(&self, m: &log::Metadata) -> bool {
        m.level() <= log::Level::Info
    }
    fn log(&self, r: &log::Record) {
        if self.enabled(r.metadata()) {
            eprintln!("[{} {}] {}", r.level(), r.target(), r.args());
        }
    }
    fn flush(&self) {}
}

static LOGGER: StderrLog = StderrLog;

/// Asks for the adapter's own limits: eframe's defaults are above what the iOS Simulator's Metal
/// adapter reports (`max_inter_stage_shader_variables` 15 < 16), and device creation fails.
fn adapter_limits(options: &mut eframe::NativeOptions) {
    if let eframe::egui_wgpu::WgpuSetup::CreateNew(n) = &mut options.wgpu_options.wgpu_setup {
        n.device_descriptor = std::sync::Arc::new(|adapter| eframe::wgpu::DeviceDescriptor {
            label: Some("egui wgpu device"),
            required_limits: adapter.limits(),
            ..Default::default()
        });
    }
}

/// Runs the app with its library in `library_dir`; returns when the event loop ends.
pub fn run(mut options: eframe::NativeOptions, library_dir: std::path::PathBuf) -> eframe::Result {
    let _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(log::LevelFilter::Info));
    adapter_limits(&mut options);
    log::info!("library at {}", library_dir.display());
    eframe::run_native(
        "LightCraft",
        options,
        Box::new(move |_cc| {
            let mut s = Session::new().with_fs().with_system_clock();
            let t0 = std::time::Instant::now();
            match s.open_library(&library_dir, true) {
                Ok(r) => {
                    let replayed = r.replayed;
                    log::info!("library: {} photos, {replayed} log records replayed ({:.1} ms)", s.catalog.len(), t0.elapsed().as_secs_f64() * 1e3);
                }
                Err(e) => log::error!("can't open library {}: {e}", library_dir.display()),
            }
            Ok(Box::new(App(LightcraftApp::new(s, services()))))
        }),
    )
}

/// iOS: `$HOME` is the app container; `Documents/` shows up in the Files app.
#[cfg(target_os = "ios")]
pub fn ios_main() {
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from).unwrap_or_default();
    if let Err(e) = run(eframe::NativeOptions::default(), home.join("Documents").join("LightCraft Library")) {
        log::error!("{e}");
    }
}

/// Android entry point, called by NativeActivity (via android-activity) on its own thread.
#[cfg(target_os = "android")]
#[allow(unsafe_code)] // the symbol name is the contract with android-activity
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    let dir = app.internal_data_path().unwrap_or_default().join("LightCraft Library");
    let mut options = eframe::NativeOptions { android_app: Some(app), ..Default::default() };
    // GL window unless /data/local/tmp/lc_vulkan exists: the emulator's Vulkan surface fails to
    // configure ("Invalid surface"); GPU compute (lightcraft-gpu) still picks Vulkan itself
    if !std::path::Path::new("/data/local/tmp/lc_vulkan").exists()
        && let eframe::egui_wgpu::WgpuSetup::CreateNew(n) = &mut options.wgpu_options.wgpu_setup
    {
        n.instance_descriptor.backends = eframe::wgpu::Backends::GL;
    }
    if let Err(e) = run(options, dir) {
        log::error!("{e}");
    }
}
