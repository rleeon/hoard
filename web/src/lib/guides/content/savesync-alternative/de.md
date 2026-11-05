---
title: "Alternative zu SaveSync: kostenloser Open-Source-Sync für jedes Spiel"
description: "Kostenlose SaveSync-Alternative gesucht? Hoard sichert und synchronisiert Spielstände jedes Spiels, nicht nur Koop, unter Windows, Linux, macOS und Deck."
order: 29
updated: 2026-11-05
---

SaveSync ist eine kostenpflichtige App auf Steam, im Early Access, gebaut für Koop: Es hält eine gemeinsame Welt über private Steam-Workshop-Einträge für die Gruppe verfügbar, auch wenn der Host offline ist. Willst du deine eigenen Spielstände kostenlos synchronisieren, für jedes Spiel und auf jedem deiner Rechner, macht [Hoard](/download) genau das, und es ist Open Source.

## Was SaveSync gut macht

- **Koop-Welten.** Eine Valheim- oder Factorio-Welt an den weiterzugeben, der heute Abend hostet, ist genau sein Job.
- **Steam als Transportweg.** Spielstände reisen als private, nicht gelistete, verschlüsselte Workshop-Einträge, also brauchst du kein weiteres Konto.
- **Verträgt sich mit Steam Cloud.** Es arbeitet neben dem Sync von Steam, statt dagegen.

## Wo Hoard anders ist

| | SaveSync | Hoard |
|---|---|---|
| Preis | Kostenpflichtig, auf Steam | Kostenlos und Open Source (AGPL-3.0) |
| Spiele | Eine Reihe unterstützter Koop-Spiele | Jedes Spiel in der Community-Datenbank, 19 Emulatoren und jeder Ordner, den du hinzufügst |
| Speicher | 500 MB inklusive, mehr in der App zu kaufen | Hoard One kostenlos mit 2 GB und drei Geräten, oder dein eigener Server ohne Kontingent von uns |
| Historie | Je nach App | Jede Version, dedupliziert |
| Plattformen | Windows | Windows, Linux, macOS und SteamOS |
| Eine Welt mit Freunden teilen | Ja | Nein |

Die letzte Zeile ist die ehrliche. Geht es darum, eine Welt unter Freunden weiterzureichen, ist SaveSync dafür gebaut und Hoard nicht. Für alles andere, deine eigenen Spielstände auf Desktop, Laptop und Steam Deck, deckt Hoard mehr Spiele ab und kostet nichts.

## Einrichten

1. Installiere Hoard und melde dich bei Hoard One an, empfohlen für alle, oder richte es auf [deinen eigenen Server](/guides/self-host-hoard).
2. Öffne die **Bibliothek**. Deine Spiele werden über deine Launcher und die Community-Datenbank für Spielstände erkannt.
3. Spiele. Jedes Speichern fügt eine Version hinzu, und mit Hoard auf deinen anderen Rechnern unter demselben Konto wartet dort die neueste. Siehe [Spielstände zwischen Steam Deck und PC synchronisieren](/guides/sync-saves-steam-deck-pc).

## Hoard One oder dein eigener Server

Hoard ist so oder so kostenlos, und die App ist bei beiden dieselbe:

- **Hoard One**, empfohlen für alle. Nichts einzurichten und nichts am Laufen zu halten: ein verwalteter Server in der EU, kostenlos mit 2 GB und drei Geräten, ohne Karte. [Pro](/pricing) bringt nur mehr Platz, 100 GB und unbegrenzt viele Geräte, für den Fall, dass deine Spielstände darüber hinauswachsen.
- **Dein eigener Server.** Lass `hoard-server` auf einem PC, einem NAS oder einem Raspberry Pi laufen, und alle deine Rechner synchronisieren darüber. Kein Konto bei uns, keine Telemetrie an uns und kein Kontingent von uns: Nichts läuft über unsere Server, weil nichts von uns dazwischen ist. Siehe [Hoard selbst hosten](/guides/self-host-hoard).

Dasselbe Programm, dieselbe Erkennung, derselbe Verlauf. Hoard One behält jede Version für immer; auf deinem eigenen Server entscheidest du, wie viel Verlauf du behältst.

<!-- faq -->

## Häufige Fragen

### Gibt es eine kostenlose Alternative zu SaveSync?

Hoard. App, Kommandozeile und Server sind kostenlos und Open Source, und Hoard One, der verwaltete Server, ist kostenlos mit 2 GB und drei Geräten. Es teilt keine Welten zwischen Freunden, synchronisiert aber deine eigenen Spielstände für jedes Spiel.

### Kann ich mit Hoard einem Freund einen Spielstand schicken?

Nicht als Funktion: Hoard hält deine eigenen Spielstände auf deinen eigenen Rechnern synchron. Du kannst deine Spielstände so exportieren, wie das Spiel sie geschrieben hat, und die Dateien selbst verschicken.

### Ist Hoard kostenlos?

Ja. Die App, die Kommandozeile und der Server sind kostenlos und Open Source (AGPL-3.0), und ein eigener Server kostet dich nur die Hardware. Hoard One, der verwaltete Server, den wir allen empfehlen, ist ebenfalls kostenlos mit 2 GB und drei Geräten; Pro bringt mehr Platz für 1,99 € im Monat.

### Sendet Self-Hosting irgendetwas an Hoard?

Nein. Auf deinem eigenen Server gibt es kein Konto bei uns und keine Telemetrie an uns: Deine Spielstände, deine Nutzer und deine Logs bleiben auf deiner Maschine und berühren unsere nie.
