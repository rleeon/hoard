---
title: "Syncthing für Spielstände: was klappt und was bricht"
description: "Syncthing ist ein hervorragender universeller Datei-Sync, aber Spielstände brechen drei seiner Annahmen. Was schiefgeht, wie man es umgeht, und wann ein Werkzeug besser ist, das weiß, was ein Spielstand ist."
order: 9
updated: 2026-09-01
---

Syncthing ist die Antwort, zu der viele zuerst greifen, und das aus gutem Grund: kostenlos, quelloffen, peer-to-peer, und es funktioniert. Doch Spielstände brechen drei Annahmen, auf denen ein universeller Datei-Sync aufbaut, und die Fehler sind leise. Diese Anleitung handelt davon, was wirklich schiefgeht, und wann sich ein Werkzeug lohnt, das weiß, was ein Spielstand ist.

## Warum man dort landet

Es ist wirklich gute Software. Kein Konto, kein Abo, deine Dateien liegen nie auf der Platte einer Firma, und es synchronisiert alles: Dokumente, Fotos, einen Ordner mit Spielständen. Wenn du es ohnehin betreibst, kostet dich ein zusätzlicher Ordner dreißig Sekunden. Das ist ein echtes Argument, und für manche Setups das richtige.

## Die drei Dinge, die brechen

**Es synchronisiert, während das Spiel läuft.** Syncthing reagiert darauf, dass sich eine Datei ändert — für ein Dokument genau richtig. Ein Spiel schreibt seinen Stand mitten in der Sitzung, manchmal in mehreren Durchgängen, und eine Datei, die mitten im Schreiben erwischt wird, verbreitet sich halbfertig. Die andere Maschine hat dann einen Stand, den das Spiel womöglich nicht lädt.

**Konflikte werden zu Dateien statt zu Entscheidungen.** Ändern beide Maschinen denselben Stand, tut Syncthing das Sichere und behält beide, indem es einen in `etwas.sync-conflict-20260901-143022-ABCDEFG.sav` umbenennt. Verloren geht nichts — aber das Spiel weiß nicht, was diese Datei ist, und du vergleichst Zeitstempel im Dateimanager, um zu entscheiden, welchen Spielnachmittag du behältst. Ein paar Mal, und der Ordner füllt sich mit Konfliktdateien, die niemand zu löschen wagt.

**Versionierung ist pro Datei, nicht pro Sitzung.** Syncthing kann alte Kopien in `.stversions` aufheben, besser als nichts. Aber ein Spielstand besteht oft aus mehreren Dateien, die nur zusammen Sinn ergeben, und Wiederherstellen heißt, für jede den richtigen Zeitstempel von Hand zu finden. Ein "setz dieses Spiel auf Dienstag zurück" gibt es nicht.

Und ein vierter Punkt, speziell für Steam: richtest du es auf `userdata/<UserID>/<AppID>/` statt auf den `remote/`-Ordner darin, synchronisierst du auch `remotecache.vdf` sowie Dateien für Erfolge und Spielzeit, die sich zwischen Maschinen unterscheiden **sollen**. Dann sieht jeder Start nach einem Konflikt aus, obwohl sich kein Stand bewegt hat. Das ist der häufigste Grund, warum ein selbstgebautes Setup zwischen Steam Deck und Desktop kaputt wirkt.

## Was du am Ende selbst baust

Nichts davon ist unlösbar. Man behilft sich mit Ignore-Mustern je Spiel, einer Versionierungsrichtlinie und der Gewohnheit, das Spiel zu schließen und zu warten, bevor man den anderen PC anfasst. Das funktioniert, und es ist Pflege, die dir für immer gehört: ein neues Spiel heißt neue Pfade, und der Tag, an dem du das Warten vergisst, ist der Tag, an dem du es merkst.

## Was ein spielstandbewusstes Werkzeug stattdessen tut

Hoard sichert **nachdem du aufgehört hast**, sobald der Ordner zur Ruhe kommt, ein Snapshot ist also nie eine halb geschriebene Datei. Jede Sicherung ist eine Version des ganzen Spielstands, nicht einzelner Dateien, das Wiederherstellen ist ein Klick und setzt alles gemeinsam zurück. Es weiß, welcher Ordner zu welchem Spiel gehört — es liest dasselbe Community-Manifest für Speicherorte, das im Open-Source-Umfeld geteilt wird, mit über 20.000 Titeln — es gibt also keine Pfade zu pflegen, und es verfolgt `<AppID>/remote/` statt des Ordners darüber.

## Wann Syncthing die bessere Antwort ist

Fairerweise:

- **Du betreibst es ohnehin**, ein Ordner mehr ist gratis.
- **Du willst peer-to-peer ganz ohne Server**, nicht einmal einen eigenen.
- **Du synchronisierst weit mehr als Spielstände** und hättest lieber ein Werkzeug für alles.
- **Du rollst nie zurück.** Wenn der letzte Stand immer gereicht hat, ist eine Versionshistorie Maschinerie, die du nicht nutzt.

## Beides nutzen

Sie vertragen sich, und das ist ein vernünftiges Setup: der universelle Sync übernimmt Dokumente und den Rest, ein spielstandbewusstes Werkzeug die Speicherordner. Die einzige Regel: richte nicht beide auf denselben Ordner — zwei Programme, die dieselben Dateien schreiben, erzeugen genau die Konflikte, die du vermeiden wolltest.

## Auch ohne unsere Server

Wenn ein Teil des Reizes ist, dass nichts die Platte einer Firma berührt: Hoard geht genauso. `hoard-server` auf deinem eigenen PC oder NAS, und deine Stände gehen von deiner Maschine auf deine Platte. Es gibt **kein Konto bei uns, keine Telemetrie zu uns und kein Relay** — nichts läuft über unsere Server, weil nichts von uns im Weg steht. Siehe [wie du Hoard selbst hostest](/guides/self-host-hoard).

Dasselbe Binary, dieselbe Erkennung, dieselbe Historie. Es ändert sich nur, wem der Speicher gehört. Es gibt außerdem einen vollständigen [Vergleich aller Sync-Tools](/guides/game-save-sync-comparison).

<!-- faq -->

## Häufige Fragen

### Kann Syncthing Spielstände überhaupt synchronisieren?

Ja, und in einfachen Fällen tut es das gut. Schwierig wird es bei Spielen, die während des Spielens schreiben, bei Spielständen aus mehreren Dateien, und überall dort, wo beide Maschinen zwischen zwei Synchronisierungen bearbeitet werden.

### Was sind die .sync-conflict-Dateien in meinem Speicherordner?

Das ist der Sync, der nach einem Konflikt beide Fassungen behält, statt eine zu wählen. Verloren geht nichts, aber das Spiel kann sie nicht lesen, und die Entscheidung ist jedes Mal Handarbeit.

### Warum kollidiert mein Steam-Spielstand bei jedem Start?

Fast immer, weil der synchronisierte Ordner der über `remote/` ist. Er enthält `remotecache.vdf` sowie Dateien für Erfolge und Spielzeit, die sich zu Recht je Rechner unterscheiden — die beiden Enden werden sich also nie einig.

### Muss ich das Spiel vor dem Synchronisieren schließen?

Mit einem universellen Sync ja, das ist die Gewohnheit, die halb geschriebene Stände verhindert. Ein spielstandbewusstes Werkzeug wartet von selbst, bis der Ordner ruhig ist.

### Kann ich beide zusammen nutzen?

Ja. Richte sie nur nicht auf denselben Ordner, sonst streiten sie sich um dieselben Dateien.
