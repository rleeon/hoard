---
title: "Wo speichert Steam Spielstände? Alle Speicherorte auf PC und Steam Deck"
description: "Steam hat einen userdata-Ordner, aber die meisten Spiele speichern in AppData, Dokumente oder im eigenen Ordner. Wo du unter Windows, Deck und Mac suchst."
order: 30
updated: 2026-11-05
---

Steam hat einen eigenen Ordner für Spielstände, `userdata\<account-id>\<AppID>\remote` im Steam-Ordner, aber die meisten Spiele nutzen ihn nie. In der Community-Datenbank für Spielstände, die den Windows-Pfad von 10.937 Steam-Spielen kennt, speichert nur etwa **jedes zehnte** dort. Der Rest schreibt in AppData, in Dokumente oder in den eigenen Installationsordner, und Steam Cloud holt die Dateien, wenn das Spiel es unterstützt, von dort, wo sie liegen.

## Der eigene Ordner von Steam: userdata

- **Windows:** `C:\Program Files (x86)\Steam\userdata\<account-id>\<AppID>\remote`
- **Steam Deck und Linux:** `~/.local/share/Steam/userdata/<account-id>/<AppID>/remote`
- **macOS:** `~/Library/Application Support/Steam/userdata/<account-id>/<AppID>/remote`

`<account-id>` ist die Zahl, die Steam als deinen **Freundescode** anzeigt (Freunde, dann Freund hinzufügen). Es gibt einen Ordner pro Konto, das sich auf diesem Rechner angemeldet hat. `<AppID>` ist die Zahl in der Store-Adresse des Spiels: `store.steampowered.com/app/413150` ist Stardew Valley.

Der Ordner über `remote`, der nackte `<AppID>`-Ordner, enthält außerdem `remotecache.vdf`. Das ist die Buchhaltung von Steam, nicht dein Spielstand, und sie soll sich zwischen PC und Deck unterscheiden.

## Wo die meisten Spiele wirklich speichern

Am 5. November 2026 in derselben Datenbank gezählt, der Anteil der Steam-Spiele, die jeden Ort nutzen (manche nutzen mehrere):

- **`AppData\LocalLow`**, 23 %, meist mit Unity gebaute Spiele: `%USERPROFILE%\AppData\LocalLow\<Entwickler>\<Spiel>`
- **Der Installationsordner des Spiels**, 22 %: `C:\Program Files (x86)\Steam\steamapps\common\<Spiel>`
- **`AppData\Local`**, 21 %: `%LOCALAPPDATA%\<Entwickler oder Spiel>`
- **`Dokumente`**, 15 %, oft unter `My Games`: `%USERPROFILE%\Documents\My Games\<Spiel>`
- **`AppData\Roaming`**, 12 %: `%APPDATA%\<Spiel>`
- **Der `userdata`-Ordner von Steam**, 10 %: siehe oben
- **`Gespeicherte Spiele`**, 3 %: `%USERPROFILE%\Saved Games\<Entwickler>\<Spiel>`

## Auf dem Steam Deck: das Proton-Präfix

Windows-Spiele laufen auf dem Deck über Proton, und jedes bekommt sein eigenes kleines Windows in `~/.local/share/Steam/steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/`. AppData, Dokumente und Gespeicherte Spiele liegen alle darin, also wird `%APPDATA%\Spiel` am PC auf dem Deck zu `compatdata/<AppID>/pfx/drive_c/users/steamuser/AppData/Roaming/Spiel`. Das Präfix entsteht beim ersten Start des Spiels.

Spiele mit nativer Linux-Version umgehen das Präfix und speichern meist unter `~/.local/share` oder `~/.config`. Der Installationsordner ist `~/.local/share/Steam/steamapps/common/<Spiel>`, oder derselbe `steamapps`-Baum auf der microSD-Karte, wenn das Spiel dort installiert ist.

## Die Spielstände eines Spiels schnell finden

1. Schlag das Spiel in der [Liste der Speicherorte](/saves) nach, oder auf seiner PCGamingWiki-Seite unter „Save game data location“.
2. Nutzt es den Steam-Ordner, öffne `userdata\<account-id>\<AppID>\remote` mit der AppID von seiner Store-Seite.
3. Wenn nicht, speichere im Spiel und sortiere `%LOCALAPPDATA%`, `%USERPROFILE%\AppData\LocalLow`, `%APPDATA%` und `Dokumente` nach Änderungsdatum. Der Ordner, der sich gerade geändert hat, ist der richtige.

## Was Steam Cloud behält

