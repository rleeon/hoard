---
title: "Come trasferire i salvataggi dei giochi su un nuovo PC"
description: "PC nuovo o reinstalli Windows? Porta con te ogni salvataggio: cosa copre Steam Cloud, il metodo manuale, quello automatico e le trappole."
order: 13
updated: 2026-10-09
---

I giochi con Steam Cloud tornano da soli quando accedi sul nuovo PC. Tutto il resto è una cartella che devi portarti tu: giochi senza salvataggi cloud, emulatori, qualsiasi cosa fuori da Steam. Puoi copiare quelle cartelle a mano, oppure lasciare che Hoard le salvi sul vecchio PC e rimetta ognuna al suo posto sul nuovo. Ecco entrambi i metodi, e le trappole che fanno perdere i salvataggi alla gente.

## Prima di cancellare qualsiasi cosa

- **Fai una lista di quello a cui giochi**, compresi i giochi che non tocchi da mesi. Sono quelli che si dimenticano.
- **Controlla quali giochi hanno salvataggi cloud.** Su Steam lo dice la pagina del negozio, e **Proprietà → Generale** mostra se è attivo. Anche Epic e GOG lo indicano gioco per gioco.
- **Fai il backup del resto, e possibilmente di tutto.** I salvataggi cloud tengono una sola copia, l'ultima. Se è rovinata, è rovinata ovunque.

## Il metodo manuale

1. **Trova la cartella di ogni gioco.** La maggior parte sta in `Documents\My Games`, `Saved Games` o nelle cartelle `AppData` (`Roaming`, `Local`, `LocalLow`). L'elenco completo è in [dove i giochi per PC tengono i salvataggi](/guides/where-are-pc-game-saves-stored).
2. **Copiale su un disco esterno**, mantenendo la struttura delle cartelle. Prendi anche l'intera cartella `userdata` di Steam: pesa poco e copre i giochi che salvano tramite Steam senza avere Steam Cloud attivo.
3. **Sul nuovo PC, installa prima il gioco.** Se il gioco deve creare le sue cartelle, avvialo una volta ed esci dal menu principale. Non iniziare una nuova partita.
4. **Copia i salvataggi al loro posto** e avvia il gioco. Controlla che i progressi ci siano prima di cancellare qualcosa dal vecchio disco.

Funziona. Il difetto è che è una copia una tantum: devi ricordarti ogni cartella, e se continui a giocare sul vecchio PC, da quel giorno i due si separano.

## Le trappole

- **Salvataggi legati a un account.** Alcuni giochi mettono l'ID del tuo account nel nome della cartella o dentro il salvataggio: Elden Ring archivia i salvataggi sotto il tuo SteamID, i giochi Ubisoft sotto il tuo ID Ubisoft. Stesso account su entrambi i PC: nessun problema. Un altro account: il gioco vede uno slot vuoto.
- **OneDrive ha spostato Documenti.** Se un PC fa il backup di `Documenti` con OneDrive e l'altro no, la "stessa" cartella sta in due posti diversi. Fai clic destro su `Documenti` e apri **Proprietà → Percorso** per vedere dove si trova davvero. Altro in [OneDrive e i salvataggi dei giochi](/guides/onedrive-game-saves).
- **Versioni del gioco.** Un salvataggio di una versione più recente potrebbe non caricarsi in una più vecchia. Aggiorna il gioco sul nuovo PC prima di copiare.
- **Mod.** Un salvataggio con mod (soprattutto nei giochi Bethesda) può rifiutarsi di caricare senza le stesse mod. Reinstallale prima.
- **Da Windows a Steam Deck o Linux.** Il salvataggio va dentro il prefisso Proton del gioco, che esiste solo dopo il primo avvio. Vedi [sincronizzare i salvataggi tra Steam Deck e PC](/guides/sync-saves-steam-deck-pc).

## Il metodo automatico

Hoard trasforma il trasloco in quello che fa ogni giorno: salvare su una macchina, ripristinare su un'altra.

1. **Sul vecchio PC**, installa Hoard e accedi. Apri la **Libreria**: Hoard elenca i salvataggi che ha trovato per i tuoi giochi, con lo stesso database della community di Ludusavi. Aggiungi quello che manca indicando la sua cartella.
2. **Controlla che ogni gioco abbia una versione** nella sua **Cronologia**. È la tua rete di sicurezza prima di cancellare il vecchio disco.
3. **Sul nuovo PC**, installa Hoard, accedi con lo stesso account e installa i tuoi giochi. Hoard li abbina ai loro backup gioco per gioco e ripristina l'ultima versione nella cartella che questa macchina si aspetta, anche se il percorso cambia (un altro disco, un altro nome utente, un prefisso Proton su un Deck).
4. **Prima di iniziare una nuova partita**, lascia che Hoard finisca di rimettere a posto i salvataggi. L'app mostra lo stato di ogni gioco.

Due dettagli lo rendono più sicuro di una copia. I file di impostazioni come `graphics.ini` vengono salvati ma non scritti sopra quelli del nuovo PC, quindi il nuovo hardware parte con impostazioni adatte a lui (puoi portarle con te al ripristino, se le due macchine si somigliano). E niente è definitivo: ogni versione resta nella cronologia, quindi un ripristino sbagliato si annulla ripristinando quella prima.

Se il vecchio PC resta in uso, continua semplicemente a sincronizzarsi con il nuovo. Se se ne va per sempre, toglilo dai tuoi dispositivi. Il piano gratuito ne include tre.

## Senza i nostri server

Puoi fare tutto questo con il tuo server: avvia `hoard-server` su un PC o un NAS, punta entrambe le macchine lì e i salvataggi non escono mai di casa. Nessun account con noi, nessuna telemetria verso di noi. Vedi [come ospitare Hoard da solo](/guides/self-host-hoard).

<!-- faq -->

## Domande frequenti

### I salvataggi di Steam si trasferiscono da soli?

Solo per i giochi con Steam Cloud. Accedi sul nuovo PC, installa il gioco e il salvataggio si scarica. I giochi senza richiedono di copiare la loro cartella, o uno strumento che lo faccia per te.

### Posso semplicemente copiare tutta la mia cartella utente?

Funziona per la maggior parte dei salvataggi, ma si porta dietro anche gigabyte di cache, impostazioni pensate per il vecchio hardware e dati delle app che possono dare problemi su un'installazione nuova. Copiare solo le cartelle dei salvataggi è più pulito.

### I miei salvataggi funzioneranno con un altro nome utente di Windows?

Sì, quasi sempre. I salvataggi stanno dentro la tua cartella utente, quindi il nome nel percorso non conta. Hoard se ne occupa da solo.

### Posso spostare i salvataggi da Windows a uno Steam Deck?

Sì. Avvia il gioco una volta sul Deck perché esista il suo prefisso Proton, poi metti dentro il salvataggio, oppure lascia fare a Hoard. Vedi [la guida per Steam Deck](/guides/sync-saves-steam-deck-pc).

### Devo tenere il vecchio PC finché il nuovo non è pronto?

Con una copia manuale, tieni il disco esterno finché non hai controllato ogni gioco. Con Hoard, i salvataggi sono già sul server, quindi il vecchio PC può andarsene appena ogni gioco mostra una versione nella sua cronologia.
