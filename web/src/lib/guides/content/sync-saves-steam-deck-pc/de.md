---
title: "Spielstände zwischen Steam Deck und PC synchronisieren"
description: "Steam-Deck- und PC-Spielstände automatisch synchron halten, auch Nicht-Steam-Spiele, Emulatoren und Spiele ohne Steam Cloud. Einrichtung, Pfade, Fallen."
order: 10
updated: 2026-10-09
---

Bei Steam-Spielen mit Steam Cloud teilen sich dein Deck und dein PC die Spielstände bereits. Alles andere braucht Hilfe: Spiele, bei denen der Entwickler Steam Cloud nie eingeschaltet hat, Epic- und GOG-Spiele über Heroic, Emulatoren und alles, was du als Nicht-Steam-Spiel hinzugefügt hast. Hoard deckt all das automatisch ab. Beendest du ein Spiel auf einem Gerät, sichert Hoard den Spielstand, und das andere Gerät holt ihn sich, wobei jede frühere Version erhalten bleibt, falls etwas schiefgeht.

## Was Steam Cloud auf dem Deck schon erledigt

Unterstützt ein Spiel Steam Cloud, lädt Steam den Spielstand beim Beenden hoch und beim Start auf einem anderen Gerät herunter. Ob ein Spiel das kann, steht auf seiner Shopseite; abschalten lässt es sich pro Spiel unter **Eigenschaften → Allgemein**.

Die Lücken sind die üblichen:

- **Spiele ohne Steam Cloud.** Das entscheidet der Entwickler, Spiel für Spiel, und viele PC-Spiele haben nie mitgemacht.
- **Alles außerhalb von Steam.** Heroic, Lutris, Emulatoren, ein von Hand installiertes Spiel.
- **Kein Zurück.** Steam speichert den aktuellen Spielstand, keinen Verlauf. Wird ein kaputter Spielstand synchronisiert, ist der gute auf beiden Geräten weg.

Mehr dazu im Leitfaden [Alternative zu Steam Cloud](/guides/steam-cloud-alternative).

## Wo das Deck deine Spielstände ablegt

Das Deck startet Windows-Spiele über Proton, also speichert dasselbe Spiel an einem anderen Ort als auf deinem PC:

- **Windows-Spiele (Proton):** `~/.local/share/Steam/steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/`, gefolgt vom üblichen Windows-Pfad: `Documents`, `AppData/Roaming`, `AppData/Local`, `AppData/LocalLow` oder `Saved Games`. Die AppID ist die Zahl in der Shop-URL des Spiels.
- **Spiele auf der microSD-Karte:** Die Karte hat ihr eigenes `steamapps/compatdata/<AppID>` mit demselben Baum darin.
- **Native Linux-Spiele:** meist `~/.local/share/<Spiel>` oder `~/.config/<Spiel>`. Unity-Spiele nutzen `~/.config/unity3d/<Firma>/<Spiel>`.
- **Heroic, Lutris und Bottles:** Jedes hält ein eigenes Wine-Präfix pro Spiel, mit dem Windows-Baum unter `drive_c/users/<dein Benutzer>/` statt `steamuser`.
- **Emulatoren:** EmuDeck sammelt sie unter `~/Emulation/saves/`. Siehe [Emulator-Spielstände](/guides/back-up-emulator-saves) und [RetroArch](/guides/retroarch-save-sync).

Auf deinem PC schreibt dasselbe Spiel nach `C:\Users\<du>\...`. Zwei verschiedene Pfade für einen Spielstand: genau deshalb geht das Kopieren von Ordnern per Hand schief. Hoard sucht an all diesen Orten und ordnet das Gefundene dem Spiel zu, sodass der Spielstand vom Deck und der vom PC zu zwei Versionen einer gemeinsamen Historie werden.

## Einrichten

1. Wechsle auf dem Deck in den Desktop-Modus: **Steam-Taste → Ein/Aus → Zum Desktop wechseln**.
2. Öffne einen Browser, geh auf [die Downloadseite](/download) und lade **Hoard Setup** für Linux. Öffne im Dateimanager die Eigenschaften der Datei, erlaube das Ausführen als Programm und starte sie.
3. Melde dich mit dem Konto an, das du auch am PC nutzt, oder richte die App auf deinen eigenen Server aus.
4. Öffne die **Bibliothek** und prüfe, was Hoard gefunden hat. Ergänze Fehlendes, indem du auf den Ordner zeigst: ein Heroic-Präfix, einen Emulator, ein selbst installiertes Spiel.
5. Installiere Hoard auf deinem PC mit demselben Konto. Dieselben Spiele finden von selbst zueinander.
6. Wechsle zurück in den Spielmodus. In den Desktop-Modus musst du nicht wieder.

Hoard Setup legt die App in deinem Home-Ordner ab und den Sync-Motor in einen Hintergrunddienst, der mit dem Deck startet. Ins schreibgeschützte System von SteamOS wird nichts geschrieben, also lassen Systemupdates es in Ruhe.

## Wie ein normaler Tag aussieht

Du spielst abends am PC und beendest das Spiel. Hoard wartet, bis das Spiel geschlossen ist und sich der Spielstand nicht mehr ändert, und lädt ihn dann hoch. Am nächsten Morgen nimmst du das Deck. Sobald es online ist, sieht Hoard die neuere Version und schreibt sie ins Proton-Präfix. Du startest das Spiel und machst weiter. Beendest du es auf dem Deck, passiert dasselbe in umgekehrter Richtung.

