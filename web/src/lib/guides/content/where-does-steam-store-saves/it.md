---
title: "Dove salva Steam i salvataggi? Tutte le posizioni su PC e Steam Deck"
description: "Steam ha una cartella userdata, ma quasi tutti i giochi salvano in AppData, Documenti o nella loro cartella. Dove guardare su Windows, Steam Deck e Mac."
order: 30
updated: 2026-11-05
---

Steam ha una sua cartella per i salvataggi, `userdata\<account-id>\<AppID>\remote` dentro la cartella di Steam, ma quasi tutti i giochi non la usano mai. Nel database comunitario dei salvataggi, che conosce il percorso Windows di 10.937 giochi Steam, solo **1 su 10** circa salva lì. Gli altri scrivono in AppData, in Documenti o nella propria cartella di installazione, e Steam Cloud, quando il gioco ce l'ha, prende i file ovunque si trovino.

## La cartella di Steam: userdata

- **Windows:** `C:\Program Files (x86)\Steam\userdata\<account-id>\<AppID>\remote`
- **Steam Deck e Linux:** `~/.local/share/Steam/userdata/<account-id>/<AppID>/remote`
- **macOS:** `~/Library/Application Support/Steam/userdata/<account-id>/<AppID>/remote`

`<account-id>` è il numero che Steam mostra come tuo **codice amico** (Amici, poi Aggiungi un amico). C'è una cartella per ogni account che ha fatto l'accesso su quella macchina. `<AppID>` è il numero nell'indirizzo del gioco sul negozio: `store.steampowered.com/app/413150` è Stardew Valley.

La cartella sopra `remote`, quella del solo `<AppID>`, contiene anche `remotecache.vdf`. È la contabilità di Steam, non il tuo salvataggio, ed è normale che sia diversa tra PC e Deck.

## Dove salvano davvero quasi tutti i giochi

Contato nello stesso database il 5 novembre 2026, la quota di giochi Steam che usa ciascun posto (alcuni ne usano più di uno):

- **`AppData\LocalLow`**, 23%, soprattutto giochi fatti con Unity: `%USERPROFILE%\AppData\LocalLow\<sviluppatore>\<gioco>`
- **La cartella di installazione del gioco**, 22%: `C:\Program Files (x86)\Steam\steamapps\common\<gioco>`
- **`AppData\Local`**, 21%: `%LOCALAPPDATA%\<sviluppatore o gioco>`
- **`Documenti`**, 15%, spesso sotto `My Games`: `%USERPROFILE%\Documents\My Games\<gioco>`
- **`AppData\Roaming`**, 12%: `%APPDATA%\<gioco>`
- **La cartella `userdata` di Steam**, 10%: vedi sopra
- **`Giochi salvati`**, 3%: `%USERPROFILE%\Saved Games\<sviluppatore>\<gioco>`

## Su Steam Deck: il prefisso Proton

I giochi Windows girano sul Deck tramite Proton, e ognuno ha il suo piccolo Windows dentro `~/.local/share/Steam/steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/`. AppData, Documenti e Giochi salvati stanno tutti lì dentro, quindi `%APPDATA%\Gioco` su un PC diventa `compatdata/<AppID>/pfx/drive_c/users/steamuser/AppData/Roaming/Gioco` sul Deck. Il prefisso compare al primo avvio del gioco.

I giochi con una versione nativa per Linux saltano il prefisso e di solito salvano sotto `~/.local/share` o `~/.config`. La cartella di installazione è `~/.local/share/Steam/steamapps/common/<gioco>`, oppure lo stesso albero `steamapps` sulla microSD se il gioco è installato lì.

## Trovare in fretta i salvataggi di un gioco

1. Cerca il gioco nell'[elenco delle posizioni dei salvataggi](/saves), o sulla sua pagina di PCGamingWiki, alla voce «Save game data location».
2. Se usa la cartella di Steam, apri `userdata\<account-id>\<AppID>\remote` con l'AppID della sua pagina sul negozio.
3. Altrimenti salva nel gioco e ordina `%LOCALAPPDATA%`, `%USERPROFILE%\AppData\LocalLow`, `%APPDATA%` e `Documenti` per data di modifica. La cartella appena cambiata è quella giusta.

## Cosa conserva Steam Cloud

