---
title: "So synchronisierst du Spielstände über mehrere PCs"
description: "Spiele dasselbe Spiel auf Desktop und Laptop, ohne Fortschritt zu verlieren. Synchronisiere deine Spielstände automatisch über mehrere PCs mit Hoard — verwaltete Cloud-Synchronisierung, ohne Ludusavi und Rclone von Hand einzurichten."
order: 2
updated: 2026-09-01
---

Wenn du an mehr als einem Computer spielst — ein Desktop zu Hause und ein Laptop unterwegs — hält Hoard deine Stände synchron, damit du immer dort weitermachst, wo du aufgehört hast.

## So funktioniert die Synchronisierung

Hoard sichert jeden Stand in deine Cloud und lädt die neueste Version auf deinen anderen Geräten herunter. Wenn du auf einem PC fertig bist, wartet der neueste Stand auf dem nächsten.

## Synchronisierung einrichten

1. Installiere **Hoard** auf jedem PC, auf dem du spielst (Windows, macOS oder Linux).
2. Melde dich mit **demselben Konto** auf jedem Gerät an oder verbinde sie mit demselben selbst gehosteten Server.
3. Füge auf jedem PC dieselben Spiele zur **Bibliothek** hinzu. Hoard ordnet sie nach Spiel zu, sodass ein auf einem Gerät gesicherter Stand auf den anderen erscheint.
4. Lass den **Automatikmodus** an. Hoard lädt nach dem Spielen hoch und vor dem Start die neueste Version herunter.

## Wechsel von Ludusavi?

Ludusavi ist ein großartiges Open-Source-Tool, um Stände lokal zu sichern und wiederherzustellen, und es kann diese Backups in eine selbst konfigurierte Cloud mit Rclone übertragen. Aber die Synchronisierung über Geräte hinweg richtest du manuell ein: Backup planen, Remote einrichten, dann auf dem anderen PC wiederherstellen, bevor du spielst.

Hoard macht daraus verwaltete Synchronisierung. Es nutzt dieselben Community-Daten für Speicherorte wie Ludusavi, um deine Stände zu finden, lädt dann nach jeder Sitzung hoch und vor der nächsten die neueste Version herunter — auf jedem PC deines Kontos, mit versionierter Historie in der Cloud. Keine Rclone-Remotes, keine Skripte. Und wie Ludusavi ist Hoard Open Source und selbst hostbar. Siehe den vollständigen [Ludusavi-Alternative-Vergleich](/guides/ludusavi-alternative).

## Konflikte vermeiden

Hoard ist konfliktbewusst: Es vergleicht Änderungszeiten und behält eine lokale Kopie jedes ersetzten Stands, sodass eine Synchronisierung nie stillschweigend Fortschritt zerstört. Läuft ein Spiel noch oder wurde ein Stand in den letzten Minuten berührt, wartet Hoard.

## Steam Deck und Desktop

Das häufigste Zwei-Geräte-Setup ist auch das, was von Hand gebaut am öftesten kaputtgeht, und fast immer aus demselben Grund.

Unter Windows liegt der Spielstand vielleicht in `Dokumente\My Games\…` oder in Steams `userdata`. Auf einem Steam Deck läuft dasselbe Windows-Spiel über Proton, sein Stand liegt also in einem Kompatibilitäts-Prefix: `steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…`. Zwei sehr verschiedene Pfade, ein Spiel, ein Spielfortschritt. Hoard liest die Proton-Prefixes ebenso wie die nativen Orte und ordnet Gefundenes dem Spiel zu, sodass Deck-Stand und Desktop-Stand zwei Versionen einer Historie werden statt zweier zusammenhangloser Ordner.

Das Detail, an dem alles hängt: Bei Steam-Spielen verfolgt Hoard `<AppID>/remote/` innerhalb von `userdata`, **nicht** den Ordner darüber. Der übergeordnete Ordner enthält auch `remotecache.vdf` sowie gerätebezogene Dateien für Erfolge und Spielzeit, die sich zwischen Deck und Desktop unterscheiden sollen. Synchronisierst du den übergeordneten Ordner, sieht jeder Start nach einem Konflikt aus, obwohl sich kein Stand bewegt hat. Genau dieser eine Fehler lässt die meisten selbstgebauten Deck-PC-Setups defekt wirken.

## Spiele, die Steam Cloud nicht abdeckt

