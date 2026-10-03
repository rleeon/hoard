---
title: "Alternativa a Ludusavi: sincronizzazione cloud automatica dei salvataggi"
description: "Ludusavi è ottimo per i backup locali. Hoard aggiunge il sync automatico tra PC e Steam Deck con cronologia, sullo stesso database dei salvataggi."
order: 5
updated: 2026-10-01
---

Se cerchi un modo per fare backup e sincronizzare i tuoi salvataggi, probabilmente hai trovato **Ludusavi** — ed è eccellente. Questa guida è un confronto onesto per aiutarti a scegliere lo strumento giusto, e spiega dove si inserisce Hoard se vuoi sincronizzazione cloud automatica tra macchine.

## Cosa fa bene Ludusavi

Ludusavi è uno strumento gratuito e open source (creato da mtkennerly) per fare backup e ripristinare i salvataggi PC su Windows, macOS e Linux. Ha una GUI pulita e una CLI, trova automaticamente i salvataggi di migliaia di giochi, conserva backup locali versionati e può inviare quei backup a un cloud tuo configurando **Rclone** (Google Drive, Dropbox e molti altri). Se vuoi pieno controllo e un setup fai-da-te, Ludusavi è una scelta fantastica — e completamente gratuita.

Hoard non vuole sostituirlo. Anzi, **Hoard usa lo stesso database comunitario di posizioni su cui si basa Ludusavi** per individuare dove ogni gioco conserva i salvataggi, quindi la qualità del rilevamento è alla pari.

## In cosa Hoard è diverso

Il punto in cui quasi tutti si bloccano con qualsiasi strumento locale è la **sincronizzazione tra dispositivi**. Con Ludusavi la fai tu: programmare un backup, configurare un remoto Rclone, poi ripristinare sull'altro PC prima di giocare. Funziona, ma è manuale.

Hoard la trasforma in **sincronizzazione cloud gestita**:

- **Accedi e via.** Niente remoti Rclone, niente script. Hoard carica il salvataggio dopo che giochi e scarica l'ultima versione prima che inizi, su ogni PC del tuo account.
- **Cronologia versionata nel cloud.** Ogni backup viene conservato, quindi puoi tornare a qualsiasi salvataggio precedente — anche dopo un guasto del disco o un'installazione pulita.
- **Consapevole dei conflitti.** Hoard confronta i timestamp e conserva una copia locale di tutto ciò che sostituisce, così una sincronizzazione non distrugge mai i progressi in silenzio.
- **Sempre open source e self-hostable.** Come Ludusavi, nessun vincolo — usa Hoard Cloud o ospita il server tu stesso.

## Testa a testa

| | Ludusavi | Hoard |
|---|---|---|
| Backup locali | Sì | Sì |
| Rilevamento dei salvataggi | Manifest comunitario | Lo stesso manifest, più le librerie Steam, i processi in esecuzione e una scansione del disco |
| Spazio cloud | Il tuo, tramite Rclone | Incluso, oppure il tuo server |
| Sincronizzazione tra PC | Manuale: backup qui, ripristino là | Automatica, dopo che smetti di giocare e prima che inizi |
| Cronologia versioni | Backup locali che poti tu | Ogni versione nel cloud, deduplicata per hash del contenuto |
| Emulatori | Sì | Sì |
| Interfacce | App desktop e CLI | App desktop, CLI e overlay in gioco |
| Prezzo | Gratuito | Piano gratis da 2 GB e 3 dispositivi, Pro oltre, nessuna quota in self-hosting |
| Licenza | MIT | AGPL-3.0 |

## Quando Ludusavi è la scelta migliore

È la parte che quasi nessuna pagina di confronto include. Ludusavi è lo strumento migliore quando:

- **Giochi su un solo PC.** La sincronizzazione cloud risolve un problema che non hai. Basta un backup locale, e Ludusavi li fa molto bene.
- **Hai già un remoto Rclone di cui ti fidi.** Se il tuo spazio è configurato e funziona, il vantaggio principale di Hoard è un passaggio che hai già pagato.
- **Vuoi usarlo dalla modalità gioco di uno Steam Deck.** Ludusavi ha un plugin Decky, quindi puoi lanciare un backup senza uscire dall'interfaccia console.
- **Vuoi una licenza permissiva.** Ludusavi è MIT, Hoard è AGPL-3.0. Se hai in mente di costruirci sopra qualcosa senza pubblicare il risultato, quella differenza pesa.
- **Non vuoi niente che giri in sottofondo.** Ospitare Hoard da soli significa tenere in piedi un piccolo server da qualche parte, anche sullo stesso PC. Ludusavi è un'app che apri quando ti serve.

