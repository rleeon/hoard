---
title: "Cyberpunk 2077: Speicherort der Spielstände (PC & Steam Deck)"
description: "Wo Cyberpunk 2077 seine Spielstände unter Windows, auf dem Steam Deck und dem Mac ablegt, was in den Ordnern steckt und wie du sie sicherst oder umziehst."
order: 20
updated: 2026-10-02
---

Unter Windows legt Cyberpunk 2077 seine Spielstände in `%USERPROFILE%\Saved Games\CD Projekt Red\Cyberpunk 2077` ab, ein Ordner pro Spielstand. Das ist die kurze Antwort. Der Rest der Seite behandelt die Pfade auf Steam Deck und Mac, was wirklich im Ordner liegt und wie du ihn gesichert hältst.

## Wo Cyberpunk 2077 seine Spielstände ablegt

- **Windows** (Steam, GOG oder Epic): `%USERPROFILE%\Saved Games\CD Projekt Red\Cyberpunk 2077`
- **Steam Deck und Linux** (Proton): `~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077`
- **Mac:** `~/Library/Application Support/CD Projekt Red/Cyberpunk 2077/Saves`

Die Pfade für Windows und Mac nennt CD Projekt Red auf seinen eigenen Support-Seiten. Auf dem Steam Deck läuft das Spiel in einem Proton-Präfix, einem kleinen Windows-Ordnerbaum, den Steam für jedes Spiel anlegt; `1091500` ist die Steam-App-ID von Cyberpunk. Ist das Spiel auf der microSD-Karte installiert, suche `steamapps/compatdata/1091500` auf der Karte.

## Was im Ordner liegt

Cyberpunk schreibt keine einzelne Spielstand-Datei, sondern **einen Ordner pro Spielstand**: `AutoSave-0`, `AutoSave-1` und so weiter, `ManualSave-0`, `ManualSave-1` und `QuickSave-0`. Jeder enthält den Spielstand selbst (`sav.dat`) sowie Screenshot und Metadaten für das Lademenü.

Daraus folgen zwei Dinge:

- **Sichere den übergeordneten Ordner, nicht einen einzelnen Spielstand.** Wer nur den neuesten `ManualSave` kopiert, lässt die Autosaves weg, die oft den jüngsten Fortschritt haben.
- **Autosaves rotieren.** Das Spiel verwendet eine kleine Zahl von `AutoSave`-Ordnern immer wieder und überschreibt den ältesten. Ein Autosave von vor drei Stunden ist meist schon weg — deshalb lohnt sich ein Verlauf außerhalb des Spiels.

Die Einstellungen liegen nicht hier. Grafik und Steuerung stehen in `UserSettings.json` unter `%LOCALAPPDATA%\CD Projekt Red\Cyberpunk 2077`, neben Caches und Logs. Genau diesen Ordner sichern viele (und manche Werkzeuge) aus Versehen: Er enthält nichts, dessen Verlust dich Fortschritt kostet.

## Hat Cyberpunk 2077 Cloud-Saves?

Ja. Die Steam-Version nutzt Steam Cloud, die GOG-Version die Cloud von GOG Galaxy. Beide halten den neuesten Stand deiner Spielstände zwischen Rechnern desselben Stores synchron.

Was keine von beiden tut:

- **Ältere Versionen behalten.** Wird ein Spielstand beschädigt oder von einem Mod zerstört, liegt auch in der Cloud die kaputte Kopie.
- **Stores verbinden.** Steam Cloud und GOGs Cloud sprechen nicht miteinander, obwohl PC-Spielstände von Steam, GOG und Epic in jeder Version laden, wenn du den Ordner hinüberkopierst.

## Von Hand sichern

1. Beende das Spiel vollständig.
2. Kopiere den ganzen Ordner `Cyberpunk 2077` vom obigen Pfad auf einen USB-Stick, ein anderes Laufwerk oder in einen Cloud-Ordner.
3. Zum Wiederherstellen das Spiel schließen und den Ordner zurückkopieren, Vorhandenes ersetzen.

Das funktioniert, aber nur so oft, wie du daran denkst, und du hast immer nur die Kopie vom letzten Mal.

## Automatisch sichern und synchronisieren mit Hoard

[Hoard](/download) sichert den Spielstand-Ordner jedes Mal, wenn du aufhörst zu spielen, und behält jede Version — ein beschädigter Spielstand oder ein weggerotierter Autosave ist also einen Klick entfernt. Außerdem synchronisiert es den Ordner zwischen deinen PCs und einem Steam Deck.

1. Installiere Hoard und melde dich an, oder richte es auf [deinen eigenen Server](/guides/self-host-hoard).
2. Öffne die **Bibliothek**. Cyberpunk wird über deine Steam-Bibliothek und die Community-Datenbank für Spielstände erkannt.
3. Prüfe, dass der angezeigte Ordner der unter `Saved Games\CD Projekt Red\Cyberpunk 2077` ist. Steht dort der Ordner unter `AppData\Local`, ändere ihn: Der enthält nur Einstellungen.
4. Spiele. Beim Beenden erscheint die erste Version im Verlauf.

Hoard verfolgt den ganzen Ordner, also landet jeder `AutoSave`, `ManualSave` und `QuickSave` in derselben Version. Mit Deck und Desktop wartet die neueste Version auf dem Gerät, das du als Nächstes in die Hand nimmst — siehe [wie die Synchronisierung zwischen PCs funktioniert](/guides/sync-game-saves-across-pcs).

<!-- faq -->

## Häufige Fragen

### Wo liegen die Spielstände von Cyberpunk 2077 auf dem Steam Deck?

Im Proton-Präfix des Spiels: `~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077`. Liegt das Spiel auf der microSD-Karte, ist auch der `compatdata`-Ordner dort.

### Kann ich meine Spielstände von GOG zu Steam mitnehmen?

Ja. PC-Spielstände sind bei Steam, GOG und Epic dieselben. Kopiere die Spielstand-Ordner bei geschlossenem Spiel an denselben Pfad der anderen Installation, und sie erscheinen im Lademenü.

### Warum gibt es so viele AutoSave-Ordner?

Das Spiel hält ein paar Autosave-Plätze vor und überschreibt jedes Mal den ältesten. Es sind normale Spielstände, die eben von selbst ersetzt werden.

### Warum ist mein älterer Autosave verschwunden?

Weil sein Platz wiederverwendet wurde. Das Spiel behält nur wenige. Ein Backup-Werkzeug mit Versionen ist der einzige Weg, ihn nach dem Rotieren zurückzuholen.

### Synchronisiert Hoard auch meine Einstellungen?

Nein. Die Einstellungen liegen in einem anderen Ordner, der nicht zum Spielstand gehört, also behält jeder Rechner seine eigenen — meistens genau richtig, denn Deck und Desktop brauchen unterschiedliche Grafikeinstellungen. Mehr dazu unter [Spielstände zwischen PCs synchronisieren](/guides/sync-game-saves-across-pcs).
