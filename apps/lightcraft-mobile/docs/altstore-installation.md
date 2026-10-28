# LightCraft per AltStore Classic auf dem iPhone installieren (privat)

> Stand 28.10.2026. Die Installation über AltStore ist mit dieser IPA **noch nicht praktisch erprobt**
> (siehe `ios-status.md`). Für das Einrichten von AltServer/AltStore selbst gilt die offizielle
> Anleitung: <https://faq.altstore.io/> – dieser Text beschreibt nur die LightCraft-spezifischen Schritte.

## 1. IPA bauen (GitHub Actions, kein Mac nötig)

1. Den Branch `ios-altstore` in **deinen eigenen** GitHub-Fork pushen (öffentliches Repo: macOS-Minuten
   kostenlos; privates Repo: macOS-Minuten zählen zehnfach gegen das Kontingent).
2. GitHub ▸ *Actions* ▸ **iOS IPA (unsigned, for AltStore)** ▸ *Run workflow* ▸ Branch `ios-altstore`.
   Bundle-ID unverändert lassen (`local.lightcraft.ios`), damit spätere Builds als Update installiert werden.
3. Nach dem grünen Lauf unten auf der Run-Seite das Artefakt
   `LightCraft-iOS-<version>-<build>-<commit>-unsigned` herunterladen und entpacken. Darin liegen
   - `…-unsigned.ipa` – die App,
   - `…dSYM.zip` – Symbole, um Absturzberichte auszuwerten (nicht installieren),
   - `verify.json` – das Prüfergebnis.
   Die Zusammenfassung des Runs zeigt Commit, Xcode-/Rust-Version und die Prüfung.

## 2. Einmalig: AltStore Classic einrichten (Windows)

Nach der offiziellen Anleitung AltServer für Windows installieren und damit AltStore aufs iPhone bringen
(<https://faq.altstore.io/>). Ab iOS 16 muss auf dem iPhone der **Entwicklermodus** aktiviert sein
(Einstellungen ▸ Datenschutz & Sicherheit ▸ Entwicklermodus), sonst starten seitengeladene Apps nicht.
Apple-ID und Passwort gibst du nur in AltServer/AltStore ein – niemals in dieses Repo, den Workflow oder
einen Chat.

## 3. LightCraft installieren

1. Die `.ipa` aufs iPhone bringen (z. B. iCloud Drive, AirDrop, USB → App *Dateien*).
2. AltStore öffnen ▸ *My Apps* ▸ **+** ▸ die `.ipa` wählen. AltStore signiert sie mit deiner Apple-ID
   und installiert sie (iPhone und PC mit AltServer im selben WLAN, oder per Kabel).
3. LightCraft starten. Erwartet: die Bibliothek mit den Demo-Fotos.

Fehlermeldungen von AltStore bitte **wörtlich** notieren, zusammen mit iPhone-Modell und iOS-Version
(Einstellungen ▸ Allgemein ▸ Info) – das ist die wichtigste Eingabe für die nächste Phase.

## 4. Grenzen eines kostenlosen Apple-Kontos

- Die Signatur läuft nach **7 Tagen** ab; danach öffnet sich die App nicht mehr, bis AltStore sie
  aktualisiert („Refresh All“ in *My Apps*, oder Hintergrund-Refresh mit AltServer im WLAN).
- Höchstens **3 aktive seitengeladene Apps** gleichzeitig (AltStore selbst zählt mit).
- Ein Refresh oder ein Update mit **gleicher Bundle-ID** behält die App-Daten. Löschen der App, „Deactivate“
  ohne späteres Reaktivieren oder ein Build mit **anderer** Bundle-ID bedeuten einen neuen Container:
  Katalog, Bearbeitungen und importierte Kopien sind dann weg.

## 5. Wo die Daten liegen, und wie man sie sichert

- Bibliothek: *Dateien* ▸ Auf meinem iPhone ▸ LightCraft ▸ `LightCraft Library` (Katalog, Journal,
  `Picked/`-Kopien importierter Fotos).
- Exporte: *Dateien* ▸ Auf meinem iPhone ▸ LightCraft ▸ `LightCraft Exports`.
- Die Originale in der Fotos-App ändert LightCraft nicht; es arbeitet mit Kopien.
- **Vor dem Löschen der App** oder einem Wechsel der Bundle-ID den Ordner `LightCraft Library` in der
  Dateien-App woanders hin kopieren. Ob das Zurückkopieren eine Bibliothek vollständig wiederherstellt,
  ist noch nicht getestet.

## 6. Update auf einen neuen Build

Neues Artefakt laden ▸ in AltStore *My Apps* ▸ **+** ▸ neue `.ipa`. Bei gleicher Bundle-ID ersetzt sie
die alte App; die Daten sollten bleiben (auf dem Gerät noch nicht überprüft – siehe Checkliste in
`ios-status.md`).
