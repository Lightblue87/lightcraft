CLAUDE.md — LightCraft iOS (private AltStore Classic build)

> Projektanweisung für Claude Code. Sprache für Kommunikation und Dokumentation: **Deutsch**.  
> Stand der Ausgangsrecherche: **28.10.2026**. Alle Repository-Fakten beim Arbeitsbeginn erneut prüfen.  
> **Primärziel:** Eine **echte, auf einem iPhone lauffähige, unsignierte `.ipa`** der LightCraft-App erzeugen, die anschließend **AltStore Classic mit einer kostenlosen Apple-ID signieren und installieren** kann.

1. Projektziel und Grenzen

Wir bauen keinen neuen Fotoeditor von Grund auf, sondern entwickeln den vorhandenen LightCraft-iOS-Port zu einer nutzbaren privaten iPhone-App weiter. Verwendet werden sollen die vorhandene Rust-Bildbearbeitungsengine, die bestehende egui-Oberfläche und die bereits geschaffene iOS-Hülle. Der erste Erfolg ist nicht ein hübsches Mock-up, sondern eine reproduzierbar gebaute IPA mit funktionierender Import–Bearbeiten–Export-Kette.

Für die erste Version gilt:

• Zielplattform: iPhone, arm64 / aarch64-apple-ios, aktuelle iOS-Versionen; zunächst genau ein reales iPhone testen. iPad-Unterstützung darf erhalten bleiben, hat aber keine Priorität.
• Distribution: AltStore Classic, privat, kein App Store, kein TestFlight, keine Apple-Notarisierung, kein kostenpflichtiges Developer-Programm voraussetzen.
• Art des Build-Artefakts: unsigned / re-signable IPA. AltStore signiert die App bei der Installation. „Keine Zertifizierung“ bedeutet nicht „keine Signierung“.
• Offline-first: Bilder, Katalog, Presets und Bearbeitung lokal. Kein Benutzerkonto, kein Cloud-Zwang, keine Telemetrie oder Tracker.
• Budget: keine laufenden API-/KI-Kosten; vorhandene optionale KI-Modelle nur bei tatsächlicher Offline-Unterstützung, nicht als MVP-Abhängigkeit.
• Möglichst große Funktionsübernahme aus LightCraft; keine unbelegten Aussagen über vollständige Lightroom-Parität oder alle RAW-Formate.
• Eine spätere Veröffentlichung ist denkbar, aber ausdrücklich nicht Gegenstand des MVP.

Nicht-Ziele für v0.1

Kein React-Native-/Flutter-/SwiftUI-Neubau, kein vollständiges Redesign, kein Adobe-Cloud-Sync, kein Abo-/Zahlungssystem, kein Android-Release, keine App-Store-Optimierung und keine neuen generativen KI-Funktionen. Keine Änderung am Bildbearbeitungskern, sofern für den mobilen Betrieb nicht notwendig.

2. Verbindliche Quellen und Codebasis

Upstream: https://github.com/storytold/lightcraft
Bereits vorhandener Mobile-Port: https://github.com/storytold/lightcraft/pull/818
Port-Fork/-Branch: https://github.com/fmsouza/lightcraft/tree/mobile-apps
Mobile-README: https://github.com/fmsouza/lightcraft/blob/mobile-apps/apps/lightcraft-mobile/README.md
Portierungsbericht: https://github.com/fmsouza/lightcraft/blob/mobile-apps/apps/lightcraft-mobile/docs/porting-report.md
Upstream-Agentenregeln: https://github.com/storytold/lightcraft/blob/main/AGENTS.md
Bestehende Claude-Regeln: https://github.com/fmsouza/lightcraft/blob/mobile-apps/CLAUDE.md
AltStore Classic: https://faq.altstore.io/altstore-classic/your-altstore

WICHTIG: Im Original-Repository existieren bereits AGENTS.md und CLAUDE.md. Nicht blind überschreiben oder deren Regeln verwerfen. Diese Projektanweisung ergänzt sie als produktbezogene Vorgabe. Vor Integration in einen Fork vorhandene Regeln prüfen, Konflikte dokumentieren und die Anweisungen zusammenführen. Die Upstream-Regeln zu Clean-Room-Entwicklung, Lizenzen, Tests und Crate-Grenzen bleiben erhalten.

