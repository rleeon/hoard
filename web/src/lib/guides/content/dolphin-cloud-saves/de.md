---
title: "Dolphin-Cloud-Speicherstände: GameCube und Wii zwischen PC und Steam Deck synchronisieren"
description: "Dolphin hat keine Cloud-Spielstände. Synchronisiere GameCube und Wii automatisch zwischen PCs und Steam Deck, mit Verlauf: Pfade, Kartentypen, Fallen."
order: 17
updated: 2026-10-09
---

Dolphin synchronisiert Spielstände nicht zwischen Geräten: Deine GameCube-Memory-Cards und deine emulierte Wii liegen in einem Ordner auf einem einzigen PC. Hoard synchronisiert sie automatisch. Wenn du Dolphin schließt, sichert es deine GameCube- und Wii-Spielstände, holt sie auf deine anderen PCs und dein Steam Deck und behält jede Version, damit du immer zurückkannst.

## Wo Dolphin deine Spielstände ablegt

Alles liegt im Benutzerordner von Dolphin. Am schnellsten findest du ihn über **File → Open User Folder** in Dolphin. Darin:

- `GC` enthält die GameCube-Memory-Cards.
- `Wii` ist der interne Speicher der emulierten Wii, Spielstände inklusive.
- `StateSaves` enthält die Savestates.

Wo dieser Ordner liegt:

- **Windows:** `Documents\Dolphin Emulator`. Neuere Installationen nutzen eventuell `%APPDATA%\Dolphin Emulator`, und eine portable Installation hat einen Ordner `User` neben `Dolphin.exe`.
- **Linux:** `~/.local/share/dolphin-emu`.
- **Steam Deck** (das Flatpak aus Discover): `~/.var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu`.
- **Mac:** `~/Library/Application Support/Dolphin`.

Hoard findet die Ordner unter `Documents`, Linux und auf dem Steam Deck selbst. Für `%APPDATA%`, eine portable Installation oder einen Mac zeigst du einmal auf die Ordner `GC` und `Wii`.

## GameCube: Kartendateien oder GCI-Ordner

Unter **Options → Configuration → GameCube** kann jeder Memory-Card-Slot eines von zwei Dingen sein:

- **Eine Memory-Card-Datei**, ein Rohabbild wie `MemoryCardA.USA.raw` mit den Spielständen aller Spiele. Jedes Speichern schreibt die ganze Datei neu.
- **Ein GCI-Ordner**, in dem jeder Spielstand eine eigene `.gci`-Datei ist, in einem Ordner wie `GC/USA/Card A`. Neu ist nur der Spielstand, der sich geändert hat, also bleiben die Versionen klein und gut lesbar.

Zum Synchronisieren passen GCI-Ordner besser. So oder so: **Nutze auf jedem Gerät dieselbe Einstellung.** Eine Kartendatei auf dem einen PC und ein GCI-Ordner auf dem anderen heißt, dass jeder eine leere Karte sieht. Musst du Spielstände von einer Art in die andere bringen, importiert und exportiert Dolphins **Tools → Memory Card Manager** `.gci`-Dateien.

Karten gibt es außerdem **pro Region** (USA, EUR, JAP). Eine PAL- und eine NTSC-Kopie desselben Spiels sehen die Spielstände der anderen nicht, also nutze überall dasselbe Disc-Image.

## Wii: der Speicher der emulierten Konsole

Wii-Spielstände liegen im Ordner `Wii`, bei Disc-Spielen unter `Wii/title/00010000/<Spiel-ID>/data`. Dieser Ordner ist der gesamte Speicher der emulierten Konsole: Spielstände, Miis, Systemeinstellungen und alle installierten Kanäle. Hoard sichert ihn als einen Eintrag, eine Wiederherstellung bringt den Konsolenspeicher also so zurück, wie er in dem Moment war. Vor dem Bestätigen zeigt Hoard, was sich ändert, und deine aktuellen Dateien werden zuerst gesichert.

Willst du nur einen Wii-Spielstand von Hand verschieben, kann Dolphin ihn exportieren: Rechtsklick auf das Spiel in der Liste und **Export Wii Save**.

## Wie der Sync im Alltag läuft

Du spielst am Desktop und schließt Dolphin. Hoard wartet, bis Dolphin beendet ist und die Ordner zur Ruhe kommen, und lädt dann die neue Version hoch. Später nimmst du das Steam Deck; sobald es online ist, holt Hoard die neueren Spielstände herunter. Schließt du Dolphin auf dem Deck, passiert dasselbe in die andere Richtung. Keines der Geräte muss gleichzeitig mit dem anderen an sein.

## Fallen, die du kennen solltest

- **Schließ Dolphin, nicht nur das Spiel.** Hoard sichert, sobald der Emulator beendet ist. Auf einem Deck zählt Standby nicht als Schließen.
- **Savestates sind empfindlich.** Dolphins States gehen zwischen Dolphin-Versionen oft kaputt. Hoard synchronisiert die echten Spielstände; willst du auch `StateSaves`, füg den Ordner als eigenen Eintrag hinzu und halte Dolphin überall auf derselben Version.
- **Eigene Pfade.** Hast du den Wii-NAND-Stamm oder den GCI-Pfad unter **Options → Configuration → Paths** geändert, zeig Hoard stattdessen auf diese Ordner.

## Einrichten

1. Installiere Hoard auf jedem Gerät und melde dich mit demselben Konto an.
2. Füge in der **Bibliothek** Dolphin aus der Emulatorliste hinzu.
3. Nutze auf jedem Gerät dieselbe Kartenart und dieselbe Region.
4. Spiel, schließ Dolphin und mach auf dem anderen Gerät weiter.

Lieber alles zu Hause? Starte `hoard-server` auf deinem PC oder NAS und richte alle Geräte darauf aus. Kein Konto bei uns, keine Telemetrie an uns, nichts über unsere Server. Siehe [Hoard selbst hosten](/guides/self-host-hoard). Für die anderen Emulatoren siehe [Emulator-Spielstände](/guides/back-up-emulator-saves).

<!-- faq -->

## Häufige Fragen

### Hat Dolphin Cloud-Spielstände?

Nein. Dolphin legt Spielstände in einem lokalen Ordner ab und überlässt dir das Synchronisieren. Hoard ist ein Weg, sie automatisch zu synchronisieren, mit einem Versionsverlauf obendrauf.

### Kann ich Dolphin-Spielstände zwischen PC und Steam Deck synchronisieren?

Ja. Installiere Hoard auf beiden mit demselben Konto. Hoard weiß, wo Dolphin seine Spielstände unter Windows, Linux und im Flatpak des Steam Deck ablegt, und ordnet sie geräteübergreifend zu.

### Kartendatei oder GCI-Ordner?

Zum Synchronisieren der GCI-Ordner: Jeder Spielstand ist eine eigene Datei, also sind die Versionen klein und zeigen, welches Spiel sich geändert hat. Was du auch wählst, nutze auf jedem Gerät dasselbe.

### Synchronisiert es auch Wii-Spielstände?

Ja. Der Ordner `Wii` enthält den Speicher der emulierten Konsole, Spielstände inklusive, und Hoard sichert und synchronisiert ihn wie die GameCube-Karten.

### Synchronisiert Hoard Dolphin-Savestates?

Nicht standardmäßig, weil States zwischen Dolphin-Versionen kaputtgehen. Füg den Ordner `StateSaves` von Hand hinzu, wenn du sie willst.

### Funktioniert es mit Dolphin auf Android?

Heute nicht. Hoard läuft auf Windows, macOS, Linux und Steam Deck.
