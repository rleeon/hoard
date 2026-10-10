---
title: "Salvataggio corrotto? Come recuperarlo"
description: "Un salvataggio che non si carica non è sempre perso. Dove trovare una copia buona (backup del gioco, Steam Cloud, Windows, OneDrive) e come prevenire."
order: 12
updated: 2026-10-09
---

Un salvataggio che non si carica raramente è perso per sempre. Quasi sempre una copia buona esiste da qualche parte: un backup fatto dal gioco stesso, la copia di Steam Cloud, una versione precedente conservata da Windows o OneDrive, o il salvataggio su un altro PC. Ma l'ordine conta, perché una mossa sbagliata può sovrascrivere la copia buona con quella rotta. Comincia da qui.

## Prima di tutto: fermati e copia la cartella

1. **Chiudi il gioco** e non iniziare una nuova partita in quello slot. Ogni salvataggio da ora in poi può spingere fuori una copia più vecchia.
2. **Copia l'intera cartella dei salvataggi** sul desktop o su una chiavetta USB. Così tutto quello che proverai dopo si potrà annullare. Se non sai dov'è la cartella, vedi [dove i giochi per PC tengono i salvataggi](/guides/where-are-pc-game-saves-stored).
3. **Metti in pausa tutto ciò che sincronizza quella cartella.** Steam Cloud (gioco per gioco, in **Proprietà → Generale**), OneDrive, Syncthing. Altrimenti il file rovinato può arrivare proprio nell'unico posto che ha ancora una copia buona.

## Controlla che sia davvero corrotto

Alcune cose sembrano corruzione e non lo sono:

- **Il gioco è stato aggiornato** e i vecchi salvataggi non si caricano, o richiedono una patch. Guarda le notizie o il forum del gioco.
- **Mancano delle mod.** I giochi Bethesda in particolare segnalano i plugin mancanti e possono rifiutare un salvataggio che li usava. Reinstalla prima le mod.
- **Sei su un altro account.** Alcuni giochi archiviano i salvataggi sotto l'ID del tuo account Steam o Ubisoft, quindi un altro account vede uno slot vuoto.
- **Il file è solo online.** Con OneDrive, un salvataggio con l'icona della nuvola è stato tolto dal disco per liberare spazio. Fai clic destro e scegli **Mantieni sempre su questo dispositivo**.

Un salvataggio da **0 KB**, o molto più piccolo di quelli accanto, è davvero rotto: la scrittura si è interrotta a metà.

## Dove può trovarsi una copia buona

Procedi in ordine. Le prime strade sono più rapide e hanno più probabilità di funzionare.

### 1. I backup del gioco stesso

Molti giochi tengono una copia di riserva senza dirlo. Cerca nella cartella dei salvataggi file che finiscono in `.bak`, `_old` o `.backup`, e slot di salvataggio automatico in più. Alcuni casi noti:

- **Elden Ring** scrive `ER0000.sl2.bak` accanto al salvataggio.
- **Stardew Valley** tiene una copia `_old` di ogni fattoria, che è il giorno di gioco precedente.
- **Terraria** tiene file `.bak` per personaggi e mondi.
- **Minecraft Java** tiene `level.dat_old` dentro ogni mondo.

Per usarne uno, metti da parte il file rotto (hai già copiato la cartella) e rinomina il backup con il nome del file originale.

### 2. Steam Cloud

Steam Cloud tiene la copia **più recente**, non una cronologia. Aiuta solo se il salvataggio si è rotto dopo l'ultimo caricamento, per esempio perché il gioco è andato in crash e il file rovinato non è mai stato sincronizzato. Puoi vedere e scaricare quello che Steam conserva per ogni gioco su [store.steampowered.com/account/remotestorage](https://store.steampowered.com/account/remotestorage) e rimettere il file al suo posto a mano.

### 3. Versioni precedenti di Windows

Fai clic destro sulla cartella dei salvataggi e apri **Proprietà → Versioni precedenti**. Se Cronologia file o Protezione sistema erano attivi per quell'unità, qui compaiono copie più vecchie della cartella, e puoi aprirle per prendere i file che ti servono. Se l'elenco è vuoto, nessuno dei due era attivo.

### 4. La cronologia delle versioni di OneDrive

