# LightCraft mobile (spike)

The desktop egui UI and engine, **unmodified**, in a thin iOS/iPadOS or Android shell. It's a
feasibility spike, not a usable mobile app. The assessment, the effort estimate and the blockers
are in [docs/porting-report.md](docs/porting-report.md).

| iPad (iOS 26.5 Simulator) | iPhone | Android tablet (API 35 emulator) |
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

## Known problems

- **iOS 27 crashes at launch.** winit 0.30 doesn't adopt the UIScene lifecycle. Use an iOS 26
  runtime.
- **Phone layout.** The desktop panels squeeze the photo to a sliver, and the top bar sits
  under the status bar and Dynamic Island. There are no safe areas.
- **Touch.** Drags fight scrolling (the edit panel doesn't scroll), there's no pinch zoom,
  some controls appear only on hover, and hit targets are small.
- **No pickers, sharing or memory budget yet.** All `Services` except export writing are
  `None`, so nothing can be imported beyond the demo photos.
- **Android.** The window uses GL, because the emulator's Vulkan surface fails to configure;
  `adb shell touch /data/local/tmp/lc_vulkan` tries Vulkan. Logs go to stderr, which Android
  discards.
