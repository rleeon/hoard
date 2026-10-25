---
title: "Dove sono i salvataggi di Elden Ring (PC e Steam Deck)"
description: "Dove Elden Ring tiene ER0000.sl2 su PC e Steam Deck, cosa contiene, i salvataggi di Seamless Co-op e Nightreign, e come salvare tutti i personaggi."
order: 26
updated: 2026-10-18
---

Su PC, Elden Ring tiene i salvataggi in `%APPDATA%\EldenRing\<il tuo ID Steam>`, in un unico file, `ER0000.sl2`, che contiene tutti i personaggi che hai creato, fino a dieci. Qui sotto trovi il percorso su Steam Deck, cos'altro c'è nella cartella, dove salvano Seamless Co-op e Nightreign e come tenere tutto al sicuro e sincronizzato tra PC e Steam Deck.

## Dove Elden Ring tiene i salvataggi

- **Windows (Steam):** `%APPDATA%\EldenRing\<il tuo ID Steam>\ER0000.sl2`
- **Steam Deck e Linux** (Proton): `~/.local/share/Steam/steamapps/compatdata/1245620/pfx/drive_c/users/steamuser/AppData/Roaming/EldenRing/<il tuo ID Steam>/`

`%APPDATA%` è `C:\Users\<il tuo utente>\AppData\Roaming`: incolla `%APPDATA%\EldenRing` nella barra degli indirizzi di Esplora file per andarci direttamente. La cartella dell'ID Steam è un numero lungo legato al tuo account Steam. Su Steam Deck il gioco gira con Proton, quindi il salvataggio sta nel prefisso Proton che Steam tiene per lui; `1245620` è l'ID Steam del gioco.

## Cosa c'è nella cartella dei salvataggi

- **`ER0000.sl2`** è il tuo progresso: tutti i personaggi, il loro equipaggiamento e dove si trovano nel mondo, Shadow of the Erdtree compreso.
- **`ER0000.sl2.bak`** è una copia di sicurezza che il gioco tiene accanto.
- **`ER0000.co2`**, se lo vedi, è della mod Seamless Co-op (più sotto).

Le impostazioni grafiche non sono in questa cartella: stanno un livello più su, in `%APPDATA%\EldenRing\GraphicsConfig.xml`.

Dal modo in cui funziona il file derivano due cose:

- **I personaggi non si salvano né si ripristinano uno alla volta.** Ripristinare un `ER0000.sl2` vecchio riporta tutti i personaggi a quel momento, non solo quello che volevi sistemare.
- **Il salvataggio è legato al tuo account Steam.** Il tuo ID Steam è scritto dentro il file, e il gioco non carica un salvataggio creato con un altro account. Spostare il tuo salvataggio tra i tuoi PC e una Steam Deck funziona senza problemi.

## Seamless Co-op e Nightreign

### Seamless Co-op

La mod Seamless Co-op non tocca `ER0000.sl2`: tiene un salvataggio tutto suo, `ER0000.co2`, nella stessa cartella, quindi i personaggi con e senza mod sono file separati. La community della mod sconsiglia di copiare un `.co2` sopra il salvataggio normale per giocare online.

### Elden Ring Nightreign

Nightreign è un gioco a parte, con la sua cartella: `%APPDATA%\Nightreign\<il tuo ID Steam>\NR0000.sl2` su Windows, e lo stesso percorso nel suo prefisso Proton su Steam Deck (`compatdata/2622380`). Non tocca mai i tuoi personaggi di Elden Ring.

## Elden Ring ha i salvataggi nel cloud?

Sì, con Steam Cloud, che tiene `ER0000.sl2` allineato tra le macchine dello stesso account. Conserva solo l'ultimo stato, senza versioni precedenti a cui tornare, e il `.bak` del gioco sta sullo stesso disco del salvataggio: un disco morto si porta via entrambi.

## Backup a mano

