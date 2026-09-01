---
title: "Come sincronizzare i salvataggi tra più PC"
description: "Gioca allo stesso gioco su fisso e portatile senza perdere progressi. Sincronizza i tuoi salvataggi tra PC automaticamente con Hoard — sincronizzazione cloud gestita, senza configurare Ludusavi e Rclone a mano."
order: 2
updated: 2026-09-01
---

Se giochi su più di un computer — un fisso a casa e un portatile in giro — Hoard mantiene i salvataggi sincronizzati così riprendi sempre da dove avevi lasciato.

## Come funziona la sincronizzazione

Hoard fa il backup di ogni salvataggio sul tuo cloud e scarica l'ultima versione sulle altre macchine. Quando finisci di giocare su un PC, il salvataggio più recente ti aspetta sul successivo.

## Imposta la sincronizzazione

1. Installa **Hoard** su ogni PC su cui giochi (Windows, macOS o Linux).
2. Accedi con lo **stesso account** su ogni macchina, o collegale allo stesso server self-hosted.
3. Aggiungi gli stessi giochi alla **Libreria** su ogni PC. Hoard li abbina per gioco, così un salvataggio fatto su uno appare sugli altri.
4. Tieni attiva la **modalità automatica**. Hoard carica dopo che giochi e scarica l'ultima versione prima che inizi.

## Arrivi da Ludusavi?

Ludusavi è un ottimo strumento open source per fare backup e ripristinare salvataggi in locale, e può inviare quei backup a un cloud che configuri tu stesso con Rclone. Ma la sincronizzazione tra dispositivi la imposti a mano: programmare il backup, configurare il remoto, poi ripristinare sull'altro PC prima di giocare.

Hoard trasforma tutto questo in sincronizzazione gestita. Usa gli stessi dati comunitari di posizione di Ludusavi per trovare i tuoi salvataggi, poi carica dopo ogni sessione e scarica l'ultima versione prima della successiva — su ogni PC del tuo account, con cronologia versionata nel cloud. Niente remoti Rclone, niente script. E come Ludusavi, Hoard è open source e può essere self-hosted. Vedi il [confronto completo con Ludusavi](/guides/ludusavi-alternative).

## Evitare i conflitti

Hoard è consapevole dei conflitti: confronta le date di modifica e conserva una copia locale di ogni salvataggio sostituito, così una sincronizzazione non distrugge mai i progressi in silenzio. Se un gioco è ancora aperto o un salvataggio è stato toccato negli ultimi minuti, Hoard aspetta.

## Steam Deck e desktop

Il setup a due macchine più comune è anche quello che si rompe più spesso quando lo si monta a mano, e quasi sempre per lo stesso motivo.

Su Windows il salvataggio di un gioco può stare in `Documenti\My Games\…` oppure dentro `userdata` di Steam. Su una Steam Deck lo stesso gioco Windows gira con Proton, quindi il salvataggio vive dentro un prefisso di compatibilità: `steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…`. Due percorsi molto diversi, un gioco solo, un solo progresso. Hoard legge i prefissi Proton oltre alle posizioni native e abbina quello che trova per gioco, così il salvataggio della Deck e quello del desktop diventano due versioni della stessa cronologia invece di due cartelle scollegate.

Il dettaglio da cui dipende tutto: per i giochi Steam, Hoard traccia `<AppID>/remote/` dentro `userdata`, **non** la cartella superiore. Quella superiore contiene anche `remotecache.vdf` e i file di obiettivi e tempo di gioco propri di ogni macchina, che tra Deck e desktop devono essere diversi. Se sincronizzi la cartella superiore, ogni avvio sembra un conflitto anche se nessun salvataggio si è mosso. È quell'unico errore a far sembrare rotti quasi tutti i setup artigianali tra Deck e PC.

## I giochi che Steam Cloud non copre

Se tutti i giochi a cui giochi supportassero Steam Cloud non ti servirebbe niente di tutto questo. Nella pratica:

