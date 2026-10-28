# iOS / AltStore – Status

Ehrliche Übersicht, was gebaut, geprüft und offen ist. Begriffe strikt getrennt:
**gebaut** (CI hat eine IPA erzeugt) · **simulatorgetestet** · **auf iPhone getestet** · **AltStore-installiert**.

## Ausgangsbasis

| | |
|---|---|
| Basis | `fmsouza/lightcraft` Branch `mobile-apps`, Commit `476e55b9` (27.10.2026, Merge von upstream/main) |
| Upstream-PR | storytold/lightcraft#818 – am 28.10.2026 **offen**, kein Merge; laut PR „rebased branch has not been run on a device yet“ |
| Arbeitsbranch | `ios-altstore` in `Lightblue87/lightcraft` (Fork, öffentlich, Standard-Branch) |
| Bundle-ID (eigen, stabil) | `local.lightcraft.ios` (Upstream-Default `ai.storyteller.lightcraft.mobile` bleibt für `ios-device.sh`) |
| Version | `CFBundleShortVersionString` = Workspace-Version (0.14.0), `CFBundleVersion` = Commit-Anzahl, Commit als `LCBuildCommit` in der Info.plist |

## Phase 0 – Bestandsaufnahme (28.10.2026)

Gelesen: `AGENTS.md`, `CLAUDE.md`, `apps/lightcraft-mobile/README.md`, `docs/porting-report.md`,
`scripts/ios-sim.sh`, `scripts/ios-device.sh`, `ios/Info.plist`, `Cargo.toml` (Crate und Workspace).

Fakten aus dem Code:
- iOS-Binary `lightcraft-ios` (Rust, eframe/egui + wgpu/Metal), Bundle wird ohne Xcode-Projekt per Skript
  gebaut: `cargo build --target aarch64-apple-ios`, Binary + `ios/Info.plist` in `LightCraft.app`, Icon per
  `xcrun actool` (→ `Assets.car`, `CFBundleIcons` in die Info.plist gemerged).
- `ios-device.sh` braucht Zertifikat, Provisioning-Profil und Team-ID, bettet das Profil ein und signiert mit
  Entitlements – **nicht** für AltStore geeignet, bleibt unverändert.
- `Info.plist`: UIScene-Manifest mit `LCSceneDelegate` (iOS-27-Pflicht), `MinimumOSVersion` 17.0,
  iPhone + iPad, Dateien-App-Freigabe. **Keine** `NS…UsageDescription` – passt, da PHPicker und Share-Sheet
  keine Fotomediathek-Berechtigung brauchen.
- Rust-Mindestversion 1.95; Release-Profil mit `debug = "line-tables-only"`.

Build-Umgebung: Arbeitsumgebung dieser Sitzung ist Linux ohne Xcode; ein Mac ist nicht vorhanden → Build
über **GitHub Actions mit macOS-Runner**. Ein lokaler `cargo check`/`clippy` für `aarch64-apple-ios` wurde in
dieser Sitzung **nicht ausgeführt** (Kompilieren fremden Codes war in der Sitzungsumgebung gesperrt).

## Phase 1 – Unsigned IPA

### Geändert
| Datei | Zweck |
|---|---|
| `apps/lightcraft-mobile/scripts/ios-ipa.sh` | Release-Build für `aarch64-apple-ios`, `.app` wie `ios-device.sh` aber **ohne** Profil/Entitlements/Signierung, setzt Bundle-ID/Version/Build/Commit, `IPHONEOS_DEPLOYMENT_TARGET` = `MinimumOSVersion`, packt `Payload/LightCraft.app` → `dist/ios/LightCraft-iOS-<ver>-<build>-<commit>-unsigned.ipa`, plus dSYM-Zip |
| `apps/lightcraft-mobile/scripts/verify-ipa.py` | Prüft die IPA (nur Python-Standardbibliothek): genau ein `Payload/*.app`, Info.plist-Pflichtfelder, Executable vorhanden + ausführbar, Mach-O arm64 **iphoneos** (nicht Simulator), Deployment-Target ≤ `MinimumOSVersion`, UIScene-Manifest, Launch-Screen, Icon, keine Profile/Zertifikate/Schlüssel/Entitlements |
| `.github/workflows/ios-ipa.yml` | Manuell (`workflow_dispatch`), macOS-Runner wählbar, optional Xcode-Version; fmt, `xtask layers`, Clippy `-D warnings` für das iOS-Target, Build, Verifikation, Upload als Artefakt (30 Tage). Keine Secrets, keine Releases |
| `apps/lightcraft-mobile/CLAUDE.md` | Projektvorgabe + Abgleich mit Upstream-Regeln |
| `CLAUDE.md` (Root) | Verweis auf die obige Datei, Upstream-Inhalt unverändert |
| `apps/lightcraft-mobile/docs/altstore-installation.md` | Installations- und Update-Anleitung |

### Tatsächlich ausgeführte Prüfungen (Linux, 28.10.2026)
| Prüfung | Ergebnis |
|---|---|
| `sh -n` + `shellcheck -s sh` auf `ios-ipa.sh` | ✅ ohne Befund |
| `actionlint` (inkl. shellcheck) auf `ios-ipa.yml` | ✅ ohne Befund |
| `verify-ipa.py` gegen 9 synthetische IPAs (gültig; Simulator-Binary; x86-CPU; Deployment-Target zu hoch; ohne Scene-Manifest; mit `embedded.mobileprovision`; mit Private Key; ohne Ausführungsrecht; zwei Apps) | ✅ gültige besteht, alle 8 fehlerhaften werden mit passender Meldung abgelehnt |
| Versions-Extraktion aus `Cargo.toml` | ✅ `0.14.0` |
| `cargo fmt/clippy/build` für `aarch64-apple-ios` lokal | ❌ nicht gelaufen (siehe oben); in CI siehe unten |

