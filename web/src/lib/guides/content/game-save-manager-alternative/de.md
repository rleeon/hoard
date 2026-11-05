---
title: "Alternative zu GameSave Manager: automatisches Backup und Sync, auch unter Linux"
description: "Auf der Suche nach einer GameSave-Manager-Alternative? Hoard sichert jeden Spielstand von selbst, synchronisiert PCs und Steam Deck, auch unter Linux."
order: 27
updated: 2026-11-05
---

Drei verschiedene Programme heißen Game Save Manager, und alle drei sichern Spielstände auf einem einzigen Windows-PC. Wenn du Backups willst, die von selbst passieren und dir auf deine anderen PCs, ein Steam Deck oder Linux folgen, ist [Hoard](/download) kostenlos, Open Source und genau dafür gebaut.

## Welchen Game Save Manager du hast

- **GameSave Manager**, der langjährige Closed-Source-Klassiker für Windows. Er hat eine eigene Spieledatenbank, sichert auf Knopfdruck oder nach Zeitplan in ein einzelnes Archiv, und sein **Sync & Link** verschiebt einen Spielstand-Ordner in Dropbox, OneDrive oder Google Drive und lässt eine Verknüpfung zurück.
- **Game Save Manager (GSM)** von dyang886, GPL-3.0, Windows. Er findet installierte Spiele über die Datenbank von PCGamingWiki, behält pro Spiel eine rotierende Reihe lokaler Backups und nimmt deine Historie in einem `.gsmr`-Archiv auf einen neuen PC mit.
- **game-save-manager** von mcthesw, AGPL-3.0, Windows, beliebt in chinesischen Communitys. Er sichert die Ordner, die du hinzufügst, nach Zeitplan, mit WebDAV für eine Kopie außerhalb des Rechners.

## Was sie gut machen

- **Einfach und lokal.** Spiele auswählen, Backup drücken, Archiv bekommen. Kein Konto, nichts einzurichten.
- **Gute Erkennung unter Windows.** Große Spieledatenbanken finden die meisten installierten Spiele, ohne dass du Pfade suchen musst.
- **Ein sauberer Umzug auf einen neuen PC.** Ein exportiertes Archiv nimmt alles mit.

Spielst du nur an einem Windows-PC und drückst gern einen Knopf, macht jedes davon seinen Job.

## Wo Hoard weiter geht

| | Game-Save-Manager-Tools | Hoard |
|---|---|---|
| Wann gesichert wird | Auf Knopfdruck oder nach Zeitplan | Bei jedem Speichern und noch einmal beim Beenden |
| Sync zwischen PCs | Export und Import, oder ein Cloud-Ordner über Sync & Link | Automatisch: Der neueste Stand wartet auf dem nächsten Rechner |
| Anderer PC darf aus sein | Nur über einen Cloud-Ordner | Ja, der Stand wartet auf dem Server |
| Historie | Lokale Backups, oder was der Cloud-Speicher behält | Jede Version, dedupliziert, auf jedem Rechner |
| Halb geschriebene Stände | Ein Cloud-Client kann einen verknüpften Ordner synchronisieren, während das Spiel schreibt | Lädt nie eine halb geschriebene Datei hoch und tauscht nie den Stand eines laufenden Spiels aus |
| Plattformen | Windows | Windows, Linux, macOS und SteamOS |
| Lizenz | Closed Source, GPL-3.0 oder AGPL-3.0 | AGPL-3.0, kostenlos |

Die Linux-Zeile ist die, nach der gesucht wird. Keines der Game-Save-Manager-Tools läuft unter Linux oder auf einem Steam Deck. Hoard schon, nativ: dieselbe App auf allen vier Systemen, mit erkannten Proton- und Wine-Präfixen, sodass der Spielstand eines Spiels auf deinem Windows-PC und der desselben Spiels auf dem Deck zwei Versionen einer Historie werden.