Steam Cloud conserva l'ultimo salvataggio nel tuo account Steam, per i giochi in cui lo sviluppatore l'ha attivato. La [pagina dell'archiviazione remota](https://store.steampowered.com/account/remotestorage) di Steam elenca ogni gioco con file nel tuo Steam Cloud e ti permette di scaricarli.

Conserva l'ultimo stato, non una cronologia, quindi un salvataggio corrotto si sincronizza fedelmente quanto uno buono, e copre solo i giochi Steam: niente da GOG, Epic, un emulatore o un collegamento non Steam. Più dettagli nella [guida all'alternativa a Steam Cloud](/guides/steam-cloud-alternative).

## Backup e sync automatici con Hoard

[Hoard](/download) è gratuito e open source. Trova i salvataggi di ogni gioco dove il gioco li ha messi, userdata, AppData, Documenti, la cartella di installazione o un prefisso Proton sul Deck, ne fa il backup ogni volta che il gioco ne scrive uno e di nuovo quando esci, tiene una cronologia a cui puoi tornare e li sincronizza tra i tuoi PC e una Steam Deck. Nella cartella di Steam segue `<AppID>/remote/` e lascia stare `remotecache.vdf`, così il tuo PC e il tuo Deck non sembrano mai in disaccordo.

1. Installa Hoard e accedi a Hoard One, consigliato per tutti, oppure puntalo verso [il tuo server](/guides/self-host-hoard).
2. Apri la **Libreria**. I tuoi giochi Steam vengono rilevati dalla tua libreria Steam e dal database comunitario dei salvataggi.
3. Gioca. Quando esci, la prima versione compare nella cronologia.

## Hoard One, o il tuo server

Hoard è gratuito in entrambi i casi, e l'app è la stessa:

- **Hoard One**, consigliato per tutti. Niente da configurare né da tenere acceso: un server gestito nell'UE, gratuito con 2 GB e tre dispositivi, senza carta. [Pro](/pricing) aggiunge solo spazio, 100 GB e dispositivi illimitati, per quando i tuoi salvataggi non ci stanno più.
- **Il tuo server.** Fai girare `hoard-server` su un PC, un NAS o un Raspberry Pi e tutte le tue macchine si sincronizzano attraverso quel server. Nessun account da noi, nessuna telemetria verso di noi e nessuna quota nostra: niente passa dai nostri server, perché sul percorso non c'è niente di nostro. Vedi [come fare il self-hosting di Hoard](/guides/self-host-hoard).

Stesso programma, stesso rilevamento, stessa cronologia. Hoard One conserva ogni versione per sempre; sul tuo server decidi tu quanta cronologia tenere.

<!-- faq -->

## Domande frequenti

### Dov'è la cartella userdata di Steam?

Dentro la cartella di Steam: `C:\Program Files (x86)\Steam\userdata` su Windows se Steam è nella sua posizione predefinita, `~/.local/share/Steam/userdata` su uno Steam Deck o su Linux, e `~/Library/Application Support/Steam/userdata` su un Mac.

### Disinstallare un gioco cancella i suoi salvataggi?

I salvataggi nella cartella di installazione se ne vanno con lei. Quelli in AppData, Documenti o userdata di solito restano. Sul Deck, disinstallare può anche rimuovere il prefisso Proton del gioco, e con lui tutti i salvataggi che contiene, quindi fanne prima un backup.

### Posso copiare a mano i salvataggi di Steam su un altro PC?

Sì. Chiudi il gioco, copia la cartella dei salvataggi nello stesso posto sull'altro PC e avvia lì il gioco. Se usa Steam Cloud, Steam può chiederti quale copia tenere al prossimo avvio; scegli quella con la data più recente.

### Hoard è gratuito?

Sì. L'app, la riga di comando e il server sono gratuiti e open source (AGPL-3.0), e far girare il tuo server non costa altro che la macchina. Hoard One, il server gestito che consigliamo a tutti, è gratuito anche lui con 2 GB e tre dispositivi; Pro aggiunge spazio per 1,99 € al mese.

### Il self-hosting invia qualcosa a Hoard?

No. Sul tuo server non c'è nessun account da noi e nessuna telemetria verso di noi: i tuoi salvataggi, i tuoi utenti e i tuoi log restano sulla tua macchina e non toccano mai i nostri.