Steam Cloud hält den neuesten Spielstand in deinem Steam-Konto, bei den Spielen, deren Entwickler es eingeschaltet hat. Die [Remote-Speicher-Seite](https://store.steampowered.com/account/remotestorage) von Steam listet jedes Spiel mit Dateien in deiner Steam Cloud und lässt dich sie herunterladen.

Sie behält den letzten Stand, keine Historie, also wird ein beschädigter Spielstand genauso treu synchronisiert wie ein guter, und sie deckt nur Steam-Spiele ab: nichts von GOG, Epic, einem Emulator oder einer Nicht-Steam-Verknüpfung. Mehr dazu im [Guide zur Steam-Cloud-Alternative](/guides/steam-cloud-alternative).

## Automatisch sichern und synchronisieren mit Hoard

[Hoard](/download) ist kostenlos und Open Source. Es findet die Spielstände jedes Spiels dort, wo das Spiel sie ablegt, in userdata, AppData, Dokumente, dem Installationsordner oder einem Proton-Präfix auf dem Deck, sichert sie bei jedem Speichern und noch einmal beim Beenden, führt einen Verlauf, zu dem du zurückkehren kannst, und synchronisiert sie zwischen deinen PCs und einem Steam Deck. Im Steam-Ordner verfolgt es `<AppID>/remote/` und lässt `remotecache.vdf` in Ruhe, damit PC und Deck nie uneinig wirken.

1. Installiere Hoard und melde dich bei Hoard One an, empfohlen für alle, oder richte es auf [deinen eigenen Server](/guides/self-host-hoard).
2. Öffne die **Bibliothek**. Deine Steam-Spiele werden über deine Steam-Bibliothek und die Community-Datenbank für Spielstände erkannt.
3. Spiele. Beim Beenden erscheint die erste Version im Verlauf.

## Hoard One oder dein eigener Server

Hoard ist so oder so kostenlos, und die App ist bei beiden dieselbe:

- **Hoard One**, empfohlen für alle. Nichts einzurichten und nichts am Laufen zu halten: ein verwalteter Server in der EU, kostenlos mit 2 GB und drei Geräten, ohne Karte. [Pro](/pricing) bringt nur mehr Platz, 100 GB und unbegrenzt viele Geräte, für den Fall, dass deine Spielstände darüber hinauswachsen.
- **Dein eigener Server.** Lass `hoard-server` auf einem PC, einem NAS oder einem Raspberry Pi laufen, und alle deine Rechner synchronisieren darüber. Kein Konto bei uns, keine Telemetrie an uns und kein Kontingent von uns: Nichts läuft über unsere Server, weil nichts von uns dazwischen ist. Siehe [Hoard selbst hosten](/guides/self-host-hoard).

Dasselbe Programm, dieselbe Erkennung, derselbe Verlauf. Hoard One behält jede Version für immer; auf deinem eigenen Server entscheidest du, wie viel Verlauf du behältst.

<!-- faq -->

## Häufige Fragen

### Wo ist der userdata-Ordner von Steam?

Im Steam-Ordner: `C:\Program Files (x86)\Steam\userdata` unter Windows, wenn Steam am Standardort liegt, `~/.local/share/Steam/userdata` auf einem Steam Deck oder unter Linux, und `~/Library/Application Support/Steam/userdata` auf einem Mac.

### Löscht das Deinstallieren eines Spiels seine Spielstände?

Spielstände im Installationsordner verschwinden mit ihm. Spielstände in AppData, Dokumente oder userdata bleiben meist erhalten. Auf dem Deck kann das Deinstallieren auch das Proton-Präfix des Spiels entfernen, und damit jeden Spielstand darin, also sichere es vorher.

### Kann ich Steam-Spielstände von Hand auf einen anderen PC kopieren?

Ja. Schließ das Spiel, kopiere den Spielstand-Ordner an denselben Ort auf dem anderen PC und starte das Spiel dort. Nutzt es Steam Cloud, fragt Steam beim nächsten Start vielleicht, welche Kopie bleiben soll; nimm die mit dem neueren Datum.

### Ist Hoard kostenlos?

Ja. Die App, die Kommandozeile und der Server sind kostenlos und Open Source (AGPL-3.0), und ein eigener Server kostet dich nur die Hardware. Hoard One, der verwaltete Server, den wir allen empfehlen, ist ebenfalls kostenlos mit 2 GB und drei Geräten; Pro bringt mehr Platz für 1,99 € im Monat.

### Sendet Self-Hosting irgendetwas an Hoard?

Nein. Auf deinem eigenen Server gibt es kein Konto bei uns und keine Telemetrie an uns: Deine Spielstände, deine Nutzer und deine Logs bleiben auf deiner Maschine und berühren unsere nie.