Keines der Geräte muss gleichzeitig mit dem anderen an sein. Der Spielstand wartet auf dem Server, bis das andere Gerät ihn abholt.

## Die Fallen, die du kennen solltest

### Standby ist nicht Beenden

Auf dem Deck ist es sehr leicht, die Ein/Aus-Taste zu drücken und das Spiel offen zu lassen. Hoard sichert einen Spielstand erst, wenn das Spiel geschlossen ist, denn ein laufendes Spiel kann gerade mitten im Schreiben sein. Und es tauscht nie den Spielstand eines laufenden Spiels aus. Schickst du das Deck in den Standby und spielst dann am PC, ist der Fortschritt vom Deck noch nicht hochgeladen, und der neue Spielstand vom PC wartet, bis du das Spiel auf dem Deck beendest.

Die Gewohnheit, die all das verhindert: **Beende das Spiel, bevor du das Gerät wechselst.** Lässt Proton nach dem Beenden einen toten Prozess zurück, was oft passiert, merkt Hoard, dass das Spiel weg ist, und macht weiter.

### Gib ihm nach dem Aufwachen ein paar Sekunden

Wenn das Deck aufwacht, braucht das WLAN einen Moment, und erst dann kann Hoard nach einem neueren Spielstand schauen. Startest du in diesen ersten Sekunden ein Spiel, wartet der Download, bis du es wieder beendest. Gib ihm einen Moment online, bevor du loslegst.

### Die microSD-Karte

Liegt ein Spiel auf der Karte und die Karte steckt nicht, lädt Hoard keinen Spielstand in einen Ordner, den es nicht gibt. Es wartet, bis die Karte wieder da ist.

### Einstellungen bleiben auf jedem Gerät

Das Deck läuft mit 1280×800 auf einer Handheld-GPU. Dein Desktop vermutlich nicht. Hoard sichert Einstellungsdateien wie `graphics.ini` mit dem Spielstand, schreibt sie aber nicht über die des anderen Geräts, also behält das Deck seine eigenen. Willst du sie trotzdem übernehmen, gibt es beim Wiederherstellen eine Option dafür. Mehr unter [Spielstände zwischen mehreren PCs synchronisieren](/guides/sync-game-saves-across-pcs).

### Der Ordner `remote` von Steam

Bei Steam-Spielen liegt der Spielstand in `userdata/<UserID>/<AppID>/remote/`. Der Ordner darüber enthält außerdem `remotecache.vdf` sowie Spielzeit- und Erfolgsdateien, die sich zwischen Deck und PC unterscheiden sollen. Synchronisierst du den übergeordneten Ordner von Hand, sieht jeder Start wie ein Konflikt aus. Hoard verfolgt nur `remote/`.

## Steam Cloud und Hoard zusammen

Sie kommen sich nicht in die Quere. Bei einem Spiel mit Steam Cloud lass Steam weiter synchronisieren. Was Hoard dort beiträgt, ist die Versionshistorie, damit ein kaputter Spielstand auf einem Gerät deinen Fortschritt nicht mitreißt. Bei allen anderen Spielen übernimmt Hoard auch das Synchronisieren.

## Ohne unsere Server

Sollen die Spielstände zu Hause bleiben, starte `hoard-server` auf deinem PC oder einem NAS und richte Deck und PC darauf aus. Kein Konto bei uns, keine Telemetrie an uns, nichts läuft über unsere Server. Siehe [Hoard selbst hosten](/guides/self-host-hoard).

<!-- faq -->

## Häufige Fragen

### Funktioniert Hoard im Spielmodus?

Ja. Der Sync-Motor läuft als Hintergrunddienst, der mit dem Deck startet, und sichert und stellt wieder her, ohne dass ein Fenster offen ist. Den Desktop-Modus brauchst du nur zum Installieren und um Ordner von Hand hinzuzufügen.

### Entfernt ein SteamOS-Update Hoard?

Nein. Alles, was Hoard installiert, liegt in deinem Home-Ordner, und den rühren SteamOS-Updates nicht an.

### Synchronisiert es Spiele aus Heroic, Lutris oder EmuDeck?

Ja. Hoard sucht in Präfixen von Heroic, Lutris und Bottles und in den Ordnern von EmuDeck. Wird ein Spiel nicht erkannt, zeig einmal auf seinen Speicherordner, dann wird es wie jedes andere verfolgt.

### Was, wenn ich auf beiden gespielt habe, ohne zu synchronisieren?

Hoard überschreibt nie blind. Es vergleicht Versionen, behält eine Kopie von allem, was es ersetzt, und jede frühere Version bleibt in der Historie. Zwei verschiedene Spielsitzungen zu einem Spielstand zusammenführen kann es nicht (das kann nichts), aber du kannst wählen, welchen du behältst.

### Zählt das Deck als Gerät?

Ja. Der kostenlose Tarif umfasst drei Geräte, also passen ein PC, ein Laptop und ein Deck hinein. Pro und selbst gehostete Server haben kein Gerätelimit.

### Kann ich auf dem Deck stattdessen die Kommandozeilenversion nutzen?

Ja. Der Befehl `hoard` startet denselben Motor ohne Fenster, was manche auf einem Handheld bevorzugen. Siehe [die CLI-Seite](/cli).
