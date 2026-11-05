---
title: "Sincronizzazione dei salvataggi a confronto: Hoard contro Ludusavi, Syncthing, OpenSave e le altre"
description: "Hoard, Ludusavi, Syncthing, OpenSave, GameSave Manager, SaveState, EmuSync e altri a confronto: i punti di forza di ognuno e una tabella."
order: 4
updated: 2026-11-05
---

Steam Cloud copre solo i giochi comprati su Steam, e solo quando lo sviluppatore si è preso la briga di attivarlo. Emulatori, GOG, Epic, itch.io, giochi non Steam, qualsiasi cosa con mod: niente di tutto questo rientra. Se giochi su più macchine, un fisso e uno Steam Deck per dire, finisci a copiare cartelle a mano sperando di aver preso la più recente.

Diversi strumenti risolvono la cosa, e non fanno tutti lo stesso. Alcuni fanno copie locali, altri replicano cartelle tra dispositivi, altri caricano su un cloud. Questa pagina li passa in rassegna e dice in cosa ciascuno è davvero bravo. Hoard è il mio progetto, quindi arriva per ultimo, con quello che non fa, e con una tabella che puoi leggere senza credere a una parola del testo.

## Ludusavi

Il più noto, e a ragione. Ludusavi (di mtkennerly) è uno strumento di backup gratuito e open source, con interfaccia e con CLI, costruito sul manifesto comunitario delle posizioni dei salvataggi che copre decine di migliaia di giochi: lo stesso manifesto che usano quasi tutti quelli di questa lista, Hoard compreso. Tiene copie locali versionate e può spingerle su un cloud tuo tramite Rclone.

**Il migliore se:** vuoi copie locali, controllo totale e nessun server da nessuna parte. È la scelta più sicura della lista e non costa nulla.

**Dove si ferma:** la sincronizzazione tra macchine è una cosa che monti tu. Pianifichi un backup, configuri un remote Rclone e ti ricordi di ripristinare sull'altro PC *prima* di giocare. Funziona, ma nulla ti impedisce di dimenticare l'ultimo passo.

## Syncthing

Non è affatto uno strumento per giochi: è uno specchio di cartelle peer-to-peer generico, e molto buono. Gli indichi una cartella di salvataggi e compare sugli altri dispositivi.

**Il migliore se:** lo usi già e vuoi i file in due posti senza cloud in mezzo.

**Dove si ferma:** replica, non fotografa. Un salvataggio corrotto raggiunge ogni dispositivo in pochi secondi, esattamente alla stessa velocità di uno buono. Il versionamento è per file, senza alcuna idea di cosa sia una sessione di gioco, quindi «torna a com'era martedì sera» te lo ricostruisci a mano. Due macchine che hanno giocato entrambe offline ti danno file di conflitto, non una fusione.

## OpenSave

Sincronizzazione peer-to-peer costruita apposta per i salvataggi, in Go, con licenza MIT, per Windows, Linux, Steam Deck e, dalla versione 2.4, macOS. Nessun account, nessun server: i dispositivi si accoppiano tra loro e sincronizzano sulla rete locale o tramite un codice stanza su un relay. Ogni modifica diventa uno snapshot, ci sono i branch per partite parallele, i conflitti si risolvono per lignaggio di sincronizzazione invece che per orologio, e viaggiano solo i blocchi cambiati. Volendo può replicare su Drive, Dropbox, OneDrive o WebDAV.

**Il migliore se:** ti rifiuti di avere un account e i tuoi dispositivi sono accesi insieme abbastanza spesso.

**Dove si ferma:** peer-to-peer vuol dire che il salvataggio vive solo sui tuoi dispositivi. Se muore il Deck con l'unica copia recente e la replica non era configurata, è finita. Per sincronizzare devono essere accesi entrambi i dispositivi.

## OpenCloudSaves

Un'interfaccia multipiattaforma che sincronizza le cartelle dei salvataggi su un cloud che già paghi — OneDrive, Google Drive, Dropbox, Nextcloud — con Rclone sotto.

**Il migliore se:** vuoi i salvataggi in uno spazio di archiviazione che hai già, con un'interfaccia invece dei file di configurazione di Rclone.

