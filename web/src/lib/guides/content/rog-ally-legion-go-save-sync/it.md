---
title: "Sincronizzare i salvataggi tra ROG Ally, Legion Go, MSI Claw e il tuo PC"
description: "Le console portatili con Windows come ROG Ally, Legion Go e MSI Claw sono PC. Sincronizza i loro salvataggi con il desktop in automatico, con cronologia."
order: 19
updated: 2026-10-09
---

ROG Ally, Legion Go e MSI Claw funzionano con Windows, quindi per un gioco sono solo un altro PC. Ed è proprio questo il problema: il desktop e la portatile tengono ognuno i propri salvataggi. Steam Cloud copre parte della tua libreria e il cloud di Xbox copre il Game Pass, ma tutto il resto resta sulla macchina dove hai giocato. Hoard tiene i salvataggi sincronizzati tra la portatile e il desktop in automatico: smetti di giocare su una e il gioco ti aspetta sull'altra, con ogni versione precedente conservata.

## Cosa ti segue già

- **I giochi Steam con Steam Cloud** si sincronizzano da soli.
- **I giochi Game Pass e dell'app Xbox** usano il cloud di Xbox, purché giochi la versione Xbox su entrambe le macchine.
- **Epic, GOG, Ubisoft ed EA** hanno salvataggi cloud per alcuni dei loro giochi, dentro i loro launcher. Vedi [i salvataggi cloud di Epic e GOG](/guides/epic-gog-cloud-saves).

Quello che resta fuori: giochi in cui lo sviluppatore non ha mai attivato il cloud, emulatori, giochi installati a mano e qualsiasi gioco per cui desktop e portatile non usano lo stesso launcher.

## Configurazione

1. **Sulla portatile**, passa al desktop di Windows, apri la [pagina di download](/download) e installa Hoard per Windows.
2. **Accedi** con l'account che usi sul desktop, oppure punta l'app al tuo server.
3. Apri la **Libreria** e controlla cosa ha trovato Hoard. Aggiungi quello che manca indicando la sua cartella, per esempio un emulatore.
4. **Sul desktop**, installa Hoard con lo stesso account. Gli stessi giochi si abbinano da soli.

Il motore di sincronizzazione è un servizio in background che parte con Windows, quindi continua a funzionare mentre sei in Armoury Crate, Legion Space, MSI Center M o nella modalità Big Picture di Steam. Non serve aprire la finestra di Hoard per giocare.

## Le trappole delle portatili

### Sospendere non è chiudere

Su una portatile è facilissimo premere il tasto di accensione e metterla via con il gioco aperto. Hoard salva solo quando il gioco è chiuso, perché un gioco in esecuzione potrebbe essere a metà della scrittura, e non sostituisce mai il salvataggio di un gioco aperto. Se sospendi la portatile e poi giochi sul desktop, i progressi della portatile non sono ancora stati caricati. **Chiudi il gioco prima di cambiare macchina.**

### Giochi sulla microSD

Installare i giochi sulla scheda è normale su una portatile, e per i salvataggi conta di rado: la maggior parte dei giochi salva nella tua cartella utente sul disco interno, ovunque siano installati. L'eccezione sono i giochi che salvano accanto alla loro cartella di installazione; se uno di questi non viene rilevato, aggiungi la sua cartella a mano.

### Schermo e impostazioni

La tua portatile gira a una risoluzione più bassa e con una GPU più piccola del desktop. Hoard salva i file di impostazioni come `graphics.ini` insieme alla partita, ma non li scrive sopra quelli dell'altra macchina, quindi ognuna tiene impostazioni adatte a lei. Se vuoi comunque copiarli, c'è un'opzione apposta al momento del ripristino.

### Stesso gioco, negozio diverso

Un gioco comprato su Steam per il desktop e giocato con il Game Pass sulla portatile sono due installazioni diverse, e la versione Xbox salva in un formato che solo l'app Xbox capisce. Per condividere un salvataggio, gioca la versione dello stesso negozio su entrambe.

### SteamOS o Bazzite invece di Windows?

Allora la tua portatile è una macchina Linux e i salvataggi vivono nei prefissi Proton, esattamente come su uno Steam Deck. Vedi [sincronizzare i salvataggi tra Steam Deck e PC](/guides/sync-saves-steam-deck-pc).

## Senza i nostri server

Se preferisci tenere i salvataggi a casa, avvia `hoard-server` sul tuo PC o su un NAS e punta lì entrambe le macchine. Nessun account con noi, nessuna telemetria verso di noi, niente che passi dai nostri server. Vedi [come ospitare Hoard da solo](/guides/self-host-hoard).

<!-- faq -->

## Domande frequenti

### La ROG Ally ha i salvataggi nel cloud?

Ha quello che ha ogni launcher: Steam Cloud, il cloud di Xbox e quelli di Epic, GOG, Ubisoft o EA per i giochi che lo supportano. Non c'è una sincronizzazione dei salvataggi per tutto il sistema. Hoard ne aggiunge una per i giochi che quelli lasciano fuori.

### Hoard funziona con Armoury Crate o Legion Space?

Sì. Il motore di sincronizzazione di Hoard è un servizio in background di Windows, indipendente dal launcher con cui avvii i giochi.

### La portatile conta come dispositivo?

Sì. Il piano gratuito include tre dispositivi, quindi ci stanno un desktop, un portatile e una console portatile. Pro e i server ospitati in proprio non hanno limiti di dispositivi.

### E i salvataggi del Game Pass?

Lasciali al cloud di Xbox, che li sincronizza tra le installazioni dell'app Xbox delle due macchine. Hoard copre i giochi che non hanno un cloud proprio.

### Posso sincronizzare la portatile anche con uno Steam Deck?

Sì. Hoard funziona su entrambi e abbina ogni gioco tra Windows e i prefissi Proton del Deck.
