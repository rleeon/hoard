---
title: "Sincronizzazione dei salvataggi a confronto: Hoard contro Ludusavi, Syncthing, OpenSave e le altre"
description: "Confronto onesto degli strumenti che copiano e sincronizzano i salvataggi PC — Ludusavi, Syncthing, OpenSave, OpenCloudSaves, Game Backup Monitor, Aletheia, SaveSync e Hoard — con tabella e una sezione su dove Hoard perde."
order: 4
updated: 2026-09-01
---

Steam Cloud copre solo i giochi comprati su Steam, e solo quando lo sviluppatore si è preso la briga di attivarlo. Emulatori, GOG, Epic, itch.io, giochi non Steam, qualsiasi cosa con mod: niente di tutto questo rientra. Se giochi su più macchine, un fisso e uno Steam Deck per dire, finisci a copiare cartelle a mano sperando di aver preso la più recente.

Diversi strumenti risolvono la cosa, e non fanno tutti lo stesso. Alcuni fanno copie locali, altri replicano cartelle tra dispositivi, altri caricano su un cloud. Questa pagina li passa in rassegna e dice in cosa ciascuno è davvero bravo. Hoard è il mio progetto, quindi la parte onesta arriva alla fine: una sezione su dove Hoard perde, e una tabella che puoi leggere senza credere a una parola del testo.

## Ludusavi

Il più noto, e a ragione. Ludusavi (di mtkennerly) è uno strumento di backup gratuito e open source, con interfaccia e con CLI, costruito sul manifesto comunitario delle posizioni dei salvataggi che copre decine di migliaia di giochi: lo stesso manifesto che usano quasi tutti quelli di questa lista, Hoard compreso. Tiene copie locali versionate e può spingerle su un cloud tuo tramite Rclone.

**Il migliore se:** vuoi copie locali, controllo totale e nessun server da nessuna parte. È la scelta più sicura della lista e non costa nulla.

**Dove si ferma:** la sincronizzazione tra macchine è una cosa che monti tu. Pianifichi un backup, configuri un remote Rclone e ti ricordi di ripristinare sull'altro PC *prima* di giocare. Funziona, ma nulla ti impedisce di dimenticare l'ultimo passo.

## Syncthing

Non è affatto uno strumento per giochi: è uno specchio di cartelle peer-to-peer generico, e molto buono. Gli indichi una cartella di salvataggi e compare sugli altri dispositivi.

**Il migliore se:** lo usi già e vuoi i file in due posti senza cloud in mezzo.

**Dove si ferma:** replica, non fotografa. Un salvataggio corrotto raggiunge ogni dispositivo in pochi secondi, esattamente alla stessa velocità di uno buono. Il versionamento è per file, senza alcuna idea di cosa sia una sessione di gioco, quindi «torna a com'era martedì sera» te lo ricostruisci a mano. Due macchine che hanno giocato entrambe offline ti danno file di conflitto, non una fusione.

## OpenSave

Sincronizzazione peer-to-peer costruita apposta per i salvataggi, in Go, con licenza MIT, per Windows, Linux e Steam Deck. Nessun account, nessun server: i dispositivi si accoppiano tra loro e sincronizzano sulla rete locale o tramite un codice stanza su un relay. Ogni modifica diventa uno snapshot, ci sono i branch per partite parallele, i conflitti si risolvono per lignaggio di sincronizzazione invece che per orologio, e viaggiano solo i blocchi cambiati. Volendo può replicare su Drive, Dropbox, OneDrive o WebDAV.

**Il migliore se:** ti rifiuti di avere un account e i tuoi dispositivi sono accesi insieme abbastanza spesso.

**Dove si ferma:** peer-to-peer vuol dire che il salvataggio vive solo sui tuoi dispositivi. Se muore il Deck con l'unica copia recente e la replica non era configurata, è finita. Per sincronizzare devono essere accesi entrambi i dispositivi, e non c'è una build per macOS.

## OpenCloudSaves

