# LightCraft mobile (spike)

The desktop egui UI and engine, **unmodified**, in a thin iOS/iPadOS or Android shell. It's a
feasibility spike, not a usable mobile app. The assessment, the effort estimate and the blockers
are in [docs/porting-report.md](docs/porting-report.md).

| iPad (iOS 26.5 Simulator) | iPhone, before the compact layout | Android tablet (API 35 emulator) |
|---|---|---|
| ![iPad](docs/screens/ipad-exposure-drag.jpg) | ![iPhone](docs/screens/iphone.jpg) | ![Android](docs/screens/android-tablet-slider.jpg) |

## Build and run

The rest of the workspace builds this crate empty. It only has code for `target_os = "ios"` and
`"android"`.

```sh
rustup target add aarch64-apple-ios-sim aarch64-linux-android
apps/lightcraft-mobile/scripts/ios-sim.sh            # booted simulator, or pass a device name/UDID
apps/lightcraft-mobile/scripts/android-apk.sh --run  # builds the APK; --run installs and starts it via adb
```

If `cargo` on `PATH` isn't rustup's, set `CARGO` (for example
`CARGO="rustup run stable cargo"`) so the mobile targets' standard libraries are found.

## What works

- The library opens on disk in the sandbox, seeded with the demo photos. On iOS it's
  `Documents/LightCraft Library`; on Android it's the app's internal data dir.
- The grid, the loupe and every develop control work. A touch drag on a slider renders live.
- GPU compute runs on Metal (iOS). Edits survive a hard kill, because the journal is fsynced
  per command.

## Phone layout

Below 600pt of width (inside the safe area), `lightcraft-ui-egui` switches to a compact layout
(`crates/ui-egui/src/panels/compact.rs`). It's used on phones, iPad Split View and phone
browsers:

- **Top bar:** My Photos, Back, Menu, a search field, Filter and Export.
- **Detail view:** the photo on top, and a bottom sheet with a tool row (Presets, Edit, Crop,
  Remove, Masking, Info …). The sheet shows that panel at full width; Hide puts it away.
- **My Photos:** a full-width drawer that closes when you pick a source.
- **Grid:** about three thumbnails across.
- **Bottom bar:** view modes, Sort, Filmstrip and an Edit toggle.

The whole UI stays inside the iOS safe area (status bar, Dynamic Island, home indicator).

| | Edit sheet | Grid | My Photos |
|---|---|---|---|
| iPhone 17 (iOS 27) | ![sheet](docs/screens/phone-edit.jpg) | ![grid](docs/screens/phone-grid.jpg) | ![drawer](docs/screens/phone-drawer.jpg) |

## Known problems

- **iOS scene lifecycle is adopted by the shell, not winit.** winit 0.30 has no UIScene support,
  and iOS 27 kills apps without it. `src/ios_scene.rs` and the scene manifest in `ios/Info.plist`
  attach winit's window to the scene, which works on iOS 26.5 and 27.0. There's one scene only
  (no multiwindow), and this goes away when eframe moves to winit 0.31.
- **Phone layout is a first pass.** Narrow windows get a compact layout (see below), but
  dialogs (Import, Export, Settings) are still desktop-sized, and there's no landscape-specific
  arrangement.
- **Touch.** A swipe that starts on a slider's track moves the slider, not the sheet. There's
  no pinch zoom, some controls appear only on hover, and the desktop layout's hit targets are
  small.
- **No pickers, sharing or memory budget yet.** All `Services` except export writing are
  `None`, so nothing can be imported beyond the demo photos.
- **Android.** The window uses GL, because the emulator's Vulkan surface fails to configure;
  `adb shell touch /data/local/tmp/lc_vulkan` tries Vulkan. Logs go to stderr, which Android
  discards.