### CI-Läufe (`Lightblue87/lightcraft`, Workflow „iOS IPA“)
| Lauf | Commit | Runner | Ergebnis |
|---|---|---|---|
| 40729960388 | `d42d31ee` | macos-latest (arm64) | abgebrochen: 15 min kein Runner zugeteilt (GitHub-Kapazität), kein Schritt lief |
| 2. Lauf | `02520a94` | macos-latest (arm64) | ebenso abgebrochen |
| [40735026283](https://github.com/Lightblue87/lightcraft/actions/runs/40735026283) | `55833e89` | **macos-15-intel** | ✅ **grün**, 36 min |

Im grünen Lauf (28.10.2026) ausgeführt: `cargo fmt --all --check` ✅, `cargo xtask layers` ✅,
`cargo clippy --release -p lightcraft-mobile --target aarch64-apple-ios -- -D warnings` ✅, Release-Build ✅,
`verify-ipa.py` ✅. Umgebung: Xcode 16.4, iphoneos SDK 18.5, rustc 1.99.1 (x86_64-Host, Cross-Build nach arm64).

### Ergebnis – Meilenstein A erreicht (gebaut, nicht installiert)
Artefakt `LightCraft-iOS-0.14.0-4154-55833e895540-unsigned` (30 Tage im Run):
- `LightCraft-iOS-0.14.0-4154-55833e895540-unsigned.ipa`, 26,8 MB, SHA-256
  `3858d14ec27a3e0e64c1ff6febdb34f89c87569ef80384fdfc6238d0847b5947` (lokal nachgeprüft: IPA OK)
- Inhalt: `Payload/LightCraft.app/` mit `lightcraft-ios` (76,6 MB, arm64, Plattform iOS, minOS 17.0, ohne
  Signatur), `Info.plist`, `Assets.car`, zwei Icon-PNGs. Keine Usage-Descriptions, keine Profile.
- Bundle-ID `local.lightcraft.ios`, Version 0.14.0 (Build 4154), `LCBuildCommit` 55833e895540.
- Kein dSYM-Zip: cargo erzeugt mit `debug = "line-tables-only"` für iOS kein `.dSYM`; die Zeilentabellen stecken
  im Binary.

Weiterhin **nicht belegt**: dass AltStore die IPA ohne jede Signatur annimmt und signiert, und dass die App auf
einem echten iPhone startet. Das gebaute SDK ist iOS 18.5 (Xcode 16.4 auf dem Intel-Image); die App läuft
damit auch auf neueren iOS-Versionen, nutzt aber keine neueren SDK-Funktionen. Für iOS 27 ist das UIScene-Manifest
entscheidend, und es ist enthalten.

## Offene Risiken
- winit 0.30 ohne UIScene: die Shell-Lösung ist nur auf Simulatoren (iOS 26.5/27.0) belegt.
- Speicher: Budgets aus `os_proc_available_memory` – auf echtem iPhone ungetestet; 24-MP-Test steht aus.
- HEIC wird beim Import zu JPEG konvertiert (kein HEIC-Decoder); RAW/ProRAW aus der Fotomediathek ungetestet.
- Ohne `CRAFT_FONTS_DIR` fehlen CJK-Glyphen (für Deutsch/Englisch unerheblich).
- Kostenloses Apple-Konto: 7-Tage-Refresh, max. 3 aktive Apps.

## Checkliste echtes iPhone (Phase 2)
Nur Ausgeführtes abhaken.

- [ ] IPA aus GitHub Actions geladen (Commit: ____)
- [ ] In AltStore Classic installiert (iPhone-Modell / iOS-Version: ____; Fehlertext: ____)
  - 28.10.2026, 19:40: Versuch 1 über Remote AltServer fehlgeschlagen, `AltStore.OperationError 1500` „AltStore couldn't reach the remote AltServer“ (Verbindungsfehler WLAN/LocalDevVPN, vor jeder Verarbeitung der IPA; sagt nichts über die IPA)
  - 28.10.2026, 19:46: Versuch 2 erreicht den Server, scheitert aber an der Apple-Anmeldung: `AltStore.AppleDeveloperError 3000`, HTTP 503 von Apples Authentifizierungsservern (Apple-Störung oder Rate-Limit; ebenfalls vor dem Signieren)
- [ ] Start, Icon, Katalog, Hoch-/Querformat
- [ ] JPEG importiert, Regler ändern das Bild
- [ ] HEIC importiert (Konvertierung nach JPEG dokumentiert)
- [ ] DNG/ProRAW (falls vorhanden)
- [ ] Belichtung + Weißabgleich geändert, App beendet, Änderungen nach Neustart vorhanden
- [ ] JPEG exportiert, über Teilen/Dateien/Fotos geprüft
- [ ] Zoom und Gesten
- [ ] 24-MP-Foto
- [ ] Hintergrund/Vordergrund, Export-Abbruch
- [ ] Update mit gleicher Bundle-ID behält Daten
- [ ] Refresh nach 7 Tagen

## Nächster Schritt
Phase 2: die IPA aus Lauf 40735026283 per AltStore Classic installieren (`altstore-installation.md`) und
Ergebnis, iPhone-Modell, iOS-Version sowie etwaige AltStore-Fehlertexte oder Absturzberichte hier eintragen.
