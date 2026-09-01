---
title: "Ludusavi-Alternative: automatische Cloud-Synchronisierung für deine Spielstände"
description: "Ein fairer Vergleich von Ludusavi und Hoard. Ludusavi ist ein großartiges Open-Source-Tool für lokale Backups; Hoard ergänzt verwaltete Cloud-Synchronisierung und versionierte Historie über alle deine PCs — mit denselben Speicherort-Daten."
order: 5
updated: 2026-09-01
---

Wenn du nach einer Möglichkeit suchst, deine Spielstände zu sichern und zu synchronisieren, bist du wahrscheinlich auf **Ludusavi** gestoßen — und es ist hervorragend. Diese Anleitung ist ein ehrlicher Vergleich, damit du das richtige Tool wählst, und erklärt, wo Hoard passt, wenn du automatische Cloud-Synchronisierung über mehrere Geräte willst.

## Was Ludusavi gut macht

Ludusavi ist ein kostenloses Open-Source-Tool (von mtkennerly), um PC-Spielstände unter Windows, macOS und Linux zu sichern und wiederherzustellen. Es hat eine aufgeräumte GUI und eine CLI, findet Stände für Tausende Spiele automatisch, führt versionierte lokale Backups und kann diese über **Rclone** in eine eigene Cloud übertragen (Google Drive, Dropbox und viele andere). Wenn du volle Kontrolle und ein Do-it-yourself-Setup willst, ist Ludusavi eine fantastische Wahl — und völlig kostenlos.

Hoard will das nicht ersetzen. Tatsächlich nutzt **Hoard dieselbe Community-Datenbank für Speicherorte, auf die sich auch Ludusavi stützt**, um zu finden, wo jedes Spiel seine Stände ablegt — die Erkennungsqualität ist also gleichwertig.

## Worin sich Hoard unterscheidet

Die Lücke, auf die die meisten bei jedem lokalen Tool stoßen, ist die **Synchronisierung über Geräte hinweg**. Mit Ludusavi machst du das selbst: Backup planen, Rclone-Remote konfigurieren, dann auf dem anderen PC wiederherstellen, bevor du spielst. Das funktioniert, ist aber manuell.

Hoard macht daraus **verwaltete Cloud-Synchronisierung**:

- **Anmelden und loslegen.** Keine Rclone-Remotes, keine Skripte. Hoard lädt deinen Stand nach dem Spielen hoch und vor dem Start die neueste Version herunter, auf jedem PC deines Kontos.
- **Versionierte Historie in der Cloud.** Jedes Backup bleibt erhalten, du kannst also zu jedem früheren Stand zurück — sogar nach einem Festplattenausfall oder einer Neuinstallation.
- **Konfliktbewusst.** Hoard vergleicht Zeitstempel und behält eine lokale Kopie von allem, was es ersetzt, sodass eine Synchronisierung nie stillschweigend Fortschritt zerstört.
- **Weiterhin Open Source und selbst hostbar.** Wie bei Ludusavi gibt es keine Bindung — nutze Hoard Cloud oder hoste den Server selbst.

## Direkter Vergleich

| | Ludusavi | Hoard |
|---|---|---|
| Lokale Backups | Ja | Ja |
| Erkennung der Stände | Community-Manifest | Dasselbe Manifest, dazu Steam-Bibliotheken, laufende Prozesse und ein Dateisystem-Scan |
| Cloud-Speicher | Eigener, über Rclone | Enthalten, oder dein eigener Server |
| Synchronisierung zwischen PCs | Manuell: hier sichern, dort wiederherstellen | Automatisch, nach dem Spielen und vor dem Start |
| Versionshistorie | Lokale Backups, die du selbst aufräumst | Jede Version in der Cloud, dedupliziert per Inhalts-Hash |
| Emulatoren | Ja | Ja |
| Oberflächen | Desktop-App und CLI | Desktop-App, CLI und ein Overlay im Spiel |
| Preis | Kostenlos | Kostenlos mit 2 GB und 3 Geräten, Pro darüber, ohne Limit beim Selbsthosten |
| Lizenz | MIT | AGPL-3.0 |

## Wann Ludusavi die bessere Wahl ist

