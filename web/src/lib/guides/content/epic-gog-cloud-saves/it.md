---
title: "Salvataggi cloud di Epic e GOG: cosa coprono e come sincronizzare il resto"
description: "Il cloud di Epic e GOG funziona solo per alcuni giochi, solo nel loro launcher e senza cronologia. Cosa copre e come sincronizzare il resto in automatico."
order: 19.5
updated: 2026-10-09
---

Sia Epic sia GOG hanno i salvataggi nel cloud, con gli stessi limiti di Steam: lo sviluppatore deve supportarli gioco per gioco, funzionano solo tramite il launcher del negozio e tengono l'ultima copia invece di una cronologia. Ecco cosa copre ciascuno, dove sono i buchi e come tenere sincronizzati tutti i tuoi giochi tra i PC e uno Steam Deck, ovunque tu li abbia comprati.

## Epic Games Store

Il launcher di Epic ha un interruttore per i salvataggi cloud nelle impostazioni, e i giochi che li supportano si sincronizzano tramite esso quando giochi su un altro PC. Il supporto è gioco per gioco: lo sviluppatore deve implementarlo, e molti giochi del negozio non l'hanno mai fatto.

Su Linux e sullo Steam Deck non c'è un launcher Epic ufficiale. Heroic può sincronizzare il cloud di Epic per i giochi che lo supportano, ma va attivato gioco per gioco.

## GOG

GOG Galaxy sincronizza i salvataggi cloud dei giochi che riportano «Cloud saves» tra le caratteristiche nella pagina del negozio. Due limiti sono tipici di GOG:

- **Solo tramite Galaxy.** Gli installer offline di GOG, la parte senza DRM che è il loro punto di forza, non hanno nessun cloud. Giochi con l'installer e i salvataggi restano su quel PC.
- **Gioco per gioco e piattaforma per piattaforma.** Un gioco si sincronizza solo tra le piattaforme previste dallo sviluppatore.

Come per Epic, Heroic può sincronizzare il cloud di GOG su Linux e sullo Steam Deck se lo attivi gioco per gioco.

## Ubisoft, EA e gli altri

Ubisoft Connect e l'app EA hanno salvataggi cloud per molti dei loro giochi, ciascuna solo nel proprio launcher. Per Amazon Games e i negozi più piccoli dipende dal gioco.

## Cosa non fa nessuno di loro

- **Cronologia.** Ogni launcher tiene il salvataggio attuale. Se un salvataggio si rompe e si sincronizza, quello buono sparisce ovunque.
- **Tra negozi.** Lo stesso gioco comprato su Steam per un PC e su GOG per un altro ha due cloud separati che non si parlano mai.
- **Tutto ciò che sta fuori dal launcher.** Emulatori, installer senza DRM, giochi installati a mano.
- **Giochi senza supporto.** Se lo sviluppatore non l'ha implementato, il launcher non può farci niente.

## Sincronizzare il resto

Hoard lavora per gioco, non per negozio. Trova la cartella di salvataggio di ogni gioco grazie a un database della community che copre migliaia di titoli, da dovunque arrivi il gioco, la salva in automatico quando smetti di giocare e la sincronizza con gli altri PC e lo Steam Deck, conservando ogni versione.

Questo copre i buchi qui sopra:

- **Qualsiasi launcher, o nessuno.** Epic, GOG, Galaxy o l'installer offline, Heroic su Linux, un gioco estratto in una cartella.
- **Tra negozi.** La maggior parte dei giochi salva nello stesso posto qualunque sia il negozio che li ha venduti, di solito in `AppData` o `Documents`, quindi un'installazione GOG su un PC e una Steam su un altro possono condividere un salvataggio. Alcuni aggiungono una cartella con l'ID dell'account o cambiano nome a seconda del negozio; controlla prima di contarci.
- **Una cronologia.** Ogni sessione è una versione a cui puoi tornare.

Dove il cloud di un launcher sincronizza già un gioco, lascialo fare. Hoard aggiunge la cronologia e sincronizza tutto il resto.

Se preferisci non usare il cloud di nessuno, avvia `hoard-server` sul tuo PC o NAS. Nessun account con noi, nessuna telemetria verso di noi, niente che passi dai nostri server. Vedi [come ospitare Hoard da solo](/guides/self-host-hoard).

<!-- faq -->

## Domande frequenti

### Epic ha i salvataggi nel cloud?

Sì, per i giochi il cui sviluppatore li ha implementati, tramite il launcher di Epic. Molti giochi del negozio non li supportano, e il launcher non tiene una cronologia dei salvataggi precedenti.

### GOG ha i salvataggi nel cloud?

Sì, tramite GOG Galaxy, per i giochi che lo indicano nella pagina del negozio. Gli installer offline non sincronizzano niente.

### Epic o GOG conservano le vecchie versioni dei miei salvataggi?

No. Entrambi tengono solo l'ultima copia. Per tornare a un salvataggio precedente serve un backup che conservi le versioni.

### Posso spostare un salvataggio dalla versione GOG a quella Steam?

Spesso sì: la maggior parte dei giochi salva nella stessa cartella qualunque sia il negozio. Alcuni aggiungono una cartella con l'ID dell'account o usano un altro nome di cartella, quindi controlla prima i percorsi.

### Gli installer offline di GOG sincronizzano i salvataggi?

Non tramite GOG, perché il suo cloud funziona solo in Galaxy. Hoard li sincronizza come qualsiasi altro gioco, perché segue la cartella di salvataggio e non il launcher.