- **Giochi che non vengono da Steam.** GOG, Epic, itch, Battle.net, l'app Xbox e tutto ciò che hai installato a mano.
- **Giochi Steam in cui lo sviluppatore non l'ha mai attivato**, o l'ha attivato per una sola piattaforma.
- **Emulatori.** RetroArch, Dolphin, PCSX2, RPCS3 e gli altri salvano dove preferiscono, e Steam non ne sa nulla.
- **Giochi che scrivono fuori dalla cartella sorvegliata da Steam**, e sono più di quanti immagini.

A Hoard non importa chi abbia pubblicato un gioco né da dove arrivi: traccia la cartella che cambia quando giochi.

## Quando due PC toccano lo stesso salvataggio

Giochi sul portatile senza lasciare che il fisso finisca di sincronizzare ed ecco il problema classico: due salvataggi, entrambi più recenti dell'ultima versione comune.

Hoard non sovrascrive mai alla cieca. Confronta le date di modifica, conserva una copia locale di ciò che sostituisce e aspetta finché un gioco è aperto o il salvataggio è stato toccato negli ultimi minuti: un file in scrittura non è un file da caricare a metà. Tutte le versioni precedenti restano nella cronologia cloud, quindi sbagliare versione costa due clic e non un fine settimana.

Il limite onesto: **Hoard non fonde due salvataggi divergenti.** Nessuno strumento può farlo — un file di salvataggio è opaco e non esiste un modo corretto di mescolare due pomeriggi di gioco diversi. Quello che ottieni invece è ogni versione, su ogni macchina, e la possibilità di scegliere.

## Sincronizzare senza passare dai nostri server

Vale la pena dirlo chiaramente, perché è il punto su cui quasi tutti i confronti sbagliano. Ci sono due modi di usarlo:

- **Hoard Cloud** è l'opzione gestita: accedi e i salvataggi stanno sui nostri server, nell'UE.
- **Il self-hosting è interamente tuo.** Fai girare `hoard-server` sul tuo PC o sul tuo NAS e le tue macchine si sincronizzano attraverso quello. **Nessun account con noi, nessuna telemetria verso di noi, nessuna quota e nessun relay**: non passa nulla dai nostri server, perché sul percorso non c'è niente di nostro. Vedi [come ospitare Hoard da solo](/guides/self-host-hoard).

Stesso programma, stesso rilevamento, stessa cronologia delle versioni. L'unica cosa che cambia è di chi è lo spazio di archiviazione.

## Suggerimento

Lascia che ogni macchina finisca di sincronizzare prima di avviare un gioco — la dashboard mostra lo stato in tempo reale, così sai che l'ultimo salvataggio è al suo posto.

<!-- faq -->

## Domande frequenti

### Quanti PC posso sincronizzare?

Tre nel piano gratuito, illimitati con Pro e illimitati in self-hosting: il tuo server, le tue regole.

### Le due macchine devono essere accese nello stesso momento?

No. Il salvataggio sale al server quando smetti di giocare e scende quando l'altra macchina lo chiede: il secondo PC può restare spento una settimana e ricevere comunque l'ultima versione all'accensione.

### E se gioco offline?

Nessun problema. Lo snapshot viene preso in locale quando smetti di giocare e parte da solo appena la macchina torna online.

### Sincronizza anche mod e impostazioni?

I salvataggi sì. I file che appartengono a una macchina specifica — configurazione, log e simili — vengono caricati per essere nel backup, ma non riscritti sopra la copia di un altro PC: un'impostazione grafica che va bene al fisso è raramente quella che vuole il portatile.

### Il self-hosting manda qualcosa a Hoard?

No. In modalità self-hosted non c'è alcun account con noi né telemetria verso di noi: i tuoi salvataggi, i tuoi utenti e i tuoi log stanno sul tuo server e non toccano mai il nostro.
