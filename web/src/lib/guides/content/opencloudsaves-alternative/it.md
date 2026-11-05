---
title: "Alternativa a OpenCloudSaves: è archiviato, ecco cosa usare al suo posto"
description: "OpenCloudSaves è archiviato da gennaio 2025. Hoard è un sostituto mantenuto e open source che sincronizza da solo e conserva ogni versione."
order: 28
updated: 2026-11-05
---

OpenCloudSaves è stato archiviato il 20 gennaio 2025. Il codice e l'ultima versione ci sono ancora, ma il progetto non accetta più modifiche, quindi nessuno lo sistemerà per giochi nuovi, launcher nuovi o aggiornamenti del sistema. Se sincronizzava i tuoi salvataggi tramite un cloud, [Hoard](/download) è un sostituto gratuito e open source, mantenuto attivamente, che trova i salvataggi da solo e li sincronizza ogni volta che il gioco ne scrive uno.

## Cosa faceva bene OpenCloudSaves

- **Il tuo cloud.** I salvataggi finivano nel OneDrive, Google Drive, Dropbox o Nextcloud che avevi già, tramite Rclone, con una vera interfaccia al posto dei file di configurazione.
- **Prudente di serie.** Faceva una prova a vuoto prima di ogni sincronizzazione, così vedevi cosa sarebbe cambiato prima che cambiasse.
- **Filtri e giochi personalizzati.** Potevi sincronizzare solo certi file, lasciare fuori le impostazioni grafiche e definire i giochi da solo, anche tra negozi diversi.
- **Ogni desktop.** Windows, macOS e Linux, Steam Deck compreso.

## Cosa serve a un sostituto

- **Qualcuno che lo mantenga.** Le posizioni dei salvataggi si spostano e i launcher cambiano; uno strumento di sincronizzazione che nessuno aggiorna finisce per sincronizzare la cartella sbagliata.
- **Una cronologia, non solo l'ultima copia.** Un cloud sincronizza file, quindi un salvataggio corrotto raggiunge ogni macchina fedelmente quanto uno buono.
- **Niente file scritti a metà.** Una sincronizzazione generica può prendere un salvataggio mentre il gioco lo sta ancora scrivendo; la [guida a OneDrive](/guides/onedrive-game-saves) mostra cosa rompe.

## Hoard come sostituto

| | OpenCloudSaves | Hoard |
|---|---|---|
| Mantenuto | Archiviato da gennaio 2025 | Sì, open source (AGPL-3.0) |
| Trovare i salvataggi | Definizioni dei salvataggi, più le cartelle che aggiungi | Automatico: database comunitario, Steam, Epic, GOG, prefissi Proton e Wine, 19 emulatori |
| Quando sincronizza | Quando parte una sincronizzazione, dopo una prova a vuoto | A ogni salvataggio, e ancora quando esci |
| Dove vivono i salvataggi | Il tuo cloud | Hoard One, o un tuo server su disco locale o qualsiasi storage compatibile S3 |
| Cronologia | Quello che tiene il cloud | Ogni versione, deduplicata |
| Dieci versioni di un salvataggio da 2 GB | Circa 20 GB di quota del cloud | Circa 2 GB, perché le versioni condividono ciò che non è cambiato |
| Piattaforme | Windows, macOS, Linux | Windows, Linux, macOS e SteamOS |

L'unica cosa che Hoard non fa è scrivere nel tuo Google Drive o OneDrive. Sincronizza invece attraverso un server, ed è questo a dargli la cronologia e a permettere che l'altra macchina sia spenta. Se volevi soprattutto uno storage che controlli tu, più che quel cloud in particolare, un Hoard self-hosted scrive su un NAS, un disco locale o qualsiasi bucket compatibile S3 come Backblaze B2. Se deve essere proprio quel cloud, Ludusavi con un remoto Rclone è l'opzione mantenuta più vicina.

## Come passare a Hoard

1. Prima di toccare qualsiasi cosa, copia le cartelle dei salvataggi in un posto sicuro.
2. Smetti di sincronizzare quelle cartelle con OpenCloudSaves, così due strumenti non scrivono mai sullo stesso salvataggio.
3. Installa Hoard e accedi a Hoard One, consigliato a tutti, oppure puntalo sul [tuo server](/guides/self-host-hoard).
4. Apri la **Libreria**. Hoard trova i tuoi giochi; aggiungi a mano qualsiasi cartella personalizzata. La prima versione compare nella cronologia appena il gioco salva.
5. Installa Hoard sulle altre macchine con lo stesso account.

## Hoard One, o il tuo server

Hoard è gratuito in entrambi i casi, e l'app è la stessa:

- **Hoard One**, consigliato per tutti. Niente da configurare né da tenere acceso: un server gestito nell'UE, gratuito con 2 GB e tre dispositivi, senza carta. [Pro](/pricing) aggiunge solo spazio, 100 GB e dispositivi illimitati, per quando i tuoi salvataggi non ci stanno più.
- **Il tuo server.** Fai girare `hoard-server` su un PC, un NAS o un Raspberry Pi e tutte le tue macchine si sincronizzano attraverso quel server. Nessun account da noi, nessuna telemetria verso di noi e nessuna quota nostra: niente passa dai nostri server, perché sul percorso non c'è niente di nostro. Vedi [come fare il self-hosting di Hoard](/guides/self-host-hoard).

Stesso programma, stesso rilevamento, stessa cronologia. Hoard One conserva ogni versione per sempre; sul tuo server decidi tu quanta cronologia tenere.

<!-- faq -->

## Domande frequenti

### OpenCloudSaves è ancora sicuro da usare?

L'ultima versione è ancora disponibile, ma il progetto è archiviato: nessuna correzione per giochi nuovi o cambi del sistema. È anche uscito come beta, e il suo autore chiedeva di fare un backup dei salvataggi prima di usarlo. Se resti su di lui, tieni una copia manuale.

### Hoard può salvare i salvataggi su Google Drive o OneDrive?

No. Hoard sincronizza attraverso un server: Hoard One, o il tuo su disco locale o storage compatibile S3. La tua cartella `Documenti` può restare in OneDrive; Hoard chiede a Windows dove si trova davvero e lì trova i salvataggi.

### Hoard è gratuito?

Sì. L'app, la riga di comando e il server sono gratuiti e open source (AGPL-3.0), e far girare il tuo server non costa altro che la macchina. Hoard One, il server gestito che consigliamo a tutti, è gratuito anche lui con 2 GB e tre dispositivi; Pro aggiunge spazio per 1,99 € al mese.

### Il self-hosting invia qualcosa a Hoard?

No. Sul tuo server non c'è nessun account da noi e nessuna telemetria verso di noi: i tuoi salvataggi, i tuoi utenti e i tuoi log restano sulla tua macchina e non toccano mai i nostri.