1. Chiudi completamente il gioco.
2. Copia la cartella del tuo ID Steam da `%APPDATA%\EldenRing` in un posto sicuro. Contiene `ER0000.sl2`, il suo `.bak` e l'eventuale salvataggio di Seamless Co-op.
3. Per ripristinare, chiudi il gioco e ricopia la cartella nello stesso posto.

## Backup e sync automatici con Hoard

[Hoard](/download) è gratuito e open source. Fa il backup dei salvataggi di Elden Ring ogni volta che il gioco ne scrive uno e di nuovo quando esci, tiene una cronologia a cui puoi tornare e sincronizza la cartella tra i tuoi PC e una Steam Deck. Viaggiano solo i file cambiati, compressi, e ogni file viene verificato con il suo hash SHA-256 quando torna.

1. Installa Hoard e accedi a Hoard One, consigliato per tutti, oppure puntalo verso [il tuo server](/guides/self-host-hoard).
2. Apri la **Libreria**. Elden Ring viene rilevato dalla tua libreria Steam e dal database comunitario dei salvataggi.
3. Gioca. Quando esci, la prima versione compare nella cronologia.

Hoard prende l'intera cartella dell'ID Steam, quindi anche un salvataggio di Seamless Co-op finisce in ogni versione. Ogni versione sono tutti i tuoi personaggi in quel momento: per tornare indietro, [ripristina una versione precedente](/guides/restore-a-game-save), e le versioni più recenti restano nella cronologia se cambi idea.

## Hoard One, o il tuo server

Hoard è gratuito in entrambi i casi, e l'app è la stessa:

- **Hoard One**, consigliato per tutti. Niente da configurare né da tenere acceso: un server gestito nell'UE, gratuito con 2 GB e tre dispositivi, senza carta. [Pro](/pricing) aggiunge solo spazio, 100 GB e dispositivi illimitati, per quando i tuoi salvataggi non ci stanno più.
- **Il tuo server.** Fai girare `hoard-server` su un PC, un NAS o un Raspberry Pi e tutte le tue macchine si sincronizzano attraverso quel server. Nessun account da noi, nessuna telemetria verso di noi e nessuna quota nostra: niente passa dai nostri server, perché sul percorso non c'è niente di nostro. Vedi [come fare il self-hosting di Hoard](/guides/self-host-hoard).

Stesso programma, stesso rilevamento, stessa cronologia. Hoard One conserva ogni versione per sempre; sul tuo server decidi tu quanta cronologia tenere.

<!-- faq -->

## Domande frequenti

### Posso usare il mio salvataggio di Elden Ring su un altro account Steam?

Così com'è, no. Il tuo ID Steam è scritto dentro `ER0000.sl2`, e il gioco non carica un salvataggio creato con un altro account. Il tuo salvataggio, sul tuo account, passa senza problemi tra i tuoi PC e una Steam Deck.

### Ripristinare un vecchio salvataggio riporta indietro tutti i miei personaggi?

Sì. Tutti i personaggi condividono `ER0000.sl2`, quindi ogni ripristino li riporta tutti a quel momento. Con Hoard le versioni più recenti restano nella cronologia, quindi tornare indietro non è mai un viaggio di sola andata.

### Hoard fa il backup dei salvataggi di Seamless Co-op?

Sì. La mod tiene `ER0000.co2` nella stessa cartella dell'ID Steam del salvataggio normale, e Hoard fa il backup dell'intera cartella: entrambi sono in ogni versione.

### Hoard è gratuito?

Sì. L'app, la riga di comando e il server sono gratuiti e open source (AGPL-3.0), e far girare il tuo server non costa altro che la macchina. Hoard One, il server gestito che consigliamo a tutti, è gratuito anche lui con 2 GB e tre dispositivi; Pro aggiunge spazio per 1,99 € al mese.

### Il self-hosting invia qualcosa a Hoard?

No. Sul tuo server non c'è nessun account da noi e nessuna telemetria verso di noi: i tuoi salvataggi, i tuoi utenti e i tuoi log restano sulla tua macchina e non toccano mai i nostri.
