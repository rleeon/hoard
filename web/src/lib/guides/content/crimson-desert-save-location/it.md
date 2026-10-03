---
title: "Dove sono i salvataggi di Crimson Desert (PC e Steam Deck)"
description: "Dove Crimson Desert tiene i salvataggi su Windows, Steam Deck e Mac, quale cartella li contiene davvero e come farne il backup o spostarli tra PC."
order: 21
updated: 2026-10-02
---

Su Windows, Crimson Desert tiene i salvataggi in `%LOCALAPPDATA%\Pearl Abyss\CD\save`. È la cartella che Pearl Abyss indica nella propria FAQ. Qui sotto trovi i percorsi su Steam Deck e Mac, cosa c'è dentro e come tenerla al sicuro.

## Dove Crimson Desert tiene i salvataggi

- **Windows:** `%LOCALAPPDATA%\Pearl Abyss\CD\save` (cioè `C:\Users\<tu>\AppData\Local\Pearl Abyss\CD\save`)
- **Steam Deck e Linux** (Proton): `~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save`
- **Mac, versione Steam:** `~/Library/Application Support/Pearl Abyss/CD/save`
- **Mac, versione App Store:** `~/Library/Containers/com.pearlabyss.CrimsonDesert/Data/Library/Application Support/Pearl Abyss/CD/save`

Non esiste una versione Linux nativa, quindi su Steam Deck il gioco gira con Proton e i salvataggi stanno nel prefisso Proton che Steam tiene per lui; `3321460` è l'ID Steam del gioco. Se è sulla microSD, la cartella `compatdata` è sulla scheda.

`AppData` è una cartella nascosta su Windows. Il modo più rapido è incollare `%LOCALAPPDATA%\Pearl Abyss\CD\save` nella barra degli indirizzi di Esplora file.

## Cosa c'è nella cartella

Dentro `save` ci sono due sottocartelle. Secondo Pearl Abyss, **quella con il nome numerico contiene i salvataggi che crei nel gioco**. Per il backup prendi l'intera cartella `save` invece di scegliere i file: è leggera e non lasci indietro nulla che serva al gioco.

Le versioni Steam e App Store su Mac usano percorsi diversi. Se passi dall'una all'altra, copia i salvataggi a mano una volta.

## Crimson Desert ha i salvataggi nel cloud?

Sì, la versione Steam ha Steam Cloud, che tiene allineati gli ultimi salvataggi tra le macchine con lo stesso account Steam.

Cosa non fa:

- **Tenere le versioni precedenti.** Steam Cloud tiene lo stato attuale. Se un salvataggio si corrompe, si sincronizza quello corrotto.
- **Coprire altri store.** Una copia del Mac App Store e una di Steam non condividono il cloud.

## Backup a mano

1. Chiudi completamente il gioco.
2. Copia l'intera cartella `save` dal percorso qui sopra in un posto sicuro: un altro disco, una chiavetta USB, una cartella cloud.
3. Per ripristinare, chiudi il gioco e ricopiala sostituendo quella esistente.

Va bene come copia occasionale prima di un grosso aggiornamento o di una reinstallazione. Come routine dipende dalla tua memoria, e hai solo l'ultima copia.

## Backup e sync automatici con Hoard

[Hoard](/download) fa il backup della cartella ogni volta che smetti di giocare e tiene tutte le versioni, così puoi tornare indietro da un salvataggio rotto o da una scelta di cui ti penti. Tiene anche la cartella sincronizzata tra i tuoi PC e una Steam Deck.

1. Installa Hoard e accedi, oppure puntalo verso [il tuo server](/guides/self-host-hoard).
2. Apri la **Libreria**. Crimson Desert viene rilevato dalla tua libreria Steam e dal database comunitario dei salvataggi, nel percorso qui sopra.
3. Gioca. Quando esci, la prima versione compare nella cronologia.

Da lì ogni sessione aggiunge una versione, e la più recente è sulla macchina a cui ti siedi dopo. Se un salvataggio si rompe, [ripristinarne uno precedente](/guides/restore-a-game-save) richiede un paio di clic.

<!-- faq -->

## Domande frequenti

### Dove sono i salvataggi di Crimson Desert su Steam Deck?

Nel prefisso Proton del gioco: `~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save`. Se il gioco è sulla microSD, la cartella `compatdata` è sulla scheda.

### Quale sottocartella contiene i miei salvataggi?

Quella con il nome numerico dentro `save`. Fai comunque il backup dell'intera cartella `save`, così non manca nulla.

### Non trovo la cartella AppData. Dov'è?

È nascosta di default. Incolla `%LOCALAPPDATA%\Pearl Abyss\CD\save` nella barra degli indirizzi di Esplora file e premi Invio, oppure attiva gli elementi nascosti nel menu Visualizza.

### Posso giocare sul fisso e sulla Steam Deck con lo stesso salvataggio?

Sì. Steam Cloud lo fa per l'ultimo salvataggio sullo stesso account Steam. Anche Hoard, e tiene una versione per sessione, così puoi tornare indietro se qualcosa si rompe.
