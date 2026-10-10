---
title: "Spielstände zwischen Windows und Linux auf einem Dual-Boot-PC synchronisieren"
description: "Ein PC, zwei Systeme, zwei Speicherordner: Spielstände automatisch zwischen Windows und Linux synchronisieren, warum NTFS-Teilen bricht, und die Fallen."
order: 18
updated: 2026-10-09
---

Auf einem Dual-Boot-PC hat dasselbe Spiel zwei getrennte Spielstände: einen in deinem Windows-Benutzerordner und einen in einem Proton-Präfix unter Linux. Steam Cloud verbindet die beiden bei Spielen, die es unterstützen; alles andere läuft beim ersten Systemwechsel auseinander. Hoard hält sie automatisch synchron. Installiere es auf beiden Systemen mit demselben Konto, und der Spielstand jedes Spiels folgt dir, egal welches System du startest.

## Warum ein Spiel zwei Spielstände hat

Die Platte ist dieselbe, die Speicherordner nicht:

- **Unter Windows** schreibt ein Spiel nach `Documents`, `Saved Games` oder in einen der `AppData`-Ordner unter `C:\Users\<du>`.
- **Unter Linux** läuft dasselbe Windows-Spiel über Proton und schreibt in sein eigenes Präfix: `~/.local/share/Steam/steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/`, gefolgt von demselben Windows-Pfad.

Zwei Kopien eines Spielstands, in zwei Systemen, die nie gleichzeitig laufen. Wo du zuletzt gespielt hast, davon weiß das andere System nichts. Hoard sucht an beiden Orten und ordnet sie dem Spiel zu, sodass der Windows- und der Linux-Spielstand zu zwei Versionen einer gemeinsamen Historie werden.

## Was Steam Cloud schon abdeckt

Bei Steam-Spielen mit Steam Cloud synchronisiert Steam den Spielstand selbst zwischen deiner Windows- und deiner Proton-Installation. Hoards Beitrag dort ist der Verlauf: Steam speichert nur den aktuellen Spielstand, ein kaputter ersetzt also den guten auf beiden Systemen. Bei Spielen ohne Steam Cloud und bei allem außerhalb von Steam übernimmt Hoard auch das Synchronisieren.

## Warum nicht einfach einen Ordner auf der Windows-Platte teilen?

Das ist die erste Idee der meisten: Linux auf die Spielstände der Windows-Partition zeigen lassen und fertig. Das geht meist auf drei Arten schief:

- **Schnellstart und Ruhezustand.** Fährt Windows mit aktivem Schnellstart herunter, lässt es seine Partition halb im Ruhezustand, und Linux bindet sie nur lesend oder gar nicht ein. Dein Spiel kann seinen Spielstand nicht schreiben.
- **NTFS unter Proton.** Proton-Präfixe oder Steam-Bibliotheken auf einem NTFS-Laufwerk sind eine bekannte Quelle für Probleme mit Rechten und Dateinamen. Linux-Spiele fühlen sich auf einem Linux-Dateisystem wohler.
- **Verknüpfungen werden ersetzt.** Den Speicherordner des einen Systems ins andere zu verlinken funktioniert, bis ein Spiel, ein Update oder eine Neuinstallation die Verknüpfung still durch einen echten Ordner ersetzt.

Jedes System seine Spielstände dort ablegen zu lassen, wo das Spiel sie erwartet, und zwischen beiden zu synchronisieren, vermeidet alle drei.

## Einrichten

1. **Unter Windows** installierst du Hoard und meldest dich an.
2. **Unter Linux** installierst du Hoard von [der Downloadseite](/download) und meldest dich mit demselben Konto an.
3. **Starte jedes Proton-Spiel einmal unter Linux**, damit sein Präfix existiert. Vorher gibt es keinen Ordner für den Spielstand.
4. Prüfe die **Bibliothek** auf beiden Systemen: Dieselben Spiele sollten auf beiden auftauchen, und Hoard ordnet sie dem Spiel zu.