Annahmen, die nicht als bewiesen gelten dürfen

• PR #818 war am 28.10.2026 offen und als Feasibility Spike / Prototyp ausgewiesen, nicht als releasefähige App.
• Der Autor hat Simulator-Tests beschrieben; der rebased Branch war laut PR noch nicht erneut auf echter Hardware validiert.
• Bereits implementiert bzw. behauptet: iOS-UIScene-Lifecycle, Touch-Layout, Foto-/Dateiauswahl, Export-Share-Sheet, Metal, RAM-Budgets, gekachelte Exporte und lokaler Katalog. Quellcode und reales Verhalten prüfen, nicht lediglich die PR-Beschreibung übernehmen.
• apps/lightcraft-mobile/scripts/ios-device.sh ist ein signierender und installierender Device-Build. Er liefert nicht automatisch eine AltStore-taugliche unsigned IPA. Diesen Unterschied beachten.

3. Arbeitsregeln für Claude

1. Vor Codeänderungen Bestandsaufnahme: Repository, aktueller Branch/Commit, Abhängigkeiten, verfügbare Build-Umgebung, vorhandene Shell, Artefakte und offene TODOs feststellen.
2. Nicht neu implementieren, was der Port schon kann. Zuerst lesen, dann gezielt ergänzen.
3. Arbeite selbstständig in kleinen, testbaren Schritten. Keine unnötigen Rückfragen bei Entscheidungen, die sich aus Projektziel und Codebasis ergeben. Bei externen Blockern konkret sagen, welche Eingabe bzw. Hardware fehlt.
4. Keine erfundenen Erfolge. „Gebaut“, „simulatorgetestet“, „auf iPhone getestet“ und „AltStore-installiert“ strikt unterscheiden. Ohne Device-Zugriff niemals reale iPhone-Tests behaupten.
5. Nicht ungefragt deployen/pushen, keine Accounts anlegen und keine fremden PRs verändern. Nur am eigenen Arbeitsbranch arbeiten; vor destruktiven Git-Aktionen Zustand sichern.
6. Architektur der Engine erhalten, Funktionen nur gezielt hinter iOS-spezifischen Interfaces ergänzen. Keine unnötigen neuen Frameworks.
7. Neue unsafe-Blöcke nur in den schon vorgesehenen und begründeten FFI-Grenzen, jeweils mit // SAFETY: und passender Prüfung; AGENTS.md beachten.
8. Keine Secrets ins Repository oder in CI-Logs: keine Apple-ID, Passwörter, Zertifikate, Provisioning Profiles, persönliche Fotos, API-Keys, Gerätekennungen.
9. Keine Adobe-Assets oder proprietären Profile/LUTs übernehmen. Namens- und Lizenzhinweise des Open-Source-Projekts erhalten; Lizenzdateien und Drittanbieterpflichten überprüfen.
10. Nach jedem Meilenstein: implementierte Änderungen, tatsächlich ausgeführte Prüfungen, nicht getestete Funktionen, offene Risiken und nächsten Schritt in docs/ios-status.md dokumentieren.

4. Technische Ausgangslage (prüfen, nicht erfinden)

Die bestehende Rust-Workspace-Struktur enthält unter anderem:

• apps/lightcraft-mobile/ — iOS-/Android-Shell und Dokumentation.
• apps/lightcraft-mobile/scripts/ios-sim.sh — Simulator-Build und Start.
• apps/lightcraft-mobile/scripts/ios-device.sh — Build/Signierung/Installation auf Device über IOS_*-Variablen.
• apps/lightcraft-mobile/docs/porting-report.md — Bewertung und Blocker.
• crates/ui-egui/ — bestehende Oberfläche und kompaktes Touch-Layout.
• Rust-Crates für Engine, RAW-/Bilddaten, Entwicklung, Katalog und GPU.

Die Portierung nutzt Rust + egui/eframe + wgpu/Metal, nicht eine neu geschriebene SwiftUI-App. Die aktuelle iOS-Shell berücksichtigt iOS-27-UIScene-Anforderungen. Diese Mechanik nicht versehentlich entfernen.

Erste Bestandsaufnahme im Terminal