Un'interfaccia multipiattaforma che sincronizza le cartelle dei salvataggi su un cloud che già paghi — OneDrive, Google Drive, Dropbox, Nextcloud — con Rclone sotto.

**Il migliore se:** vuoi i salvataggi in uno spazio di archiviazione che hai già, con un'interfaccia invece dei file di configurazione di Rclone.

**Dove si ferma:** non c'è deduplicazione a livello di contenuto. Dieci copie di un salvataggio da 2 GB sono 20 GB della tua quota Drive, e i cloud di file sincronizzano file, non sessioni di gioco: quel che recuperi è com'era la cartella in quel momento.

## Game Backup Monitor

Prima Windows, e il capostipite di tutto il genere. GBM sorveglia il processo del gioco e, quando esci, comprime il salvataggio con 7-Zip tenendo una cronologia numerata.

**Il migliore se:** sei su un solo PC Windows e vuoi un archivio locale compresso senza pensarci.

**Dove si ferma:** è uno strumento di backup, non di sincronizzazione. Portare l'archivio su una seconda macchina è affare tuo, e Steam Deck / SteamOS non è il suo terreno.

## Aletheia

Il più nuovo del gruppo, AGPL, e va proprio sulla parte che gli altri coprono a metà: i launcher. Heroic, itch.io, Lutris, Steam, GOG Galaxy e Xbox, su Windows, Linux e macOS.

**Il migliore se:** la tua libreria è sparsa tra launcher che gli altri strumenti rilevano male, soprattutto Xbox/Game Pass e Heroic.

**Dove si ferma:** è un progetto giovane con un perimetro volutamente stretto. Copia e ripristino sono tutto il set di funzioni; dietro non c'è un cloud versionato.

## SaveSync

Quello commerciale, venduto su Steam con acquisto unico, centrato su Windows. Il suo trucco è che non punta a te-su-due-PC ma al cooperativo: i salvataggi finiscono in voci private e non elencate dello Steam Workshop così che un amico possa scaricarsi il tuo mondo di Valheim o di Factorio, e c'è anche la sincronizzazione in rete locale.

**Il migliore se:** il problema che risolvi è «ospita il mio amico e mi serve il suo salvataggio», non «che i miei salvataggi mi seguano».

**Dove si ferma:** codice chiuso, Windows, legato a Steam come mezzo di trasporto, e un elenco di giochi cooperativi supportati invece di tutto quello che possiedi.

## Una nota su EmuDeck

EmuDeck salta fuori in queste discussioni e non è un concorrente nel senso normale: è un installatore e configuratore di emulatori per Steam Deck, e la sincronizzazione che offre è una comodità innestata su quel lavoro (Rclone verso un cloud di file, solo per i salvataggi degli emulatori). Si sovrappone agli strumenti qui sopra senza essere la stessa cosa: EmuDeck ti sistema gli emulatori, quelli di qui si occupano dei salvataggi dell'intera libreria. C'è chi usa EmuDeck accanto a uno di questi, ed è una configurazione sensata, non ridondante.

## Hoard

Hoard prende la sessione di gioco come unità. Il motore gira come servizio in background — `hoardd`, senza finestra, quindi funziona in modalità gioco su SteamOS —, si accorge che hai smesso di giocare e scatta lo snapshot allora, invece di reagire a ogni scrittura di file mentre giochi.

- **Cronologia versionata per sessione.** Ogni sessione è una versione a cui tornare, anche dopo un guasto al disco o un'installazione pulita.
- **Deduplicazione per hash del contenuto.** Dieci versioni di un salvataggio da 2 GB costano circa 2 GB, non 20 GB. I trasferimenti sono compressi con zstd.
- **SHA-256 in salita e in discesa.** La corruzione viene intercettata prima che possa sovrascrivere un salvataggio buono. Niente viene mai sovrascritto in silenzio: è tutto il senso del progetto.
- **Cloud o self-hosted, lo stesso binario.** Hoard Cloud ha un piano gratuito (2 GB, 3 dispositivi, cronologia completa). Oppure avvii `hoard-server` da solo con Docker Compose su qualsiasi archiviazione compatibile S3 — MinIO, Garage, Backblaze B2 — senza account e senza quota. AGPL-3.0.
- **Windows, Linux, macOS**, più una CLI senza interfaccia per uno Steam Deck o un server.
- **Emulatori in beta:** PCSX2, RPCS3, Dolphin, Cemu, Ryujinx, RetroArch, DuckStation, PPSSPP e altri come preimpostazioni.

