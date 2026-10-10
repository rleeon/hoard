---
title: "Spielstand beschädigt? So stellst du ihn wieder her"
description: "Ein Spielstand, der nicht lädt, ist nicht immer verloren. Wo eine heile Kopie liegt (Spiel-Backups, Steam Cloud, Windows, OneDrive) und wie du vorsorgst."
order: 12
updated: 2026-10-09
---

Ein Spielstand, der nicht lädt, ist selten endgültig verloren. Meist gibt es irgendwo eine funktionierende Kopie: ein Backup, das das Spiel selbst angelegt hat, die Kopie in Steam Cloud, eine frühere Version bei Windows oder OneDrive oder den Spielstand auf einem anderen PC. Die Reihenfolge zählt aber, denn ein falscher Schritt kann die gute Kopie mit der kaputten überschreiben. Fang hier an.

## Zuerst: anhalten und den Ordner kopieren

1. **Beende das Spiel** und fang in diesem Slot kein neues Spiel an. Jedes weitere Speichern kann eine ältere Kopie verdrängen.
2. **Kopiere den ganzen Speicherordner** auf den Desktop oder einen USB-Stick. Dann lässt sich alles, was du danach versuchst, rückgängig machen. Wenn du nicht weißt, wo der Ordner liegt, siehe [wo PC-Spiele ihre Spielstände speichern](/guides/where-are-pc-game-saves-stored).
3. **Pausiere alles, was diesen Ordner synchronisiert.** Steam Cloud (pro Spiel unter **Eigenschaften → Allgemein**), OneDrive, Syncthing. Sonst wandert die kaputte Datei genau an den einen Ort, der noch eine gute Kopie hat.

## Prüfe, ob er wirklich beschädigt ist

Manches sieht nach Beschädigung aus und ist keine:

- **Das Spiel wurde aktualisiert** und alte Spielstände laden nicht oder brauchen einen Patch. Schau in die News oder ins Forum des Spiels.
- **Mods fehlen.** Vor allem Bethesda-Spiele warnen vor fehlenden Plugins und lehnen einen Spielstand ab, der sie benutzt hat. Installiere die Mods zuerst neu.
- **Du bist in einem anderen Konto.** Manche Spiele legen Spielstände unter deiner Steam- oder Ubisoft-Konto-ID ab, ein anderes Konto sieht also einen leeren Slot.
- **Die Datei ist nur online.** Bei OneDrive wurde ein Spielstand mit Wolkensymbol von der Platte genommen, um Platz zu sparen. Rechtsklick und **Immer auf diesem Gerät beibehalten** wählen.

Ein Spielstand mit **0 KB**, oder viel kleiner als seine Nachbarn, ist wirklich kaputt: Das Schreiben wurde mittendrin abgebrochen.

## Wo eine funktionierende Kopie liegen kann

Geh der Reihe nach vor. Die ersten Wege sind schneller und klappen eher.

### 1. Die eigenen Backups des Spiels

Viele Spiele legen stillschweigend eine Ersatzkopie an. Such im Speicherordner nach Dateien mit der Endung `.bak`, `_old` oder `.backup` und nach zusätzlichen Autosave-Slots. Einige bekannte Fälle:

- **Elden Ring** schreibt `ER0000.sl2.bak` neben den Spielstand.
- **Stardew Valley** behält eine `_old`-Kopie jeder Farm, nämlich den vorherigen Spieltag.
- **Terraria** behält `.bak`-Dateien für Spieler und Welten.
- **Minecraft Java** behält `level.dat_old` in jeder Welt.

Um eine davon zu nutzen, leg die kaputte Datei beiseite (den Ordner hast du schon kopiert) und benenn das Backup in den ursprünglichen Dateinamen um.

### 2. Steam Cloud