# Falls noch kein Projektverzeichnis existiert:
git clone https://github.com/fmsouza/lightcraft.git lightcraft-ios
cd lightcraft-ios
git fetch origin
git switch -c ios-altstore origin/mobile-apps

# Bei bestehendem Checkout stattdessen Status und Remotes prüfen:
git status --short
git branch --show-current
git remote -v
git log -1 --oneline

# Zuerst lesen:
cat AGENTS.md
cat CLAUDE.md
cat apps/lightcraft-mobile/README.md
cat apps/lightcraft-mobile/docs/porting-report.md
sed -n '1,220p' apps/lightcraft-mobile/scripts/ios-sim.sh
sed -n '1,220p' apps/lightcraft-mobile/scripts/ios-device.sh

Hinweis: Die git clone-Befehle dienen nur zum Start in einem leeren Arbeitsverzeichnis. Wenn bereits ein eigenes Repo mit Commits existiert, nichts ersetzen oder überschreiben. Vor dem Checkout Branch-Status und eventuelle lokale Änderungen sichern. Wenn PR #818 inzwischen gemergt wurde oder der Branch verschwand, den aktuellen Upstream-Stand prüfen und den besten vorhandenen mobilen Commit als Basis nehmen; Entscheidung dokumentieren.

5. Build-Strategie: AltStore statt Xcode-Signing

5.1 Grundprinzip

Ein normales iPhone erfordert eine gültige Code-Signatur. Für unseren Workflow muss Claude eine unsignierte, aber technisch vollständige iPhoneOS-IPA erzeugen. AltStore Classic übernimmt beim Sideloading das Signieren mit der Apple-ID. Keine Apple-Zugangsdaten in Build-Skripten oder CI verwenden.

Die erwartete ZIP-Struktur ist:

LightCraft-iOS-<version>-unsigned.ipa
└── Payload/
    └── LightCraft.app/
        ├── Info.plist
        ├── <tatsaechlicher-CFBundleExecutable-Name>
        ├── Assets.car / App-Icons (je nach Build)
        └── weitere erforderliche Ressourcen

<...> sind Platzhalter: Die tatsächlichen Dateinamen aus dem bestehenden Projekt auslesen. Keine Simulator-App paketieren. Das iOS-Device-Binary muss arm64 für iphoneos sein und die korrekte minimale iOS-Version und vollständige Ressourcen enthalten.

5.2 Vorrangig: reproduzierbarer Build über macOS

• Native iOS-Kompilierung mit Apple-SDK erfordert eine passende macOS-/Xcode-Umgebung. Bei einem Windows-Entwicklungsrechner ist GitHub Actions mit macos-Runner ein sinnvoller Weg.
• Ein eigener Mac mit Xcode ist optional, nicht Voraussetzung für die Entwicklung am Windows-PC, solange der unsigned iOS-Build über CI gelingt.
• ios-device.sh zunächst nicht durch Ausschalten seiner Signierung beschädigen. Besser ein separates, wartbares apps/lightcraft-mobile/scripts/ios-ipa.sh oder ein gleichwertiges Build-Target entwickeln, das den vorhandenen Device-Build soweit möglich wiederverwendet.
• rustup target add aarch64-apple-ios für echte iPhones; aarch64-apple-ios-sim ist nur für Apple-Silicon-Simulatoren relevant.
• Die vorhandene Rust-Toolchain, Cargo-Features, Xcode-Version und Packaging-Aufrufe aus den Skripten ermitteln, nicht blind eine erfundene xcodebuild-Projektstruktur verwenden.
• Keine lokalen Signierungszertifikate erzwingen. Falls ein Packaging-Schritt Signierung erwartet, die tatsächliche Ursache lösen und unsigned Artefakte separat erzeugen.

5.3 GitHub Actions (Priorität für Windows-Workflow)

Erstelle .github/workflows/ios-ipa.yml mit:

• manuell auslösbar über workflow_dispatch;
• macOS-Runner und dokumentierter, funktionierender Xcode-/Rust-Toolchain;
• Caching für Cargo, ohne geheimnisbehaftete Daten;
• dem nötigen aarch64-apple-ios-Target;
• cargo fmt --all --check, passende cargo check-/Clippy-Schritte und den projektseitigen Layer-/Test-Checks, soweit technisch auf dem Runner ausführbar;
• anschließendem unsigniertem Release-Device-Build;
• Validierung der erzeugten .app und Erzeugung der .ipa;
• Upload der .ipa als GitHub-Actions-Artefakt;
• verständlicher Fehlerausgabe; keine irreführende grüne Pipeline, wenn das Artefakt fehlt.

