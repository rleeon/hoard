---
title: "Hoard mit Docker selbst hosten (Self-Hosting)"
description: "Betreibe deinen eigenen Hoard-Server in Minuten mit Docker Compose. Open Source, kostenlos, auf deiner Hardware – eine voll selbst gehostete Cloud für deine Spielstände, ohne Konto und ohne Speicherlimit."
order: 0
featured: true
updated: 2026-09-01
---

Hoard ist Open Source und selbst hostbar. Statt Hoard Cloud zu nutzen, kannst du denselben `hoard-server` auf deiner eigenen Maschine betreiben und jedes Gerät darauf verweisen – ohne Konto und ohne Speicherlimit außer der Festplatte, die du ihm gibst. Diese Anleitung bringt einen Server in wenigen Minuten mit Docker zum Laufen.

## Warum Hoard selbst hosten

- **Volle Kontrolle.** Deine Spielstände liegen auf Hardware, die du kontrollierst, nicht in fremder Cloud.
- **Kein Limit.** Der Speicher wird nur von deiner eigenen Festplatte begrenzt.
- **Gleiche App, gleiche Funktionen.** Versionierter Verlauf und Hintergrund-Sync funktionieren genau wie mit Hoard Cloud – nur das Backend ändert sich.
- **Open Source.** Du kannst den Server lesen, prüfen und anpassen.

Das ist der entscheidende Unterschied zu Tools wie [Ludusavi](/guides/ludusavi-alternative): Ludusavi ist großartig für lokale Backups und eigene Cloud per Rclone, aber den Sync richtest du selbst ein. Hoard bietet dir einen verwalteten Sync-Server, den du einmal startest und mit dem sich jedes Gerät verbindet.

## Was Selbsthosten für deine Daten bedeutet

Das gehört klar gesagt, denn genau hier liegen die meisten Vergleiche bei Hoard falsch.

**Hoard Cloud** ist die verwaltete Variante: du meldest dich an, und deine Spielstände liegen auf unseren Servern in der EU.

**Ein selbst gehostetes Hoard gehört vollständig dir.** Deine Geräte sprechen mit deinem Server und mit sonst nichts. Es gibt **kein Konto bei uns, keine Telemetrie zu uns, kein Limit und kein Relay** — nichts läuft über unsere Server, weil nichts von uns im Weg steht. Wir können weder einen Spielstand noch einen Spieltitel noch eine E-Mail-Adresse sehen, schlicht weil davon nichts bei uns ankommt. Würde Hoard Cloud morgen abgeschaltet, liefe dein Setup unverändert weiter.

Eine Sache der Genauigkeit halber: dein Server hat sehr wohl eigene Zugänge — den Benutzer, den du unten anlegst, und ein Token je Gerät. Die gehören dir, auf deiner Maschine, in deiner Datenbank. Was es nicht gibt, ist ein Konto bei uns.

## Was du brauchst

- Eine Maschine, die durchläuft (Heimserver, NAS mit Docker oder ein kleiner VPS).
- Docker und Docker Compose installiert.
- Optional eine Domain und ein Reverse-Proxy für HTTPS (empfohlen für alles außerhalb deines LAN).

## Installation mit Docker Compose

Klone das Repo, erstelle eine Konfiguration aus dem Beispiel und starte den Stack:

```sh
git clone https://github.com/rleeon/hoard.git && cd hoard
mkdir -p deploy/docker/config
cp deploy/config.toml.example deploy/docker/config/config.toml
$EDITOR deploy/docker/config/config.toml      # Use nano or vim or something lol

cd deploy/docker
docker compose up -d
docker compose logs -f                         # wait for "listening"
```

Warte, bis die Logs zeigen, dass der Server lauscht. Die Daten liegen in einem benannten Docker-Volume (`hoard-data`) – sichere es wie jedes andere Volume. Der Container lauscht intern auf Port `12421`; einen anderen Host-Port setzt du mit `HOARD_PORT=9000 docker compose up -d`.