Steam Cloud speichert die **neueste** Kopie, keinen Verlauf. Das hilft nur, wenn der Spielstand nach dem letzten Upload kaputtging, etwa weil das Spiel abstürzte und die schlechte Datei nie synchronisiert wurde. Was Steam für jedes Spiel hat, kannst du unter [store.steampowered.com/account/remotestorage](https://store.steampowered.com/account/remotestorage) ansehen und herunterladen und dann von Hand zurücklegen.

### 3. Vorgängerversionen in Windows

Rechtsklick auf den Speicherordner, **Eigenschaften → Vorgängerversionen**. War der Dateiversionsverlauf oder der Computerschutz für dieses Laufwerk aktiv, tauchen hier ältere Stände des Ordners auf, und du kannst sie öffnen, um die nötigen Dateien herauszuholen. Ist die Liste leer, war keins von beiden an.

### 4. Der Versionsverlauf von OneDrive

Sichert OneDrive deinen Ordner `Dokumente`, liegen viele Spielstände darin, ohne dass du es weißt. Auf onedrive.com Rechtsklick auf die Spielstanddatei und **Versionsverlauf** wählen, um eine frühere Version herunterzuladen. OneDrive behält sie nur eine begrenzte Zeit, und gelöschte Dateien landen eine Weile in seinem Papierkorb.

### 5. Deine anderen Geräte

Kürzlich auf einem Laptop oder einem Steam Deck gespielt? Dessen Kopie kann älter als das Problem sein. Hol sie dir, bevor das Gerät die kaputte synchronisiert.

### 6. Wenn die Datei gelöscht und nicht beschädigt wurde

Schau zuerst in den Papierkorb. Danach kann ein Datenrettungsprogramm sie finden, sofern du nichts mehr auf dieses Laufwerk schreibst. Jede Installation und jeder Download senkt die Chancen.

## Wenn nichts auftaucht

Für einige beliebte Spiele gibt es Spielstand-Editoren oder Reparaturtools aus der Community, die eine beschädigte Datei neu aufbauen können: Such nach dem Spielnamen mit „save repair“. Sonst lautet die ehrliche Antwort: Die einzige Kopie, die zählt, ist eine, die vor dem Problem entstanden ist.

## Warum Spielstände kaputtgehen

- **Ein Absturz oder Stromausfall mitten im Schreiben.** Das Spiel war gerade beim Speichern, als es starb.
- **Eine volle Festplatte.** Das Spiel konnte nicht fertig schreiben und hinterließ eine abgeschnittene Datei.
- **Ein Sync-Tool hat ihn mitten im Schreiben erwischt**, oder zwei PCs haben denselben Spielstand bearbeitet und eine Kopie hat gewonnen.
- **Eine Mod** hat etwas geschrieben, das das Spiel nicht mehr lesen kann.
- **Ein sterbendes Laufwerk**, was sich meist auch an anderen Dateien zeigt.

## Nie wieder festsitzen

Jede Rettung oben hängt vom Glück ab: dass das Spiel zufällig ein Backup hatte oder Steam noch nicht synchronisiert hatte. Ein versioniertes Backup nimmt das Glück aus der Rechnung. Genau das macht Hoard: Es sichert jeden Spielstand automatisch, nachdem du aufgehört hast zu spielen, sobald der Ordner zur Ruhe kommt, also ist ein Backup nie eine halb geschriebene Datei. Jede Version bleibt erhalten. Geht etwas kaputt, öffnest du die **Historie** des Spiels und stellst mit einem Klick die letzte gute wieder her; dein aktueller Spielstand wird vorher gesichert, also lässt sich sogar das rückgängig machen. Und weil Hoard deine Spielstände auch synchron hält, ist die Kopie auf Laptop oder Steam Deck nie eine alte, vergessene: Alle Geräte arbeiten mit derselben Historie.

Ein Tipp, um den Moment zu finden, in dem es passiert ist: Ein plötzlicher Größenabfall zwischen zwei Versionen deutet meist auf einen abgeschnittenen Spielstand hin. Mehr unter [einen alten Spielstand wiederherstellen](/guides/restore-a-game-save).

Sollen die Backups zu Hause bleiben, starte `hoard-server` auf deinem eigenen PC oder NAS. Kein Konto bei uns, keine Telemetrie an uns, nichts läuft über unsere Server. Siehe [Hoard selbst hosten](/guides/self-host-hoard).

<!-- faq -->

## Häufige Fragen

### Lässt sich ein beschädigter Spielstand reparieren?

Selten direkt. Für einige Spiele gibt es Reparaturtools aus der Community, aber meist heißt Rettung, eine ältere Kopie zu finden: das Backup des Spiels, Steam Cloud, Windows oder OneDrive oder einen anderen PC.

### Bewahrt Steam Cloud alte Versionen meiner Spielstände auf?

Nein. Es behält nur die aktuelle Datei. Wurde ein kaputter Spielstand schon hochgeladen, hat Steam Cloud ebenfalls den kaputten.

### Hilft „Dateien auf Fehler überprüfen“ bei einem beschädigten Spielstand?

Nein. Die Überprüfung vergleicht die Spieldateien mit denen von Steam, nicht deine Spielstände. Sie hilft, wenn das Spiel selbst beschädigt ist, bringt aber keinen Fortschritt zurück.

### Warum ist mein Spielstand 0 KB groß?

Das Spiel hat angefangen zu schreiben und nie aufgehört: Absturz, Stromausfall oder volle Platte. Such daneben nach einer `.bak`- oder `_old`-Datei oder anderswo nach einer früheren Version.

### Wie verhindere ich, dass das wieder passiert?

Mit versionierten Backups, die entstehen, wenn das Spiel nicht läuft. Hoard macht das nach jeder Sitzung automatisch und behält jede Version, sodass du zu jeder zurückkannst.
