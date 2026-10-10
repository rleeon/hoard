---
title: "Wo werden PC-Spielstände gespeichert? Alle üblichen Orte"
description: "Wo PC-Spiele ihre Spielstände unter Windows, Steam Deck, Linux und Mac ablegen, welche Launcher eigene Ordner anlegen und wie du die Saves jedes Spiels findest."
order: 11
updated: 2026-10-09
---

Es gibt nicht den einen Ordner. Unter Windows speichert fast jedes Spiel an einem von sechs Orten: `Documents`, `Saved Games`, einer der drei `AppData`-Ordner, Steams `userdata` oder der eigene Installationsordner. Das entscheiden Engine und Entwickler, nicht der Shop, in dem du es gekauft hast. Hier stehen alle üblichen Orte, die Launcher mit einer eigenen Ebene und ein schneller Weg, die Spielstände jedes Spiels zu finden, auch eines, das niemand dokumentiert hat.

## Windows: die sechs üblichen Orte

| Ordner | Typischer Pfad | Wer ihn nutzt |
|---|---|---|
| Dokumente | `%USERPROFILE%\Documents\My Games\<Spiel>` | Bethesda-Spiele, Rockstar (`Documents\Rockstar Games`), viele ältere große Titel |
| Gespeicherte Spiele | `%USERPROFILE%\Saved Games\<Publisher>\<Spiel>` | Cyberpunk 2077 und eine hartnäckige Minderheit |
| AppData\Roaming | `%APPDATA%\<Spiel>` | Elden Ring, Stardew Valley, Minecraft (`.minecraft`), viele Indies |
| AppData\Local | `%LOCALAPPDATA%\<Spiel>\Saved\SaveGames` | Unreal-Engine-Spiele (Palworld nutzt `Pal\Saved\SaveGames`) |
| AppData\LocalLow | `%USERPROFILE%\AppData\LocalLow\<Firma>\<Spiel>` | Unity-Spiele (Hollow Knight und viele mehr) |
| Steam userdata | `C:\Program Files (x86)\Steam\userdata\<UserID>\<AppID>\remote` | Spiele, die Steams eigenen Speicherplatz für Spielstände nutzen |

Und ein siebter, der nicht totzukriegen ist: **der Installationsordner des Spiels**, in den viele ältere Spiele und einige Indies immer noch schreiben.

Zwei praktische Hinweise. `AppData` ist versteckt, also tipp `%APPDATA%` oder `%LOCALAPPDATA%` in die Adressleiste des Explorers, statt dich durchzuklicken. Und wenn OneDrive deinen Ordner `Dokumente` sichert, ist der echte Pfad `C:\Users\<du>\OneDrive\Documents`, was viele überrascht. Siehe [OneDrive und Spielstände](/guides/onedrive-game-saves).

## Launcher mit eigener Ebene

Die meisten Launcher bestimmen nicht, wo Spielstände landen; das tut das Spiel. Ein paar Ausnahmen:

- **Steam** hat pro Spiel einen Speicherbereich in `userdata`. `<UserID>` ist eine Zahl, die zu deinem Steam-Konto gehört (es gibt einen Ordner pro Konto, das sich auf dem PC angemeldet hat), und `<AppID>` ist die Zahl in der Shop-URL des Spiels.
- **Ubisoft Connect** speichert in `C:\Program Files (x86)\Ubisoft\Ubisoft Game Launcher\savegames\<Benutzer-ID>\<Spiel-ID>`, auf beiden Ebenen mit Zahlen statt Namen.
- **Die Xbox-App und PC Game Pass** nutzen `%LOCALAPPDATA%\Packages\<Paket>\SystemAppData\wgs`, mit Dateien mit zufälligen Namen, die nur die Xbox-App versteht. Überlass die den Xbox-Cloud-Speicherständen; sie von Hand zu kopieren klappt selten.
- **Epic, GOG und die EA-App** überlassen es meist dem Spiel, also landen ihre Titel an den üblichen Orten oben. Ihre Cloud-Spielstände, wo es sie gibt, kopieren von dort.

## Die Registry, selten

Einige Spiele, vor allem kleine Unity-Titel, speichern den Fortschritt in der Windows-Registry unter `HKEY_CURRENT_USER\Software\<Firma>\<Spiel>` statt in einer Datei. In einem Ordner gibt es dann nichts zu kopieren, und ordnerbasierte Backup-Tools, Hoard eingeschlossen, sehen es nicht. Wenn du ihn brauchst, exportiere den Schlüssel mit `regedit`.

## Steam Deck und Linux

- **Windows-Spiele über Proton** speichern in einem Präfix pro Spiel: `~/.local/share/Steam/steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/`, dann der Windows-Pfad aus der Tabelle (`Documents`, `AppData/Roaming` usw.). Spiele auf einer microSD-Karte haben denselben Baum unter dem `steamapps/compatdata` der Karte.
- **Native Linux-Spiele** nutzen `~/.local/share/<Spiel>` oder `~/.config/<Spiel>`. Unity-Spiele landen in `~/.config/unity3d/<Firma>/<Spiel>`.
- **Steam userdata** liegt in `~/.local/share/Steam/userdata/<UserID>/<AppID>/remote`.
- **Heroic, Lutris und Bottles** halten ein Wine-Präfix pro Spiel. Darin hängt der Windows-Baum unter `drive_c/users/<dein Benutzer>/`, nicht unter `steamuser`.

