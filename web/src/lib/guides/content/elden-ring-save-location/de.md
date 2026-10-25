---
title: "Elden Ring: Speicherort der Spielstände (PC & Steam Deck)"
description: "Wo Elden Ring ER0000.sl2 auf PC und Steam Deck ablegt, was darin steckt, Seamless Co-op und Nightreign, und wie du alle Charaktere sicherst."
order: 26
updated: 2026-10-18
---

Auf dem PC legt Elden Ring seine Spielstände in `%APPDATA%\EldenRing\<deine Steam-ID>` ab, in einer einzigen Datei namens `ER0000.sl2`, die alle deine Charaktere enthält, bis zu zehn. Darunter findest du den Pfad auf dem Steam Deck, was sonst im Ordner liegt, wo Seamless Co-op und Nightreign speichern und wie du alles gesichert und zwischen PC und Steam Deck synchron hältst.

## Wo Elden Ring seine Spielstände ablegt

- **Windows (Steam):** `%APPDATA%\EldenRing\<deine Steam-ID>\ER0000.sl2`
- **Steam Deck und Linux** (Proton): `~/.local/share/Steam/steamapps/compatdata/1245620/pfx/drive_c/users/steamuser/AppData/Roaming/EldenRing/<deine Steam-ID>/`

`%APPDATA%` ist `C:\Users\<dein Name>\AppData\Roaming`: Füge `%APPDATA%\EldenRing` in die Adressleiste des Datei-Explorers ein, um direkt dorthin zu springen. Der Ordner mit der Steam-ID ist eine lange Zahl, die an dein Steam-Konto gebunden ist. Auf dem Steam Deck läuft das Spiel über Proton, der Spielstand liegt also im Proton-Präfix, das Steam dafür anlegt; `1245620` ist die Steam-App-ID des Spiels.

## Was im Speicherordner liegt

- **`ER0000.sl2`** ist dein Fortschritt: alle Charaktere, ihre Ausrüstung und wo sie in der Welt stehen, Shadow of the Erdtree inklusive.
- **`ER0000.sl2.bak`** ist eine Sicherung, die das Spiel daneben ablegt.
- **`ER0000.co2`**, falls vorhanden, gehört zum Mod Seamless Co-op (mehr unten).

Die Grafikeinstellungen liegen nicht in diesem Ordner, sondern eine Ebene höher in `%APPDATA%\EldenRing\GraphicsConfig.xml`.

Aus der Art, wie die Datei funktioniert, folgen zwei Dinge:

- **Charaktere lassen sich nicht einzeln sichern oder wiederherstellen.** Spielst du eine ältere `ER0000.sl2` zurück, setzt das alle Charaktere auf diesen Moment zurück, nicht nur den, den du reparieren wolltest.
- **Der Spielstand ist an dein Steam-Konto gebunden.** Deine Steam-ID steht in der Datei, und das Spiel lädt keinen Spielstand, der mit einem anderen Konto erstellt wurde. Deinen eigenen Spielstand zwischen deinen eigenen PCs und einem Steam Deck zu verschieben, funktioniert problemlos.

## Seamless Co-op und Nightreign

### Seamless Co-op

Der Mod Seamless Co-op lässt `ER0000.sl2` in Ruhe und legt seinen eigenen Spielstand, `ER0000.co2`, im selben Ordner ab: Deine Charaktere mit und ohne Mod sind also getrennte Dateien. Die Community des Mods rät davon ab, eine `.co2` über den normalen Spielstand zu kopieren, um online zu spielen.

### Elden Ring Nightreign

Nightreign ist ein eigenes Spiel mit eigenem Ordner: `%APPDATA%\Nightreign\<deine Steam-ID>\NR0000.sl2` unter Windows und auf dem Steam Deck derselbe Pfad in seinem eigenen Proton-Präfix (`compatdata/2622380`). Deine Elden-Ring-Charaktere berührt es nie.

## Hat Elden Ring Cloud-Saves?

Ja, über Steam Cloud, die `ER0000.sl2` zwischen Rechnern mit demselben Konto abgleicht. Sie hält nur den neuesten Stand, ohne ältere Versionen zum Zurückgehen, und die `.bak` des Spiels liegt auf derselben Platte wie der Spielstand: Eine tote Platte nimmt beide mit.

## Von Hand sichern

