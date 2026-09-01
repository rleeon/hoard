---
title: "So stellst du einen alten Spielstand wieder her"
description: "Falsche Entscheidung getroffen, Datei beschädigt oder Neustart gewünscht? Springe mit Hoards Cloud-Historie zu jeder früheren Version deines Spielstands zurück — auch zu Ständen, die mit Tools wie Ludusavi gesichert wurden."
order: 3
updated: 2026-09-01
---

Eine schlechte Entscheidung im Spiel, eine beschädigte Datei oder ein verpfuschter Mod — manchmal musst du einfach zurück. Da Hoard eine vollständige Versionshistorie jedes Stands führt, dauert die Wiederherstellung eines früheren nur Sekunden.

## Eine frühere Version wiederherstellen

1. Öffne **Hoard** und gehe zum Spiel in deiner **Bibliothek**.
2. Öffne den Reiter **Historie**. Du siehst jedes Backup mit Datum und Größe.
3. Wähle die gewünschte Version und klicke auf **Wiederherstellen**.
4. Hoard schreibt diesen Snapshot zurück in den Speicherordner des Spiels. Dein aktueller Stand wird zuerst gesichert, die Wiederherstellung ist also umkehrbar.

## Auf einem neuen oder neu installierten PC wiederherstellen

1. Installiere Hoard und melde dich mit deinem Konto an.
2. Füge das Spiel zu deiner Bibliothek hinzu — Hoard findet das passende Cloud-Backup.
3. Stelle die neueste Version oder eine ältere wieder her und spiele weiter.

Da Hoard Speicherordner mit derselben Community-Datenbank wie Ludusavi findet, weiß es selbst bei einer Neuinstallation, wohin ein wiederhergestellter Stand gehört — ohne manuelle Pfadsuche.

## Wenn ein Spielstand beschädigt ist oder ein Mod ihn zerlegt hat

Ein Spiel, das beim Laden abstürzt, ein Mod, der etwas überschrieben hat, ein Autosave mitten im Schreibvorgang: die Lösung ist dieselbe. Öffne die **Historie** des Spiels, wähle die letzte Version von vor dem Problem und stelle sie wieder her. Datum und Größe reichen meist, um den Moment zu finden — ein plötzlicher Größensturz ist ein gutes Zeichen dafür, dass ein Stand abgeschnitten wurde.

Wenn du nicht sicher bist, welche die richtige ist, stelle die wahrscheinlichste wieder her und prüfe es im Spiel. Ein zweiter Versuch kostet nichts, denn die eben ersetzte Version wurde ebenfalls behalten.

## Was beim Wiederherstellen tatsächlich passiert

Drei Dinge, die man wissen sollte, denn sie machen einen Versuch gefahrlos:

1. **Dein aktueller Stand wird zuerst gesichert.** Die Wiederherstellung ist umkehrbar: das Ersetzte wird eine Version in der Historie wie jede andere.
2. **Es wird nur geladen, was fehlt.** Dateien, die mit dem richtigen Inhalt schon auf der Platte liegen, werden so verwendet — einen großen Spielstand nach einer kleinen Änderung wiederherzustellen bewegt ein paar Megabyte statt des ganzen Ordners.
3. **Dateien dieses Rechners bleiben unangetastet.** Konfiguration und Logs neben dem Spielstand werden gesichert, aber nicht über deine lokalen Kopien geschrieben: Tastenbelegung und Grafikeinstellungen überleben eine Wiederherstellung von einem anderen PC.

## Wiederherstellen ohne unsere Server

Wenn du deinen eigenen `hoard-server` betreibst, funktioniert das Wiederherstellen genauso, nur kommen die Versionen von deiner Maschine statt von unserer. Es gibt kein Konto bei uns, keine Telemetrie zu uns und nichts, was über unsere Server läuft. Siehe [wie du Hoard selbst hostest](/guides/self-host-hoard).

## Tipp

Wiederherstellungen sind nie zerstörerisch: Der ersetzte Stand wird zuerst als neue Version erfasst, du kannst eine Wiederherstellung also immer rückgängig machen, indem du den vorherigen Eintrag wiederherstellst. Hast du bisher nur lokale Backups geführt (etwa mit Ludusavi), ergänzt der Wechsel zu Hoard eine geräteunabhängige, versionierte Historie, aus der du selbst nach einem Festplattenausfall wiederherstellen kannst.

<!-- faq -->

## Häufige Fragen

### Überschreibt eine Wiederherstellung meinen aktuellen Fortschritt?

Erst nachdem dein aktueller Stand als neue Version gesichert wurde. Hast du die falsche gewählt, stelle den vorherigen Eintrag wieder her und du bist zurück am Ausgangspunkt.

### Wie weit reicht die Historie zurück?

So weit, wie das Versionslimit deines Tarifs erlaubt, und eine angeheftete Version wird nie weggeräumt, um Platz zu schaffen. Auf einem selbst gehosteten Server ist die einzige Grenze deine Platte.

### Kann ich auf einen PC wiederherstellen, auf dem das Spiel noch nicht installiert ist?

Installiere zuerst das Spiel, damit sein Speicherordner existiert, und stelle dann wieder her. Hoard weiß, wo jedes Spiel seine Stände erwartet, und schreibt den Snapshot an die richtige Stelle, ohne dass du den Pfad suchen musst.

### Klappt das zwischen Windows und einem Steam Deck?

Ja. Dasselbe Spiel legt seinen Stand auf beiden Geräten woanders ab — auf dem Deck im Proton-Prefix — und Hoard schreibt die wiederhergestellte Version dorthin, wo diese Maschine sie erwartet.

### Ist die Wiederherstellung auf einem selbst gehosteten Server anders?

Nein. Gleiche App, gleiche Historie, gleiche Wiederherstellung per Klick. Nur der Speicher gehört dir.
