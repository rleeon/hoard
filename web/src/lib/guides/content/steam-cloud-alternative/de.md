---
title: "Steam-Cloud-Alternative: sichere die Spielstände, die Steam nicht sichert"
description: "Steam Cloud deckt nur Steam-Spiele ab, deren Entwickler sie aktiviert hat, und führt keine Versionshistorie. Hoard sichert jedes Spiel, das du spielst, aus jedem Store, mit einer Historie zum Zurückrollen — in der Cloud oder auf deinem eigenen Server."
order: 7
updated: 2026-09-01
---

Steam Cloud macht die eng umrissene Aufgabe, die sie hat, wirklich gut, und die meisten stoßen erst an dem Tag an ihre Grenzen, an dem etwas verloren geht. Diese Anleitung zeigt, wo diese Grenzen liegen und was mit den Spielen zu tun ist, die dahinter liegen.

## Was Steam Cloud tatsächlich abdeckt

Steam Cloud synchronisiert den Ordner eines Spiels, wenn **der Entwickler es eingerichtet hat** — indem er angibt, welche Dateien zu synchronisieren sind, oder indem das Spiel die Steam-API aufruft. Das ist das ganze Modell, und daraus folgen drei Dinge:

- Es funktioniert nur für Spiele, die über Steam gekauft und gestartet werden.
- Ob es überhaupt funktioniert, entscheidet der Entwickler, pro Spiel und manchmal pro Plattform.
- Jedes Spiel hat sein eigenes Speicherkontingent, festgelegt von diesem Entwickler.

Wenn es funktioniert, ist es unsichtbar und hervorragend: Spiel auf dem einen PC schließen, auf dem anderen öffnen, Fortschritt ist da.

## Wo es dich im Regen stehen lässt

- **Alles, was kein Steam-Spiel ist.** GOG, Epic, itch, Battle.net, die Xbox-App, Emulatoren, alles von Hand Installierte. Steam weiß nicht, dass es existiert.
- **Steam-Spiele, bei denen es nie aktiviert wurde.** Viele Titel, gerade ältere oder kleinere, haben es schlicht nicht. Die Shop-Seite sagt es, aber niemand schaut nach, bevor er 60 Stunden investiert.
- **Es gibt kein Zurück.** Das ist der große Punkt. Steam hält den aktuellen Zustand deines Spielstands, nicht dessen Geschichte. Wird die Datei beschädigt, frisst ein Mod deine Welt, oder überschreibst du einen guten Stand mit einem schlechten, dann ist die Cloud-Kopie bereits der schlechte. Du kannst die Dateien ansehen, die Steam für ein Spiel hält, aber es gibt keine frühere Version zum Wiederherstellen.
- **Der Konfliktdialog.** Wenn Steam local und remote für uneinig hält, sollst du wählen — mit kaum mehr als zwei Zeitstempeln als Grundlage. Wählst du falsch, ist die andere Kopie weg.

## Was Hoard ergänzt

Hoard beobachtet den Ordner, in den ein Spiel wirklich schreibt, und sichert **nach jedem Spielen eine neue Version**:

- **Woher das Spiel stammt, ist egal.** Steam, GOG, Epic, itch, Emulatoren oder ein Ordner, auf den du es von Hand richtest.
- **Jede Version bleibt erhalten**, ein beschädigter Stand oder eine schlechte Entscheidung kosten also zwei Klicks statt eines Spieldurchgangs.
- **Es synchronisiert zwischen deinen Geräten**, Steam Deck und Desktop eingeschlossen.
- **Nichts wird stillschweigend zerstört.** Der ersetzte Stand wird zuerst gesichert, selbst eine falsche Wiederherstellung ist also umkehrbar.

Snapshots werden per Inhalts-Hash gespeichert, zehn Versionen eines 2 GB großen Stands kosten also etwa 2 GB, nicht 20 — und das macht die komplette Historie überhaupt praktikabel.

## Beides gleichzeitig nutzen

Sie stören sich nicht, du musst dich nicht entscheiden. Bei einem Steam-Spiel mit Cloud-Unterstützung lass Steam synchronisieren, was es ohnehin tut; Hoards Beitrag dort ist die Historie — genau das, was Steam nicht führt. Für alles andere übernimmt Hoard auch die Synchronisierung.

Ein Detail, das zählt, wenn du neben dem Desktop ein Steam Deck hast: Hoard verfolgt `<AppID>/remote/` innerhalb von `userdata`, nicht den Ordner darüber, denn der enthält `remotecache.vdf` und gerätebezogene Dateien für Erfolge und Spielzeit. Genau diese Unterscheidung geht bei selbstgebauter Synchronisierung meist schief, weshalb solche Setups bei jedem Start zu kollidieren scheinen.

## Wann Steam Cloud reicht

Ehrlich gesagt: wenn alle deine Spiele Steam-Spiele mit Cloud-Unterstützung sind, du an einem PC spielst und noch nie einen Spielstand zurücknehmen musstest, erledigt Steam Cloud die Aufgabe und du brauchst nichts weiter. Für Hoard sprechen die Versionshistorie, Spiele außerhalb von Steam und Geräte, die Steam Cloud nicht erreicht.

## Ganz ohne fremde Cloud

Wenn der Reiz darin liegt, von keiner Plattform abzuhängen: Hoard läuft komplett auf deiner eigenen Hardware — `hoard-server` auf einem PC oder NAS, und deine Stände gehen von deiner Maschine auf deine Platte. Es gibt **kein Konto bei uns, keine Telemetrie zu uns und kein Relay** — nichts läuft über unsere Server, weil nichts von uns im Weg steht. Siehe [wie du Hoard selbst hostest](/guides/self-host-hoard).

Dasselbe Programm, dieselbe Erkennung, dieselbe Versionshistorie. Es ändert sich nur, wem der Speicher gehört.

<!-- faq -->

## Häufige Fragen

### Ersetzt Hoard Steam Cloud?

Muss es nicht. Steam Cloud hält deinen aktuellen Stand für die unterstützten Spiele synchron; Hoard ergänzt die Versionshistorie und deckt die übrigen Spiele ab. Beides parallel zu nutzen ist der Normalfall.

### Kann Steam Cloud zu einem älteren Spielstand zurück?

Nein. Steam hält den aktuellen Zustand der Dateien, nicht deren Geschichte. Ist ein schlechter Stand einmal synchronisiert, steht genau der in der Cloud. Zurück geht es nur mit einem versionierenden Werkzeug.

### Warum synchronisieren nicht alle meine Steam-Spiele?

Weil der Entwickler es aktiviert, pro Spiel und manchmal pro Plattform. Die Shop-Seite führt Steam Cloud unter den Features auf, wenn es unterstützt wird — und viele Titel tun das schlicht nicht.

### Funktioniert Hoard mit Nicht-Steam-Spielen?

Ja, das ist ein Großteil des Sinns. Es findet Spielstände über eine Community-Datenbank mit über 20.000 Titeln, aus jedem Store, und für Ungewöhnliches richtest du es von Hand auf einen Ordner.

### Gibt es Konflikte, wenn beides läuft?

Nein. Hoard sichert eine Version, nachdem du aufgehört hast und der Ordner zur Ruhe kommt, und überschreibt nie, ohne das Ersetzte vorher zu sichern.

### Kann ich meine Stände aus beiden Clouds heraushalten?

Ja. Hoste den Server selbst, dann verlassen deine Spielstände nie deine eigene Hardware — ohne Konto und ohne Telemetrie an irgendwen.