1. Beende das Spiel vollständig.
2. Kopiere deinen Steam-ID-Ordner aus `%APPDATA%\EldenRing` an einen sicheren Ort. Er enthält `ER0000.sl2`, ihre `.bak` und einen eventuellen Seamless-Co-op-Spielstand.
3. Zum Wiederherstellen schließe das Spiel und kopiere den Ordner an dieselbe Stelle zurück.

## Automatisch sichern und synchronisieren mit Hoard

[Hoard](/download) ist kostenlos und Open Source. Es sichert deine Elden-Ring-Spielstände jedes Mal, wenn das Spiel speichert, und noch einmal beim Beenden, führt einen Verlauf, zu dem du zurückkehren kannst, und synchronisiert den Ordner zwischen deinen PCs und einem Steam Deck. Nur geänderte Dateien werden übertragen, komprimiert, und jede Datei wird bei der Rückkehr gegen ihren SHA-256-Hash geprüft.

1. Installiere Hoard und melde dich bei Hoard One an, empfohlen für alle, oder richte es auf [deinen eigenen Server](/guides/self-host-hoard).
2. Öffne die **Bibliothek**. Elden Ring wird über deine Steam-Bibliothek und die Community-Datenbank für Spielstände erkannt.
3. Spiele. Beim Beenden erscheint die erste Version im Verlauf.

Hoard nimmt den ganzen Steam-ID-Ordner mit, also steckt auch ein Seamless-Co-op-Spielstand in jeder Version. Jede Version enthält alle deine Charaktere zu diesem Zeitpunkt: Zum Zurückgehen [stellst du eine frühere Version wieder her](/guides/restore-a-game-save), und die neueren Versionen bleiben im Verlauf, falls du es dir anders überlegst.

## Hoard One oder dein eigener Server

Hoard ist so oder so kostenlos, und die App ist bei beiden dieselbe:

- **Hoard One**, empfohlen für alle. Nichts einzurichten und nichts am Laufen zu halten: ein verwalteter Server in der EU, kostenlos mit 2 GB und drei Geräten, ohne Karte. [Pro](/pricing) bringt nur mehr Platz, 100 GB und unbegrenzt viele Geräte, für den Fall, dass deine Spielstände darüber hinauswachsen.
- **Dein eigener Server.** Lass `hoard-server` auf einem PC, einem NAS oder einem Raspberry Pi laufen, und alle deine Rechner synchronisieren darüber. Kein Konto bei uns, keine Telemetrie an uns und kein Kontingent von uns: Nichts läuft über unsere Server, weil nichts von uns dazwischen ist. Siehe [Hoard selbst hosten](/guides/self-host-hoard).

Dasselbe Programm, dieselbe Erkennung, derselbe Verlauf. Hoard One behält jede Version für immer; auf deinem eigenen Server entscheidest du, wie viel Verlauf du behältst.

<!-- faq -->

## Häufige Fragen

### Kann ich meinen Elden-Ring-Spielstand mit einem anderen Steam-Konto nutzen?

So nicht. Deine Steam-ID steht in `ER0000.sl2`, und das Spiel lädt keinen Spielstand, der mit einem anderen Konto erstellt wurde. Dein eigener Spielstand auf deinem eigenen Konto wandert problemlos zwischen deinen PCs und einem Steam Deck.

### Setzt das Wiederherstellen eines alten Spielstands alle meine Charaktere zurück?

Ja. Alle Charaktere teilen sich `ER0000.sl2`, also setzt jede Wiederherstellung alle auf diesen Moment zurück. Mit Hoard bleiben die neueren Versionen im Verlauf, ein Zurückgehen ist also nie eine Einbahnstraße.

### Sichert Hoard Seamless-Co-op-Spielstände?

Ja. Der Mod legt `ER0000.co2` im selben Steam-ID-Ordner ab wie den normalen Spielstand, und Hoard sichert den ganzen Ordner: Beide stecken in jeder Version.

### Ist Hoard kostenlos?

Ja. Die App, die Kommandozeile und der Server sind kostenlos und Open Source (AGPL-3.0), und ein eigener Server kostet dich nur die Hardware. Hoard One, der verwaltete Server, den wir allen empfehlen, ist ebenfalls kostenlos mit 2 GB und drei Geräten; Pro bringt mehr Platz für 1,99 € im Monat.

### Sendet Self-Hosting irgendetwas an Hoard?

Nein. Auf deinem eigenen Server gibt es kein Konto bei uns und keine Telemetrie an uns: Deine Spielstände, deine Nutzer und deine Logs bleiben auf deiner Maschine und berühren unsere nie.