Se OneDrive fa il backup della tua cartella `Documenti`, molti salvataggi sono lì dentro senza che tu lo sappia. Su onedrive.com, fai clic destro sul file del salvataggio e scegli **Cronologia versioni** per scaricare una versione precedente. OneDrive le conserva per un tempo limitato, e anche i file eliminati restano per un po' nel suo cestino.

### 5. Le tue altre macchine

Hai giocato di recente su un portatile o su uno Steam Deck? La sua copia potrebbe essere precedente al problema. Recuperala prima che quella macchina sincronizzi quella rotta.

### 6. Se il file è stato cancellato, non rovinato

Controlla prima il Cestino. Dopo, un programma di recupero file può ritrovarlo, a patto che tu smetta di scrivere su quell'unità. Ogni installazione e ogni download riducono le probabilità.

## Quando non salta fuori niente

Per alcuni giochi famosi la community ha editor di salvataggi o strumenti di riparazione che possono ricostruire un file danneggiato: cerca il nome del gioco con "save repair". Altrimenti, la risposta onesta è che l'unica copia che conta è quella fatta prima del problema.

## Perché i salvataggi si rompono

- **Un crash o un calo di corrente durante la scrittura.** Il gioco stava salvando quando si è fermato.
- **Un disco pieno.** Il gioco non è riuscito a finire di scrivere e ha lasciato un file troncato.
- **Uno strumento di sincronizzazione l'ha copiato a metà scrittura**, o due PC hanno modificato lo stesso salvataggio e una copia ha prevalso.
- **Una mod** ha scritto qualcosa che il gioco non sa più rileggere.
- **Un disco che sta morendo**, cosa che di solito si nota anche su altri file.

## Non restare mai più a piedi

Tutti i recuperi qui sopra dipendono dalla fortuna: che il gioco avesse un backup, o che Steam non avesse ancora sincronizzato. Un backup con versioni toglie la fortuna dall'equazione. È quello che fa Hoard: salva ogni partita in automatico dopo che hai smesso di giocare, appena la cartella è ferma, quindi un backup non è mai un file scritto a metà. Ogni versione viene conservata. Quando qualcosa si rompe, apri la **Cronologia** del gioco e ripristini l'ultima buona con un clic; il salvataggio attuale viene copiato prima, quindi anche quello si può annullare. E poiché Hoard tiene anche i salvataggi sincronizzati, la copia sul portatile o sullo Steam Deck non è mai una vecchia copia dimenticata: tutte le tue macchine lavorano sulla stessa cronologia.

Un trucco per individuare il momento in cui si è rotto: un calo improvviso di dimensione tra due versioni di solito indica un salvataggio troncato. Altro in [come ripristinare un vecchio salvataggio](/guides/restore-a-game-save).

Se preferisci tenere i backup a casa, avvia `hoard-server` sul tuo PC o NAS. Nessun account con noi, nessuna telemetria verso di noi, niente che passi dai nostri server. Vedi [come ospitare Hoard da solo](/guides/self-host-hoard).

<!-- faq -->

## Domande frequenti

### Un salvataggio corrotto si può riparare?

Raramente sul posto. Alcuni giochi hanno strumenti di riparazione della community, ma quasi sempre recuperare significa trovare una copia più vecchia: il backup del gioco, Steam Cloud, Windows o OneDrive, o un altro PC.

### Steam Cloud conserva le vecchie versioni dei miei salvataggi?

No. Tiene solo il file attuale. Se un salvataggio rotto è già stato caricato, anche Steam Cloud ha quello rotto.

### Verificare i file del gioco ripara un salvataggio corrotto?

No. La verifica confronta i file del gioco con quelli di Steam, non i tuoi salvataggi. Aiuta se è danneggiato il gioco, ma non ti restituisce i progressi.

### Perché il mio salvataggio pesa 0 KB?

Il gioco ha iniziato a scriverlo e non ha mai finito: un crash, un calo di corrente o il disco pieno. Cerca accanto un file `.bak` o `_old`, o una versione precedente altrove.

### Come evito che succeda di nuovo?

Con backup a versioni fatti quando il gioco non è in esecuzione. Hoard lo fa in automatico dopo ogni sessione e conserva ogni versione, così puoi tornare a qualsiasi di esse.
