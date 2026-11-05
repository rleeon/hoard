---
title: "Alternative zu OpenCloudSaves: archiviert, das kannst du stattdessen nutzen"
description: "OpenCloudSaves ist seit Januar 2025 archiviert. Hoard ist ein gepflegter Open-Source-Ersatz, der von selbst synchronisiert und jede Version behält."
order: 28
updated: 2026-11-05
---

OpenCloudSaves wurde am 20. Januar 2025 archiviert. Der Code und die letzte Version sind noch da, aber das Projekt nimmt keine Änderungen mehr an, also repariert es niemand für neue Spiele, neue Launcher oder Systemupdates. Hat es deine Spielstände über einen Cloud-Speicher synchronisiert, ist [Hoard](/download) ein kostenloser Open-Source-Ersatz, der aktiv gepflegt wird, deine Spielstände selbst findet und sie bei jedem Speichern synchronisiert.

## Was OpenCloudSaves gut gemacht hat

- **Deine eigene Cloud.** Spielstände landeten über Rclone in dem OneDrive, Google Drive, Dropbox oder Nextcloud, das du schon hattest, mit einer echten Oberfläche statt Konfigurationsdateien.
- **Vorsichtig von Haus aus.** Vor jedem Sync lief ein Probelauf, sodass du sahst, was sich ändern würde, bevor es passierte.
- **Filter und eigene Spiele.** Du konntest nur bestimmte Dateien synchronisieren, Grafikeinstellungen auslassen und Spiele selbst definieren, sogar über Stores hinweg.
- **Jeder Desktop.** Windows, macOS und Linux, Steam Deck eingeschlossen.

## Was ein Ersatz braucht

- **Jemanden, der ihn pflegt.** Speicherorte wandern und Launcher ändern sich; ein Sync-Tool, das niemand aktualisiert, synchronisiert irgendwann den falschen Ordner.
- **Eine Historie, nicht nur die letzte Kopie.** Ein Cloud-Speicher synchronisiert Dateien, also erreicht ein beschädigter Spielstand jeden Rechner so zuverlässig wie ein guter.
- **Keine halb geschriebenen Dateien.** Ein allgemeiner Sync kann einen Spielstand erwischen, während das Spiel noch schreibt; der [OneDrive-Guide](/guides/onedrive-game-saves) zeigt, was dabei kaputtgeht.

## Hoard als Ersatz

| | OpenCloudSaves | Hoard |
|---|---|---|
| Gepflegt | Seit Januar 2025 archiviert | Ja, Open Source (AGPL-3.0) |
| Spielstände finden | Spielstand-Definitionen, dazu Ordner, die du hinzufügst | Automatisch: Community-Datenbank, Steam, Epic, GOG, Proton- und Wine-Präfixe, 19 Emulatoren |
| Wann synchronisiert wird | Wenn ein Sync läuft, nach einem Probelauf | Bei jedem Speichern und noch einmal beim Beenden |
| Wo die Spielstände liegen | Dein Cloud-Speicher | Hoard One oder dein eigener Server auf lokaler Platte oder beliebigem S3-kompatiblem Speicher |
| Historie | Was der Cloud-Speicher behält | Jede Version, dedupliziert |
| Zehn Versionen eines 2-GB-Spielstands | Etwa 20 GB Cloud-Kontingent | Etwa 2 GB, weil Versionen teilen, was gleich blieb |
| Plattformen | Windows, macOS, Linux | Windows, Linux, macOS und SteamOS |

Das Einzige, was Hoard nicht macht, ist in dein Google Drive oder OneDrive zu schreiben. Es synchronisiert stattdessen über einen Server, und genau das gibt ihm die Historie und erlaubt, dass der andere Rechner aus ist. Wolltest du eigentlich Speicher, den du kontrollierst, und nicht genau diesen Cloud-Speicher, schreibt ein selbst gehostetes Hoard auf ein NAS, eine lokale Platte oder in jeden S3-kompatiblen Bucket wie Backblaze B2. Muss es der Cloud-Speicher selbst sein, ist Ludusavi mit einem Rclone-Remote die nächste gepflegte Option.

## Umsteigen

1. Kopiere deine Spielstand-Ordner an einen sicheren Ort, bevor du irgendetwas änderst.
2. Lass OpenCloudSaves diese Ordner nicht mehr synchronisieren, damit nie zwei Tools in denselben Spielstand schreiben.
3. Installiere Hoard und melde dich bei Hoard One an, empfohlen für alle, oder richte es auf [deinen eigenen Server](/guides/self-host-hoard).
4. Öffne die **Bibliothek**. Hoard findet deine Spiele; eigene Ordner fügst du von Hand hinzu. Die erste Version erscheint im Verlauf, sobald das Spiel speichert.
5. Installiere Hoard mit demselben Konto auf deinen anderen Rechnern.

## Hoard One oder dein eigener Server

Hoard ist so oder so kostenlos, und die App ist bei beiden dieselbe:

- **Hoard One**, empfohlen für alle. Nichts einzurichten und nichts am Laufen zu halten: ein verwalteter Server in der EU, kostenlos mit 2 GB und drei Geräten, ohne Karte. [Pro](/pricing) bringt nur mehr Platz, 100 GB und unbegrenzt viele Geräte, für den Fall, dass deine Spielstände darüber hinauswachsen.
- **Dein eigener Server.** Lass `hoard-server` auf einem PC, einem NAS oder einem Raspberry Pi laufen, und alle deine Rechner synchronisieren darüber. Kein Konto bei uns, keine Telemetrie an uns und kein Kontingent von uns: Nichts läuft über unsere Server, weil nichts von uns dazwischen ist. Siehe [Hoard selbst hosten](/guides/self-host-hoard).

Dasselbe Programm, dieselbe Erkennung, derselbe Verlauf. Hoard One behält jede Version für immer; auf deinem eigenen Server entscheidest du, wie viel Verlauf du behältst.

<!-- faq -->

## Häufige Fragen

### Kann ich OpenCloudSaves noch sicher nutzen?

Die letzte Version ist noch verfügbar, aber das Projekt ist archiviert: keine Fixes für neue Spiele oder Systemänderungen. Es erschien außerdem als Beta, deren Autor darum bat, Spielstände vor der Nutzung zu sichern. Bleibst du dabei, behalte eine manuelle Kopie.

### Kann Hoard Spielstände in Google Drive oder OneDrive speichern?

Nein. Hoard synchronisiert über einen Server: Hoard One oder deinen eigenen auf lokaler Platte oder S3-kompatiblem Speicher. Dein Ordner `Dokumente` kann trotzdem in OneDrive liegen; Hoard fragt Windows, wo er wirklich ist, und findet die Spielstände dort.

### Ist Hoard kostenlos?

Ja. Die App, die Kommandozeile und der Server sind kostenlos und Open Source (AGPL-3.0), und ein eigener Server kostet dich nur die Hardware. Hoard One, der verwaltete Server, den wir allen empfehlen, ist ebenfalls kostenlos mit 2 GB und drei Geräten; Pro bringt mehr Platz für 1,99 € im Monat.

### Sendet Self-Hosting irgendetwas an Hoard?

Nein. Auf deinem eigenen Server gibt es kein Konto bei uns und keine Telemetrie an uns: Deine Spielstände, deine Nutzer und deine Logs bleiben auf deiner Maschine und berühren unsere nie.
