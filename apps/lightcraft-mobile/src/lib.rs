//! LightCraft mobile shell (spike): the desktop egui UI and engine, unmodified, in an iOS or
//! Android app. What works and what doesn't is in `README.md`; the porting assessment is in
//! `docs/porting-report.md`.
//!
//! The library lives in the app's sandbox: `Documents/LightCraft Library` on iOS (visible in the
//! Files app), `<internal data>/LightCraft Library` on Android. A new library gets the demo photos.
//! There are no pickers yet: every host service except export writing is `None`.
#![cfg(any(target_os = "ios", target_os = "android"))]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

#[cfg(target_os = "android")]
mod android_jni;
#[cfg(target_os = "ios")]
mod ios_picker;
#[cfg(target_os = "ios")]
mod ios_scene;
#[cfg(target_os = "ios")]
mod ios_share;
mod memory;

use lightcraft_engine::Session;
use lightcraft_ui_egui::{LightcraftApp, Services};

struct App {
    app: LightcraftApp,
    /// The touch screen has been announced to egui (see `raw_input_hook`).
    touch_announced: bool,
    /// iOS: the safe area as egui-winit last reported it, before the keyboard is added.
    /// Android: the insets last reported to egui.
    #[cfg(any(target_os = "ios", target_os = "android"))]
    insets: egui::SafeAreaInsets,
    /// The keyboard height last folded into the safe area.
    #[cfg(target_os = "ios")]
    keyboard: f32,
    /// When the window insets were last read (egui time, s).
    #[cfg(target_os = "android")]
    insets_polled: f64,
    /// The text field the soft keyboard types into (see `android_jni::start_text_input`), and
    /// what the keyboard was last told about it.
    #[cfg(target_os = "android")]
    text_input: Option<egui::Id>,
    #[cfg(target_os = "android")]
    field: android_jni::Field,
    /// The widget that had focus when a drag value or slider last reported itself (a click on
    /// one focuses its text field before that field exists).
    #[cfg(target_os = "android")]
    numeric_focus: Option<egui::Id>,
    /// Until when (egui time, s) the insets are read often after the keyboard went away: it
    /// slides out after the last frame, and nothing else would draw one.
    #[cfg(target_os = "android")]
    insets_settle: f64,
}

impl App {
    fn new(app: LightcraftApp) -> Self {
        App {
            app,
            touch_announced: false,
            #[cfg(any(target_os = "ios", target_os = "android"))]
            insets: egui::SafeAreaInsets::default(),
            #[cfg(target_os = "ios")]
            keyboard: 0.0,
            #[cfg(target_os = "android")]
            insets_polled: f64::NEG_INFINITY,
            #[cfg(target_os = "android")]
            text_input: None,
            #[cfg(target_os = "android")]
            field: android_jni::Field::default(),
            #[cfg(target_os = "android")]
            numeric_focus: None,
            #[cfg(target_os = "android")]
            insets_settle: f64::NEG_INFINITY,
        }
    }

    /// iOS: the soft keyboard joins the bottom of the safe area, so the layout (bottom bar, edit
    /// sheet, dialogs) sits above it rather than under it.
    #[cfg(target_os = "ios")]
    fn keyboard_inset(&mut self, raw: &mut egui::RawInput) {
        if let Some(insets) = raw.safe_area_insets {
            self.insets = insets;
        }
        let keyboard = ios_scene::keyboard_height();
        if raw.safe_area_insets.is_some() || keyboard != self.keyboard {
            self.keyboard = keyboard;
            let mut insets = self.insets;
            insets.0.bottom = insets.0.bottom.max(keyboard);
            raw.safe_area_insets = Some(insets);
        }
    }

