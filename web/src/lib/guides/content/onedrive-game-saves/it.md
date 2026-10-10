---
title: "OneDrive e i salvataggi dei giochi: cosa si rompe e come rimediare"
description: "OneDrive ha spostato i Documenti e ora i salvataggi falliscono, spariscono o compaiono doppi. Perché succede, come rimediare e come sincronizzarli meglio."
order: 15
updated: 2026-10-09
---

Su molti PC Windows, OneDrive fa il backup della cartella `Documenti`, spesso attivato durante la configurazione senza che nessuno se ne accorga. I giochi che salvano in `Documenti`, cioè quasi tutto `My Games`, la seguono dentro OneDrive. E lì cominciano i guai: salvataggi che non vengono scritti, salvataggi da scaricare prima di poterli caricare, e copie con il nome del tuo PC che il gioco non legge mai. Ecco perché succede e come rimediare.

## Come sono finiti i tuoi salvataggi in OneDrive

Il backup delle cartelle di OneDrive sposta `Documenti`, `Desktop` e `Immagini` in `C:\Users\<tu>\OneDrive\...`. I giochi chiedono a Windows dove si trova `Documenti`, quindi la seguono in silenzio. Per controllare, fai clic destro su `Documenti` e apri **Proprietà → Percorso**: se il percorso contiene `OneDrive`, i tuoi salvataggi sono lì dentro.

`AppData` e `Saved Games` non fanno parte di quel backup, quindi i giochi che salvano lì non ne risentono.

## Cosa va storto

- **Scritture che si scontrano.** OneDrive carica i file appena cambiano. Un gioco che scrive il salvataggio nello stesso istante può trovare il file in uso.
- **Salvataggi solo online.** OneDrive può liberare spazio tenendo i file solo nel cloud (l'icona della nuvola). Il gioco deve allora scaricare il salvataggio prima di caricarlo, e offline non c'è niente da caricare.
- **Copie in conflitto.** Usa OneDrive su due PC con lo stesso account ed entrambi sincronizzano lo stesso `My Games`. Gioca su tutti e due prima che uno si sia messo in pari e OneDrive tiene entrambe le versioni rinominandone una con il nome del PC. Il gioco ignora quel file.
- **Spazio.** Il piano gratuito ha 5 GB, e alcuni giochi mettono in `Documenti` anche mod, cache o registrazioni.
- **Nessuna idea di sessione di gioco.** OneDrive sincronizza file per file, a partita in corso, e versiona ogni file per conto suo invece del salvataggio intero.

## Le soluzioni

### Rapida: tieni i salvataggi sul dispositivo

Fai clic destro su `Documenti\My Games` (o sulla cartella del gioco) e scegli **Mantieni sempre su questo dispositivo**. Questo risolve il problema dei file solo online. Non impedisce a OneDrive di sincronizzare mentre giochi.

### Pulita: smetti di fare il backup di Documenti

In OneDrive, apri **Impostazioni → Sincronizzazione e backup → Gestisci backup** e disattiva `Documenti`. Windows torna a puntare `Documenti` sulla cartella locale, ma i file già salvati restano nella cartella di OneDrive. Prima di disattivare, segnali come **Mantieni sempre su questo dispositivo** perché siano davvero sul disco. Poi, a giochi chiusi, sposta le cartelle dei giochi nella `Documenti` locale, altrimenti i giochi ripartiranno da zero.

## Una divisione migliore

OneDrive è bravo con i documenti. I salvataggi hanno bisogno d'altro: un backup fatto quando il gioco è chiuso, versioni dell'intero salvataggio invece di file singoli, e la sincronizzazione con gli altri PC e con uno Steam Deck.

È quello che fa Hoard. Trova i tuoi salvataggi che `Documenti` sia in OneDrive o no, perché chiede a Windows dove si trova davvero la cartella. Li salva in automatico dopo ogni sessione, conserva ogni versione e li sincronizza con le altre macchine.

Una regola: la sincronizzazione tra PC deve gestirla un solo strumento. Se OneDrive fa il backup di `Documenti` su più PC da gioco con lo stesso account, sta già sincronizzando quei salvataggi tra di loro. Disattiva il backup di `Documenti` su quei PC e lascia i salvataggi a Hoard.

Preferisci niente cloud? Avvia `hoard-server` sul tuo PC o NAS: nessun account con noi, nessuna telemetria verso di noi, niente che passi dai nostri server. Vedi [come ospitare Hoard da solo](/guides/self-host-hoard).

<!-- faq -->

## Domande frequenti

### Devo lasciare che OneDrive faccia il backup dei miei salvataggi?

Su un solo PC, come backup, è meglio di niente. Come modo per sincronizzare i salvataggi tra PC crea conflitti, perché non sa quando un gioco è aperto.

### Ho disattivato il backup e i miei salvataggi sono spariti. Dove sono?

Ancora nella cartella di OneDrive: `C:\Users\<tu>\OneDrive\Documents\My Games`. Chiudi i giochi e riportali nella `Documenti` locale.

### Cosa sono i file di salvataggio con il nome del mio PC?

Copie in conflitto di OneDrive. Due PC hanno modificato lo stesso file prima di sincronizzarsi, e OneDrive li ha tenuti entrambi. Capisci qual è il più recente, dagli il nome originale e metti da parte l'altro.

### OneDrive può restituirmi un salvataggio precedente?

A volte. Su onedrive.com, fai clic destro sul file e scegli **Cronologia versioni**. Funziona file per file e solo per un tempo limitato. Vedi [come recuperare un salvataggio corrotto](/guides/recover-corrupted-game-save).

### Hoard funziona se la mia cartella Documenti è in OneDrive?

Sì. Hoard legge dove Windows dice che si trova `Documenti`, quindi trova i salvataggi in entrambi i casi.