Das ist der Teil, den die meisten Vergleichsseiten weglassen. Ludusavi ist das bessere Werkzeug, wenn:

- **Du nur an einem PC spielst.** Cloud-Synchronisierung löst dann ein Problem, das du nicht hast. Ein lokales Backup reicht, und darin ist Ludusavi sehr gut.
- **Du bereits ein Rclone-Remote hast, dem du vertraust.** Wenn dein Speicher eingerichtet ist und läuft, ist Hoards Hauptvorteil ein Einrichtungsschritt, den du längst hinter dir hast.
- **Du es im Spielmodus des Steam Deck nutzen willst.** Für Ludusavi gibt es ein Decky-Plugin, du kannst ein Backup also anstoßen, ohne die Konsolenoberfläche zu verlassen.
- **Du eine permissive Lizenz brauchst.** Ludusavi ist MIT, Hoard ist AGPL-3.0. Wenn du etwas darauf aufbauen und das Ergebnis nicht veröffentlichen willst, macht dieser Unterschied viel aus.
- **Du willst nichts laufen haben.** Hoard selbst zu hosten heißt, irgendwo einen kleinen Server am Laufen zu halten, und sei es derselbe PC. Ludusavi ist eine App, die du öffnest, wenn du sie brauchst.

## Von Ludusavi zu Hoard wechseln

Es gibt keinen Import, und das ist Absicht. Die Schritte:

1. **Lass deine Ludusavi-Backups genau dort, wo sie sind.** Es wird nichts migriert und nichts gelöscht. Behalte sie in den ersten Wochen als Sicherheitsnetz.
2. **Installiere Hoard und melde dich an**, oder richte es auf deinen eigenen Server.
3. **Lass es scannen.** Es liest dasselbe Manifest, die Liste der erkannten Spiele sollte dir also bekannt vorkommen.
4. **Richte Hoard nicht auf deinen Ludusavi-Backup-Ordner.** Verfolge den Ordner, in den das Spiel selbst schreibt. Ein Backup-Ordner ist eine Kopie, die sich nach Zeitplan ändert statt beim Spielen, und die Kopie einer Kopie zu synchronisieren ist der Weg, am Ende den Fortschritt von gestern wiederherzustellen. Hoard versucht das selbst zu erkennen — `hoard doctor` meldet einen verfolgten Ordner, der wie ein Backup-Spiegel aussieht — aber am einfachsten ist, ihn gar nicht erst aufzunehmen.
5. **Spiel einmal.** Beim Beenden erscheint die erste Version in der Historie.
6. **Wiederhole das am zweiten PC.** Dort anmelden, und die Versionen liegen schon bereit.

## Zwei Details, die man kennen sollte

**Steam-Spielstände liegen einen Ordner tiefer als gedacht.** Bei Steam-Spielen verfolgt Hoard `<AppID>/remote/` innerhalb von `userdata`, nicht den Ordner darüber. Der übergeordnete Ordner enthält auch `remotecache.vdf` sowie Dateien für Erfolge und Spielzeit, und die unterscheiden sich zu Recht von Rechner zu Rechner. Synchronisierst du den übergeordneten Ordner, sieht jeder Start nach einem Konflikt aus, obwohl sich kein einziger Spielstand bewegt hat. Das ist der häufigste Grund, warum ein selbstgebautes Setup zwischen Steam Deck und Desktop gegen sich selbst arbeitet.

**Versionen sind billig.** Snapshots werden per Inhalts-Hash gespeichert, unveränderte Dateien also nur einmal. Zehn Versionen eines 2 GB großen Spielstands kosten etwa 2 GB, nicht 20 — und genau das macht es praktikabel, die komplette Historie zu behalten, statt sie auszudünnen.

## Was Selbsthosten wirklich bedeutet

Genau hier liegen die meisten Vergleiche bei Hoard falsch, deshalb der Punkt im Detail. Es gibt zwei Betriebsarten, und sie unterscheiden sich wirklich:

