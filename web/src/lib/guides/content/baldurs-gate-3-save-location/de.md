---
title: "Baldur's Gate 3: Speicherort der Spielstände (PC & Steam Deck)"
description: "Wo Baldur's Gate 3 seine Spielstände unter Windows, auf dem Steam Deck und dem Mac ablegt, was Spielstand und was Mods sind, der Ehrenmodus und Backups."
order: 23
updated: 2026-10-02
---

Unter Windows legt Baldur's Gate 3 seine Spielstände in `%LOCALAPPDATA%\Larian Studios\Baldur's Gate 3\PlayerProfiles\Public\Savegames\Story` ab, ein Ordner pro Spielstand. Diesen Pfad nennt Larian in der eigenen Support-FAQ. Darunter findest du die Pfade für Steam Deck und Mac, was neben den Spielständen liegt, den Ehrenmodus und wie du alles gesichert hältst.

## Wo Baldur's Gate 3 seine Spielstände ablegt

- **Windows:** `%LOCALAPPDATA%\Larian Studios\Baldur's Gate 3\PlayerProfiles\Public\Savegames\Story`
- **Steam Deck und Linux** (Proton): `~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story`
- **Mac:** `~/Documents/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story`

Larian hat die native Linux-Version eingestellt, also läuft das Spiel auf dem Steam Deck über Proton, und die Spielstände liegen im Proton-Präfix, das Steam dafür anlegt; `1086940` ist die Steam-App-ID des Spiels. Ist es auf der microSD-Karte installiert, liegt der `compatdata`-Ordner auf der Karte.

## Was Spielstand ist und was nicht

Jeder Spielstand ist **ein Ordner** in `Story` mit einer `.lsv`-Datei und einem Vorschaubild. Alles drumherum ist etwas anderes:

- **`Mods`** (unter `Baldur's Gate 3`) enthält die Mod-Dateien.
- **`modsettings.lsx`** (unter `PlayerProfiles\Public`) ist die Liste der aktiven Mods und ihre Ladereihenfolge.
- **Einstellungen** wie Grafik und Steuerung sind Konfigurationsdateien neben dem Profil, kein Teil eines Spielstands.

Die Falle sind die Mods. Ein mit Mods erstellter Spielstand erwartet beim Laden dieselben aktiven Mods. Nimmst du einen gemoddeten Spielstand auf einen anderen PC mit, nimm die Mod-Liste mit, sonst warnt das Spiel vor fehlenden Mods und der Spielstand lädt womöglich nicht wie erwartet.

## Der Ehrenmodus

Der Ehrenmodus hat einen einzigen Spielstand, den das Spiel beim Spielen überschreibt, und fällt deine Gruppe, ist der Ehrendurchlauf vorbei (du kannst im benutzerdefinierten Modus weiterspielen, ohne die Ehre). Diesen Spielstand zu sichern ist deine Entscheidung: Eine Kopie vor einem schweren Kampf ist technisch ein Weg zurück, und manche wollen genau das nach einem Absturz oder Bug, während andere es als Schummeln am Modus sehen. Ein Backup-Werkzeug behält die Versionen so oder so; ob du je eine wiederherstellst, ist eine Sache zwischen dir und den Würfeln.

## Hat Baldur's Gate 3 Cloud-Saves?

Ja. Auf Steam nutzt es Steam Cloud, die die neuesten Spielstände zwischen Rechnern mit demselben Konto synchron hält. Sie hält nur den aktuellen Stand: Wird ein Spielstand beschädigt oder von einem Mod-Update zerstört, wird genau diese Version synchronisiert.

## Von Hand sichern

1. Beende das Spiel vollständig.
2. Kopiere den ganzen Ordner `PlayerProfiles` vom obigen Pfad (er enthält `Savegames` und `modsettings.lsx`).
3. Zum Wiederherstellen das Spiel schließen und den Ordner zurückkopieren.

## Automatisch sichern und synchronisieren mit Hoard

[Hoard](/download) sichert den Spielstand-Ordner jedes Mal, wenn du aufhörst zu spielen, und behält jede Version — ein von einem Mod-Update oder Patch zerstörter Spielstand ist also nur eine Wiederherstellung entfernt. Außerdem hält es den Ordner zwischen deinen PCs und einem Steam Deck synchron.

1. Installiere Hoard und melde dich an, oder richte es auf [deinen eigenen Server](/guides/self-host-hoard).
2. Öffne die **Bibliothek**. Baldur's Gate 3 wird über deine Steam-Bibliothek und die Community-Datenbank für Spielstände erkannt.
3. Spiele. Beim Beenden erscheint die erste Version im Verlauf.

Jede Session fügt eine Version mit allen Spielständen hinzu. Um zurückzugehen, öffne den Verlauf und [stelle eine ältere Version wieder her](/guides/restore-a-game-save); was gerade auf deinem PC liegt, wird vorher gesichert, ein Versuch mit einer alten Version ist also nie eine Einbahnstraße.

<!-- faq -->

## Häufige Fragen

### Wo liegen die Spielstände von Baldur's Gate 3 auf dem Steam Deck?

Im Proton-Präfix des Spiels: `~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story`.

### Kann ich einen gemoddeten Spielstand auf einen anderen PC mitnehmen?

Ja, sofern auf dem anderen PC dieselben Mods installiert und in derselben Reihenfolge aktiv sind. Kopiere `modsettings.lsx` zusammen mit dem Spielstand und installiere dieselben Mod-Dateien.

### Kann ich einen Spielstand im Ehrenmodus sichern?

Der Spielstand ist ein gewöhnlicher Ordner, also ja, jedes Backup-Werkzeug kann ihn kopieren. Ob das Wiederherstellen zum Geist des Modus passt, entscheidest du.

### Warum meldet mein Spielstand fehlende Mods?

Er wurde mit Mods erstellt, die jetzt nicht aktiv sind. Aktiviere dieselben Mods in derselben Reihenfolge, und er lädt normal.