    /// Android: egui-winit reports no safe area here, so the shell does: the window edges that
    /// system bars, the display cutout and the soft keyboard cover (see `android_jni`). With
    /// the keyboard in the bottom inset, the layout (bottom bar, edit sheet, dialogs) sits above
    /// it. Polled a few times a second, more often while a text field has focus, since nothing
    /// signals a change.
    #[cfg(target_os = "android")]
    fn window_insets(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        let now = raw.time.unwrap_or_default();
        let typing = ctx.egui_wants_keyboard_input() || now < self.insets_settle;
        if now < self.insets_polled + if typing { 0.08 } else { 0.5 } {
            return;
        }
        self.insets_polled = now;
        let Some(android) = ANDROID_APP.get() else { return };
        let Some(px) = android_jni::window_insets(android) else { return };
        let ppp = raw.viewports.get(&raw.viewport_id).and_then(|v| v.native_pixels_per_point).unwrap_or(1.0);
        let [left, top, right, bottom] = px.map(|p| p as f32 / ppp);
        let insets = egui::SafeAreaInsets(egui::epaint::MarginF32 { left, top, right, bottom });
        if insets != self.insets {
            self.insets = insets;
            raw.safe_area_insets = Some(insets);
        }
    }
}

/// Android: the soft keyboard follows egui's text fields. While one has focus (it asks for IME
/// input), the keyboard types into the activity's own text input, which supports composing
/// (CJK, Korean, Indic scripts), autocorrect and swipe typing; its edits arrive as egui IME events
/// (see `android_jni`). NativeActivity's own view only gets key events.
#[cfg(target_os = "android")]
impl App {
    /// After a frame: start, move or stop the text input with the focused text field.
    fn follow_text_field(&mut self, ctx: &egui::Context) {
        // the window insets are polled before frames (see `window_insets`): keep frames coming
        // while the keyboard can move (requested here, in the frame, so it isn't used up by it)
        if ctx.egui_wants_keyboard_input() || ctx.input(|i| i.time) < self.insets_settle {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        let Some(android) = ANDROID_APP.get() else { return };
        let ime = ctx.output(|o| o.ime.map(|i| i.purpose));
        let wanted = ime.and_then(|_| ctx.memory(|m| m.focused()));
        let purpose = ime.unwrap_or_default();
        // the field's text, from what it reported this frame (focus, a change, a selection), and
        // its selection, from its state
        let reported = ctx.output(|o| {
            o.events.iter().rev().find_map(|e| {
                let info = e.widget_info();
                (info.typ == egui::WidgetType::TextEdit).then(|| info.current_text_value.clone()).flatten()
            })
        });
        let field = |id: egui::Id, text: String| {
            let n = text.chars().count();
            let selection = egui::text_edit::TextEditState::load(ctx, id)
                .and_then(|s| s.cursor.char_range())
                .map(|r| r.as_sorted_char_range())
                .map_or((n, n), |r| (r.start.0.min(n), r.end.0.min(n)));
            android_jni::Field { text, selection, purpose, numeric: false }
        };
        // a drag value (or slider) reports itself when clicked, which focuses the text field it
        // turns into from the next frame on: remember which focus came with it
        if ctx.output(|o| o.events.iter().any(|e| matches!(e.widget_info().typ, egui::WidgetType::DragValue | egui::WidgetType::Slider))) {
            self.numeric_focus = ctx.memory(|m| m.focused());
        }
        // the same field, now for another purpose (e.g. a password shown or hidden): start afresh
        if wanted == self.text_input && purpose != self.field.purpose {
            self.text_input = None;
        }
        if wanted == self.text_input {
            if let Some(id) = wanted {
                let f = field(id, reported.unwrap_or_else(|| self.field.text.clone()));
                let f = android_jni::Field { numeric: self.field.numeric, ..f };
                if f != self.field {
                    android_jni::sync_text_input(android, &f);
                    self.field = f;
                }
            }
            return;
        }
        match wanted {
            Some(id) => {
                // (a field that reported nothing yet starts empty; its first report syncs)
                self.field = field(id, reported.unwrap_or_default());
                // a drag value (or slider) being typed into: the keyboard is the number pad
                self.field.numeric = self.numeric_focus == Some(id);
                android_jni::start_text_input(android, &self.field);
            }
            None => {
                android_jni::stop_text_input(android);
                self.insets_settle = ctx.input(|i| i.time) + 1.0;
            }
        }
        self.text_input = wanted;
    }

    /// Before a frame: the keyboard's edits since the last one.
    fn text_input_events(&mut self, raw: &mut egui::RawInput) {
        if self.text_input.is_none() {
            return;
        }
        if let Some(android) = ANDROID_APP.get() {
            raw.events.extend(android_jni::take_text_input(android).into_iter().map(egui::Event::Ime));
        }
    }
}

/// A photo pick is under way (Android: its result is polled, see `App::logic`).
#[cfg(target_os = "android")]
static PICKING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The activity, for its window insets and the photo picker (set once by `android_main`).
#[cfg(target_os = "android")]
static ANDROID_APP: std::sync::OnceLock<winit::platform::android::activity::AndroidApp> = std::sync::OnceLock::new();

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // photos picked (asynchronously) and copied in: the usual Import review
        // while a pick is under way, look for its photos a few times a second (they're copied in
        // the background; nothing else wakes the app when they're in)
        #[cfg(target_os = "android")]
        let picked: Vec<Vec<String>> = if PICKING.load(std::sync::atomic::Ordering::Relaxed) {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
            let got = ANDROID_APP.get().and_then(android_jni::take_picked);
            if got.is_some() {
                PICKING.store(false, std::sync::atomic::Ordering::Relaxed);
            }
            got.into_iter().filter(|p| !p.is_empty()).collect()
        } else {
            Vec::new()
        };
        #[cfg(target_os = "ios")]
        let picked = ios_picker::take_ready();
        #[cfg(any(target_os = "ios", target_os = "android"))]
        for paths in picked {
            if let Err(e) = self.app.run("file.addPhotos", serde_json::json!({ "paths": paths })) {
                log::warn!("import: {e}");
            }
        }
        memory::poll(&mut self.app.session);
        self.app.logic(ctx);
    }
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        // egui turns on touch drag-to-scroll once it has seen a touch, but hit-tests against the
        // previous frame, so the first swipe after launch would never scroll: announce the touch
        // screen up front with a cancelled touch
        if !std::mem::replace(&mut self.touch_announced, true) {
            raw.events.push(egui::Event::Touch {
                device_id: egui::TouchDeviceId(0),
                id: egui::TouchId(0),
                phase: egui::TouchPhase::Cancel,
                pos: egui::Pos2::ZERO,
                force: None,
            });
        }
        #[cfg(target_os = "ios")]
        self.keyboard_inset(raw);
        #[cfg(target_os = "android")]
        {
            self.window_insets(_ctx, raw);
            self.text_input_events(raw);
        }
        self.app.raw_input_hook(raw);
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.app.ui(ui);
        #[cfg(target_os = "android")]
        self.follow_text_field(ui.ctx());
    }
    fn on_exit(&mut self) {
        if let Err(e) = self.app.session.close_library() {
            log::error!("saving the library failed: {e}");
        }
    }
}