## Il dettaglio che decide la sincronizzazione Steam Deck ↔ PC

Vale la pena saperlo qualunque strumento tu scelga. Il salvataggio cloud di un gioco Steam vive in `<AppID>/remote/`, e la cartella *sopra* contiene `remotecache.vdf`, lo stato degli obiettivi, le statistiche e i contatori delle ore giocate: tutte cose che legittimamente differiscono tra il Deck e il fisso.

Sincronizza la cartella padre e ottieni un conflitto permanente tra due macchine che non hanno mai discordato su un solo salvataggio. Hoard traccia `remote/`, non la cartella padre. A qualsiasi strumento a cui indichi una cartella a mano si può dire lo stesso, ed è la prima cosa da controllare quando una configurazione di sincronizzazione segnala conflitti senza motivo visibile.

## Dove Hoard perde

- **Vuole un server.** Account cloud o macchina tua, in ogni caso è infrastruttura, mentre OpenSave o Ludusavi non ne richiedono nessuna.
- **Il supporto agli emulatori è in beta.** Le installazioni portatili e le manie dei singoli emulatori lo colgono ancora in fallo, e oggi Aletheia e OpenSave coprono meglio certi casi limite di launcher ed emulatori.
- **macOS è provato pochissimo su hardware vero.** Compila e gira, ma nessuno ci ha vissuto per mesi.
- **È giovane.** Ludusavi e Game Backup Monitor hanno anni di segnalazioni alle spalle. Hoard no, e per qualcosa che custodisce una partita da 200 ore la differenza conta.
- **Non fa condivisione cooperativa.** Se vuoi passare un mondo a un amico, SaveSync è fatto per quello e Hoard no.

## La distinzione tra Hoard Cloud e self-hosting

I confronti su Hoard quasi sempre fondono i due in uno solo, e il risultato è sbagliato. Quindi, chiaramente:

- **Hoard Cloud** è l'opzione gestita: accedi e i tuoi salvataggi stanno sui nostri server, nell'UE.
- **Un Hoard self-hosted è interamente tuo.** Fai girare `hoard-server` sul tuo PC o NAS e i salvataggi vanno dalla tua macchina al tuo disco. **Nessun account con noi, nessuna telemetria verso di noi, nessuna quota e nessun relay**: non passa nulla dai nostri server, perché sul percorso non c'è niente di nostro. Non vediamo un salvataggio, il nome di un gioco o un indirizzo email, perché niente di tutto ciò ci arriva. Se Hoard Cloud chiudesse domani, un'installazione self-hosted continuerebbe uguale.

Stesso binario, stesso rilevamento, stessa cronologia. L'unica cosa che cambia è di chi è lo spazio di archiviazione. E per essere esatti su un dettaglio: il tuo server ha eccome i suoi accessi — un utente e un token per dispositivo — ma vivono nel tuo database, non nel nostro.

## La tabella

