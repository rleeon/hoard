---
title: "PCSX2-Cloud-Speicherstände: PS2-Memory-Cards zwischen PC und Steam Deck synchronisieren"
description: "PCSX2 hat keine Cloud-Spielstände. Synchronisiere deine PS2-Memory-Cards automatisch zwischen PCs und Steam Deck, mit Verlauf: Pfade, Ordnerkarten, Fallen."
order: 16
updated: 2026-10-09
---

PCSX2 synchronisiert Spielstände nicht von selbst: Dein PS2-Fortschritt liegt in Memory-Card-Dateien auf einem einzigen Rechner, und der andere erfährt nie davon. Hoard synchronisiert sie automatisch. Wenn du PCSX2 schließt, sichert es deine Memory Cards, holt sie auf deine anderen PCs und dein Steam Deck und behält jede Version, damit ein schlechter Spielstand dich nie einen ganzen Durchgang kostet.

## Wo PCSX2 deine Spielstände ablegt

PCSX2 speichert wie eine echte PS2: auf Memory Cards. Standardmäßig gibt es zwei, `Mcd001.ps2` und `Mcd002.ps2`, je 8 MB, in einem Ordner `memcards`. Eine Karte enthält die Spielstände aller Spiele, die du damit gespielt hast.

- **Windows:** `Documents\PCSX2\memcards`. Im portablen Modus liegt der Ordner stattdessen neben dem Programm.
- **Linux:** `~/.config/PCSX2/memcards`.
- **Steam Deck** (das Flatpak aus Discover): `~/.var/app/net.pcsx2.PCSX2/config/PCSX2/memcards`. Mit EmuDeck zeigt die Verknüpfung unter `~/Emulation/saves/pcsx2` auf einen dieser Ordner.
- **Mac:** `~/Library/Application Support/PCSX2/memcards`.

In PCSX2 zeigt **Settings → Memory Cards**, welchen Ordner es wirklich nutzt und welche Karte in welchem Slot steckt. Hoard findet die Ordner unter Windows, Linux und auf dem Steam Deck selbst; auf einem Mac oder bei einer portablen Installation zeigst du einmal auf den Ordner `memcards`.

## Dateikarten und Ordnerkarten

PCSX2 kann zwei Arten von Memory Card anlegen, und beim Synchronisieren ist die Wahl wichtiger, als sie aussieht.

- **Eine Dateikarte** (`.ps2`) ist eine einzige 8-MB-Datei mit den Spielständen aller Spiele. Speicherst du in irgendeinem Spiel, ändert sich die ganze Datei, also ist jede neue Version die vollen 8 MB.
- **Eine Ordnerkarte** ist ein Ordner statt einer Datei, mit jedem Spielstand in einem eigenen Unterordner. Speicherst du in einem Spiel, ändern sich nur dessen Dateien, also bleiben die Versionen klein und der Verlauf zeigt, welcher Spielstand sich bewegt hat.

Beide Arten legst du unter **Settings → Memory Cards** an. Egal welche du wählst, nutze **denselben Typ, dieselben Kartennamen und dieselben Slots** auf jedem Gerät. Eine Dateikarte auf dem Desktop und eine Ordnerkarte auf dem Deck sind zwei verschiedene Karten, und jedes Gerät hält den Spielstand des anderen für nicht vorhanden.

## Wie der Sync im Alltag läuft

Du spielst am Desktop und schließt PCSX2. Hoard wartet, bis PCSX2 beendet ist und sich die Karten nicht mehr ändern, und lädt dann die neue Version hoch. Später nimmst du das Steam Deck. Sobald es online ist, holt Hoard die neueren Karten herunter, und wenn du PCSX2 startest, ist dein Spielstand da. Schließt du es auf dem Deck, passiert dasselbe in die andere Richtung.

Keines der Geräte muss gleichzeitig an sein. Die Karten warten auf dem Server, bis das andere Gerät sie abholt.

## Fallen, die du kennen solltest

- **Schließ PCSX2, nicht nur das Spiel.** Hoard sichert, sobald der Emulator beendet ist, und kopiert so nie eine Karte mitten im Schreiben. Auf einem Deck zählt Standby nicht als Schließen.
- **Dieselbe Disc, dieselbe Region.** Die PAL- und die NTSC-Fassung eines Spiels haben verschiedene Seriennummern (zum Beispiel SLES und SLUS) und sehen die Spielstände der anderen nicht. Nutze überall dasselbe Disc-Image.
- **Savestates sind etwas anderes.** States (`.p2s`-Dateien in `sstates`) sind Abbilder des Emulators und laden oft nicht in einer anderen PCSX2-Version. Hoard synchronisiert die Memory Cards; sollen auch die States reisen, füg den Ordner `sstates` als eigenen Eintrag hinzu und halte PCSX2 überall auf derselben Version.
- **Eine Version ist die ganze Karte.** Stellst du eine frühere Version wieder her, kommt die ganze Karte so zurück, wie sie war, mit allen Spielen darauf. Hoard zeigt vor dem Bestätigen, was sich ändert, und sichert deine aktuelle Karte vorher, also lässt sich eine Wiederherstellung immer rückgängig machen.

## Einrichten

1. Installiere Hoard auf jedem Gerät und melde dich mit demselben Konto an.
2. Füge in der **Bibliothek** PCSX2 aus der Emulatorliste hinzu.
3. Prüfe, dass alle Geräte denselben Kartentyp, dieselben Namen und dieselben Slots nutzen.
4. Spiel, schließ PCSX2 und mach auf dem anderen Gerät weiter.

Lieber alles zu Hause? Starte `hoard-server` auf deinem PC oder NAS und richte alle Geräte darauf aus. Kein Konto bei uns, keine Telemetrie an uns, nichts über unsere Server. Siehe [Hoard selbst hosten](/guides/self-host-hoard). Für die anderen Emulatoren siehe [Emulator-Spielstände](/guides/back-up-emulator-saves).

<!-- faq -->

## Häufige Fragen

### Hat PCSX2 Cloud-Spielstände?

Nein. PCSX2 schreibt die Memory Cards in einen lokalen Ordner und überlässt dir das Synchronisieren. Hoard ist ein Weg, das automatisch zu tun, mit einem Versionsverlauf obendrauf.

### Kann ich PCSX2-Spielstände zwischen PC und Steam Deck synchronisieren?

Ja. Installiere Hoard auf beiden mit demselben Konto. Hoard weiß, wo PCSX2 seine Karten unter Windows und im Flatpak des Steam Deck ablegt, und ordnet sie geräteübergreifend zu.

### Dateikarte oder Ordnerkarte?

Zum Synchronisieren passt die Ordnerkarte besser: Es werden nur die Spielstände hochgeladen, die sich geändert haben, und der Verlauf zeigt, welches Spiel sich bewegt hat. Beides funktioniert, solange alle Geräte dasselbe nutzen.

### Synchronisiert Hoard PCSX2-Savestates?

Nicht standardmäßig, weil States zwischen PCSX2-Versionen kaputtgehen. Füg den Ordner `sstates` von Hand hinzu, wenn du sie willst, und halte PCSX2 überall auf derselben Version.

### Setzt eine alte Version jedes Spiel auf der Karte zurück?

Ja. Eine Version ist die ganze Karte. Hoard zeigt vorher, was sich ändert, und behält die ersetzte Karte ebenfalls als Version.

### Funktioniert es mit PS2-Emulatoren auf Android?

Heute nicht. Hoard läuft auf Windows, macOS, Linux und Steam Deck.