## Die Falle, die nur Dual-Boot hat

Bei zwei getrennten PCs wartet der Spielstand auf dem Server, bis das andere Gerät ihn abholt. Auf einem Dual-Boot-PC ist das „andere Gerät“ derselbe Rechner nach einem Neustart, und das ändert eine Gewohnheit.

Hoard lädt einen Spielstand hoch, sobald das Spiel geschlossen ist und der Ordner zur Ruhe kommt. **Beendest du das Spiel und startest sofort neu, ist der Upload womöglich noch nicht passiert**, und das andere System startet ohne deinen letzten Fortschritt. Es holt das nach, wenn du das nächste Mal ins erste System bootest, aber bis dahin hast du vielleicht auf dem alten Spielstand weitergespielt.

Also: Spiel beenden, Hoard einen Moment geben, in der App prüfen, dass der Spielstand aktuell ist, und dann neu starten.

## Native Linux-Versionen

Manche Spiele haben neben der Windows-Version einen nativen Linux-Build. Die beiden nutzen nicht immer dasselbe Spielstandformat, und manche legen ihre Spielstände an völlig anderen Orten ab. Willst du denselben Spielstand auf beiden Systemen, ist es am sichersten, auch unter Linux die Windows-Version über Proton zu starten: in Steam **Eigenschaften → Kompatibilität** und eine Proton-Version erzwingen. Dann laufen auf beiden Systemen dasselbe Spiel und dieselben Dateien.

## Einstellungen und Geräte

Grafikeinstellungen können sich zwischen den Systemen unterscheiden, deshalb sichert Hoard Einstellungsdateien wie `graphics.ini`, schreibt sie aber nicht über die des anderen Systems. Willst du sie trotzdem übernehmen, gibt es beim Wiederherstellen eine Option dafür.

Jedes Betriebssystem zählt als eigenes Gerät, ein Dual-Boot-PC belegt also zwei der drei Geräte im kostenlosen Tarif. Pro und selbst gehostete Server haben kein Gerätelimit.

Sollen die Spielstände zu Hause bleiben? Starte `hoard-server` auf einem NAS oder einem anderen Rechner und richte beide Systeme darauf aus. Kein Konto bei uns, keine Telemetrie an uns, nichts über unsere Server. Siehe [Hoard selbst hosten](/guides/self-host-hoard).

<!-- faq -->

## Häufige Fragen

### Synchronisiert Steam Cloud zwischen Windows und Linux?

Ja, bei Spielen, die es unterstützen: Steam hält pro Konto eine Cloud-Kopie, egal auf welchem System du spielst. Einen Verlauf gibt es nicht, und Spiele ohne Steam Cloud oder außerhalb von Steam deckt es nicht ab.

### Kann ich meine Spielstände auf der geteilten NTFS-Partition lassen?

Das ist nicht zu empfehlen. Der Schnellstart kann die Partition unter Linux schreibgeschützt hinterlassen, und Proton hat mit NTFS bekannte Probleme. Zuverlässiger ist es, wenn jedes System seine Spielstände an seinem Ort hat und man sie synchronisiert.

### Warum war mein Spielstand nach dem Neustart nicht da?

Wahrscheinlich war der Upload beim Neustart noch nicht fertig. Starte das erste System wieder, lass Hoard hochladen und prüf die App, bevor du wieder wechselst.

### Zählt ein Dual-Boot-PC als ein Gerät?

Nein, als zwei: Jedes Betriebssystem meldet sich als eigenes Gerät an. Im kostenlosen Tarif sind das zwei von drei; Pro und selbst gehostete Server haben kein Limit.

### Und wenn ein Spiel eine native Linux-Version hat?

Ihre Spielstände passen womöglich nicht zu denen der Windows-Version. Um einen Spielstand zu teilen, starte auch unter Linux die Windows-Version über Proton.