## Benutzer und Geräte-Token anlegen

Der Server hat keine Registrierungsseite – Benutzer legst du auf der Kommandozeile an:

```sh
docker compose exec server hoard-admin --config /etc/hoard/config.toml \
    user create alice --admin --password 'CHANGE_ME'
docker compose exec server hoard-admin --config /etc/hoard/config.toml \
    token create alice --device 'desktop'
```

Das Token wird nur einmal angezeigt und **kann später nicht wiederhergestellt werden**, also kopiere es jetzt.

## Die Desktop-App verbinden

Installiere die [Hoard-Desktop-App](/download) auf jedem Rechner. Wähle im Onboarding **Self-Host** und füge deine Server-URL und das eben erstellte Token ein. Ab da verhält es sich genau wie Hoard Cloud: Es erkennt deine Spiele, sichert Spielstände automatisch und führt einen versionierten Verlauf. Siehe [Spielstände zwischen PCs synchronisieren](/guides/sync-game-saves-across-pcs) für den Alltag.

## Im Produktivbetrieb

Für alles, was über dein lokales Netz hinausgeht, beende TLS an einem Reverse-Proxy (Caddy, nginx oder Traefik). Lieber Bare Metal? Das Repo liefert auch ein `systemd`-Installationsskript und einen Befehl `hoard-server upgrade`, der die Binärdatei atomar austauscht, ohne einen laufenden Sync abzubrechen.

## Selbst hosten oder Hoard Cloud?

Selbst-Hosting ist ideal, wenn du schon einen Server betreibst und volle Kontrolle ohne Limit willst. Wenn du keine Infrastruktur pflegen möchtest, bietet dir [Hoard Cloud](/pricing) denselben Sync verwaltet, mit einem kostenlosen Einstieg. So oder so bleiben App und Spielstände portabel – du kannst später wechseln.

<!-- faq -->

## Häufige Fragen

### Funkt ein selbst gehostetes Hoard nach Hause?

Nein. Die Desktop-App spricht mit der Serveradresse, die du ihr gibst. Deine Stände, deine Nutzer und deine Logs bleiben auf deiner Maschine, und nichts davon erreicht uns.

### Ist der selbst gehostete Server derselbe Code wie Hoard Cloud?

Ja, dasselbe `hoard-server`-Binary unter AGPL-3.0. Es gibt keine abgespeckte Community-Edition und keine Funktion, die der gehosteten Version vorbehalten wäre.

### Wo liegen die Spielstände tatsächlich?

Standardmäßig in dem Docker-Volume, das du dem Container gibst, auf deiner eigenen Platte. Wenn du bereits Objektspeicher betreibst, spricht der Server auch S3 — MinIO, Garage oder Backblaze B2 funktionieren als Ablage. So oder so sprechen deine Geräte ausschließlich mit deinem Server.

### Läuft das auf einem NAS?

Ja, auf jedem NAS mit Docker. Das Repository enthält eine Unraid-Vorlage, und das Image wechselt auf die `PUID`/`PGID`, die du angibst, damit eingebundene Ordner dem richtigen Benutzer gehören statt root.

### Brauche ich Domain und HTTPS?

Im eigenen LAN nicht. Sobald der Server von außen erreichbar ist, gehört ein Reverse Proxy davor, der TLS terminiert — Caddy, nginx oder Traefik.

### Was, wenn mein Server aus ist, wenn ich aufhöre zu spielen?

Der Snapshot entsteht lokal, es geht also nichts verloren. Er wird von selbst hochgeladen, sobald der Server wieder antwortet.

### Kann ich mit Hoard Cloud anfangen und später wechseln?

Ja, in beide Richtungen. Über die Kontoseite lässt sich alles exportieren, und die App kann ohne Neuinstallation auf einen anderen Server zeigen.
