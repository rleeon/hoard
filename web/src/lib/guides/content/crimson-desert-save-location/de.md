---
title: "Crimson Desert: Speicherort der Spielstände (PC & Steam Deck)"
description: "Wo Crimson Desert seine Spielstände unter Windows, Steam Deck und Mac ablegt, welcher Ordner sie enthält und wie du sie sicherst und synchronisierst."
order: 21
updated: 2026-10-09
---

Unter Windows legt Crimson Desert seine Spielstände in `%LOCALAPPDATA%\Pearl Abyss\CD\save` ab. Diesen Ordner nennt Pearl Abyss in seiner eigenen FAQ. Darunter findest du die Pfade für Steam Deck und Mac, was drinsteckt und wie du ihn gesichert und zwischen PC und Steam Deck synchron hältst.

## Wo Crimson Desert seine Spielstände ablegt

- **Windows:** `%LOCALAPPDATA%\Pearl Abyss\CD\save` (also `C:\Users\<du>\AppData\Local\Pearl Abyss\CD\save`)
- **Steam Deck und Linux** (Proton): `~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save`
- **Mac, Steam-Version:** `~/Library/Application Support/Pearl Abyss/CD/save`
- **Mac, App-Store-Version:** `~/Library/Containers/com.pearlabyss.CrimsonDesert/Data/Library/Application Support/Pearl Abyss/CD/save`

Es gibt keine native Linux-Version, also läuft das Spiel auf dem Steam Deck über Proton, und die Spielstände liegen im Proton-Präfix, das Steam dafür anlegt; `3321460` ist die Steam-App-ID des Spiels. Ist es auf der microSD-Karte installiert, liegt der `compatdata`-Ordner auf der Karte.

`AppData` ist unter Windows ein versteckter Ordner. Am schnellsten fügst du `%LOCALAPPDATA%\Pearl Abyss\CD\save` in die Adressleiste des Datei-Explorers ein.

## Was im Ordner liegt

In `save` gibt es zwei Unterordner. Laut Pearl Abyss **enthält der mit dem numerischen Namen die Spielstände, die du im Spiel anlegst**. Sichere beim Backup den ganzen Ordner `save`, statt einzelne Dateien herauszusuchen: Er ist klein, und du lässt nichts zurück, was das Spiel braucht.

Steam- und App-Store-Version nutzen auf dem Mac unterschiedliche Pfade. Wechselst du zwischen ihnen, kopiere die Spielstände einmal von Hand hinüber.

## Hat Crimson Desert Cloud-Saves?

Ja, die Steam-Version hat Steam Cloud, die die neuesten Spielstände zwischen Rechnern mit demselben Steam-Konto synchron hält.

Was sie nicht tut:

- **Ältere Versionen behalten.** Steam Cloud hält den aktuellen Stand. Wird ein Spielstand beschädigt, wird der beschädigte synchronisiert.
- **Andere Stores abdecken.** Eine Kopie aus dem Mac App Store und eine von Steam teilen sich keine Cloud.

## Von Hand sichern

1. Beende das Spiel vollständig.
2. Kopiere den ganzen Ordner `save` vom obigen Pfad an einen sicheren Ort: ein anderes Laufwerk, einen USB-Stick, einen Cloud-Ordner.
3. Zum Wiederherstellen das Spiel schließen und den Ordner zurückkopieren, Vorhandenes ersetzen.

Als einmalige Sicherung vor einem großen Update oder einer Neuinstallation ist das in Ordnung. Als Routine hängt es davon ab, dass du daran denkst, und du hast immer nur die letzte Kopie.

## Automatisch sichern und synchronisieren mit Hoard

[Hoard](/download) sichert den Spielstand-Ordner jedes Mal, wenn du aufhörst zu spielen, und behält jede Version — so kommst du von einem kaputten Spielstand oder einer bereuten Entscheidung zurück. Außerdem hält es den Ordner zwischen deinen PCs und einem Steam Deck synchron.

1. Installiere Hoard und melde dich an, oder richte es auf [deinen eigenen Server](/guides/self-host-hoard).
2. Öffne die **Bibliothek**. Crimson Desert wird über deine Steam-Bibliothek und die Community-Datenbank für Spielstände erkannt, am obigen Pfad.
3. Spiele. Beim Beenden erscheint die erste Version im Verlauf.

Ab dann fügt jede Session eine Version hinzu, und die neueste liegt auf dem Rechner, an den du dich als Nächstes setzt. Geht ein Spielstand kaputt, ist [eine ältere Version wiederherzustellen](/guides/restore-a-game-save) eine Sache von zwei Klicks.

<!-- faq -->

## Häufige Fragen

### Wo liegen die Spielstände von Crimson Desert auf dem Steam Deck?

Im Proton-Präfix des Spiels: `~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save`. Liegt das Spiel auf der microSD-Karte, ist auch der `compatdata`-Ordner dort.

### Welcher Unterordner enthält meine Spielstände?

Der mit dem numerischen Namen in `save`. Sichere trotzdem den ganzen Ordner `save`, damit nichts fehlt.

### Ich finde den AppData-Ordner nicht. Wo ist er?

Er ist standardmäßig versteckt. Füge `%LOCALAPPDATA%\Pearl Abyss\CD\save` in die Adressleiste des Datei-Explorers ein und drück Enter, oder blende versteckte Elemente im Menü „Ansicht“ ein.

### Kann ich auf Desktop und Steam Deck mit demselben Spielstand spielen?

Ja. Steam Cloud macht das für den neuesten Spielstand auf demselben Steam-Konto. Hoard auch, und es behält eine Version pro Session, sodass du zurückkannst, wenn etwas kaputtgeht.