**Dove si ferma:** il progetto è stato archiviato a gennaio 2025 e non è più mantenuto, quindi non riceverà correzioni per giochi nuovi o aggiornamenti di sistema. In più, non c'è deduplicazione a livello di contenuto. Dieci copie di un salvataggio da 2 GB sono 20 GB della tua quota Drive, e i cloud di file sincronizzano file, non sessioni di gioco: quel che recuperi è com'era la cartella in quel momento.

## Game Backup Monitor

Prima Windows, e il capostipite di tutto il genere. GBM sorveglia il processo del gioco e, quando esci, comprime il salvataggio con 7-Zip tenendo una cronologia numerata.

**Il migliore se:** sei su un solo PC Windows e vuoi un archivio locale compresso senza pensarci.

**Dove si ferma:** è uno strumento di backup, non di sincronizzazione. Portare l'archivio su una seconda macchina è affare tuo, e Steam Deck / SteamOS non è il suo terreno.

## GameSave Manager

Uno strumento Windows gratuito di lunga data, closed source, con un proprio database di giochi. Fa backup e ripristino, e la sua funzione più nota, **Sync & Link**, sposta una cartella di salvataggio in una cartella cloud come Dropbox o OneDrive e lascia un collegamento al suo posto, così il client cloud la tiene sincronizzata.

**Ideale se:** sei su Windows, vivi già in Dropbox o OneDrive e vuoi semplicemente che il salvataggio stia lì.

**Dove si ferma:** solo Windows, e Sync & Link significa che un client cloud generico sincronizza la cartella attiva mentre il gioco ci scrive, lo stesso schema che rende [Syncthing rischioso per i salvataggi](/guides/syncthing-game-saves). La cronologia è quella che tiene il tuo cloud.

## Game Save Manager (GSM)

Nonostante il nome, non è lo stesso programma di quello sopra. GSM, di dyang886, è un'app per Windows con licenza GPL-3.0 che trova i giochi installati su Steam, Epic, Battle.net e altri launcher grazie al database di PCGamingWiki, tiene per ogni gioco una serie di backup locali a rotazione fino al limite che scegli tu, e porta tutta la cronologia su un PC nuovo in un archivio `.gsmr` esportato. C'è perfino un terzo omonimo: il game-save-manager di mcthesw, popolare nelle community cinesi, che salva a intervalli programmati le cartelle che aggiungi a mano, con WebDAV per una copia fuori dalla macchina.

**Il migliore se:** giochi su un solo PC Windows e vuoi backup locali con versioni quasi senza configurazione, più un trasloco facile su una macchina nuova.

**Dove si ferma:** non sincronizza tra dispositivi: il secondo PC riceve i salvataggi quando esporti un archivio e lo importi lì. Solo Windows.

## SaveState

SaveState, di Matteo842, è un gestore di backup con licenza GPL-3.0 per Windows e Linux, con un'app Android a parte. Ci trascini sopra il collegamento di un launcher e trova la cartella dei salvataggi, e il supporto agli emulatori è molto ampio: RetroArch, Dolphin, le memory card di PCSX2, xemu e più di altri venti. I backup sono archivi `.zip` compressi a rotazione, e una sincronizzazione opzionale li copia su Google Drive, WebDAV, FTP, una cartella di rete o Git.

**Il migliore se:** vuoi uno strumento di backup amichevole, con un'ampia copertura di emulatori, e non ti pesa ripristinare a mano sull'altra macchina.

**Dove si ferma:** è costruito attorno agli archivi di backup, non al salvataggio vivo. La sincronizzazione sposta quegli archivi tra le macchine, ma rimetterne uno nella cartella del gioco dall'altra parte resta un ripristino che avvii tu. Niente macOS.

## EmuSync

EmuSync è uno strumento di sincronizzazione con licenza GPL-3.0 per salvataggi PC e di emulatori, pensato prima di tutto per un PC Windows e uno Steam Deck. Associ la posizione dei salvataggi di ogni gioco, su ogni dispositivo, a un unico punto di sincronizzazione nel tuo Google Drive, Dropbox o OneDrive, o in una cartella condivisa, e sincronizza in automatico o su richiesta. Un plugin Decky mostra lo stato in modalità Gioco.