| Strumento | Sincronizzazione automatica tra dispositivi | Dove vivono i salvataggi | Cronologia | Piattaforme | Licenza |
|---|---|---|---|---|---|
| **Hoard** | Sì, per sessione di gioco | Hoard Cloud o un tuo server (compatibile S3) | Versionata per sessione, deduplicata | Win · Linux · macOS · Deck | AGPL-3.0, piano gratuito |
| **Ludusavi** | Manuale, o Rclone montato da te | Locale, più il tuo remote Rclone | Copie locali versionate | Win · Linux · macOS | Gratis, open source |
| **Syncthing** | Sì, specchio continuo | Solo i tuoi dispositivi | Versionamento per file | Tutto | Gratis, open source |
| **OpenSave** | Sì, peer-to-peer | I tuoi dispositivi, replica cloud opzionale | Snapshot e branch | Win · Linux · Deck | MIT |
| **OpenCloudSaves** | Sì, tramite il tuo cloud | OneDrive / Drive / Dropbox / Nextcloud | Quello che tiene il cloud | Win · Linux · macOS | Gratis, open source |
| **Game Backup Monitor** | No | Archivi 7-Zip locali | Backup numerati | Windows | Gratis, open source |
| **Aletheia** | Copia e ripristino per launcher | Il tuo spazio | Copie | Win · Linux · macOS | AGPL-3.0 |
| **SaveSync** | Sì, e con gli amici | Voci private dello Steam Workshop | Secondo l'app | Windows | A pagamento, codice chiuso |

## Quindi quale

Se vuoi una sola macchina messa al sicuro e nient'altro, prendi Ludusavi o Game Backup Monitor. Se non vuoi un account per nessun motivo e i tuoi dispositivi sono di solito accesi insieme, OpenSave. Se i salvataggi devono finire in una cartella di Drive che già paghi, OpenCloudSaves. Se condividi un mondo cooperativo con gli amici, SaveSync.

Se invece vuoi che copia *e* sincronizzazione tra PC e Steam Deck avvengano da sole, con una versione per sessione a cui tornare e la possibilità di ospitare tutto da te, è per questo che c'è Hoard. [Scaricalo](/download), o leggi prima [come ospitarlo da solo con Docker](/guides/self-host-hoard). C'è anche un [confronto approfondito con Ludusavi](/guides/ludusavi-alternative) se è quello che stai valutando.

## Confronti uno contro uno

Ognuno va più a fondo del blocco qui sopra, compresi i punti in cui vince l'altro strumento:

- [Hoard contro Ludusavi](/guides/ludusavi-alternative)
- [Hoard come alternativa a Steam Cloud](/guides/steam-cloud-alternative)
- [Sincronizzazione peer-to-peer contro un server tuo](/guides/opensave-alternative)
- [Syncthing per i salvataggi: cosa si rompe](/guides/syncthing-game-saves)

<!-- faq -->

## Domande frequenti

### Quale di questi strumenti tiene una cronologia delle versioni?

Hoard conserva ogni sessione come una versione a cui tornare. Ludusavi tiene backup locali versionati. Quasi tutti gli altri sincronizzano o copiano lo stato attuale, quindi un salvataggio corrotto viene propagato fedelmente all'altra macchina.

### Quale funziona senza server né account?

Ludusavi con i backup locali, e qualsiasi strumento peer-to-peer. Ci rientra anche Hoard se fai self-hosting: nessun account con noi e niente che passi dai nostri server.

### Quale copre i giochi che non stanno su Steam?

Tutti i gestori di salvataggi elencati, perché individuano i file tramite lo stesso database comunitario e non attraverso un negozio. L'eccezione è Steam Cloud: copre solo i giochi Steam il cui sviluppatore l'ha attivata.

### Devo sceglierne uno solo?

No, e molti non lo fanno. Uno strumento di backup locale e uno di sincronizzazione risolvono metà diverse del problema. L'unica regola è non puntare mai uno alla cartella di backup dell'altro, o finisci per sincronizzare un mirror vecchio invece del salvataggio reale.

### Qual è il dettaglio che rompe quasi tutti i setup fai-da-te?

Sincronizzare la cartella sopra `<AppID>/remote/` dentro `userdata` di Steam. Quella superiore contiene `remotecache.vdf` e i file di obiettivi e tempo di gioco, che devono differire da macchina a macchina: ogni avvio sembra un conflitto anche se nessun salvataggio si è mosso.
