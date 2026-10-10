---
title: "OneDrive und Spielstände: was kaputtgeht und wie du es behebst"
description: "OneDrive hat deine Dokumente verschoben und jetzt scheitern Spielstände, verschwinden oder tauchen doppelt auf. Warum, wie du es behebst, und ein besserer Sync."
order: 15
updated: 2026-10-09
---

Auf vielen Windows-PCs sichert OneDrive den Ordner `Dokumente`, oft schon bei der Einrichtung eingeschaltet, ohne dass es jemand merkt. Spiele, die in `Dokumente` speichern, also fast alles in `My Games`, folgen ihm in OneDrive hinein. Dann geht es los: Spielstände, die nicht geschrieben werden, Spielstände, die vor dem Laden erst heruntergeladen werden müssen, und Kopien mit dem Namen deines PCs, die das Spiel nie liest. Hier steht, warum das passiert und wie du es behebst.

## Wie deine Spielstände in OneDrive gelandet sind

Die Ordnersicherung von OneDrive verschiebt `Dokumente`, `Desktop` und `Bilder` nach `C:\Users\<du>\OneDrive\...`. Spiele fragen Windows, wo `Dokumente` liegt, und folgen still. Zum Prüfen: Rechtsklick auf `Dokumente`, **Eigenschaften → Pfad**. Enthält der Pfad `OneDrive`, liegen deine Spielstände darin.

`AppData` und `Saved Games` gehören nicht zu dieser Sicherung, Spiele, die dort speichern, sind also nicht betroffen.

## Was schiefgeht

- **Schreibzugriffe kollidieren.** OneDrive lädt Dateien hoch, sobald sie sich ändern. Ein Spiel, das im selben Moment seinen Spielstand schreibt, kann die Datei in Benutzung vorfinden.
- **Spielstände nur online.** OneDrive kann Platz sparen, indem es Dateien nur in der Cloud behält (das Wolkensymbol). Das Spiel muss den Spielstand dann vor dem Laden herunterladen, und offline gibt es nichts zu laden.
- **Konfliktkopien.** Nutzt du OneDrive auf zwei PCs mit demselben Konto, synchronisieren beide dasselbe `My Games`. Spielst du auf beiden, bevor einer aufgeholt hat, behält OneDrive beide Versionen und benennt eine mit dem Namen des PCs um. Das Spiel ignoriert diese Datei.
- **Platz.** Der kostenlose Tarif hat 5 GB, und manche Spiele legen zusätzlich Mods, Caches oder Aufnahmen in `Dokumente` ab.
- **Kein Gefühl für eine Spielsitzung.** OneDrive synchronisiert Datei für Datei, mitten im Spiel, und versioniert jede Datei für sich statt den Spielstand als Ganzes.

## Die Lösungen

### Schnell: Spielstände auf dem Gerät behalten

Rechtsklick auf `Dokumente\My Games` (oder den Ordner des Spiels) und **Immer auf diesem Gerät beibehalten** wählen. Das beendet das Problem mit Dateien, die nur online liegen. Es hält OneDrive nicht davon ab, während des Spielens zu synchronisieren.

### Sauber: Dokumente nicht mehr sichern

Öffne in OneDrive **Einstellungen → Synchronisieren und sichern → Sicherung verwalten** und schalte `Dokumente` aus. Windows richtet `Dokumente` wieder auf den lokalen Ordner aus, aber die schon gesicherten Dateien bleiben im OneDrive-Ordner. Markiere sie vor dem Ausschalten mit **Immer auf diesem Gerät beibehalten**, damit sie wirklich auf der Platte liegen. Verschiebe dann, mit geschlossenen Spielen, die Spielordner zurück in die lokalen `Dokumente`, sonst fangen die Spiele von vorn an.

## Eine bessere Aufteilung

OneDrive kann gut mit Dokumenten. Spielstände brauchen etwas anderes: ein Backup, das entsteht, wenn das Spiel geschlossen ist, Versionen des ganzen Spielstands statt einzelner Dateien und Sync zu deinen anderen PCs und einem Steam Deck.

Genau das macht Hoard. Es findet deine Spielstände, ob `Dokumente` in OneDrive liegt oder nicht, weil es Windows fragt, wo der Ordner wirklich ist. Es sichert sie nach jeder Sitzung automatisch, behält jede Version und synchronisiert sie mit deinen anderen Geräten.

Eine Regel: Das Synchronisieren zwischen PCs sollte ein einziges Tool übernehmen. Sichert OneDrive `Dokumente` auf mehreren Spiele-PCs mit demselben Konto, synchronisiert es diese Spielstände schon zwischen ihnen. Schalte die Sicherung von `Dokumente` auf diesen PCs aus und überlass die Spielstände Hoard.

Lieber gar keine Cloud? Starte `hoard-server` auf deinem PC oder NAS: kein Konto bei uns, keine Telemetrie an uns, nichts über unsere Server. Siehe [Hoard selbst hosten](/guides/self-host-hoard).

<!-- faq -->

## Häufige Fragen

### Soll ich OneDrive meine Spielstände sichern lassen?

Auf einem einzelnen PC, als Backup, ist es besser als nichts. Als Sync zwischen PCs verursacht es Konflikte, weil es nicht weiß, wann ein Spiel läuft.

### Ich habe die Sicherung ausgeschaltet und meine Spielstände sind weg. Wo sind sie?

Noch im OneDrive-Ordner: `C:\Users\<du>\OneDrive\Documents\My Games`. Schließ deine Spiele und verschieb sie zurück in die lokalen `Dokumente`.

### Was sind die Spielstanddateien mit dem Namen meines PCs?

Konfliktkopien von OneDrive. Zwei PCs haben dieselbe Datei geändert, bevor sie synchronisiert waren, und OneDrive hat beide behalten. Finde heraus, welche neuer ist, gib ihr den ursprünglichen Namen und leg die andere beiseite.

### Kann OneDrive einen älteren Spielstand zurückholen?

Manchmal. Auf onedrive.com Rechtsklick auf die Datei und **Versionsverlauf** wählen. Das geht Datei für Datei und nur für begrenzte Zeit. Siehe [einen beschädigten Spielstand wiederherstellen](/guides/recover-corrupted-game-save).

### Funktioniert Hoard, wenn mein Ordner Dokumente in OneDrive liegt?

Ja. Hoard liest, wo Windows `Dokumente` verortet, und findet die Spielstände an beiden Orten.
