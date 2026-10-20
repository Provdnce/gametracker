# GameTracker

Trackt automatisch, **welche Spiele du wann wie lange spielst** und **wie oft du jede Taste drückst** – damit du auch nach Monaten und Jahren sehen kannst, was du in welchem Abschnitt deines Lebens gespielt hast.

Backend in Rust, Oberfläche mit [GPUI-CE](https://github.com/gpui-ce/gpui-ce).

## Funktionen

- **Automatische Spiele-Erkennung**: Spiele aus Steam-, Epic-, GOG-, Xbox-, Ubisoft-, EA-, Riot-, Rockstar- und Battle.net-Ordnern sowie eine Liste bekannter Spiele (LoL, Valorant, WoW, Fortnite, …). Fehlt ein Spiel, fügst du es mit einem Klick hinzu, entweder über „Laufende Programme“ oder indem du die EXE auswählst.
- **Sessions**: Start, Ende und Dauer jeder Spielsession. Startest du ein Spiel innerhalb von 3 Minuten neu, läuft die Session weiter. Sessions unter 1 Minute werden verworfen. Wenn der PC im Standby war, wird die Session korrekt beendet.
- **Übersicht**: aktuelles Spiel (live), Spielzeit heute, diese Woche, diesen Monat und insgesamt, Diagramm der letzten 30 Tage, zuletzt und am meisten gespielt.
- **Spiele**: alle Spiele, filterbar nach Gesamt/Jahr/Monat/Woche/Tag, mit Sessions, Umbenennen und Löschen.
- **Verlauf**: jeder Monat mit den gespielten Spielen. Ein Klick auf einen Monat oder ein Jahr springt zur gefilterten Spieleliste.
- **Tastatur**: Heatmap (QWERTZ oder QWERTY), Top 20 der Tasten, Anschläge pro Tag/Monat. Filter: Gesamt, Jahr, Monat, Woche, Tag, jeweils mit ‹ › zum Blättern.
- **Export und Import** als JSON-Datei, gedacht für PC-Wechsel. Den Import kannst du beliebig oft wiederholen, ohne dass etwas doppelt gezählt wird.
- **Automatische Backups** (täglich/wöchentlich/monatlich, die letzten 12 werden behalten).
- **Läuft im Hintergrund**: Fenster schließen beendet das Tracking nicht. GameTracker sitzt dann als Symbol im Infobereich der Taskleiste (rechts unten beim Pfeil): Linksklick öffnet das Fenster, Rechtsklick zeigt „Öffnen“, „Tracking pausieren“ und „Beenden“. Optional startet es mit Windows.
- **Programme ausblenden**: Wird ein Tool fälschlich als Spiel erkannt, klickst du unter *Spiele* auf „Nicht mehr tracken“, wahlweise mit oder ohne Löschen des bisherigen Verlaufs. Wallpaper Engine, Lossless Scaling, OBS, SteamVR & Co. werden von vornherein nicht als Spiel gezählt. Rückgängig machen kannst du das unter *Einstellungen → Ignorierte Programme*.
- **Darstellung**: Hintergrund (Graphit, Mitternacht, Hell) und Akzentfarbe (Indigo, Ozean, Salbei, Sand, Rosé, Graphit) sind wählbar.

## Datenschutz beim Tastatur-Tracking

Gespeichert wird **nur die Anzahl pro Taste und Tag**, zum Beispiel „W wurde am 20.10.2026 5.123× gedrückt“. Es wird keine Reihenfolge, keine Uhrzeit und kein Text gespeichert. Was du getippt hast (Passwörter, Chats …), lässt sich aus den Daten also **nicht** rekonstruieren. Gehaltene Tasten zählen nur einmal, Auto-Repeat wird herausgefiltert. Alle Daten bleiben lokal auf deinem PC.

## Bauen und Starten (Windows)

1. Rust installieren: <https://rustup.rs> (MSVC-Toolchain, dazu die „Visual Studio Build Tools“ mit „Desktopentwicklung mit C++“ und Windows SDK).
2. Im Projektordner:
   ```powershell
   cargo build --release
   ```
   Der erste Build dauert ein paar Minuten, weil GPUI-CE mitkompiliert wird.
3. Starten: `target\release\gametracker.exe`. Die EXE kannst du dir an einen beliebigen Ort kopieren.
4. In den **Einstellungen** „Mit dem System starten“ aktivieren, damit immer getrackt wird.

Tastenkürzel: `Strg+W` schließt das Fenster (Tracking läuft weiter), `Strg+Q` beendet GameTracker komplett.

### Linux / macOS

Funktioniert ebenfalls. Linux braucht die GPUI-Abhängigkeiten (siehe [GPUI-CE SETUP.md](https://github.com/gpui-ce/gpui-ce/blob/main/SETUP.md)) plus `libxtst-dev`. Das Tastatur-Tracking braucht unter Linux X11 bzw. XWayland. Unter macOS musst du in den Systemeinstellungen die Bedienungshilfen-Berechtigung erteilen.

## Wo liegen die Daten?

| System  | Ordner |
|---------|--------|
| Windows | `%APPDATA%\GameTracker\` |
| Linux   | `~/.local/share/GameTracker/` |
| macOS   | `~/Library/Application Support/GameTracker/` |

Darin liegen `gametracker.db` (SQLite) und der Ordner `backups/`. Mit der Umgebungsvariable `GAMETRACKER_DATA_DIR` kannst du einen anderen Ordner verwenden.

### PC-Wechsel

1. Alter PC: Einstellungen → **Exportieren …** → Datei sichern (USB-Stick, Cloud, …).
2. Neuer PC: GameTracker installieren → Einstellungen → **Importieren …**

Jeder PC bekommt eine eigene ID. Daten mehrerer PCs lassen sich so zusammenführen, ohne sich zu überschreiben. Alternativ kannst du auch einfach die `gametracker.db` kopieren, solange GameTracker dabei beendet ist.

### Exportformat

Eine lesbare JSON-Datei (`"format": "gametracker-export", "version": 1`) mit `machines`, `games`, `sessions` (UUID, Spiel, Start, Ende als UTC), `key_counts` (PC, Tag, Taste, Anzahl) und `rules`. Damit kannst du die Daten auch mit anderen Tools auswerten.

## Ausprobieren mit Demo-Daten

```bash
# Linux/macOS
GAMETRACKER_DATA_DIR=/tmp/gt-demo cargo run -p gametracker-core --example demo_data
GAMETRACKER_DATA_DIR=/tmp/gt-demo cargo run -p gametracker
```
```powershell
# Windows
$env:GAMETRACKER_DATA_DIR="$env:TEMP\gt-demo"; cargo run -p gametracker-core --example demo_data; cargo run -p gametracker
```

## Projektstruktur

```
crates/core/   Backend (ohne UI, getestet: cargo test -p gametracker-core)
  db.rs          SQLite-Schema und Abfragen
  detect.rs      Spiele-Erkennung (Store-Ordner, bekannte Spiele, eigene Regeln)
  tracker.rs     Hintergrund-Threads: Prozesse prüfen, Sessions schreiben, Tasten speichern
  keys.rs        Globaler Tastatur-Hook (Windows: eigener Low-Level-Hook, sonst rdev) + Repeat-Filter
  keymap.rs      Tasten-IDs (physische Position), QWERTZ/QWERTY-Layouts
  stats.rs       Zeiträume (Jahr/Monat/Woche/Tag) und Auswertungen
  transfer.rs    Export, Import (Merge), automatische Backups
  autostart.rs   Autostart für Windows/Linux/macOS
crates/app/    GPUI-CE-Oberfläche
```

## Danksagungen

- Schrift: [Inter](https://rsms.me/inter/) von Rasmus Andersson (SIL Open Font License, siehe `crates/app/assets/fonts/Inter-LICENSE.txt`)
- Icons: [Lucide](https://lucide.dev) (ISC License)

## Hinweise

- Tasten werden nach **physischer Position** gespeichert. Die Daten bedeuten also auf jedem Layout und jedem Betriebssystem dasselbe. Das Layout in den Einstellungen ändert nur die Beschriftung.
- Programmgesteuerte (injizierte) Tastendrücke, zum Beispiel von Makro-Tools, werden unter Windows nicht gezählt.
- Ein globaler Tastatur-Hook ist dieselbe Technik, die Discord, OBS oder AutoHotkey nutzen. Anti-Cheat-Systeme haben damit normalerweise kein Problem, eine Garantie gibt es aber nicht. Im Zweifel kannst du das Tasten-Tracking in den Einstellungen pausieren.
