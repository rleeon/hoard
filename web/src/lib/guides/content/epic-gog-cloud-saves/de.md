---
title: "Cloud-Spielstände bei Epic und GOG: was sie abdecken und wie du den Rest synchronisierst"
description: "Die Cloud von Epic und GOG gilt nur für manche Spiele, nur im Launcher, ohne Verlauf. Was sie abdeckt und wie du den Rest automatisch synchronisierst."
order: 19.5
updated: 2026-10-09
---

Epic und GOG haben beide Cloud-Spielstände, mit denselben Haken wie Steam: Der Entwickler muss sie Spiel für Spiel unterstützen, sie funktionieren nur über den Launcher des jeweiligen Stores, und sie behalten die neueste Kopie statt eines Verlaufs. Hier steht, was jeder abdeckt, wo die Lücken sind und wie du jedes Spiel zwischen deinen PCs und einem Steam Deck synchron hältst, egal wo du es gekauft hast.

## Epic Games Store

Der Epic-Launcher hat in seinen Einstellungen einen Schalter für Cloud-Spielstände, und Spiele, die sie unterstützen, synchronisieren darüber, wenn du an einem anderen PC spielst. Die Unterstützung gilt Spiel für Spiel: Der Entwickler muss sie einbauen, und viele Spiele im Store haben das nie getan.

Unter Linux und auf dem Steam Deck gibt es keinen offiziellen Epic-Launcher. Heroic kann die Epic-Cloud bei Spielen synchronisieren, die sie unterstützen, aber du musst das pro Spiel einschalten.

## GOG

GOG Galaxy synchronisiert Cloud-Spielstände bei Spielen, die auf ihrer Shopseite „Cloud saves“ unter den Features aufführen. Zwei Haken sind GOG-spezifisch:

- **Nur über Galaxy.** GOGs Offline-Installer, der DRM-freie Teil des Reizes, haben gar keine Cloud. Spielst du die Installer-Version, bleiben deine Spielstände auf diesem PC.
- **Pro Spiel und pro Plattform.** Ein Spiel synchronisiert nur zwischen den Plattformen, für die der Entwickler es eingerichtet hat.

Wie bei Epic kann Heroic die GOG-Cloud unter Linux und auf dem Steam Deck synchronisieren, wenn du es pro Spiel aktivierst.

## Ubisoft, EA und der Rest

Ubisoft Connect und die EA-App haben Cloud-Spielstände für viele ihrer eigenen Spiele, jeweils nur im eigenen Launcher. Bei Amazon Games und kleineren Stores ist es von Spiel zu Spiel verschieden.

## Was keiner von ihnen kann

- **Verlauf.** Jeder Launcher hält den aktuellen Spielstand. Geht einer kaputt und wird synchronisiert, ist der gute überall weg.
- **Store-übergreifend.** Dasselbe Spiel, auf Steam für einen PC und auf GOG für einen anderen gekauft, hat zwei getrennte Clouds, die nie miteinander reden.
- **Alles außerhalb des Launchers.** Emulatoren, DRM-freie Installer, von Hand installierte Spiele.
- **Spiele ohne Unterstützung.** Hat der Entwickler es nicht eingebaut, kann der Launcher nichts tun.

## Den Rest synchronisieren

Hoard arbeitet pro Spiel, nicht pro Store. Es findet den Speicherordner jedes Spiels über eine Community-Datenbank mit Tausenden Titeln, egal woher das Spiel kommt, sichert ihn automatisch, wenn du aufhörst zu spielen, und synchronisiert ihn mit deinen anderen PCs und deinem Steam Deck, mit jeder Version.

Das schließt die Lücken oben:

- **Jeder Launcher, oder keiner.** Epic, GOG, Galaxy oder der Offline-Installer, Heroic unter Linux, ein Spiel, das du in einen Ordner entpackt hast.
- **Store-übergreifend.** Die meisten Spiele speichern am selben Ort, egal welcher Store sie verkauft hat, meist unter `AppData` oder `Documents`, also können eine GOG-Installation auf einem PC und eine Steam-Installation auf einem anderen einen Spielstand teilen. Manche fügen einen Ordner mit der Konto-ID hinzu oder heißen je nach Store anders; prüf das, bevor du dich darauf verlässt.
- **Ein Verlauf.** Jede Sitzung ist eine Version, zu der du zurückkannst.

Wo die Cloud eines Launchers ein Spiel schon synchronisiert, lass sie weitermachen. Hoard ergänzt den Verlauf und synchronisiert alles andere.

Willst du keine fremde Cloud nutzen, starte `hoard-server` auf deinem eigenen PC oder NAS. Kein Konto bei uns, keine Telemetrie an uns, nichts über unsere Server. Siehe [Hoard selbst hosten](/guides/self-host-hoard).

<!-- faq -->

## Häufige Fragen

### Hat Epic Cloud-Spielstände?

Ja, bei Spielen, deren Entwickler sie eingebaut hat, über den Epic-Launcher. Viele Spiele im Store unterstützen sie nicht, und der Launcher hat keinen Verlauf früherer Spielstände.

### Hat GOG Cloud-Spielstände?

Ja, über GOG Galaxy, bei Spielen, die das auf ihrer Shopseite angeben. Die Offline-Installer synchronisieren gar nichts.

### Behalten Epic oder GOG alte Versionen meiner Spielstände?

Nein. Beide behalten nur die neueste Kopie. Um zu einem früheren Spielstand zurückzugehen, brauchst du ein Backup mit Versionen.

### Kann ich einen Spielstand von der GOG- in die Steam-Version übernehmen?

Oft ja: Die meisten Spiele speichern im selben Ordner, egal welcher Store. Manche fügen einen Ordner mit der Konto-ID hinzu oder nutzen einen anderen Ordnernamen, also prüf zuerst die Pfade.

### Synchronisieren GOG-Offline-Installer Spielstände?

Nicht über GOG, weil die Cloud nur in Galaxy funktioniert. Hoard synchronisiert sie wie jedes andere Spiel, weil es dem Speicherordner folgt und nicht dem Launcher.