Das Deck hat einen eigenen Leitfaden: [Spielstände zwischen Steam Deck und PC synchronisieren](/guides/sync-saves-steam-deck-pc).

## Mac

- **Die meisten Spiele:** `~/Library/Application Support/<Spiel>`. Unity-Spiele nutzen `~/Library/Application Support/<Firma>/<Spiel>`.
- **Spiele aus dem Mac App Store** laufen in einer Sandbox: `~/Library/Containers/<Bundle-ID>/Data/Library/Application Support/`.

`~/Library` ist ebenfalls versteckt. Öffne im Finder das Menü **Gehe zu** mit gedrückter Wahltaste, dann erscheint es.

## So findest du die Spielstände jedes Spiels

Steht ein Spiel auf keiner Liste, finden es drei Tricks in ein paar Minuten:

1. **Schlag es im PCGamingWiki nach.** Fast jede Spielseite hat einen Abschnitt „Save game data location“. Es ist dieselbe Quelle, aus der die Spielstand-Datenbanken hinter Hoard und Ludusavi gebaut werden.
2. **Schau, was sich ändert.** Speichere im Spiel, beende es und durchsuche deinen Benutzerordner nach Dateien, die sich in den letzten Minuten geändert haben. Unter Windows suchst du in `C:\Users\<du>` nach `datemodified:today` und sortierst nach Datum. Unter Linux oder auf einem Deck: `find ~ -type f -mmin -5 -not -path '*/.cache/*'`.
3. **Frag Steam.** Bei einem Spiel mit Steam Cloud listet [store.steampowered.com/account/remotestorage](https://store.steampowered.com/account/remotestorage) die Dateien, die Steam für jedes Spiel speichert, mit Namen und Größe. Kennst du den Dateinamen, ist der Ordner schnell gefunden.

## Oder lass sie für dich finden

Hoard liest dieselbe Community-Datenbank, die Tausende Spiele abdeckt, und prüft jeden infrage kommenden Pfad auf deinem Rechner: Präfixe von Proton, Heroic und Lutris, OneDrives `Dokumente`, Emulatoren, portable Installationen. Was es findet, wird automatisch gesichert, sobald du aufhörst zu spielen, mit jeder Version, und zwischen deinen PCs und dem Steam Deck synchron gehalten. Was ihm entgeht, fügst du hinzu, indem du einmal auf den Ordner zeigst. Siehe [Spielstände automatisch sichern](/guides/back-up-game-saves).

Für einige beliebte Spiele gibt es außerdem Seiten mit den genauen Pfaden: [Cyberpunk 2077](/guides/cyberpunk-2077-save-location), [Baldur's Gate 3](/guides/baldurs-gate-3-save-location), [Palworld](/guides/palworld-save-location), [Crimson Desert](/guides/crimson-desert-save-location) und [Marvel's Spider-Man 2](/guides/spider-man-2-save-location).

<!-- faq -->

## Häufige Fragen

### Wo speichert Steam Spielstände?

Das hängt vom Spiel ab. Manche nutzen Steams eigenen Bereich, `Steam\userdata\<UserID>\<AppID>\remote`; die meisten schreiben wie jedes andere Spiel nach `Documents`, `AppData` oder `Saved Games`, und Steam Cloud kopiert sie von dort.

### Warum finde ich den Ordner AppData nicht?

Er ist versteckt. Tipp `%APPDATA%` (Roaming) oder `%LOCALAPPDATA%` (Local) in die Adressleiste des Explorers oder in Ausführen (Win + R). `LocalLow` liegt neben `Local`.

### Speichern die Versionen von Steam, GOG und Epic am selben Ort?

Meistens, weil das Spiel entscheidet und nicht der Shop. Es gibt Ausnahmen: Manche Spiele legen einen Ordner mit deiner Konto-ID an, und einzelne Shop-Versionen nutzen einen anderen Ordnernamen. Prüf das, bevor du Spielstände von einer Version in eine andere kopierst.

### Wo liegen die Spielstände der Xbox-App und von Game Pass?

In `%LOCALAPPDATA%\Packages\<Paket>\SystemAppData\wgs`, als Dateien mit zufälligen Namen, die nur die Xbox-App versteht. Sie werden über die Xbox-Cloud synchronisiert; von Hand kopieren klappt selten.

### Mein Ordner Dokumente liegt in OneDrive. Ist das ein Problem?

Es kann eins sein. Spiele folgen dem Ordner in OneDrive, und OneDrive synchronisiert dann Spielstände, während die Spiele sie schreiben. Siehe [OneDrive und Spielstände](/guides/onedrive-game-saves).
