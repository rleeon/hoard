---
title: "Syncthing per i salvataggi: cosa funziona e cosa si rompe"
description: "Syncthing è un ottimo strumento di sincronizzazione generico, ma i salvataggi ne infrangono tre presupposti. Cosa va storto, come ci si arrangia, e quando conviene uno strumento che sa cos'è un salvataggio."
order: 9
updated: 2026-09-01
---

Syncthing è la risposta a cui molti arrivano per primi, e per buone ragioni: è gratuito, open source, peer-to-peer e funziona. Ma i salvataggi infrangono tre presupposti su cui si regge un sincronizzatore generico, e i guasti sono silenziosi. Questa guida parla di cosa va storto davvero, e di quando vale la pena usare qualcosa che sappia cos'è un salvataggio.

## Perché ci si finisce

È software genuinamente buono. Nessun account, nessun abbonamento, i tuoi file non stanno mai sul disco di un'azienda, e sincronizza qualsiasi cosa: documenti, foto, una cartella di salvataggi. Se già lo usi per altro, puntarlo a una cartella di salvataggi ti costa trenta secondi. È un argomento vero, e per certi setup è quello giusto.

## Le tre cose che si rompono

**Sincronizza mentre il gioco è aperto.** Syncthing reagisce al cambiamento di un file, che è il comportamento corretto per un documento. Un gioco scrive il salvataggio a metà sessione, a volte in più passaggi, e un file colto durante la scrittura è un file che si propaga a metà. L'altra macchina si ritrova un salvataggio che il gioco può rifiutarsi di caricare.

**I conflitti diventano file, non decisioni.** Quando entrambe le macchine cambiano lo stesso salvataggio, Syncthing fa la cosa sicura e li tiene entrambi, rinominandone uno in `qualcosa.sync-conflict-20260901-143022-ABCDEFG.sav`. Non si perde nulla, ma il gioco non sa cosa sia quel file, e tu finisci a confrontare date in un gestore file per decidere quale pomeriggio di gioco tenere. Ripetilo qualche volta e la cartella si riempie di file di conflitto che nessuno osa cancellare.

**Il versionamento è per file, non per sessione.** Syncthing può conservare copie vecchie in `.stversions`, ed è meglio di niente. Ma un salvataggio è spesso fatto di più file che hanno senso solo insieme, e ripristinare significa trovare a mano la data giusta per ciascuno. Non esiste un "rimetti questo gioco com'era martedì".

E un quarto punto, specifico di Steam: se lo punti a `userdata/<UserID>/<AppID>/` invece che alla cartella `remote/` al suo interno, stai sincronizzando anche `remotecache.vdf` e i file di obiettivi e tempo di gioco che **devono** essere diversi tra le macchine. A quel punto ogni avvio sembra un conflitto anche se nessun salvataggio si è mosso. È il motivo più comune per cui un setup artigianale tra Steam Deck e desktop sembra rotto.

## Cosa finisci per costruire

Niente di tutto ciò è irrisolvibile. Ci si arrangia con pattern di esclusione per gioco, una politica di versionamento e l'abitudine di chiudere il gioco e aspettare prima di toccare l'altro PC. Funziona, ed è manutenzione che ti porti dietro per sempre: un gioco nuovo sono percorsi nuovi, e il giorno in cui dimentichi di aspettare è il giorno in cui lo scopri.

## Cosa fa invece uno strumento che conosce i salvataggi

Hoard cattura **dopo che hai smesso di giocare**, quando la cartella si è calmata, quindi uno snapshot non è mai un file scritto a metà. Ogni cattura è una versione dell'intero salvataggio, non dei singoli file, quindi ripristinare è un clic e rimette tutto insieme. Sa quale cartella appartiene a quale gioco — legge lo stesso manifest comunitario delle posizioni condiviso dall'ecosistema open source, oltre 20.000 titoli — quindi non ci sono percorsi da mantenere, e traccia `<AppID>/remote/` invece della cartella superiore.

## Quando Syncthing è la risposta migliore

Per essere onesti:

- **Lo hai già in funzione**, e aggiungere una cartella è gratis.
- **Vuoi peer-to-peer senza alcun server**, nemmeno il tuo.
- **Sincronizzi molto più dei salvataggi** e preferisci un solo strumento per tutto.
- **Non torni mai indietro.** Se l'ultimo salvataggio è tutto ciò che ti è servito, una cronologia è macchinario che non userai.

## Usarli entrambi

Convivono senza litigare, ed è un setup ragionevole: il sincronizzatore generico si occupa dei documenti e del resto, uno strumento che conosce i salvataggi si occupa delle cartelle di salvataggio. L'unica regola è non puntarli entrambi alla stessa cartella: due programmi che scrivono gli stessi file sono il modo di fabbricare proprio i conflitti che volevi evitare.

## Nemmeno dai nostri server

Se parte dell'attrattiva è che nulla tocchi il disco di un'azienda, Hoard si può usare allo stesso modo: `hoard-server` sul tuo PC o NAS, e i salvataggi vanno dalla tua macchina al tuo disco. **Nessun account con noi, nessuna telemetria verso di noi e nessun relay**: non passa nulla dai nostri server, perché sul percorso non c'è niente di nostro. Vedi [come ospitare Hoard da solo](/guides/self-host-hoard).

Stesso binario, stesso rilevamento, stessa cronologia. L'unica cosa che cambia è di chi è lo spazio di archiviazione. C'è anche un [confronto completo di tutti gli strumenti di sincronizzazione](/guides/game-save-sync-comparison).

<!-- faq -->

## Domande frequenti

### Syncthing può sincronizzare i salvataggi?

Sì, e nei casi semplici lo fa bene. I problemi iniziano con i giochi che scrivono mentre giochi, con i salvataggi fatti di più file e con qualsiasi situazione in cui entrambe le macchine vengano modificate tra una sincronizzazione e l'altra.

### Cosa sono i file .sync-conflict nella mia cartella dei salvataggi?

È il sincronizzatore che dopo un conflitto tiene entrambe le versioni invece di sceglierne una. Non si perde nulla, ma il gioco non sa leggerli, e decidere quale tenere è lavoro manuale ogni volta.

### Perché il mio salvataggio Steam va in conflitto a ogni avvio?

Quasi sempre perché la cartella sincronizzata è quella sopra `remote/`. Contiene `remotecache.vdf` e file di obiettivi e tempo di gioco che sono legittimamente diversi su ogni macchina, quindi i due capi non andranno mai d'accordo.

### Devo chiudere il gioco prima di sincronizzare?

Con un sincronizzatore generico sì: è l'abitudine che evita i salvataggi scritti a metà. Uno strumento che conosce i salvataggi aspetta da solo che la cartella si calmi.

### Posso continuare a usarli insieme?

Sì. Solo, non puntarli entrambi alla stessa cartella, o si contenderanno gli stessi file.
