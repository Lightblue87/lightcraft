# iOS / AltStore – Status

Ehrliche Übersicht, was gebaut, geprüft und offen ist. Begriffe strikt getrennt:
**gebaut** (CI hat eine IPA erzeugt) · **simulatorgetestet** · **auf iPhone getestet** · **AltStore-installiert**.

## Ausgangsbasis

| | |
|---|---|
| Basis | `fmsouza/lightcraft` Branch `mobile-apps`, Commit `476e55b9` (27.10.2026, Merge von upstream/main) |
| Upstream-PR | storytold/lightcraft#818 – am 28.10.2026 **offen**, kein Merge; laut PR „rebased branch has not been run on a device yet“ |
| Arbeitsbranch | `ios-altstore` (nur lokal, noch nicht gepusht) |
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
| `cargo fmt/clippy/build` für `aarch64-apple-ios` | ❌ **nicht gelaufen** (siehe oben) |
| macOS-Build / echte IPA | ❌ **nicht gelaufen** – Branch ist noch nicht in einem GitHub-Repo mit Actions |

### Ergebnis
**Es existiert noch keine echte IPA.** Meilenstein A ist erst erfüllt, wenn ein Actions-Run grün ist und das
Artefakt herunterladbar ist.

Nicht belegte Annahmen, die der erste CI-Lauf klärt:
- `macos-latest` hat ein Xcode, dessen `actool` den Asset-Katalog für `iphoneos` kompiliert (bei
  Simulator/Device lokal bei fmsouza belegt, auf dem Runner nicht).
- `cargo fmt --all --check` und Clippy `-D warnings` für das iOS-Target sind auf dem Branch-Stand grün
  (Upstream-CI prüft das iOS-Target nicht). Schlägt das fehl, ist der Befund der erste Fix.
- AltStore akzeptiert eine IPA, deren Binary nur die vom Linker gesetzte Ad-hoc-Signatur trägt (Normalfall bei
  unsignierten CI-IPAs, aber erst mit dem Gerätetest belegt).

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
Branch `ios-altstore` in einen eigenen GitHub-Fork pushen und den Workflow **iOS IPA** starten. Ein
fehlgeschlagener Lauf liefert den konkreten Blocker (Log); ein grüner Lauf liefert die IPA für Phase 2.
