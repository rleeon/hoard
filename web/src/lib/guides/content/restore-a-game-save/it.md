---
title: "Come ripristinare un vecchio salvataggio"
description: "Scelta sbagliata, file corrotto o voglia di ricominciare? Torna a qualsiasi versione precedente del tuo salvataggio con la cronologia cloud di Hoard — inclusi salvataggi fatti con strumenti come Ludusavi."
order: 3
updated: 2026-09-01
---

Una brutta decisione nel gioco, un file corrotto o una mod che rompe tutto — a volte devi solo tornare indietro. Poiché Hoard conserva una cronologia completa delle versioni di ogni salvataggio, ripristinarne uno precedente richiede pochi secondi.

## Ripristinare una versione precedente

1. Apri **Hoard** e vai al gioco nella tua **Libreria**.
2. Apri la scheda **Cronologia**. Vedrai ogni backup con data e dimensione.
3. Scegli la versione che vuoi e premi **Ripristina**.
4. Hoard riscrive quello snapshot nella cartella di salvataggio del gioco. Il salvataggio attuale viene salvato prima, quindi il ripristino è reversibile.

## Ripristinare su un PC nuovo o reinstallato

1. Installa Hoard e accedi con il tuo account.
2. Aggiungi il gioco alla Libreria — Hoard trova il backup cloud corrispondente.
3. Ripristina l'ultima versione, o una più vecchia, e continua a giocare.

Poiché Hoard individua le cartelle di salvataggio con lo stesso database comunitario di Ludusavi, sa dove mettere un salvataggio ripristinato anche su un'installazione pulita — senza cercare percorsi a mano.

## Quando un salvataggio è corrotto o l'ha rotto una mod

Un gioco che crasha al caricamento, una mod che ha riscritto ciò che non doveva, un salvataggio automatico caduto a metà scrittura: il rimedio è lo stesso. Apri la **Cronologia** del gioco, scegli l'ultima versione precedente al problema e ripristinala. Date e dimensioni bastano di solito a individuare il momento in cui è andata storta: un calo improvviso di dimensione è un buon indizio di un salvataggio troncato.

Se non sai quale sia quella buona, ripristina la candidata più probabile e verifica nel gioco. Riprovare non costa nulla, perché anche la versione appena sostituita è stata conservata.

## Cosa fa davvero un ripristino

Tre cose da sapere, perché sono quelle che rendono sicuro provarci:

1. **Il salvataggio attuale viene catturato per primo.** Il ripristino è reversibile: ciò che hai sostituito diventa una versione della cronologia come tutte le altre.
2. **Si scarica solo ciò che manca.** I file già su disco con il contenuto giusto vengono usati così come sono, quindi ripristinare un salvataggio grande dopo una piccola modifica sposta qualche megabyte e non l'intera cartella.
3. **I file che appartengono a questa macchina restano intatti.** Configurazione e log accanto al salvataggio vengono salvati, ma non riscritti sopra le tue copie locali: i tuoi comandi e le tue impostazioni grafiche sopravvivono a un ripristino arrivato da un altro PC.

## Ripristinare senza passare dai nostri server

Se fai girare il tuo `hoard-server`, i ripristini funzionano esattamente allo stesso modo, solo che le versioni arrivano dalla tua macchina invece che dalla nostra. Nessun account con noi, nessuna telemetria verso di noi, niente che passi dai nostri server. Vedi [come ospitare Hoard da solo](/guides/self-host-hoard).

## Suggerimento

I ripristini non sono mai distruttivi: il salvataggio che sostituisci viene prima catturato come nuova versione, quindi puoi sempre annullare un ripristino ripristinando la voce precedente. Se finora hai tenuto solo backup locali (ad esempio con Ludusavi), passare a Hoard aggiunge una cronologia versionata fuori dalla macchina, da cui puoi ripristinare anche dopo un guasto del disco.

<!-- faq -->

## Domande frequenti

### Il ripristino sovrascrive i miei progressi attuali?

Solo dopo che il salvataggio attuale è stato catturato come nuova versione. Se ripristini quella sbagliata, ripristina la voce precedente e sei di nuovo al punto di partenza.

### Fin dove arriva la cronologia?

Fin dove lo consente il limite di versioni del tuo piano, e una versione che fissi non viene mai eliminata per fare spazio. Su un server self-hosted l'unico limite è il tuo disco.

### Posso ripristinare su un PC dove il gioco non è ancora installato?

Installa prima il gioco, così esiste la sua cartella dei salvataggi, poi ripristina. Hoard sa dove ogni gioco si aspetta i salvataggi e scrive lo snapshot nel posto giusto senza che tu debba cercare il percorso.

### Funziona tra Windows e una Steam Deck?

Sì. Lo stesso gioco tiene il salvataggio in posti diversi sui due — sulla Deck, dentro il prefisso Proton — e Hoard scrive la versione ripristinata dove quella macchina se l'aspetta.

### Il ripristino cambia su un server self-hosted?

No. Stessa app, stessa cronologia, stesso ripristino in un clic. L'unica cosa tua è lo spazio di archiviazione.