/// `export_dir`: where exports go by default (next to the library, inside the sandbox; on iOS
/// `Documents/`, so they show in the Files app too).
fn services(export_dir: String) -> Services {
    Services {
        export_dir: Some(export_dir),
        // a finished export opens the share sheet (AirDrop, Messages, Save to Photos, other apps)
        #[cfg(target_os = "ios")]
        share: Some(Box::new(ios_share::share)),
        #[cfg(target_os = "android")]
        share: Some(Box::new(|paths: &[String]| {
            if let Some(app) = ANDROID_APP.get() {
                android_jni::share(app, paths);
            }
        })),
        // atomic (temp file + sync + rename), inside the sandbox
        write_shared: Some(std::sync::Arc::new(lightcraft_engine::export::write_file)),
        write: Some(Box::new(lightcraft_engine::export::write_file)),
        png: Some(Box::new(|img: &lightcraft_raster::Rgba8| {
            lightcraft_codecs::encode_png(&lightcraft_codecs::EncodeImage::rgba8(img), &lightcraft_codecs::EncodeMeta::default()).unwrap_or_default()
        })),
        // File ▸ Add Photos… shows the system picker; the photos arrive later (see `App::logic`)
        #[cfg(target_os = "ios")]
        pick_files: Some(Box::new(|| {
            ios_picker::present();
            Vec::new()
        })),
        #[cfg(target_os = "android")]
        pick_files: Some(Box::new(|| {
            if let Some(app) = ANDROID_APP.get() {
                let dir = app.internal_data_path().unwrap_or_default().join("LightCraft Library").join("Picked");
                PICKING.store(true, std::sync::atomic::Ordering::Relaxed);
                android_jni::pick_photos(app, &dir.to_string_lossy());
            }
            Vec::new()
        })),
        ..Default::default()
    }
}

