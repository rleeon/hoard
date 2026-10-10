---
title: "Spielstände zwischen ROG Ally, Legion Go, MSI Claw und deinem PC synchronisieren"
description: "Windows-Handhelds wie ROG Ally, Legion Go und MSI Claw sind einfach PCs. Synchronisiere ihre Spielstände automatisch mit deinem Desktop, mit Versionsverlauf."
order: 19
updated: 2026-10-09
---

ROG Ally, Legion Go und MSI Claw laufen mit Windows, für ein Spiel sind sie also einfach ein weiterer PC. Genau das ist das Problem: Dein Desktop und dein Handheld behalten jeweils ihre eigenen Spielstände. Steam Cloud deckt einen Teil deiner Bibliothek ab und die Xbox-Cloud den Game Pass, aber alles andere bleibt auf dem Gerät, auf dem du gespielt hast. Hoard hält die Spielstände zwischen Handheld und Desktop automatisch synchron: Hörst du auf einem auf, wartet dein Spiel auf dem anderen, und jede frühere Version bleibt erhalten.

## Was dir schon folgt

- **Steam-Spiele mit Steam Cloud** synchronisieren sich von selbst.
- **Game-Pass- und Xbox-App-Spiele** nutzen die Xbox-Cloud, solange du auf beiden Geräten die Xbox-Version spielst.
- **Epic, GOG, Ubisoft und EA** haben für einige ihrer Spiele Cloud-Spielstände, innerhalb ihrer eigenen Launcher. Siehe [Cloud-Spielstände bei Epic und GOG](/guides/epic-gog-cloud-saves).

Was übrig bleibt: Spiele, bei denen der Entwickler nie Cloud-Spielstände eingeschaltet hat, Emulatoren, von Hand installierte Spiele und jedes Spiel, bei dem Desktop und Handheld nicht denselben Launcher nutzen.

## Einrichten

1. **Auf dem Handheld** wechselst du auf den Windows-Desktop, öffnest [die Downloadseite](/download) und installierst Hoard für Windows.
2. **Melde dich** mit dem Konto an, das du auf dem Desktop nutzt, oder richte die App auf deinen eigenen Server aus.
3. Öffne die **Bibliothek** und prüfe, was Hoard gefunden hat. Ergänze Fehlendes, indem du auf den Ordner zeigst, etwa bei einem Emulator.
4. **Auf dem Desktop** installierst du Hoard mit demselben Konto. Dieselben Spiele finden von selbst zueinander.

Der Sync-Motor läuft als Hintergrunddienst, der mit Windows startet, er arbeitet also weiter, während du in Armoury Crate, Legion Space, MSI Center M oder Steams Big-Picture-Modus bist. Zum Spielen musst du Hoards Fenster nicht öffnen.

## Handheld-Fallen

### Standby ist nicht Beenden

Bei einem Handheld ist es leicht, die Ein/Aus-Taste zu drücken und das Gerät mit laufendem Spiel wegzulegen. Hoard sichert erst, wenn das Spiel geschlossen ist, weil ein laufendes Spiel gerade mitten im Schreiben sein kann, und es tauscht nie den Spielstand eines laufenden Spiels aus. Schickst du den Handheld in den Standby und spielst dann am Desktop, ist der Fortschritt vom Handheld noch nicht hochgeladen. **Beende das Spiel, bevor du wechselst.**

### Spiele auf der microSD-Karte

Spiele auf der Karte zu installieren ist bei einem Handheld normal, und für Spielstände spielt es selten eine Rolle: Die meisten Spiele speichern in deinem Benutzerordner auf dem internen Laufwerk, egal wo sie installiert sind. Ausnahmen sind Spiele, die neben ihrem Installationsordner speichern; wird so eines nicht erkannt, füg seinen Ordner von Hand hinzu.

### Bildschirm und Einstellungen

Dein Handheld läuft mit niedrigerer Auflösung und kleinerer GPU als dein Desktop. Hoard sichert Einstellungsdateien wie `graphics.ini` mit dem Spielstand, schreibt sie aber nicht über die des anderen Geräts, also behält jedes die Einstellungen, die zu ihm passen. Willst du sie trotzdem übernehmen, gibt es beim Wiederherstellen eine Option dafür.

### Dasselbe Spiel, ein anderer Store

Ein auf Steam für den Desktop gekauftes Spiel und dasselbe über Game Pass auf dem Handheld sind zwei verschiedene Installationen, und die Xbox-Version speichert in einem Format, das nur die Xbox-App versteht. Um einen Spielstand zu teilen, spiel auf beiden die Version desselben Stores.

### SteamOS oder Bazzite statt Windows?

Dann ist dein Handheld ein Linux-Rechner, und die Spielstände liegen in Proton-Präfixen, genau wie auf einem Steam Deck. Siehe [Spielstände zwischen Steam Deck und PC synchronisieren](/guides/sync-saves-steam-deck-pc).

## Ohne unsere Server

Sollen die Spielstände zu Hause bleiben, starte `hoard-server` auf deinem PC oder einem NAS und richte beide Geräte darauf aus. Kein Konto bei uns, keine Telemetrie an uns, nichts über unsere Server. Siehe [Hoard selbst hosten](/guides/self-host-hoard).

<!-- faq -->

## Häufige Fragen

### Hat der ROG Ally Cloud-Spielstände?

Er hat, was jeder Launcher hat: Steam Cloud, die Xbox-Cloud und die Clouds von Epic, GOG, Ubisoft oder EA für die Spiele, die sie unterstützen. Einen systemweiten Spielstand-Sync gibt es nicht. Hoard ergänzt einen für die Spiele, die diese auslassen.

### Funktioniert Hoard mit Armoury Crate oder Legion Space?

Ja. Hoards Sync-Motor ist ein Windows-Hintergrunddienst, unabhängig davon, mit welchem Launcher du Spiele startest.

### Zählt der Handheld als Gerät?

Ja. Der kostenlose Tarif umfasst drei Geräte, also passen ein Desktop, ein Laptop und ein Handheld hinein. Pro und selbst gehostete Server haben kein Gerätelimit.

### Und die Game-Pass-Spielstände?

Überlass die der Xbox-Cloud, die sie zwischen den Xbox-App-Installationen beider Geräte synchronisiert. Hoard deckt die Spiele ab, die keine eigene Cloud haben.

### Kann ich den Handheld auch mit einem Steam Deck synchronisieren?

Ja. Hoard läuft auf beiden und ordnet jedes Spiel zwischen Windows und den Proton-Präfixen des Deck zu.