## Umsteigen

1. Installiere [Hoard](/download) und melde dich bei Hoard One an, empfohlen für alle, oder richte es auf [deinen eigenen Server](/guides/self-host-hoard).
2. Hast du **Sync & Link** genutzt, bring den Spielstand-Ordner zuerst an seinen ursprünglichen Ort zurück, damit nicht ein Cloud-Client und Hoard dieselben Dateien synchronisieren.
3. Öffne die **Bibliothek**. Hoard findet deine Spiele über die Community-Datenbank für Spielstände und deine Launcher; füge jeden Ordner, den es übersieht, von Hand hinzu.
4. Spiele. Die erste Version erscheint im Verlauf, sobald das Spiel speichert. Installiere Hoard mit demselben Konto auf deinen anderen Rechnern, und der [Sync zwischen PCs](/guides/sync-game-saves-across-pcs) läuft von selbst.

Deine alten Game-Save-Manager-Archive können als kalte Kopie liegen bleiben.

## Hoard One oder dein eigener Server

Hoard ist so oder so kostenlos, und die App ist bei beiden dieselbe:

- **Hoard One**, empfohlen für alle. Nichts einzurichten und nichts am Laufen zu halten: ein verwalteter Server in der EU, kostenlos mit 2 GB und drei Geräten, ohne Karte. [Pro](/pricing) bringt nur mehr Platz, 100 GB und unbegrenzt viele Geräte, für den Fall, dass deine Spielstände darüber hinauswachsen.
- **Dein eigener Server.** Lass `hoard-server` auf einem PC, einem NAS oder einem Raspberry Pi laufen, und alle deine Rechner synchronisieren darüber. Kein Konto bei uns, keine Telemetrie an uns und kein Kontingent von uns: Nichts läuft über unsere Server, weil nichts von uns dazwischen ist. Siehe [Hoard selbst hosten](/guides/self-host-hoard).

Dasselbe Programm, dieselbe Erkennung, derselbe Verlauf. Hoard One behält jede Version für immer; auf deinem eigenen Server entscheidest du, wie viel Verlauf du behältst.

<!-- faq -->

## Häufige Fragen

### Gibt es einen GameSave Manager für Linux?

Nein: GameSave Manager läuft nur unter Windows, ebenso beide Open-Source-Game-Save-Manager. Hoard läuft nativ unter Linux und SteamOS, findet Spielstände in Proton- und Wine-Präfixen und synchronisiert sie mit deinen Windows-PCs.

### Ludusavi oder Game Save Manager?

Beides sind lokale Backup-Tools. Ludusavi ist Open Source, läuft unter Windows, Linux und macOS und baut auf dem Community-Manifest für Spielstände auf; die Game Save Manager laufen nur unter Windows. Willst du Backup und Sync, ohne selbst Rclone zu verdrahten, nutzt Hoard dieselben Community-Daten und synchronisiert von selbst. Es gibt einen ausführlichen [Ludusavi-Vergleich](/guides/ludusavi-alternative).

### Kann ich meine alten Backups behalten?

Ja. Hoard rührt sie nicht an. Behalte die Archive als kalte Kopie; ab dem ersten Speichern übernimmt Hoards eigene Historie.

### Ist Hoard kostenlos?

Ja. Die App, die Kommandozeile und der Server sind kostenlos und Open Source (AGPL-3.0), und ein eigener Server kostet dich nur die Hardware. Hoard One, der verwaltete Server, den wir allen empfehlen, ist ebenfalls kostenlos mit 2 GB und drei Geräten; Pro bringt mehr Platz für 1,99 € im Monat.

### Sendet Self-Hosting irgendetwas an Hoard?

Nein. Auf deinem eigenen Server gibt es kein Konto bei uns und keine Telemetrie an uns: Deine Spielstände, deine Nutzer und deine Logs bleiben auf deiner Maschine und berühren unsere nie.