/// Logs to stderr on iOS (Xcode / `simctl launch --console-pty`), to logcat on Android (tag
/// `LightCraft`; Android discards stderr).
struct StderrLog;

impl log::Log for StderrLog {
    fn enabled(&self, m: &log::Metadata) -> bool {
        m.level() <= log::Level::Info
    }
    fn log(&self, r: &log::Record) {
        if self.enabled(r.metadata()) {
            #[cfg(target_os = "android")]
            android_jni::logcat(r.level(), &format!("[{}] {}", r.target(), r.args()));
            #[cfg(not(target_os = "android"))]
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
    #[cfg(target_os = "ios")]
    ios_scene::register();
    log::info!("library at {}", library_dir.display());
    eframe::run_native(
        "LightCraft",
        options,
        Box::new(move |_cc| {
            #[cfg(target_os = "ios")]
            if let Ok(h) = raw_window_handle::HasWindowHandle::window_handle(_cc) {
                ios_scene::set_window(h.as_raw());
            }
            #[cfg(target_os = "android")]
            {
                if let Some(android) = ANDROID_APP.get() {
                    android_jni::register_wake(android, &_cc.egui_ctx);
                }
                // CJK typed with the keyboard (and Korean, which no bundled font has) shows in the
                // system's CJK font rather than as boxes
                if let Some((bytes, index)) = android_jni::system_cjk_font() {
                    let font = egui::FontData { index, ..egui::FontData::from_static(bytes) };
                    lightcraft_ui_egui::theme::add_fallback_font("system CJK", font);
                }
            }
            #[cfg(target_os = "ios")]
            {
                ios_scene::set_context(&_cc.egui_ctx);
                ios_picker::init(library_dir.join("Picked"), _cc.egui_ctx.clone());
            }
            let export_dir = library_dir.with_file_name("LightCraft Exports").to_string_lossy().to_string();
            let mut s = Session::new().with_fs().with_system_clock();
            let t0 = std::time::Instant::now();
            match s.open_library(&library_dir, true) {
                Ok(r) => {
                    let replayed = r.replayed;
                    log::info!("library: {} photos, {replayed} log records replayed ({:.1} ms)", s.catalog.len(), t0.elapsed().as_secs_f64() * 1e3);
                }
                Err(e) => log::error!("can't open library {}: {e}", library_dir.display()),
            }
            memory::configure(&mut s, &_cc.egui_ctx);
            Ok(Box::new(App::new(LightcraftApp::new(s, services(export_dir)))))
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
    let _ = ANDROID_APP.set(app.clone());
    // the activity's onTrimMemory, passed on within a second whether or not frames are drawn
    let trims = app.clone();
    let _ = std::thread::Builder::new().name("trim-memory".into()).spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            if android_jni::take_trim_level(&trims) > 0 {
                memory::warn();
            }
        }
    });
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