## Ludusavi contro GameSave Manager

L'altro nome che salta fuori accanto a Ludusavi è **GameSave Manager**, uno strumento Windows di lunga data. Risolvono lo stesso problema in modi diversi:

- **Piattaforma.** Ludusavi gira su Windows, macOS e Linux, Steam Deck compresa. GameSave Manager solo su Windows.
- **Licenza.** Ludusavi è open source (MIT). GameSave Manager è gratuito ma closed source.
- **Trovare i salvataggi.** Ludusavi legge il manifest comunitario ricavato da PCGamingWiki, circa 20.000 giochi. GameSave Manager ha un database proprio.
- **Portare i salvataggi nel cloud.** Ludusavi copia i suoi backup su un remote tramite Rclone. Il "Sync & Link" di GameSave Manager sposta una cartella di salvataggio in una cartella cloud come Dropbox o OneDrive e lascia un collegamento al suo posto, così il client cloud sincronizza la cartella attiva.

È quest'ultimo punto quello da valutare. Una cartella attiva sincronizzata da un client cloud generico è proprio la configurazione che trasforma un salvataggio scritto a metà in un salvataggio rotto su ogni PC; la [guida a Syncthing](/guides/syncthing-game-saves) spiega perché. Hoard sta dall'altra parte di quella linea: aspetta che il gioco sia chiuso, carica una versione e tiene le precedenti.

## Passare da Ludusavi a Hoard

Non c'è un importatore, ed è voluto. I passaggi:

1. **Lascia i backup di Ludusavi esattamente dove sono.** Non viene migrato né cancellato nulla. Tienili come rete di sicurezza per le prime settimane.
2. **Installa Hoard e accedi**, oppure puntalo al tuo server.
3. **Lascia che faccia la scansione.** Legge lo stesso manifest, quindi l'elenco dei giochi rilevati dovrebbe esserti familiare.
4. **Non puntare Hoard alla cartella dei backup di Ludusavi.** Traccia la cartella in cui scrive il gioco. Una cartella di backup è una copia che cambia secondo un orario e non quando giochi, e sincronizzare la copia di una copia è il modo in cui si finisce per ripristinare i progressi di ieri. Hoard prova a rilevarlo da solo — `hoard doctor` segnala una cartella tracciata che sembra un mirror di backup — ma è più semplice non tracciarla affatto.
5. **Gioca una volta.** All'uscita, la prima versione compare nella cronologia.
6. **Ripeti sul secondo PC.** Accedi lì e le versioni sono già pronte.

## Due dettagli da sapere

**I salvataggi di Steam stanno una cartella più in profondità di quanto sembri.** Per i giochi Steam, Hoard traccia `<AppID>/remote/` dentro `userdata`, non la cartella superiore. Quella superiore contiene anche `remotecache.vdf` e i file di obiettivi e tempo di gioco, che legittimamente cambiano da macchina a macchina. Se sincronizzi la cartella superiore, ogni avvio sembra un conflitto anche se nessun salvataggio si è mosso. È il motivo più comune per cui un setup artigianale tra Steam Deck e desktop finisce per combattere contro sé stesso.

**Le versioni costano poco.** Gli snapshot sono archiviati per hash del contenuto, quindi un file che non cambia viene salvato una volta sola. Dieci versioni di un salvataggio da 2 GB occupano circa 2 GB, non 20 — ed è questo che rende pratico conservare tutta la cronologia invece di potarla.

## Cosa significa davvero il self-hosting

È il punto su cui quasi tutti i confronti sbagliano riguardo a Hoard, quindi vale la pena essere precisi. Ci sono due modi di usarlo, e sono davvero diversi:

- **Hoard Cloud** è l'opzione gestita: accedi e i tuoi salvataggi stanno sui nostri server, nell'UE.
- **Il self-hosting è interamente tuo.** Fai girare `hoard-server` sul tuo PC o sul tuo NAS, e i salvataggi vanno dalla tua macchina al tuo disco. **Nessun account con noi, nessuna telemetria verso di noi, nessuna quota e nessun relay**: non passa nulla dai nostri server, perché sul percorso non c'è niente di nostro. Non possiamo vedere un salvataggio, il nome di un gioco o un indirizzo email, per il semplice motivo che niente di tutto ciò ci arriva. Se Hoard Cloud sparisse domani, un'installazione self-hosted continuerebbe uguale.

Stesso programma, stesso rilevamento, stessa cronologia delle versioni. L'unica cosa che cambia è di chi è lo spazio di archiviazione.

## Quale scegliere?

