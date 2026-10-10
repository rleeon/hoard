---
title: "RetroArch-Spielstände zwischen PC und Steam Deck synchronisieren"
description: "RetroArch-Spielstände und States zwischen PC, Steam Deck und Laptop synchronisieren: wo .srm-Dateien liegen, Cloud Sync gegen automatischen Sync, die Fallen."
order: 14
updated: 2026-10-09
---

RetroArch speichert Spielstände aus dem Spiel als `.srm`-Dateien in einem Ordner `saves` und Savestates in einem Ordner `states`. Um sie zwischen Geräten zu synchronisieren, kannst du RetroArchs eingebauten Cloud Sync mit einem eigenen WebDAV-Server nutzen oder ein Tool, das beide Ordner beobachtet. Hoard macht Letzteres automatisch: Es sichert beide Ordner, wenn du RetroArch beendest, holt sie auf deine anderen Geräte, behält jede Version und versteht EmuDeck-Installationen.

## Wo RetroArch Spielstände ablegt

- **Windows:** `%APPDATA%\RetroArch\saves` und `\states`, oder `saves` und `states` neben `retroarch.exe`, wenn du es in einen eigenen Ordner installiert hast.
- **Linux:** `~/.config/retroarch/saves`. Das Flatpak nutzt `~/.var/app/org.libretro.RetroArch/config/retroarch/saves`.
- **Steam Deck mit EmuDeck:** `~/Emulation/saves/retroarch/`, wobei `saves` und `states` Verknüpfungen auf die echten Ordner sind. Hoard liest `retroarch.cfg`, um herauszufinden, wohin sie wirklich zeigen.
- **RetroDECK:** standardmäßig `~/retrodeck/saves` und `~/retrodeck/states`.
- **Irgendwo anders:** **Settings → Directory** zeigt die Ordner, die RetroArch tatsächlich nutzt.

## Wann RetroArch den Spielstand wirklich schreibt

Daran scheitern viele. RetroArch hält den Spielstand im Speicher und schreibt die `.srm` erst, wenn du das Spiel schließt oder RetroArch beendest, außer **Settings → Saving → SaveRAM Autosave Interval** ist gesetzt. Bis dahin liegt nichts auf der Platte: Ein Absturz oder ein leerer Akku kostet alles seit dem letzten Schreiben, und kein Sync-Tool kann einen Spielstand bewegen, der nie geschrieben wurde.

Stell ein Autosave-Intervall von ein paar Sekunden ein. Und bevor du das Gerät wechselst, **beende RetroArch**, nicht nur das Spiel: Hoard sichert, sobald RetroArch geschlossen ist, und kopiert so nie einen halb geschriebenen Spielstand. Auf einem Deck zählt Standby nicht als Beenden.

## Alle Geräte gleich einstellen

- **Sortieroptionen.** **Settings → Saving** kann Spielstände und States nach Core-Name oder Inhaltsordner in Unterordner verteilen. Sortiert ein Gerät und das andere nicht, landet die synchronisierte Datei in einem Ordner, in dem RetroArch nicht nachsieht. Nutze überall dieselben Einstellungen.
- **ROM-Dateinamen.** Die `.srm` heißt wie das ROM: `Super Metroid (USA).sfc` speichert nach `Super Metroid (USA).srm`. Ein anders benanntes ROM auf dem anderen Gerät findet sie nicht.
- **Derselbe Core.** Zwei Cores für dieselbe Konsole lesen nicht immer die Spielstände des anderen. Wähl einen pro System und nutze ihn überall.
- **Core-Versionen, für States.** Ein State ist ein Abbild des Core-Speichers und lädt oft nicht in einer anderen Core-Version. Normale Spielstände haben dieses Problem nicht.

Noch eine Falle mit States: Ein State enthält den Speicher des Spiels, Spielstand inklusive. Lädst du einen alten State, bringt das nächste Schreiben der `.srm` diesen alten Spielstand zurück. Nutzt du **Auto Load State**, synchronisiere die States mit, damit der neueste reist.

## RetroArch Cloud Sync oder Hoard?

Fair zu beiden:

- **RetroArch Cloud Sync** ist eingebaut und synchronisiert Spielstände und States mit einem WebDAV-Server, den du betreibst oder mietest. Er läuft auch auf Android und iOS, was Hoard heute nicht tut. Betreibst du schon Nextcloud, das selbst Dateiversionen aufhebt, passt er gut, und er ist die bessere Wahl, wenn dein Handy dazugehört.
- **Hoard** braucht keinen WebDAV-Server. Es sichert und synchronisiert automatisch, führt eine Versionshistorie zum Zurückspringen und deckt auch deine eigenständigen Emulatoren und PC-Spiele ab. Es behandelt den ganzen Ordner `saves` als ein Element, ein Zurückspringen stellt also jedes Spiel darin so wieder her, wie es in dem Moment war. Vor dem Bestätigen zeigt es, was sich ändert, und deine aktuellen Dateien werden zuerst gesichert.

Wähl eines pro Ordner. Zwei Tools, die dieselben Spielstände schreiben, sind das Rezept für Konflikte.

## Mit Hoard einrichten

1. Installiere Hoard auf jedem Gerät und melde dich mit demselben Konto an.
2. Füge in der **Bibliothek** RetroArch hinzu. Spielstände und States erscheinen als zwei Einträge.
3. Gleiche die Einstellungen oben auf allen Geräten an.
4. Spiel, beende RetroArch und mach auf dem anderen Gerät weiter.

Lieber alles zu Hause? Starte `hoard-server` auf deinem PC oder NAS: kein Konto bei uns, keine Telemetrie an uns, nichts über unsere Server. Siehe [Hoard selbst hosten](/guides/self-host-hoard).

<!-- faq -->

## Häufige Fragen

### Hat RetroArch Cloud-Spielstände?

Ja, einen eingebauten Cloud Sync, der einen WebDAV-Server braucht. Hoard ist die Alternative, wenn du keinen betreiben willst, eine Versionshistorie möchtest oder auch eigenständige Emulatoren nutzt.

### Warum wurde mein RetroArch-Spielstand nicht synchronisiert?

Meist eins von drei Dingen: RetroArch hatte die `.srm` noch nicht geschrieben (es lief noch, ohne Autosave-Intervall), die beiden Geräte sortieren Spielstände in verschiedene Unterordner, oder die ROMs heißen unterschiedlich.

### Kann ich auch States synchronisieren?

Ja. Hoard verfolgt `states` als eigenen Eintrag. Ob ein State auf dem anderen Gerät lädt, hängt davon ab, dass beide dieselbe Core-Version nutzen.

### Funktioniert es mit EmuDeck und RetroDECK?

Ja. Hoard liest RetroArchs Konfiguration, um EmuDecks Verknüpfungen bis zu den echten Ordnern zu folgen. Bei RetroDECK füge `~/retrodeck/saves` und `~/retrodeck/states` hinzu, falls sie nicht von selbst auftauchen.

### Synchronisiert Hoard RetroArch auf Android?

Heute nicht. Hoard läuft auf Windows, macOS, Linux und Steam Deck.