Keine automatischen GitHub Releases, keine Veröffentlichung und kein Upload zu fremden Diensten. Das Artefakt für den privaten Download reicht. Falls ein privates GitHub-Repo genutzt wird, Runner-Kontingente berücksichtigen.

5.4 IPA-Verifikation

Vor dem Upload automatisiert prüfen:

• ZIP enthält genau die gültige Payload/*.app-Struktur;
• Info.plist vorhanden; CFBundleExecutable entspricht der enthaltenen ausführbaren Datei;
• Geräte-Build, kein Simulator-Build (arm64, iphoneos), notwendige Frameworks/Ressourcen vollständig;
• stabiler CFBundleIdentifier (ein dauerhaftes, eigenes Bundle-ID-Schema wählen, nicht bei jedem Build wechseln);
• sinnvolle CFBundleShortVersionString und CFBundleVersion;
• Icon und Launch-Konfiguration korrekt; UIScene-Konfiguration bleibt für iOS 27 erhalten;
• erforderliche Nutzungsbeschreibungen (Info.plist) nur für tatsächlich genutzte APIs; beim System-Fotopicker keine pauschale Vollzugriffsberechtigung erzwingen;
• keine eingebetteten privaten Schlüssel, Accounts, Provisioning Profiles oder unnötigen Entitlements;
• IPA kann grundsätzlich von einer externen Sideload-Signierung verarbeitet werden (vollständig belegen erst nach AltStore-Test).

Akzeptanz für Meilenstein A: Eine herunterladbare, echt erzeugte .ipa befindet sich in einem erfolgreichen Actions-Run, mit Log und nachvollziehbarer Build-Commit-ID. Ein bloßes Build-Skript oder eine leere IPA erfüllt diesen Meilenstein nicht.

6. Funktionsumfang für v0.1

|Priorität|Funktion                     |Abnahmebedingung                                                                        |
|---------|-----------------------------|----------------------------------------------------------------------------------------|
|P0       |Start auf dem iPhone         |Öffnet ohne Crash, Hintergrund/Vordergrund funktionieren                                |
|P0       |Fotobibliothek / Dateiauswahl|JPEG und HEIC importierbar; Originale bleiben unangetastet                              |
|P0       |Bildbearbeitung              |Belichtung, Weißabgleich, Kontrast und weitere vorhandene Basisregler reagieren sichtbar|
|P0       |Nicht-destruktives Speichern |Änderungen bleiben nach App-Neustart erhalten                                           |
|P0       |Export                       |JPEG (zunächst) kann lokal gespeichert oder über iOS-Teilen geöffnet werden             |
|P0       |Große Bilder                 |Keine vermeidbaren Abstürze beim 24-MP-Test; bei Grenzen verständliche Meldung          |
|P0       |AltStore-IPA                 |Download, Signierung und Installation der unsigned IPA nachweisbar                      |
|P1       |RAW/DNG/ProRAW               |Tatsächlich unterstützte Formate gerätebasiert prüfen; ungetestete Formate kennzeichnen |
|P1       |Presets, Kurven, Crop, Masken|Vorhandene Engine-Funktionen auf Touch nutzbar, soweit stabil                           |
|P1       |Hoch-/Querformat             |Keine abgeschnittenen Bedienfelder; Safe Areas beachtet                                 |
|P1       |Vorher/Nachher und Zoom      |Touch-Gesten stabil, keine unbeabsichtigten Regleränderungen                            |
|P1       |Export in voller Qualität    |Bei ausreichend Speicher sinnvoller Originalauflösungs-Export                           |
|P2       |Apple Pencil / iPad          |Vorhandene Funktionen erhalten, später gezielt testen                                   |
|P2       |KI-Rauschreduzierung         |Nur wenn Modelle offline und performant laufen; kein Showstopper                        |
|P2       |Performance-Optimierung      |Nach gemessenen Engpässen, kein vorsorglicher Komplettumbau                             |

Wichtig zu Dateiformaten: Laut Port wird HEIC beim Einlesen derzeit bei Bedarf in JPEG konvertiert, da nicht jedes Format nativ dekodiert wird. Eine solche Konvertierung nicht als unveränderten Original-HEIC-/RAW-Import darstellen. RAW-/Apple-ProRAW-Kompatibilität einzeln testen und transparent dokumentieren.

7. Speicher, Datensicherheit und Aktualisierung

• Die App arbeitet mit einem eigenen lokalen Katalog im iOS-Sandbox-Verzeichnis. Importierte Originale nicht ändern und nicht ohne eindeutige Benutzeraktion löschen.
• Beim Import Dateinamen-Kollisionen vermeiden und alle benötigten Metadaten erhalten, soweit technisch möglich.
• Update ist nicht gleich Neuinstallation: AltStore-Aktualisierung/Refresh, App-Ersetzung, Löschen und neue App-ID haben verschiedene Auswirkungen auf App-Daten. Niemals versprechen, dass Fotos bei Deinstallation erhalten bleiben.
• Persistenzpfade und Re-Linking gegen veränderte iOS-Container-Pfade prüfen; keine dauerhafte Abhängigkeit von einem absoluten Sandbox-Pfad.
• Vor riskanten Schemaänderungen eine Migration oder Backup-/Exportmöglichkeit einplanen. Keine stillschweigenden Datenverluste.
• Große Bilder über bestehende mobile RAM-Budgets und gekachelte Verarbeitung behandeln. Keine riesigen unkomprimierten Bilder mehrfach gleichzeitig im Speicher halten.
• Bei Abbruch/Fehler soll ein angefangener Export keine beschädigten Dateien als „erfolgreich exportiert“ melden.

8. Tests und Qualitätskontrollen

Automatisierte Checks

Zuerst die existierenden Projektprüfungen nutzen; je nach verfügbarer Toolchain mindestens:

cargo fmt --all --check
cargo xtask layers
cargo clippy -p lightcraft-mobile --target aarch64-apple-ios -- -D warnings

Die genaue Gültigkeit dieser Befehle, notwendige Target-/Toolchain-Einstellungen und zusätzliche Upstream-Checks im Repository verifizieren. Wenn cargo xtask ci ausführbar ist, entsprechend integrieren. Tests dürfen nicht stillschweigend übersprungen oder Fehler umbenannt werden. Bei bekannten flakigen Upstream-Tests Ursache und Referenz festhalten.

Manuelle Checkliste: echtes iPhone

☐ IPA aus dem richtigen Commit per GitHub Actions laden.
☐ IPA in AltStore Classic importieren und dort signieren/installieren; Fehlertext und iOS-Version dokumentieren.
☐ Startbildschirm, Symbol, Katalog und Orientierung geprüft.
☐ JPEG importieren; Foto sichtbar, Bearbeitungsregler ändern das Bild.
☐ HEIC importieren; dokumentieren, ob die Pipeline nach JPEG konvertiert.
☐ Wenn verfügbar: eine DNG-/ProRAW-Datei importieren, Ergebnis und Grenzen dokumentieren.
☐ Belichtung und Weißabgleich ändern; App vollständig beenden und neu öffnen; Änderungen bleiben erhalten.
☐ JPEG exportieren; Datei über Share Sheet / Dateien / Fotos prüfen.
☐ Hochformat, Querformat, Bildzoom und Gesten ohne Blockierung.
☐ 24-MP-Foto testen; Speicherabbruch und Wiederaufnahme prüfen.
☐ Hintergrund/Vordergrund und Abbruch eines Exports prüfen.
☐ Neue IPA mit gleicher Bundle-ID als Update installieren; prüfen, ob lokale Daten bleiben.
☐ 7-Tage-Refresh von AltStore Classic berücksichtigen; kein Anspruch auf dauerhaft signierte kostenlose Installation.

Nur die tatsächlich ausgeführten Prüfungen abhaken; den Rest als ungeprüft stehen lassen.

9. Umsetzungsphasen in verbindlicher Reihenfolge

Phase 0 — Audit und Entscheidungsprotokoll

1. Codebasis und PR-Status feststellen, Start-Commit pinnen.
2. Bestehende mobile Build-Skripte und Cargo.toml lesen.
3. Build-Umgebung (Mac mit Xcode? Windows + GitHub Actions?) konkret erfassen. Bei fehlendem Mac standardmäßig CI-Build mit macOS-Runner planen.
4. Machbarkeit einer unsigned iPhoneOS-IPA prüfen; echte Blocker in docs/ios-status.md festhalten.
5. Upstream-AGENTS.md und CLAUDE.md beachten; nicht überschreiben.

Ergebnis: kurze technische Bestandsaufnahme mit Fakten, offenen Fragen und konkretem nächsten Arbeitsauftrag.

Phase 1 — Unsigned IPA (allerhöchste Priorität)

1. Reproduzierbares Device-Build-Skript und IPA-Packaging ergänzen.
2. GitHub-Actions-Workflow implementieren.
3. Validierungs-Skript für IPA-Metadaten/Binary/Ressourcen ergänzen.
4. Erfolgreichen Build ausführen, IPA als Artefakt erzeugen und dokumentieren.

Definition of Done: Eine reale, nachvollziehbar gebaute .ipa lässt sich herunterladen. Wenn kein CI-/Mac-Zugriff vorhanden ist, den letzten verifizierbaren Stand samt reproduzierbaren Befehlen dokumentieren; nicht behaupten, das Artefakt existiere.

Phase 2 — AltStore und echter Gerätetest

1. Installation mit AltStore Classic testen (Benutzer führt seine Apple-ID-/Geräteschritte selbst aus).
2. Crashlogs und technische Ursachen auswerten.
3. iOS-27-Lifecycle, GPU-Initialisierung, App-Sandbox und Touch testen.
4. Start-/Import-/Edit-/Export-Blocker beheben.

Definition of Done: Auf einem realen iPhone läuft ein kompletter JPEG-Workflow. Ohne Zugriff auf das Gerät nur mit „geräteseitig noch nicht verifiziert“ abschließen.

Phase 3 — Nutzbare private Beta

1. HEIC und unterstützte RAW-Dateien prüfen.
2. Persistenz, Katalog, Exportauflösung und RAM-Verbrauch stabilisieren.
3. Bedienbarkeit auf iPhone-Hoch-/Querformat verbessern, ohne die Engine neu zu bauen.
4. Regressions-Checks, Upgrade-Test und dokumentierte Installationsanleitung erstellen.

Definition of Done: Der private Nutzer kann typische Fotos zuverlässig importieren, bearbeiten, exportieren und später erneut öffnen.

Phase 4 — Optional, erst nach Freigabe des MVP

Mehr RAW-Formate, iPad-Optimierungen, Pencil, Presets-Management, GPU-Performance, optionale Offline-KI. Nicht vor Phase 1–3 priorisieren.

10. Erwartete Dateien und Übergabe

Claude soll – soweit passend und noch nicht vorhanden – diese Artefakte im eigenen Repo anlegen oder pflegen:

CLAUDE.md                                  # mit bestehenden Agentenregeln integrieren
AGENTS.md                                  # Upstream-Regeln respektieren, nicht ungefragt ersetzen
.github/workflows/ios-ipa.yml             # manuell auslösbarer macOS-Build
apps/lightcraft-mobile/scripts/ios-ipa.sh # unsigned Geräte-Build / Packaging
scripts/verify-ios-ipa.sh                 # oder gleichwertige reproduzierbare Prüfung
docs/ios-status.md                        # Stand, Tests, Risiken, Commit / Artefakt
docs/altstore-installation.md             # konkrete private Installationsanleitung

Pfadnamen können angepasst werden, wenn die vorhandene Repository-Struktur einen sinnvolleren Ort vorgibt. Vermeide doppelte Skripte und überflüssige Abstraktionen.

Zum Abschluss jedes Arbeitsschritts muss Claude berichten:

1. Was konkret geändert wurde (Dateien und Zweck).
2. Welche Befehle und Tests wirklich liefen, jeweils mit Ergebnis.
3. Ob eine echte IPA erzeugt wurde und wo genau sie liegt (Pfad oder GitHub-Workflow-Artefakt).
4. Was noch nicht funktioniert bzw. mangels Hardware ungeprüft ist.
5. Welcher einzelne nächste Schritt den größten Fortschritt liefert.

11. Wichtige Randbedingungen für AltStore Classic

• AltStore Classic installiert IPA-Dateien und kann sie über eine kostenlose Apple-ID signieren.
• Bei einem kostenlosen Account laufen Signierungen in der Regel nach 7 Tagen ab und benötigen einen Refresh; es gelten installierte-App-/App-ID-Beschränkungen.
• Der Nutzer übernimmt Apple-ID-Login und tatsächliche Installation selbst. Niemals nach dessen Passwort fragen oder es in Build-Infrastruktur hinterlegen.
• Die gleiche stabile Bundle-ID für Builds verwenden, soweit der Installationsweg dies erlaubt; vor einem App-Wechsel den Erhalt der lokalen Daten überprüfen.
• AltStore Classic ist nicht AltStore PAL: Wir bauen keine notarisierte Marketplace-App.
• Der Entwicklerrechner darf Windows sein. AltServer auf Windows ist ein möglicher Installations-/Refresh-Weg, während der Build auf einem macOS-Runner entsteht.

12. Sofortiger Startauftrag an Claude Code

> Prüfe zuerst den aktuellen Stand von `storytold/lightcraft` und den Mobile-Port aus PR #818. Lies insbesondere die vorhandenen Dateien `AGENTS.md`, `CLAUDE.md`, `apps/lightcraft-mobile/README.md`, `docs/porting-report.md` und die iOS-Build-Skripte. Übernimm den bestehenden Port, ohne die Bildbearbeitungsengine oder Oberfläche neu zu schreiben. **Setze als Erstes einen reproduzierbaren GitHub-Actions-Build für eine unsignierte, AltStore-Classic-kompatible iPhone-IPA um.** Validiere das Paket und liefere das tatsächliche Artefakt oder benenne exakt den verbleibenden Blocker. Bearbeite danach schrittweise den Fotoimport, die nicht-destruktive Bearbeitung und den Export auf dem iPhone. Dokumentiere jeden Test ehrlich und halte das Projekt klein, offlinefähig und wartbar.

────────

Erfolgskriterium des Gesamtprojekts: Eine privat per AltStore Classic installierte LightCraft-App, die auf einem echten iPhone ein Foto importiert, sichtbar bearbeitet, Änderungen über Neustarts erhält und ein brauchbares JPEG exportiert — ohne Apple-App-Store-Veröffentlichung.
────────

13. Abgleich mit den Upstream-Regeln (`/AGENTS.md`, `/CLAUDE.md`) — Stand 28.10.2026

Diese Datei ergänzt die Upstream-Regeln, sie ersetzt sie nicht. Wo beide etwas sagen, gilt:

• Never-crash, Clean-Room, Assets/Attribution, Layering, „unsafe nur in `ios_*.rs`/`android_jni.rs` mit `// SAFETY:`“: unverändert Upstream.
• Sprache: Doku zu diesem privaten iOS-Vorhaben (diese Datei, `docs/ios-status.md`, `docs/altstore-installation.md`) auf Deutsch; Code, Code-Kommentare und Commit-Messages wie Upstream auf Englisch, damit Änderungen upstream-fähig bleiben.
• Quality Gates: Upstream verlangt `cargo xtask ci --quick` vor jedem Commit. Wo das in der Arbeitsumgebung nicht läuft (kein macOS, kein Build erlaubt), wird das im Status ausdrücklich vermerkt; der Workflow `.github/workflows/ios-ipa.yml` führt mindestens fmt, `xtask layers` und Clippy für `aarch64-apple-ios` aus. Upstreams `ci.yml` bleibt unverändert.
• Commit-IDs: Upstream „eine Task-ID pro Commit“ → hier `iOS-P<Phase>: …`.
• Tooling-Skripte (`ios-ipa.sh`, `verify-ipa.py`) sind Build-Werkzeug, nicht Produkt; „Pure Rust im Produkt“ bleibt gewahrt (Upstream hat bereits Python-Tooling, z. B. `docs/showcase/run.py`).
• `ios-device.sh` und `ios-sim.sh` bleiben unverändert; der unsignierte Weg ist `ios-ipa.sh`.
