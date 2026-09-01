---
title: "OpenSave-Alternative: direkt zwischen Geräten oder über einen eigenen Server"
description: "OpenSave synchronisiert Spielstände direkt zwischen deinen PCs, ohne etwas dazwischen. Hoard synchronisiert über einen Server — unseren oder deinen — und führt eine Versionshistorie. Ein ehrlicher Blick darauf, wann welches Design gewinnt."
order: 8
updated: 2026-09-01
---

Beide Werkzeuge lösen dasselbe Problem und sind sich über die Architektur uneinig, und genau das ist das Einzige, was einen Vergleich lohnt. Diese Seite legt die zwei Ansätze nebeneinander, samt der Fälle, in denen der andere die bessere Antwort ist.

## Der eigentliche Unterschied: direkt oder über einen Server

**OpenSave** arbeitet peer-to-peer. Deine Maschinen reden direkt miteinander, dazwischen sitzt nichts. Kein Konto, kein Speicher, den man bezahlt, und optional lässt sich eine Kopie in eine Cloud spiegeln, die du ohnehin hast.

**Hoard** synchronisiert über einen Server. Dieser Server ist entweder Hoard Cloud, von uns betrieben, oder `hoard-server` auf deinem eigenen PC oder NAS. Dein Stand geht hoch, wenn du aufhörst, und kommt herunter, wenn eine andere Maschine danach fragt.

Alles Weitere folgt aus dieser einen Entscheidung.

## Was dir ein Server bringt

- **Die andere Maschine muss nicht laufen.** Du hörst am Desktop auf, der Laptop bleibt eine Woche zu, und beim Aufklappen wartet der neueste Stand. Peer-to-peer braucht beide Enden gleichzeitig wach — am Schreibtisch kein Problem, mit einem Handheld, das du zweimal im Monat anfasst, schon.
- **Eine Versionshistorie statt nur des letzten Zustands.** Jede Sitzung wird eine Version, zu der du zurückkannst. Das zählt an dem Tag, an dem ein Mod deine Welt frisst oder ein Stand halb geschrieben landet: eine direkte Synchronisierung kopiert die kaputte Datei getreulich auf den anderen PC.
- **Eine Kopie, die die Hardware überlebt.** Dass beide PCs in derselben Wohnung sterben, ist kein exotisches Szenario. Ein Spielstand, den es nur auf diesen zwei Maschinen gab, stirbt mit ihnen.
- **Nichts am Netzwerk zu regeln.** Kein NAT zu durchqueren, kein Port zu öffnen, keine Bedingung, dass beide im selben LAN hängen.

## Was dir peer-to-peer bringt

Fairerweise die andere Seite:

- **Nie Speicher zu bezahlen.** Es gibt kein Limit zu erreichen, weil es keinen Speicherort gibt. Hoards kostenloser Tarif sind 2 GB, darüber zahlst du oder hostest selbst.
- **Von Natur aus nichts dazwischen.** Wenn das Ziel ist, dass eine Datei nie die Platte eines Dritten berührt, ist direkte Übertragung die kürzestmögliche Antwort.
- **Nichts zu betreiben.** Kein Server, der laufen muss, nicht einmal ein eigener.

Wenn du an zwei Desktops spielst, die beide eingeschaltet sind, nie zurückrollen willst und über Speicher gar nicht nachdenken möchtest, passt dieses Design sauber, und Hoard ist mehr Maschinerie als nötig.

## Die Datenschutzfrage, präzise beantwortet

Hier gehen Vergleiche zu Hoard meist schief, deshalb genau: es gibt zwei Betriebsarten, und sie unterscheiden sich wirklich.

- **Hoard Cloud** ist die verwaltete Variante: du meldest dich an, und deine Stände liegen auf unseren Servern in der EU.
- **Selbsthosten gehört vollständig dir.** Du betreibst `hoard-server` auf deinem PC oder NAS, und deine Stände gehen von deiner Maschine auf deine Platte. Es gibt **kein Konto bei uns, keine Telemetrie zu uns, kein Limit und kein Relay** — nichts läuft über unsere Server, weil nichts von uns im Weg steht. Wir sehen weder Spielstand noch Spieltitel noch E-Mail-Adresse, weil davon nichts bei uns ankommt. Würde Hoard Cloud morgen abgeschaltet, liefe ein selbst gehostetes Setup unverändert weiter.

"Server" heißt also nicht "der Computer von jemand anderem", außer du willst es so. Ein selbst gehostetes Hoard hält deine Stände auf deiner eigenen Hardware, genau wie eine direkte Übertragung, und gibt dir zusätzlich Historie und den Fall der ausgeschalteten Maschine.

## Erkennung und Abdeckung

Beide Werkzeuge finden Spielstände für einen großen Katalog automatisch. Hoard liest dasselbe Community-Manifest für Speicherorte, das im Open-Source-Umfeld geteilt wird und über 20.000 Titel abdeckt, und legt Steam-Bibliotheken, laufende Prozesse und einen Dateisystem-Scan obendrauf. Bei Steam-Spielen verfolgt es `<AppID>/remote/` in `userdata` statt des Ordners darüber, denn der enthält `remotecache.vdf` und gerätebezogene Dateien für Erfolge und Spielzeit — synchronisiert man die, sieht jeder Start nach einem Konflikt aus. Ungewöhnliches richtest du von Hand ein.

## Was solltest du nehmen?

- **Peer-to-peer**, wenn deine Maschinen gleichzeitig laufen, Speicher gar nicht vorkommen soll und der letzte Stand alles ist, was du je gebraucht hast.
- **Hoard**, wenn du eine Historie zum Zurückrollen willst, eine Maschine eine Woche aus sein darf und eine Kopie beide PCs überleben soll — wahlweise über unsere Cloud oder deinen eigenen Server.

Es gibt einen breiteren [Vergleich aller Sync-Tools](/guides/game-save-sync-comparison) und einen [Ludusavi-Vergleich](/guides/ludusavi-alternative) für die Seite der lokalen Backups.

<!-- faq -->

## Häufige Fragen

### Braucht Hoard ein Konto?

Für Hoard Cloud ja, daran hängt die Synchronisierung. Selbst gehostet gibt es gar kein Konto bei uns: dein Server hat eigene Benutzer und ein Token je Gerät, und die verlassen deine Maschine nie.

### Funktioniert Hoard ganz ohne Cloud?

Ja. Betreibe `hoard-server` auf einem PC oder NAS, und deine Stände gehen von deiner Maschine auf deine Platte, ohne dass etwas über unsere Server läuft.

### Müssen beide PCs gleichzeitig online sein?

Nein, und das ist der praktische Vorteil der Synchronisierung über einen Server. Dein Stand wird hochgeladen, wenn du aufhörst, und heruntergeladen, sobald die andere Maschine das nächste Mal danach fragt.

### Führt eine Direktübertragung eine Versionshistorie?

Von sich aus nicht — eine Datei auf eine andere Maschine zu kopieren gibt dir den aktuellen Zustand auf beiden. Hoard sichert jede Sitzung als Version, und genau das macht das Zurückrollen eines beschädigten Stands möglich.

### Ist Hoard ebenfalls Open Source?

Ja, AGPL-3.0, Server inklusive. Der selbst gehostete Server ist dasselbe Binary, das wir betreiben, keine abgespeckte Edition.
