---
title: "Alternativa a GameSave Manager: backup e sincronizzazione automatici, anche su Linux"
description: "Cerchi un'alternativa a GameSave Manager? Hoard salva ogni partita da solo, la sincronizza tra PC e Steam Deck, e funziona anche su Linux e macOS."
order: 27
updated: 2026-11-05
---

Tre programmi diversi si chiamano Game Save Manager, e tutti e tre fanno il backup dei salvataggi su un solo PC Windows. Se vuoi backup che avvengano da soli e ti seguano sugli altri PC, su uno Steam Deck o su Linux, [Hoard](/download) è gratis, open source e fatto proprio per questo.

## Quale Game Save Manager hai

- **GameSave Manager**, il veterano closed source per Windows. Ha un suo database di giochi, salva in un unico archivio su richiesta o con una pianificazione, e il suo **Sync & Link** sposta una cartella di salvataggi in Dropbox, OneDrive o Google Drive lasciando un collegamento al suo posto.
- **Game Save Manager (GSM)** di dyang886, GPL-3.0, Windows. Trova i giochi installati grazie al database di PCGamingWiki, tiene per ogni gioco una serie di backup locali a rotazione e porta la cronologia su un PC nuovo in un archivio `.gsmr`.
- **game-save-manager** di mcthesw, AGPL-3.0, Windows, popolare nelle community cinesi. Salva a intervalli programmati le cartelle che aggiungi, con WebDAV per una copia fuori dalla macchina.

## Cosa fanno bene

- **Semplici e locali.** Scegli i giochi, premi backup, ottieni un archivio. Nessun account e niente da configurare.
- **Buon rilevamento su Windows.** Grandi database di giochi trovano quasi tutti i giochi installati senza che tu debba cercare i percorsi.
- **Un trasloco pulito su un PC nuovo.** Un solo archivio esportato si porta dietro tutto.

Se giochi solo su un PC Windows e premere un pulsante ti va bene, uno qualsiasi fa il suo lavoro.

## Dove Hoard va oltre

| | Strumenti Game Save Manager | Hoard |
|---|---|---|
| Quando fa il backup | Quando premi il pulsante, o con una pianificazione | A ogni salvataggio, e ancora quando esci |
| Sincronizzazione tra PC | Esporta e importa, o una cartella cloud con Sync & Link | Automatica: il salvataggio più recente ti aspetta sulla macchina successiva |
| L'altro PC può essere spento | Solo tramite una cartella cloud | Sì, il salvataggio aspetta sul server |
| Cronologia | Backup locali, o quello che tiene il cloud | Ogni versione, deduplicata, su tutte le tue macchine |
| Salvataggi scritti a metà | Un client cloud può sincronizzare una cartella collegata mentre il gioco scrive | Non carica mai un file scritto a metà e non sostituisce mai il salvataggio di un gioco aperto |
| Piattaforme | Windows | Windows, Linux, macOS e SteamOS |
| Licenza | Closed source, GPL-3.0 o AGPL-3.0 | AGPL-3.0, gratis |

La riga di Linux è quella che la gente cerca. Nessuno degli strumenti Game Save Manager gira su Linux o su uno Steam Deck. Hoard sì, in modo nativo: la stessa app su tutti e quattro i sistemi, con i prefissi Proton e Wine rilevati, così il salvataggio di un gioco sul tuo PC Windows e quello dello stesso gioco sul Deck diventano due versioni di un'unica cronologia.

## Come passare a Hoard

1. Installa [Hoard](/download) e accedi a Hoard One, consigliato a tutti, oppure puntalo sul [tuo server](/guides/self-host-hoard).
2. Se usavi **Sync & Link**, rimetti prima la cartella dei salvataggi al suo posto originale, così un client cloud e Hoard non sincronizzano gli stessi file.
3. Apri la **Libreria**. Hoard trova i tuoi giochi grazie al database comunitario dei salvataggi e ai tuoi launcher; aggiungi a mano qualsiasi cartella che gli sfugge.
4. Gioca. La prima versione compare nella cronologia appena il gioco salva. Installa Hoard sulle altre macchine con lo stesso account e la [sincronizzazione tra PC](/guides/sync-game-saves-across-pcs) avviene da sola.

I tuoi vecchi archivi di Game Save Manager possono restare dove sono, come copia fredda.

## Hoard One, o il tuo server

Hoard è gratuito in entrambi i casi, e l'app è la stessa:

- **Hoard One**, consigliato per tutti. Niente da configurare né da tenere acceso: un server gestito nell'UE, gratuito con 2 GB e tre dispositivi, senza carta. [Pro](/pricing) aggiunge solo spazio, 100 GB e dispositivi illimitati, per quando i tuoi salvataggi non ci stanno più.
- **Il tuo server.** Fai girare `hoard-server` su un PC, un NAS o un Raspberry Pi e tutte le tue macchine si sincronizzano attraverso quel server. Nessun account da noi, nessuna telemetria verso di noi e nessuna quota nostra: niente passa dai nostri server, perché sul percorso non c'è niente di nostro. Vedi [come fare il self-hosting di Hoard](/guides/self-host-hoard).

Stesso programma, stesso rilevamento, stessa cronologia. Hoard One conserva ogni versione per sempre; sul tuo server decidi tu quanta cronologia tenere.

<!-- faq -->

## Domande frequenti

### Esiste un GameSave Manager per Linux?

No: GameSave Manager funziona solo su Windows, come i due Game Save Manager open source. Hoard gira nativamente su Linux e SteamOS, trova i salvataggi dentro i prefissi Proton e Wine, e li sincronizza con i tuoi PC Windows.

### Ludusavi o Game Save Manager?

Sono entrambi strumenti di backup locale. Ludusavi è open source, gira su Windows, Linux e macOS e si basa sul manifest comunitario dei salvataggi; i Game Save Manager girano solo su Windows. Se vuoi backup e sincronizzazione senza collegare Rclone da solo, Hoard usa gli stessi dati comunitari e sincronizza da sé. C'è un [confronto completo con Ludusavi](/guides/ludusavi-alternative).

### Posso tenere i miei vecchi backup?

Sì. Hoard non li tocca. Tieni gli archivi come copia fredda; dal primo salvataggio in poi, ci pensa la cronologia di Hoard.

### Hoard è gratuito?

Sì. L'app, la riga di comando e il server sono gratuiti e open source (AGPL-3.0), e far girare il tuo server non costa altro che la macchina. Hoard One, il server gestito che consigliamo a tutti, è gratuito anche lui con 2 GB e tre dispositivi; Pro aggiunge spazio per 1,99 € al mese.

### Il self-hosting invia qualcosa a Hoard?

No. Sul tuo server non c'è nessun account da noi e nessuna telemetria verso di noi: i tuoi salvataggi, i tuoi utenti e i tuoi log restano sulla tua macchina e non toccano mai i nostri.
