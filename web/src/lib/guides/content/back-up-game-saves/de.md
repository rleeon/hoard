---
title: "So sicherst du deine Spielstände automatisch"
description: "Richte automatische, versionierte Cloud-Backups für deine PC-Spielstände mit Hoard ein — damit ein Absturz, eine Neuinstallation oder ein fehlerhafter Mod deinen Fortschritt nie löschen kann."
order: 1
updated: 2026-09-01
---

Ein verlorener Spielstand bedeutet verlorene Stunden an Fortschritt. Hoard sichert deine PC-Spielstände automatisch und führt eine vollständige Versionshistorie, sodass du immer zurückgehen kannst.

## Was Hoard sichert

Hoard erkennt die Speicherordner der Spiele, die du spielst, und kopiert sie in deine eigene Cloud — entweder Hoard Cloud oder einen selbst gehosteten Server. Jedes Backup ist versioniert, ältere Kopien werden also nie überschrieben.

Um zu finden, wo jedes Spiel seine Stände ablegt, nutzt Hoard dieselbe Community-Datenbank für Speicherorte, die auch Ludusavi antreibt — die Erkennung funktioniert also sofort für Tausende von Titeln. Der Unterschied liegt darin, was danach passiert: Statt das Backup auf deiner Festplatte zu belassen, versioniert Hoard es automatisch in der Cloud.

## Automatische Backups einrichten

1. **Lade Hoard herunter und installiere es** für Windows, macOS oder Linux von der Download-Seite.
2. Melde dich an oder richte die App auf deinen selbst gehosteten Server aus.
3. Öffne die **Bibliothek**. Hoard sucht nach installierten Spielen und listet die gefundenen Stände auf.
4. Füge die Spiele hinzu, die du schützen willst. Hoard findet jeden Speicherordner automatisch; du kannst einen Pfad von Hand ergänzen, falls ein Spiel nicht erkannt wird.
5. Lass den **Automatikmodus** an. Hoard überwacht die Speicherordner und sichert sie, nachdem du aufhörst zu spielen.

Ab jetzt wird jede Sitzung erfasst, ohne dass du etwas tun musst.

## Wo PC-Spiele ihre Stände wirklich ablegen

Es gibt keinen einzigen Ort, und genau deshalb existiert so ein Werkzeug. In der Praxis landet ein Spielstand an einer dieser Stellen:

- **In Steam**, unter `userdata/<UserID>/<AppID>/remote/` — dem Ordner, den Steam Cloud selbst synchronisiert.
- **`Dokumente\My Games\…`**, das Nächste, was Windows an Konvention zu bieten hat.
- **`%APPDATA%`, `%LOCALAPPDATA%` oder `LocalLow`**, wo die meisten Unity- und Unreal-Spiele schreiben.
- **`%USERPROFILE%\Saved Games`**, genutzt von einer kleineren, aber hartnäckigen Gruppe von Titeln.
- **Im Installationsordner des Spiels selbst**, wo erstaunlich viele ältere Titel weiterhin speichern.
- **Unter Linux** `~/.local/share` oder `~/.config` für native Spiele, und im Proton-Prefix — `steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…` — für Windows-Spiele.
- **Unter macOS** `~/Library/Application Support`.

Woher das Spiel stammt, spielt kaum eine Rolle: Titel von GOG, Epic und itch landen an derselben Handvoll Orte, denn das entscheiden Engine und Entwickler, nicht der Store.

## Was gesichert wird und was nicht

Ein Speicherordner enthält selten nur Spielstände, deshalb sortiert Hoard, was es findet, auf drei Stapel:

- **Spielstanddaten** werden gesichert und wiederhergestellt. Das ist dein Fortschritt.
- **Dateien, die zu einem bestimmten Rechner gehören** — Konfiguration, Logs und Ähnliches — werden hochgeladen, damit sie Teil des Backups sind, aber nie über die Kopie eines anderen PCs geschrieben. Deine Grafikeinstellungen bleiben deine.
- **Müll** — Caches, Absturzberichte, temporäre Dateien — wird ignoriert, damit ein Backup nicht mit Dingen aufquillt, die du nie zurückhaben willst.

## Wann gesichert wird

Hoard beobachtet den Ordner und sichert ihn, **nachdem du aufgehört hast zu spielen**, nicht während ein Spiel Dateien offen hält. Wurde der Stand vor Sekunden geschrieben, wartet es, bis Ruhe einkehrt: eine Datei im Schreibvorgang ist keine Datei, die man halb sichern will.

Jede Sicherung ist eine Version. Snapshots werden per Inhalts-Hash gespeichert, unveränderte Dateien also nur einmal — zehn Versionen eines 2 GB großen Stands kosten etwa 2 GB, nicht 20.

## Sichern ohne unsere Server

Wenn du lieber niemandes Cloud nutzt, betreibe `hoard-server` selbst und richte die App darauf. Deine Stände gehen von deinem PC auf deine Platte: kein Konto bei uns, keine Telemetrie zu uns, und nichts, was über unsere Server läuft. Siehe [wie du Hoard selbst hostest](/guides/self-host-hoard).

## Tipp: Prüfe deine Historie

Öffne den Reiter **Historie** eines Spiels, um jedes Backup mit Datum und Größe zu sehen. Von dort kannst du jede frühere Version mit einem Klick wiederherstellen. Deine Stände werden verschlüsselt übertragen, in der EU gespeichert, und du kannst sie jederzeit exportieren oder löschen.

Nutzt du bereits ein lokales Backup-Tool wie Ludusavi? Du kannst es behalten — aber wenn diese Backups in der Cloud landen und zwischen Geräten synchronisieren sollen, ohne dass du Rclone selbst einrichtest, ist genau das, was Hoard automatisiert. Siehe [Ludusavi vs. Hoard](/guides/ludusavi-alternative) für einen fairen Vergleich.

<!-- faq -->

## Häufige Fragen

### Sichert Hoard, während ich spiele?

Nein. Es wartet, bis du aufhörst und der Speicherordner zur Ruhe kommt, damit ein Backup nie eine halb geschriebene Datei ist.

### Wie viel Platz brauchen meine Spielstände?

Weniger als gedacht. Versionen werden per Inhalts-Hash dedupliziert, neuen Platz belegt also nur, was sich zwischen zwei Sitzungen wirklich geändert hat — die meisten Sammlungen passen bequem in ein paar Gigabyte.

### Was, wenn eines meiner Spiele nicht erkannt wird?

Richte Hoard von Hand auf den Ordner, dann verfolgt es ihn wie jeden anderen. Die Erkennung deckt Tausende Titel ab, aber ein Spiel, das an einer ungewöhnlichen Stelle speichert oder das du von Hand installiert hast, braucht manchmal den Hinweis.

### Sichert es auch meine Mods?

Hoard verfolgt den Speicherordner, Mods an anderer Stelle sind also nicht Teil des Backups. Das ist Absicht: Mods sind groß, sie lassen sich neu herunterladen, und ein zwischen Rechnern synchronisierter Mod-Ordner schafft mehr Probleme, als er löst.

### Ändert Selbsthosten etwas an den Backups?

Überhaupt nicht. Gleiche Erkennung, gleiche Versionen, gleiche automatische Sicherung. Nur der Speicher gehört dir.
