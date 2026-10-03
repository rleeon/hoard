---
title: "Palworld: Speicherort der Spielstände (PC & Steam Deck)"
description: "Wo Palworld seine Welten auf PC und Steam Deck ablegt, was jede Datei ist, wie Koop-Welten funktionieren und wie du Spielstände sicherst oder umziehst."
order: 24
updated: 2026-10-02
---

Auf dem PC (Steam) legt Palworld seine Spielstände in `%LOCALAPPDATA%\Pal\Saved\SaveGames\<deine Steam-ID>` ab, darin ein Ordner pro Welt. Diesen Pfad nennt Pocketpair in seiner offiziellen FAQ. Darunter findest du den Pfad auf dem Steam Deck, was jede Datei tut, was sich im Koop ändert und wie du deine Welten gesichert hältst.

## Wo Palworld seine Spielstände ablegt

- **Windows, Steam-Version:** `%LOCALAPPDATA%\Pal\Saved\SaveGames\<deine Steam-ID>\<Welt-ID>`
- **Steam Deck und Linux** (Proton): `~/.local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames/`

Der Ordner mit der Steam-ID ist eine lange Zahl, die an dein Steam-Konto gebunden ist. Darin hat jede Welt, die du angelegt hast, einen eigenen Ordner mit einem langen Hexadezimalnamen. Auf dem Steam Deck läuft das Spiel über Proton, die Spielstände liegen also im Proton-Präfix, das Steam dafür anlegt; `1623730` ist die Steam-App-ID des Spiels.

Die Version aus der Xbox-App / dem Game Pass legt ihre Spielstände an einem anderen, verpackten Ort ab, und es sind nicht dieselben Dateien, die du zwischen Steam-Installationen kopieren würdest.

## Was im Ordner einer Welt liegt

- **`Level.sav`** ist die Welt selbst: deine Basis, die Karte, die dort platzierten Pals.
- **`LevelMeta.sav`** enthält Name und Zusammenfassung der Welt für das Menü.
- **`Players\`** enthält eine `.sav` pro Spieler, der in dieser Welt war.
- **`LocalData.sav`** und **`WorldOption.sav`** enthalten lokale Daten und die Einstellungen der Welt.
- **`backup\`** sind die automatischen Backups der Welt, die das Spiel selbst anlegt.

Die Einstellungen liegen woanders: `Pal\Saved\Config\Windows\GameUserSettings.ini` enthält Grafik und Steuerung, `Pal\Saved\Logs` die Logs. Beides gehört nicht zu deinem Fortschritt.

Sichere beim Backup **den ganzen Weltordner**, nicht nur `Level.sav`. Welt und Spielerdateien gehören zusammen, und wer eines ohne das andere wiederherstellt, hat am Ende Figuren, die nicht zu ihrer Welt passen.

## Koop und dedizierte Server

Im Koop **lebt die Welt auf dem PC des Hosts**. Deine Figur in dieser Welt ist eine Datei im `Players`-Ordner des Hosts, nicht auf deinem Rechner. Verliert der Host seinen Spielstand, geht der Fortschritt aller in dieser Welt mit. Auf einem dedizierten Server lebt die Welt auf dem Server.

Bei einer geteilten Welt braucht also der Ordner des Hosts das Backup.

## Hat Palworld Cloud-Saves?

Die Steam-Version nutzt Steam Cloud, die den neuesten Stand deiner Welten zwischen Rechnern mit demselben Konto synchron hält. Ältere Versionen behält sie nicht, und der `backup\`-Ordner des Spiels liegt auf derselben Platte wie der Spielstand — eine kaputte Platte nimmt beide mit.

## Von Hand sichern

1. Beende das Spiel vollständig.
2. Kopiere deinen Steam-ID-Ordner aus `SaveGames` (er enthält alle deine Welten) an einen sicheren Ort.
3. Zum Wiederherstellen das Spiel schließen und den Ordner an dieselbe Stelle zurückkopieren.

## Automatisch sichern und synchronisieren mit Hoard

[Hoard](/download) sichert deine Welten jedes Mal, wenn du aufhörst zu spielen, und behält jede Version außerhalb des Rechners — eine beschädigte Welt oder eine verlorene Platte ist also nicht das Ende einer Basis, an der du wochenlang gebaut hast. Außerdem synchronisiert es die Welten zwischen deinen PCs und einem Steam Deck.

1. Installiere Hoard und melde dich an, oder richte es auf [deinen eigenen Server](/guides/self-host-hoard).
2. Öffne die **Bibliothek**. Palworld wird über deine Steam-Bibliothek und die Community-Datenbank für Spielstände erkannt.
3. Spiele. Beim Beenden erscheint die erste Version im Verlauf.

Hostest du Koop-Partien, ist das der Rechner, auf den es ankommt: Sichere den Host, und die geteilte Welt ist abgedeckt. Um eine Welt zurückzusetzen, [stelle eine ältere Version wieder her](/guides/restore-a-game-save).

<!-- faq -->

## Häufige Fragen

### Wo liegen die Spielstände von Palworld auf dem Steam Deck?

Im Proton-Präfix: `~/.local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames/`, im Ordner mit deiner Steam-ID.

### Wo wird meine Figur in der Welt eines Freundes gespeichert?

Auf dem PC des Hosts, im `Players`-Ordner dieser Welt. Dein PC behält keine Kopie von Welten, die jemand anderes hostet.

### Welche Datei ist meine Welt?

`Level.sav`, aber sichere den ganzen Weltordner: Spielerdateien und Welt gehören zusammen.

### Sichert Palworld meine Welt von selbst?

Es legt automatische Backups im `backup`-Ordner der Welt an. Die liegen auf derselben Platte wie der Spielstand, schützen also vor einem schlechten Spielstand, nicht vor dem Verlust der Platte.
