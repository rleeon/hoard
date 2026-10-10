---
title: "Spielstände auf einen neuen PC übertragen"
description: "Neuer PC oder Windows neu installieren? Nimm jeden Spielstand mit: was Steam Cloud abdeckt, der Weg von Hand, der automatische Weg und die Fallen."
order: 13
updated: 2026-10-09
---

Spiele mit Steam Cloud kommen von selbst zurück, sobald du dich auf dem neuen PC anmeldest. Alles andere ist ein Ordner, den du selbst mitnehmen musst: Spiele ohne Cloud-Spielstände, Emulatoren, alles außerhalb von Steam. Du kannst diese Ordner von Hand kopieren oder Hoard sie auf dem alten PC sichern und auf dem neuen jeweils an die richtige Stelle zurücklegen lassen. Unten stehen beide Wege und die Fallen, die Leute ihre Spielstände kosten.

## Bevor du irgendetwas löschst

- **Schreib auf, was du spielst**, auch Spiele, die du seit Monaten nicht angerührt hast. Genau die gehen vergessen.
- **Prüfe, welche Spiele Cloud-Spielstände haben.** Bei Steam steht es auf der Shopseite, und **Eigenschaften → Allgemein** zeigt, ob es aktiv ist. Epic und GOG zeigen es ebenfalls pro Spiel.
- **Sichere den Rest, am besten alles.** Cloud-Spielstände behalten eine Kopie, die neueste. Ist die kaputt, ist sie überall kaputt.

## Von Hand

1. **Finde den Ordner jedes Spiels.** Die meisten liegen in `Documents\My Games`, `Saved Games` oder den `AppData`-Ordnern (`Roaming`, `Local`, `LocalLow`). Die vollständige Liste steht unter [wo PC-Spiele ihre Spielstände speichern](/guides/where-are-pc-game-saves-stored).
2. **Kopiere sie auf ein externes Laufwerk** und behalte die Ordnerstruktur bei. Nimm auch Steams ganzen `userdata`-Ordner mit: Er ist klein und deckt Spiele ab, die über Steam speichern, ohne dass Steam Cloud aktiv ist.
3. **Installiere auf dem neuen PC zuerst das Spiel.** Muss das Spiel seine Ordner anlegen, starte es einmal und beende es im Hauptmenü. Fang kein neues Spiel an.
4. **Kopiere die Spielstände an ihren Platz** und starte das Spiel. Prüfe, ob dein Fortschritt da ist, bevor du etwas auf dem alten Laufwerk löschst.

Das funktioniert. Der Nachteil: Es ist eine einmalige Kopie. Du musst an jeden Ordner denken, und spielst du auf dem alten PC weiter, laufen die beiden ab diesem Tag auseinander.

## Die Fallen

- **An ein Konto gebundene Spielstände.** Manche Spiele schreiben deine Konto-ID in den Ordnernamen oder in den Spielstand: Elden Ring legt Spielstände unter deiner SteamID ab, Ubisoft-Spiele unter deiner Ubisoft-ID. Dasselbe Konto auf beiden PCs: kein Problem. Ein anderes Konto: Das Spiel sieht einen leeren Slot.
- **OneDrive hat Dokumente verschoben.** Sichert ein PC `Dokumente` mit OneDrive und der andere nicht, liegt der „gleiche“ Ordner an zwei verschiedenen Orten. Rechtsklick auf `Dokumente`, **Eigenschaften → Pfad**, dann siehst du, wo er wirklich liegt. Mehr unter [OneDrive und Spielstände](/guides/onedrive-game-saves).
- **Spielversionen.** Ein Spielstand aus einer neueren Version lädt in einer älteren womöglich nicht. Aktualisiere das Spiel auf dem neuen PC, bevor du kopierst.
- **Mods.** Ein Spielstand mit Mods (vor allem bei Bethesda-Spielen) lädt ohne dieselben Mods womöglich nicht. Installiere sie zuerst neu.
- **Von Windows auf ein Steam Deck oder Linux.** Der Spielstand gehört ins Proton-Präfix des Spiels, das erst existiert, nachdem das Spiel einmal gestartet wurde. Siehe [Spielstände zwischen Steam Deck und PC synchronisieren](/guides/sync-saves-steam-deck-pc).