Würden alle deine Spiele Steam Cloud unterstützen, bräuchtest du nichts davon. In der Praxis:

- **Spiele von überall außer Steam.** GOG, Epic, itch, Battle.net, die Xbox-App und alles von Hand Installierte.
- **Steam-Spiele, bei denen die Entwickler es nie aktiviert haben**, oder nur für eine Plattform.
- **Emulatoren.** RetroArch, Dolphin, PCSX2, RPCS3 und der Rest speichern, wo sie wollen, und Steam weiß nichts davon.
- **Spiele, die außerhalb des von Steam beobachteten Ordners schreiben**, und das sind mehr, als man denkt.

Hoard ist egal, wer ein Spiel veröffentlicht hat oder woher es kommt. Es verfolgt den Ordner, der sich beim Spielen ändert.

## Wenn zwei PCs denselben Stand ändern

Du spielst am Laptop, ohne den Desktop zu Ende synchronisieren zu lassen, und hast das klassische Problem: zwei Stände, beide neuer als die letzte gemeinsame Version.

Hoard überschreibt nie blind. Es vergleicht Änderungszeiten, behält eine lokale Kopie von allem, was es ersetzt, und wartet, solange ein Spiel läuft oder der Stand in den letzten Minuten angefasst wurde — eine Datei, die gerade geschrieben wird, will man nicht halb hochladen. Alle früheren Versionen bleiben in der Cloud-Historie, die falsche Wahl kostet dich also zwei Klicks statt eines Wochenendes.

Die ehrliche Grenze: **Hoard führt zwei auseinandergelaufene Stände nicht zusammen.** Das kann kein Werkzeug — eine Speicherdatei ist undurchsichtig, und es gibt keinen richtigen Weg, zwei verschiedene Spielnachmittage zu vermischen. Was du stattdessen bekommst: jede Version, auf jedem Gerät, und die Wahl.

## Synchronisieren ohne unsere Server

Das gehört ausdrücklich gesagt, weil die meisten Vergleiche genau hier danebenliegen. Es gibt zwei Betriebsarten:

- **Hoard Cloud** ist die verwaltete Variante: du meldest dich an, und deine Stände liegen auf unseren Servern in der EU.
- **Selbsthosten gehört vollständig dir.** Du betreibst `hoard-server` auf deinem eigenen PC oder NAS, und deine Geräte synchronisieren darüber. Es gibt **kein Konto bei uns, keine Telemetrie zu uns, kein Limit und kein Relay** — nichts läuft über unsere Server, weil nichts von uns im Weg steht. Siehe [wie du Hoard selbst hostest](/guides/self-host-hoard).

Dasselbe Programm, dieselbe Erkennung, dieselbe Versionshistorie. Es ändert sich nur, wem der Speicher gehört.

## Tipp

Gib jedem Gerät einen Moment, um die Synchronisierung abzuschließen, bevor du ein Spiel startest — das Dashboard zeigt den Live-Status, damit du weißt, dass der neueste Stand bereit ist.

<!-- faq -->

## Häufige Fragen

### Wie viele PCs kann ich synchronisieren?

Drei im kostenlosen Tarif, unbegrenzt mit Pro und unbegrenzt beim Selbsthosten — dein Server, deine Regeln.

### Müssen beide Geräte gleichzeitig online sein?

Nein. Dein Stand geht nach dem Spielen zum Server und kommt herunter, wenn das andere Gerät danach fragt. Der zweite PC kann also eine Woche ausgeschaltet sein und bekommt beim Einschalten trotzdem die neueste Version.

### Was, wenn ich offline spiele?

Kein Problem. Der Snapshot entsteht lokal, wenn du aufhörst zu spielen, und wird von selbst hochgeladen, sobald die Maschine wieder Verbindung hat.

### Werden auch Mods und Einstellungen synchronisiert?

Spielstände ja. Dateien, die zu einem bestimmten Rechner gehören — Konfiguration, Logs und Ähnliches — werden hochgeladen, damit sie im Backup sind, aber nicht über die Kopie eines anderen PCs geschrieben: eine Grafikeinstellung, die zu deinem Desktop passt, ist selten die, die dein Laptop will.

### Sendet Selbsthosten irgendetwas an Hoard?

Nein. Im selbst gehosteten Betrieb gibt es kein Konto bei uns und keine Telemetrie zu uns: deine Stände, deine Nutzer und deine Logs liegen auf deinem eigenen Server und berühren unseren nie.