**Il migliore se:** giochi tra un PC Windows e un Deck e vuoi i salvataggi in un cloud che hai già.

**Dove si ferma:** il cloud tiene lo stato attuale, e backup e ripristino restano locali a ogni dispositivo invece di una cronologia condivisa tra loro. macOS non è testato, e l'ultima versione è di febbraio 2026.

## Aletheia

Uno dei più recenti, AGPL, e va proprio sulla parte che gli altri coprono a metà: i launcher. Heroic, itch.io, Lutris, Steam, GOG Galaxy e Xbox, su Windows, Linux e macOS.

**Il migliore se:** la tua libreria è sparsa tra launcher che gli altri strumenti rilevano male, soprattutto Xbox/Game Pass e Heroic.

**Dove si ferma:** è un progetto giovane con un perimetro volutamente stretto. Copia e ripristino sono tutto il set di funzioni; dietro non c'è un cloud versionato.

## SaveSync

Quello commerciale, venduto su Steam con acquisto unico, centrato su Windows. Il suo trucco è che non punta a te-su-due-PC ma al cooperativo: i salvataggi finiscono in voci private e non elencate dello Steam Workshop così che un amico possa scaricarsi il tuo mondo di Valheim o di Factorio, e c'è anche la sincronizzazione in rete locale.

**Il migliore se:** il problema che risolvi è «ospita il mio amico e mi serve il suo salvataggio», non «che i miei salvataggi mi seguano».

**Dove si ferma:** codice chiuso, Windows, legato a Steam come mezzo di trasporto, e un elenco di giochi cooperativi supportati invece di tutto quello che possiedi.

## Tachyon

Il nome più recente della lista, in beta gratuita su Windows. Tachyon rileva i salvataggi di oltre 5.000 giochi PC, li sincronizza tramite il proprio cloud e tiene una cronologia delle versioni con più "timeline".

**Ideale se:** sei su Windows, vuoi qualcosa che funzioni subito e non ti dà fastidio che sia una beta.

**Dove si ferma:** per ora solo Windows (Linux e macOS sono annunciati), quindi niente Steam Deck; nessun codice sorgente pubblicato; nessun modo di usare il proprio server. I prezzi dopo la beta non sono stati annunciati.

## Plugin per Steam Deck (Decky)

Sul Deck stesso, alcuni plugin di Decky Loader coprono parte del lavoro dalla modalità Gioco. Decky Cloud Save sincronizza cartelle con OneDrive, Google Drive o Dropbox tramite rclone dopo la chiusura di un gioco; il suo autore non lo mantiene più e rimanda a due successori, Decky Ludusavi (un'interfaccia di Ludusavi per la modalità Gioco) e SDH-GameSync (un fork che scarica i salvataggi all'avvio di ogni gioco e li carica alla chiusura). Steamback tiene in locale i dieci snapshot più recenti dei giochi che usano Steam Cloud, presi quando esci.

**Il migliore se:** giochi soprattutto sul Deck e vuoi qualcosa da usare senza uscire dalla modalità Gioco.

**Dove si ferma:** funzionano solo sul Deck, e la parte tra dispositivi si appoggia a qualcos'altro: rclone e il tuo cloud, Ludusavi o lo stesso Steam Cloud. Gli snapshot di Steamback restano sul Deck e coprono solo i giochi con Steam Cloud.

## Una nota su EmuDeck

EmuDeck salta fuori in queste discussioni e non è un concorrente nel senso normale: è un installatore e configuratore di emulatori per Steam Deck, e la sincronizzazione che offre è una comodità innestata su quel lavoro (Rclone verso un cloud di file, solo per i salvataggi degli emulatori). Si sovrappone agli strumenti qui sopra senza essere la stessa cosa: EmuDeck ti sistema gli emulatori, quelli di qui si occupano dei salvataggi dell'intera libreria. C'è chi usa EmuDeck accanto a uno di questi, ed è una configurazione sensata, non ridondante.

## Hoard