## Automatisch

Hoard macht aus dem Umzug dasselbe, was es jeden Tag tut: auf einem Gerät sichern, auf einem anderen wiederherstellen.

1. **Auf dem alten PC** installierst du Hoard und meldest dich an. Öffne die **Bibliothek**: Hoard listet die Spielstände, die es für deine Spiele gefunden hat, mit derselben Community-Datenbank wie Ludusavi. Ergänze Fehlendes, indem du auf den Ordner zeigst.
2. **Prüfe, dass jedes Spiel eine Version** in seiner **Historie** hat. Das ist dein Sicherheitsnetz, bevor du das alte Laufwerk löschst.
3. **Auf dem neuen PC** installierst du Hoard, meldest dich mit demselben Konto an und installierst deine Spiele. Hoard ordnet sie ihren Backups pro Spiel zu und stellt die neueste Version in dem Ordner wieder her, den dieses Gerät erwartet, auch wenn der Pfad ein anderer ist (anderes Laufwerk, anderer Benutzername, ein Proton-Präfix auf einem Deck).
4. **Bevor du ein neues Spiel anfängst**, lass Hoard deine Spielstände fertig zurücklegen. Die App zeigt den Status jedes Spiels.

Zwei Details machen das sicherer als eine Kopie. Einstellungsdateien wie `graphics.ini` werden gesichert, aber nicht über die des neuen PCs geschrieben, also startet deine neue Hardware mit Einstellungen, die zu ihr passen (beim Wiederherstellen kannst du sie mitnehmen, wenn beide Geräte ähnlich sind). Und nichts ist endgültig: Jede Version bleibt in der Historie, eine falsche Wiederherstellung machst du rückgängig, indem du die davor wiederherstellst.

Bleibt der alte PC in Gebrauch, synchronisiert er einfach weiter mit dem neuen. Ist er für immer weg, entferne ihn aus deinen Geräten. Der kostenlose Tarif umfasst drei.

## Ohne unsere Server

Das alles geht auch mit deinem eigenen Server: Starte `hoard-server` auf einem PC oder NAS, richte beide Geräte darauf aus, und die Spielstände verlassen dein Zuhause nie. Kein Konto bei uns, keine Telemetrie an uns. Siehe [Hoard selbst hosten](/guides/self-host-hoard).

<!-- faq -->

## Häufige Fragen

### Werden Steam-Spielstände automatisch übertragen?

Nur bei Spielen mit Steam Cloud. Melde dich auf dem neuen PC an, installiere das Spiel, und der Spielstand wird heruntergeladen. Spiele ohne brauchen eine Kopie ihres Ordners oder ein Tool, das das für dich erledigt.

### Kann ich einfach meinen ganzen Benutzerordner kopieren?

Für die meisten Spielstände klappt das, aber du schleppst auch Gigabytes an Caches mit, Einstellungen für die alte Hardware und App-Daten, die auf einer frischen Installation Probleme machen können. Nur die Speicherordner zu kopieren ist sauberer.

### Funktionieren meine Spielstände mit einem anderen Windows-Benutzernamen?

Ja, fast immer. Spielstände liegen relativ zu deinem Benutzerordner, der Name im Pfad spielt also keine Rolle. Hoard erledigt das von selbst.

### Kann ich Spielstände von Windows auf ein Steam Deck übertragen?

Ja. Starte das Spiel einmal auf dem Deck, damit sein Proton-Präfix existiert, und leg den Spielstand hinein, oder lass Hoard das machen. Siehe [den Steam-Deck-Leitfaden](/guides/sync-saves-steam-deck-pc).

### Muss ich den alten PC behalten, bis der neue fertig ist?

Bei einer Kopie von Hand: Behalte das externe Laufwerk, bis du jedes Spiel geprüft hast. Mit Hoard liegen die Spielstände schon auf dem Server, der alte PC kann also gehen, sobald jedes Spiel eine Version in seiner Historie zeigt.
