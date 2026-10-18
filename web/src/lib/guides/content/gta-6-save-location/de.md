---
title: "GTA 6: Speicherort der Spielstände auf PS5, Xbox und PC (was bekannt ist)"
description: "GTA 6 erscheint am 19. November 2026 für PS5 und Xbox, noch ohne PC-Version. Wo Konsolen-Spielstände liegen, wie du sie sicherst und was auf PC kommt."
order: 25
updated: 2026-10-17
---

GTA 6 erscheint am **19. November 2026, nur für PS5 und Xbox Series X|S**. Eine PC-Version gibt es noch nicht: Rockstar hat keinen Termin genannt, und der einzige Zeitraum, der kursiert, stammt aus einem unbestätigten Bericht vom Oktober 2026, der Anfang 2027 nennt. Bis dahin liegen GTA-6-Spielstände auf der Konsole und in ihrer Cloud, nicht in einem Ordner, den du öffnen kannst. Hier erfährst du, wie diese Spielstände gespeichert werden, wie du sie schützt und wo die PC-Spielstände sehr wahrscheinlich landen, gemessen an Rockstars letzten PC-Veröffentlichungen. Sobald die PC-Version erscheint, steht hier der echte Pfad.

## Wo GTA 6 seine Spielstände auf der Konsole speichert

- **PS5:** auf der Konsole. Mit PlayStation Plus lädt die PS5 Spielstände außerdem automatisch in ihren Cloud-Speicher hoch. Ohne Abo ist die einzige Kopie die auf der Konsole.
- **Xbox Series X|S:** auf der Konsole, synchronisiert mit der Xbox-Cloud deines Microsoft-Kontos. Das passiert automatisch und braucht kein Abo.

Bei keiner der beiden ist das ein Ordner, den du am Computer durchsuchen kannst. Die PS5 kann die ganze Konsole samt Spielständen auf ein USB-Laufwerk sichern, aber diese Sicherung lässt sich nur auf einer PS5 wiederherstellen.

## Kann ich GTA-6-Spielstände mit Hoard sichern?

Auf der Konsole nicht. Hoard arbeitet mit Spielständen, die als Dateien auf einem Computer liegen: Windows, macOS, Linux und das Steam Deck. Konsolen-Spielstände verlassen das System der Konsole nie, also kommt keine PC-App an sie heran.

Sobald die PC-Version erscheint, sind GTA-6-Spielstände Dateien wie bei jedem anderen PC-Spiel, und Hoard kann sie sichern und zwischen deinen PCs synchron halten.

## Wo die PC-Spielstände wahrscheinlich liegen werden

Rockstars letzte PC-Veröffentlichungen speichern alle am selben Ort, in einem Ordner pro Spiel unter `Dokumente\Rockstar Games`:

- **GTA V Legacy:** `%USERPROFILE%\Documents\Rockstar Games\GTA V\Profiles\<Profil-ID>`
- **GTA V Enhanced:** `%USERPROFILE%\Documents\Rockstar Games\GTAV Enhanced\Profiles\<Profil-ID>`
- **Red Dead Redemption 2:** `%USERPROFILE%\Documents\Rockstar Games\Red Dead Redemption 2\Profiles\<Profil-ID>`

Der letzte Ordner ist nach deinem Rockstar-Konto benannt, jedes Konto hat also seinen eigenen. Folgt GTA 6 demselben Muster, landen seine Spielstände in `Dokumente\Rockstar Games\<Spielname>\Profiles\<Profil-ID>`. **Das ist eine begründete Vermutung aus dem Präzedenzfall, kein bestätigter Pfad.**

Leitet dein PC `Dokumente` in OneDrive um, wandert der ganze Ordner `Rockstar Games` mit, nach `%USERPROFILE%\OneDrive\Documents\Rockstar Games`. OneDrive synchronisiert die Spielstände dann eigenständig, während du spielst, und das bringt eigene Probleme mit sich: siehe [Spielstände und OneDrive](/guides/onedrive-game-saves).

## Rockstars eigene Cloud auf dem PC

Auf dem PC synchronisieren GTA V und Red Dead Redemption 2 Spielstände über den Rockstar Games Launcher, nicht über Steam Cloud, auch wenn du sie bei Steam gekauft hast. Cloud-Spielstände werden pro Spiel in den Einstellungen des Launchers eingeschaltet. Wie Steam Cloud hält sie die aktuellen Spielstände zwischen Rechnern gleich; ältere Versionen zum Zurückgehen gibt sie dir nicht.

Rockstar hat nicht gesagt, wie GTA 6 Spielstände auf dem PC behandeln wird. Dieselbe Launcher-Cloud zu nutzen wäre naheliegend, aber bis zu einer Ankündigung ist das eine Annahme.

## Mit Hoard auf die PC-Version vorbereitet sein

[Hoard](/download) sichert einen Spielstand-Ordner jedes Mal, wenn du aufhörst zu spielen, und behält jede Version. Ein beschädigter Spielstand oder eine Mission, die du lieber noch einmal spielen willst, ist also nur eine Wiederherstellung entfernt. Außerdem hält es den Ordner zwischen deinen PCs und einem Steam Deck synchron, und mit GTA V und Red Dead Redemption 2 funktioniert es schon heute.

Wenn GTA 6 auf den PC kommt:

1. Installiere Hoard und melde dich an, oder richte es auf [deinen eigenen Server](/guides/self-host-hoard).
2. Öffne die **Bibliothek**. Ein brandneues Spiel braucht eine Weile, bis es in der Community-Datenbank für Spielstände steht. Ist GTA 6 noch nicht aufgeführt, füge seinen Ordner unter `Dokumente\Rockstar Games` von Hand hinzu.
3. Spiele. Beim Beenden erscheint die erste Version im Verlauf.

<!-- faq -->

## Häufige Fragen

### Erscheint GTA 6 für PC?

Nicht zum Start. Am 19. November 2026 erscheint es für PS5 und Xbox Series X|S. Rockstar hat keinen PC-Termin genannt; ein Bericht vom Oktober 2026 spricht von Anfang 2027, ohne Bestätigung.

### Wo liegen GTA-6-Spielstände auf der PS5?

Auf der Konsole und, wenn du PlayStation Plus hast und das automatische Hochladen an ist, im Cloud-Speicher. Es sind keine Dateien, die du am Computer öffnen kannst.

### Kann ich meinen GTA-6-Spielstand von der Konsole auf den PC mitnehmen?

Dazu hat Rockstar nichts gesagt. Bei GTA V konnten Spieler ihre Story-Spielstände über Rockstars eigene Cloud auf die neuen Konsolen mitnehmen, aber für GTA 6 ist nichts Vergleichbares angekündigt.

### Enthält der Spielstand-Ordner den Online-Fortschritt?

Bei Rockstars aktuellen Spielen nicht: Der Fortschritt in GTA Online und Red Dead Online liegt auf Rockstars Servern, nicht im Spielstand-Ordner. Auf der Festplatte gibt es dafür nichts zu sichern.

### Wird Hoard GTA 6 unterstützen?

Auf dem PC ja, solange es wie Rockstars andere PC-Spiele in einen Ordner speichert. Konsolen-Spielstände sind für Hoard und jede andere PC-App unerreichbar.