Hoard fa il backup di ogni salvataggio appena il gioco finisce di scriverlo e lo tiene sincronizzato su tutte le macchine su cui giochi. Il motore gira come servizio in background, `hoardd`, senza finestra, quindi continua a funzionare in modalità Gioco su SteamOS. Aspetta che la cartella resti ferma per qualche secondo, quindi non carica mai un file scritto a metà; fa un'altra copia quando esci; e non sostituisce mai il salvataggio di un gioco aperto: quello che arriva da un'altra macchina aspetta che tu chiuda il gioco.

- **L'altra macchina può essere spenta.** Il salvataggio più recente aspetta sul server, quindi il Deck che riprendi la settimana prossima lo riceve senza che il PC sia acceso. Gli strumenti peer-to-peer hanno bisogno di entrambi i dispositivi svegli nello stesso momento.
- **Ogni versione, conservata.** Ogni salvataggio è una versione a cui tornare dall'app o con `hoard restore`, anche dopo un guasto del disco o un'installazione pulita. Hoard One conserva ogni versione per sempre; sul tuo server decidi tu quanta cronologia tenere.
- **Viaggia solo ciò che cambia.** I caricamenti mandano solo i file modificati, compressi con zstd, e le versioni condividono tutto ciò che non cambia: dieci versioni di un salvataggio da 2 GB che cambia poco occupano circa 2 GB, non 20.
- **SHA-256 all'andata e al ritorno.** La corruzione viene individuata prima che possa sovrascrivere un salvataggio buono, e Hoard tiene una copia di tutto ciò che sostituisce.
- **Gratis e open source, con il tuo server se lo vuoi.** App, CLI e server sono AGPL-3.0. Hoard One, il server gestito che consigliamo a tutti, è gratis con 2 GB e tre dispositivi, e non c'è niente da installare. Oppure avvii `hoard-server` con Docker su un PC, un NAS o un Raspberry Pi, su disco locale o qualsiasi storage compatibile S3: nessun account con noi, nessuna telemetria verso di noi, nessuna quota nostra.
- **Windows, Linux, macOS e SteamOS**, con la stessa app desktop su tutti e quattro e una CLI per server e macchine senza schermo.
- **Un rilevamento che copre tutta la libreria:** Steam, Epic, GOG, prefissi Proton e Wine, migliaia di giochi grazie al database comunitario dei salvataggi, e preset per 19 emulatori, da PCSX2, Dolphin e RPCS3 a Ryujinx, Cemu, DuckStation e RetroArch.

## Il dettaglio che decide la sincronizzazione Steam Deck ↔ PC

Vale la pena saperlo qualunque strumento tu scelga. Il salvataggio cloud di un gioco Steam vive in `<AppID>/remote/`, e la cartella *sopra* contiene `remotecache.vdf`, lo stato degli obiettivi, le statistiche e i contatori delle ore giocate: tutte cose che legittimamente differiscono tra il Deck e il fisso.

Sincronizza la cartella padre e ottieni un conflitto permanente tra due macchine che non hanno mai discordato su un solo salvataggio. Hoard traccia `remote/`, non la cartella padre. A qualsiasi strumento a cui indichi una cartella a mano si può dire lo stesso, ed è la prima cosa da controllare quando una configurazione di sincronizzazione segnala conflitti senza motivo visibile.

## Cosa Hoard non fa

- **Sincronizzare senza server.** Hoard sincronizza sempre attraverso un server: Hoard One se preferisci non gestire niente, o il tuo. È proprio questo che permette all'altro dispositivo di essere spento, e che lascia una copia che sopravvive a entrambe le macchine. Se non vuoi un server in nessun caso, OpenSave, Syncthing o Ludusavi sono fatti per questo.
- **Condividere mondi in co-op.** Passare un mondo a un amico è il lavoro di SaveSync; Hoard si occupa dei tuoi salvataggi.
- **Unire due salvataggi che hanno preso strade diverse.** Nessuno strumento può fondere due partite in una. Hoard le tiene entrambe come versioni, su tutte le tue macchine, e scegli tu.

## La distinzione tra Hoard One e self-hosting

I confronti su Hoard quasi sempre fondono i due in uno solo, e il risultato è sbagliato. Quindi, chiaramente:

- **Hoard One** è l'opzione gestita, quella che consigliamo a tutti: accedi e i tuoi salvataggi stanno sui nostri server, nell'UE.
- **Un Hoard self-hosted è interamente tuo.** Fai girare `hoard-server` sul tuo PC o NAS e i salvataggi vanno dalla tua macchina al tuo disco. **Nessun account con noi, nessuna telemetria verso di noi, nessuna quota e nessun relay**: non passa nulla dai nostri server, perché sul percorso non c'è niente di nostro. Non vediamo un salvataggio, il nome di un gioco o un indirizzo email, perché niente di tutto ciò ci arriva. Se Hoard One chiudesse domani, un'installazione self-hosted continuerebbe uguale.

Stesso codice, stesso rilevamento. Cambia di chi è lo spazio di archiviazione, più un'impostazione predefinita: un server self-hosted sfoltisce le versioni vecchie col tempo, a meno che tu non lo disattivi. E per essere esatti su un dettaglio: il tuo server ha eccome i suoi accessi — un utente e un token per dispositivo — ma vivono nel tuo database, non nel nostro.

## La tabella

| Strumento | Sincronizzazione automatica | Funziona con l'altro dispositivo spento | Dove vivono i salvataggi | Cronologia | Piattaforme | Licenza |
|---|---|---|---|---|---|---|
| **Hoard** | Sì, a ogni salvataggio | Sì | Hoard One o un tuo server (disco o compatibile S3) | Ogni versione, deduplicata | Win · Linux · macOS · Deck | AGPL-3.0, gratis |
| **Ludusavi** | Manuale, o Rclone che colleghi tu | Solo con un remoto Rclone | Locale, più il tuo remoto Rclone | Backup locali versionati | Win · Linux · macOS | MIT, gratis |
| **Syncthing** | Sì, specchio continuo | No, devono essere accesi entrambi | Solo i tuoi dispositivi | Versioni per singolo file | Ovunque | MPL-2.0, gratis |
| **OpenSave** | Sì, peer-to-peer | No, a meno che configuri uno specchio cloud | I tuoi dispositivi, specchio cloud opzionale | Snapshot e rami | Win · Linux · macOS · Deck | MIT, gratis |
| **OpenCloudSaves** | Tramite il tuo cloud | Sì, tramite il cloud | OneDrive / Drive / Dropbox / Nextcloud | Quello che tiene il cloud | Win · Linux · macOS | Open source, archiviato |
| **Game Backup Monitor** | No | Nessuna sincronizzazione | Archivi 7-Zip locali | Backup numerati | Windows · Linux | GPL-3.0, gratis |
| **GameSave Manager** | Tramite il tuo cloud (Sync & Link) | Sì, tramite il cloud | Locale, più Dropbox / OneDrive | Quello che tiene il cloud | Windows | Gratis, closed source |
| **Game Save Manager (GSM)** | No, esporta e importa | Nessuna sincronizzazione | Backup locali | Backup locali a rotazione | Windows | GPL-3.0, gratis |
| **SaveState** | Opzionale, degli archivi | Sì, tramite il tuo storage | Zip locali, più Drive / WebDAV / FTP / SMB / Git | Archivi a rotazione | Windows · Linux | GPL-3.0, gratis |
| **EmuSync** | Sì, tramite il tuo cloud | Sì, tramite il cloud | Drive / Dropbox / OneDrive o una cartella condivisa | Backup locali | Windows · Linux · Deck | GPL-3.0, gratis |
| **Aletheia** | Backup e ripristino per launcher | Nessuna sincronizzazione | Il tuo storage | Backup | Win · Linux · macOS | AGPL-3.0 |
| **SaveSync** | Sì, anche con gli amici | Sì, tramite Steam | Voci private dello Steam Workshop | Secondo l'app | Windows | A pagamento, closed source |
| **Tachyon** | Sì, tramite il suo cloud | Sì | Il cloud di Tachyon | Versioni e linee temporali | Windows (beta) | Beta gratuita, senza codice pubblicato |

## Quindi quale

