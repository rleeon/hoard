---
title: "Marvel's Spider-Man 2: Speicherort der Spielstände (PC & Steam Deck)"
description: "Wo Marvel's Spider-Man 2 seine PC-Spielstände ablegt, was der Ordner mit der langen Zahl ist, die OneDrive-Falle, der Pfad auf dem Steam Deck und Backups."
order: 22
updated: 2026-10-02
---

Auf dem PC legt Marvel's Spider-Man 2 seine Spielstände in `Dokumente\Marvel's Spider-Man 2\` ab, in einem Unterordner mit einer langen Zahl. Bei Steam ist diese Zahl deine Steam-ID. Darunter erfährst du, was das praktisch bedeutet, die OneDrive-Falle, den Pfad auf dem Steam Deck und wie du die Spielstände gesichert hältst.

## Wo Marvel's Spider-Man 2 seine Spielstände ablegt

- **Windows:** `%USERPROFILE%\Documents\Marvel's Spider-Man 2\<lange Zahl>`
- **Steam Deck und Linux** (Proton): `~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/<lange Zahl>`

Die PC-Version gibt es nur für Windows, also läuft sie auf dem Steam Deck über Proton, und die Spielstände liegen im Proton-Präfix, das Steam für das Spiel anlegt; `2651280` ist seine Steam-App-ID. Ist es auf der microSD-Karte installiert, liegt der `compatdata`-Ordner auf der Karte.

Nixxes, das Studio hinter der PC-Fassung, beschreibt den Spielstand-Ordner als „einen Unterordner mit einer langen Zahl oder einer Kombination aus Buchstaben und Zahlen“ unter `Dokumente\Marvel's Spider-Man 2\`.

## Der Ordner mit der langen Zahl

Der Unterordner ist nach deinem Konto benannt: Bei Steam ist es deine **64-Bit-Steam-ID**, die Epic-Version nutzt stattdessen eine Mischung aus Buchstaben und Zahlen. So oder so ist er für jedes Konto anders. Zwei Folgen:

- Spielen zwei Personen auf demselben PC mit verschiedenen Steam-Konten, hat jede ihren eigenen Spielstand-Ordner.
- Kopierst du Spielstände von Hand auf einen anderen PC, gehören sie in den Ordner des Steam-Kontos auf **diesem** Rechner. In einem Ordner mit anderer ID sieht das Spiel sie nicht.

Der übergeordnete Ordner `Marvel's Spider-Man 2` enthält außerdem das Log und Absturzberichte des Spiels (`.log`, `.mdmp`). Das sind keine Spielstände und müssen nicht gesichert werden.

## Die OneDrive-Falle

Viele Windows-PCs leiten `Dokumente` in OneDrive um. Ist das bei dir so, lautet der echte Pfad `%USERPROFILE%\OneDrive\Documents\Marvel's Spider-Man 2\`, und OneDrive synchronisiert den Ordner eigenständig, während du spielst. Das bringt zwei Probleme: OneDrive lädt womöglich einen halb geschriebenen Spielstand hoch, und „Speicherplatz freigeben“ kann den Spielstand in einen reinen Online-Platzhalter verwandeln. Verlässt du dich hier auf OneDrive, markiere den Ordner mit **Immer auf diesem Gerät behalten**.

## Hat Spider-Man 2 Cloud-Saves?

Ja, Steam Cloud, die die neuesten Spielstände zwischen Rechnern mit demselben Steam-Konto synchron hält. Ältere Versionen behält sie nicht: Geht ein Spielstand kaputt, wird der kaputte synchronisiert.

## Von Hand sichern

1. Beende das Spiel vollständig.
2. Kopiere den Ordner `Marvel's Spider-Man 2` aus `Dokumente` an einen sicheren Ort.
3. Zum Wiederherstellen das Spiel schließen und den Ordner mit der langen Zahl an dieselbe Stelle zurückkopieren, unter demselben Steam-Konto.

## Automatisch sichern und synchronisieren mit Hoard

[Hoard](/download) sichert den Spielstand-Ordner jedes Mal, wenn du aufhörst zu spielen, und behält jede Version. Außerdem synchronisiert es ihn zwischen deinen PCs und einem Steam Deck, sodass das Spiel auf beiden dort weitermacht, wo du aufgehört hast.

1. Installiere Hoard und melde dich an, oder richte es auf [deinen eigenen Server](/guides/self-host-hoard).
2. Öffne die **Bibliothek** und prüfe, dass der für Spider-Man 2 angezeigte Ordner der unter `Dokumente` (oder `OneDrive\Documents`) ist. Zeigt er woandershin, ändere ihn auf diesen Ordner.
3. Spiele. Beim Beenden erscheint die erste Version im Verlauf.

Geht später ein Spielstand kaputt, holt ihn [das Wiederherstellen einer älteren Version](/guides/restore-a-game-save) zurück.

<!-- faq -->

## Häufige Fragen

### Was ist die lange Zahl im Spielstand-Ordner?

Bei Steam deine 64-Bit-Steam-ID, bei Epic die ID deines Kontos. Jedes Konto hat seinen eigenen Ordner, und das Spiel liest nur den des angemeldeten Kontos.

### Wo liegen die Spielstände von Spider-Man 2 auf dem Steam Deck?

Im Proton-Präfix: `~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/`, im Ordner mit der langen Zahl.

### Ich finde den Ordner in Dokumente nicht. Wo ist er?

Schau unter `OneDrive\Documents\Marvel's Spider-Man 2`. Bei den meisten neuen Windows-Installationen liegt Dokumente in OneDrive.

### Kann ich meine Spielstände auf den PC eines Freundes kopieren?

Die Dateien lassen sich kopieren, gehören aber in den Ordner mit der Steam-ID des Kontos auf jenem PC. Ob das Spiel Spielstände eines anderen Kontos annimmt, entscheidet das Spiel — behalte vor dem Versuch eine Kopie des Originals.