- Scegli **Ludusavi** se vuoi uno strumento di backup gratuito e locale e non ti dispiace montare il tuo cloud con Rclone.
- Scegli **Hoard** se vuoi che backup *e* sincronizzazione tra PC funzionino da soli, con una cronologia cloud versionata, mantenendo l'opzione del self-hosting.

Molti iniziano con Ludusavi per i backup locali e passano a Hoard quando giocano agli stessi giochi su più di una macchina. Se è il tuo caso, vedi [come sincronizzare i salvataggi tra PC](/guides/sync-game-saves-across-pcs) o semplicemente [scarica Hoard](/download) e accedi. Per il quadro completo c'è un [confronto di tutti gli strumenti di sincronizzazione](/guides/game-save-sync-comparison).

<!-- faq -->

## Domande frequenti

### Posso usare Ludusavi e Hoard insieme?

Sì. Leggono le stesse posizioni e nessuno dei due tiene i file bloccati. Molti tengono Ludusavi per i backup di archivio locali e lasciano a Hoard la sincronizzazione tra macchine. L'unica regola è non puntare uno dei due alla cartella di backup dell'altro.

### Hoard importa i miei backup di Ludusavi?

No, ed è deliberato. Una cartella di backup è una copia che cambia secondo il proprio orario: tracciarla sincronizzerebbe un mirror vecchio invece del salvataggio reale. Hoard traccia la cartella in cui scrive il gioco e avvia la propria cronologia dalla sessione successiva. Tieni l'archivio di Ludusavi come rete di sicurezza.

### Hoard è gratuito?

Hoard Cloud ha un piano gratuito con 2 GB di spazio e 3 dispositivi, che copre la maggior parte delle collezioni; Pro alza entrambi. Ospitare il server per conto proprio è gratis e non ha alcuna quota. Tutto è open source sotto AGPL-3.0.

### Hoard funziona su Steam Deck?

Sì, su Steam Deck e su qualsiasi desktop Linux, oltre che su Windows e macOS. Il Deck è proprio il caso che richiede il dettaglio su `remote/` qui sopra, perché un Deck e un desktop scrivono file di obiettivi e tempo di gioco diversi accanto allo stesso salvataggio.

### Mi serve Rclone o un account cloud mio?

No. È la differenza pratica principale: con Hoard Cloud lo spazio è già pronto quando accedi. Se preferisci essere padrone dello spazio, fai girare il server tu stesso su un bucket compatibile con S3 o una normale cartella della tua macchina.

### Il self-hosting manda qualcosa a Hoard?

No. In modalità self-hosted non c'è alcun account con noi né telemetria verso di noi: i tuoi salvataggi, i tuoi utenti e i tuoi log stanno sul tuo server e non toccano mai il nostro. È tutto il senso di questa modalità, ed è il motivo per cui il server è lo stesso binario open source che usiamo noi e non una versione ridotta.

### Ludusavi è sicuro?

Sì. È open source, molto usato, e non fa altro che copiare i file di salvataggio in una cartella di backup e viceversa; non tocca i file del gioco. L'unica cosa a cui fare attenzione vale per qualsiasi strumento di backup: ripristinare un backup vecchio sopra un salvataggio più recente lo sostituisce, quindi controlla la data prima di ripristinare.

### Ludusavi funziona su Steam Deck?

Sì. C'è una versione Linux da installare in modalità Desktop e un plugin Decky per avviare i backup dalla modalità Gioco. Quello che non fa da solo è tenere allineati la Deck e il fisso: fai il backup su uno e ripristini sull'altro. Hoard se ne occupa in automatico, da un servizio in background, senza nulla da avviare.

### Ludusavi può fare il backup su Google Drive o un altro cloud?

Sì, tramite Rclone: configuri un remote per Google Drive, Dropbox, OneDrive o qualsiasi altro provider supportato da Rclone, e Ludusavi ci copia i suoi backup. Con Hoard non c'è nessun remote da configurare, e se preferisci che lo spazio sia tuo lo punti al tuo server.

### Ludusavi fa il backup in automatico?

Da solo no: fa il backup quando lo avvii. Puoi automatizzarlo con la riga di comando e un'attività pianificata, oppure avvolgere il comando di avvio di un gioco perché faccia il backup all'uscita. Hoard si accorge da solo quando un gioco si chiude, senza avvolgere nulla, e fa il backup in quel momento.

### Ludusavi supporta i giochi non Steam?

Sì. Il manifest copre giochi di GOG, Epic, Xbox e altri launcher, oltre a molti venduti senza store, e puoi aggiungere voci personalizzate per ciò che manca. Hoard legge lo stesso manifest e aggiunge una scansione del disco per i giochi che non vi compaiono.