Se vuoi che backup e sincronizzazione avvengano da soli, tra PC e Steam Deck, con ogni versione conservata, l'altra macchina libera di restare spenta e la possibilità che tutto sia tuo, quello è Hoard. [Scaricalo](/download), accedi a Hoard One, o leggi prima [come ospitarlo sul tuo server con Docker](/guides/self-host-hoard). È gratis in entrambi i casi.

Per compiti più circoscritti: un solo PC Windows e archivi locali, Game Backup Monitor o GSM. Backup locali con pieno controllo e nessun server da nessuna parte, Ludusavi. Nessun account in nessun caso, con dispositivi di solito accesi insieme, OpenSave o Syncthing. Salvataggi che devono stare in una cartella Drive che paghi già, Ludusavi con un remoto Rclone, o EmuSync; OpenCloudSaves lo faceva, ma è archiviato. Condividere un mondo co-op con gli amici, SaveSync.

Se arrivi da OpenCloudSaves e volevi soprattutto i salvataggi in uno storage che controlli tu, più che in Drive in sé, un Hoard self-hosted lo fa, con la cronologia in più: scrive su disco locale o in qualsiasi bucket compatibile S3, come Backblaze B2 o MinIO. Se è Ludusavi che stai valutando, c'è un [confronto più lungo con Ludusavi](/guides/ludusavi-alternative).

## Confronti uno contro uno

Ognuno va più a fondo del blocco qui sopra, compresi i punti in cui vince l'altro strumento:

- [Hoard contro Ludusavi](/guides/ludusavi-alternative)
- [Hoard come alternativa a Steam Cloud](/guides/steam-cloud-alternative)
- [Sincronizzazione peer-to-peer contro un server tuo](/guides/opensave-alternative)
- [Syncthing per i salvataggi: cosa si rompe](/guides/syncthing-game-saves)
- [Hoard contro GameSave Manager e gli altri Game Save Manager](/guides/game-save-manager-alternative)
- [Sostituire OpenCloudSaves ora che è archiviato](/guides/opencloudsaves-alternative)
- [Un'alternativa gratuita a SaveSync](/guides/savesync-alternative)

<!-- faq -->

## Domande frequenti

### Quale di questi strumenti tiene una cronologia delle versioni?

Hoard conserva ogni salvataggio come una versione a cui tornare, su tutte le tue macchine. Ludusavi, GSM e SaveState tengono backup locali versionati o a rotazione, e OpenSave tiene snapshot sui tuoi dispositivi. Quasi tutti gli altri sincronizzano o copiano lo stato attuale, quindi un salvataggio corrotto viene propagato fedelmente all'altra macchina.

### Quale funziona senza server né account?

Ludusavi con i backup locali, e qualsiasi strumento peer-to-peer. Ci rientra anche Hoard se fai self-hosting: nessun account con noi e niente che passi dai nostri server.

### Quale sincronizza quando l'altro dispositivo è spento?

Qualsiasi strumento che tenga il salvataggio da qualche parte che non siano i tuoi dispositivi: Hoard, tramite Hoard One o il tuo server, e gli strumenti che scrivono su un cloud, come EmuSync o Ludusavi con Rclone. La sincronizzazione peer-to-peer (OpenSave senza il suo specchio, Syncthing) richiede entrambi i dispositivi accesi nello stesso momento.

### Quale copre i giochi che non stanno su Steam?

Tutti i gestori di salvataggi elencati, perché individuano i file tramite lo stesso database comunitario e non attraverso un negozio. L'eccezione è Steam Cloud: copre solo i giochi Steam il cui sviluppatore l'ha attivata.

### Devo sceglierne uno solo?

No, e molti non lo fanno. Uno strumento di backup locale e uno di sincronizzazione risolvono metà diverse del problema. L'unica regola è non puntare mai uno alla cartella di backup dell'altro, o finisci per sincronizzare un mirror vecchio invece del salvataggio reale.

### Qual è il dettaglio che rompe quasi tutti i setup fai-da-te?

Sincronizzare la cartella sopra `<AppID>/remote/` dentro `userdata` di Steam. Quella superiore contiene `remotecache.vdf` e i file di obiettivi e tempo di gioco, che devono differire da macchina a macchina: ogni avvio sembra un conflitto anche se nessun salvataggio si è mosso.