- **Hoard Cloud** ist die verwaltete Variante: du meldest dich an, und deine Spielstände liegen auf unseren Servern in der EU.
- **Selbsthosten gehört vollständig dir.** Du betreibst `hoard-server` auf deinem eigenen PC oder NAS, und deine Stände gehen von deiner Maschine auf deine Platte. Es gibt **kein Konto bei uns, keine Telemetrie zu uns, kein Limit und kein Relay** — nichts läuft über unsere Server, weil nichts von uns im Weg steht. Wir können weder einen Spielstand noch einen Spieltitel noch eine E-Mail-Adresse sehen, schlicht weil davon nichts bei uns ankommt. Verschwände Hoard Cloud morgen, liefe ein selbst gehostetes Setup unverändert weiter.

Dasselbe Programm, dieselbe Erkennung, dieselbe Versionshistorie. Es ändert sich nur, wem der Speicher gehört.

## Was solltest du wählen?

- Wähle **Ludusavi**, wenn du ein kostenloses, lokal orientiertes Backup-Tool willst und gern deine eigene Cloud mit Rclone einrichtest.
- Wähle **Hoard**, wenn Backups *und* automatische Synchronisierung über PCs einfach funktionieren sollen, mit versionierter Cloud-Historie und der Option, selbst zu hosten.

Viele beginnen mit Ludusavi für lokale Backups und wechseln zu Hoard, sobald sie dieselben Spiele auf mehr als einem Gerät spielen. Wenn das auf dich zutrifft, siehe [wie du Spielstände über PCs synchronisierst](/guides/sync-game-saves-across-pcs) oder [lade einfach Hoard herunter](/download) und melde dich an. Einen Blick auf das ganze Feld gibt der [Vergleich aller Sync-Tools](/guides/game-save-sync-comparison).

<!-- faq -->

## Häufige Fragen

### Kann ich Ludusavi und Hoard gleichzeitig nutzen?

Ja. Beide lesen dieselben Speicherorte und keines hält die Dateien geöffnet. Viele behalten Ludusavi für lokale Archiv-Backups und überlassen Hoard die Synchronisierung zwischen Geräten. Die einzige Regel: richte keines der beiden Werkzeuge auf den Backup-Ordner des anderen.

### Importiert Hoard meine Ludusavi-Backups?

Nein, und das ist Absicht. Ein Backup-Ordner ist eine Kopie, die sich nach eigenem Zeitplan ändert; ihn zu verfolgen würde einen veralteten Spiegel synchronisieren statt deines echten Spielstands. Hoard verfolgt den Ordner, in den das Spiel schreibt, und beginnt seine eigene Historie mit deiner nächsten Sitzung. Behalte das Ludusavi-Archiv als Sicherheitsnetz.

### Ist Hoard kostenlos?

Hoard Cloud hat einen kostenlosen Tarif mit 2 GB Speicher und 3 Geräten, was für die meisten Sammlungen reicht; Pro hebt beides an. Den Server selbst zu hosten ist kostenlos und hat überhaupt kein Limit. Alles ist Open Source unter AGPL-3.0.

### Funktioniert Hoard auf dem Steam Deck?

Ja, auf dem Steam Deck und jedem Linux-Desktop, ebenso unter Windows und macOS. Das Deck ist genau der Fall, für den das `remote/`-Detail oben wichtig ist: Deck und Desktop schreiben neben demselben Spielstand unterschiedliche Dateien für Erfolge und Spielzeit.

### Brauche ich Rclone oder ein eigenes Cloud-Konto?

Nein. Das ist der wesentliche praktische Unterschied: Bei Hoard Cloud ist der Speicher schon eingerichtet, sobald du dich anmeldest. Wenn dir der Speicher lieber selbst gehört, betreibe den Server selbst gegen einen S3-kompatiblen Bucket oder einen gewöhnlichen Ordner auf deiner eigenen Maschine.

### Sendet Selbsthosten irgendetwas an Hoard?

Nein. Im selbst gehosteten Betrieb gibt es kein Konto bei uns und keine Telemetrie zu uns: deine Spielstände, deine Nutzer und deine Logs liegen auf deinem eigenen Server und berühren unseren nie. Das ist der ganze Sinn dieses Modus, und deshalb ist der Server dasselbe quelloffene Binary, das wir selbst betreiben, und keine abgespeckte Fassung.
