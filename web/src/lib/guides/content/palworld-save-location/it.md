---
title: "Dove sono i salvataggi di Palworld (PC e Steam Deck)"
description: "Dove Palworld tiene i mondi su PC e Steam Deck, a cosa serve ogni file, come funzionano i mondi in co-op e come fare il backup e sincronizzare i salvataggi."
order: 24
updated: 2026-10-09
---

Su PC (Steam), Palworld tiene i salvataggi in `%LOCALAPPDATA%\Pal\Saved\SaveGames\<il tuo ID Steam>`, con una cartella per mondo all'interno. È il percorso che Pocketpair indica nella sua FAQ ufficiale. Qui sotto trovi il percorso su Steam Deck, a cosa serve ogni file, cosa cambia in co-op e come tenere i tuoi mondi al sicuro e sincronizzati tra PC e Steam Deck.

## Dove Palworld tiene i salvataggi

- **Windows, versione Steam:** `%LOCALAPPDATA%\Pal\Saved\SaveGames\<il tuo ID Steam>\<ID del mondo>`
- **Steam Deck e Linux** (Proton): `~/.local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames/`

La cartella dell'ID Steam è un numero lungo legato al tuo account Steam. Al suo interno, ogni mondo che hai creato ha una propria cartella con un lungo nome esadecimale. Su Steam Deck il gioco gira con Proton, quindi i salvataggi stanno nel prefisso Proton che Steam tiene per lui; `1623730` è l'ID Steam del gioco.

La versione dell'app Xbox / Game Pass tiene i salvataggi in un'altra posizione, impacchettata, e non sono gli stessi file che copieresti tra installazioni Steam.

## Cosa c'è nella cartella di un mondo

- **`Level.sav`** è il mondo vero e proprio: la tua base, la mappa, i Pal piazzati lì.
- **`LevelMeta.sav`** contiene nome e riepilogo del mondo per il menu.
- **`Players\`** contiene un `.sav` per ogni giocatore che è stato in quel mondo.
- **`LocalData.sav`** e **`WorldOption.sav`** contengono dati locali e le impostazioni del mondo.
- **`backup\`** sono i backup automatici del mondo fatti dal gioco stesso.

Le impostazioni sono altrove: `Pal\Saved\Config\Windows\GameUserSettings.ini` per grafica e comandi, `Pal\Saved\Logs` per i log. Nessuno dei due fa parte dei tuoi progressi.

Per il backup prendi **l'intera cartella del mondo**, non solo `Level.sav`. Il mondo e i file dei giocatori vanno insieme, e ripristinarne uno senza l'altro è il modo in cui i personaggi finiscono fuori sincrono con il mondo in cui si trovano.

## Co-op e server dedicati

In co-op **il mondo vive sul PC dell'host**. Il tuo personaggio in quel mondo è un file nella cartella `Players` dell'host, non sulla tua macchina. Se l'host perde il salvataggio, i progressi di tutti in quel mondo se ne vanno con lui. Su un server dedicato, il mondo vive sul server.

Per un mondo condiviso, quindi, è la cartella dell'host a dover avere il backup.

## Palworld ha i salvataggi nel cloud?

La versione Steam usa Steam Cloud, che tiene allineato l'ultimo stato dei tuoi mondi tra le macchine con lo stesso account. Non tiene le versioni precedenti, e la cartella `backup\` del gioco sta sullo stesso disco del salvataggio: un disco morto si porta via entrambi.

## Backup a mano

1. Chiudi completamente il gioco.
2. Copia la cartella del tuo ID Steam da `SaveGames` (contiene tutti i tuoi mondi) in un posto sicuro.
3. Per ripristinare, chiudi il gioco e ricopiala nello stesso posto.

## Backup e sync automatici con Hoard

[Hoard](/download) fa il backup dei tuoi mondi ogni volta che smetti di giocare e tiene tutte le versioni fuori dalla macchina, così un mondo corrotto o un disco perso non sono la fine di una base costruita in settimane. Sincronizza anche i mondi tra i tuoi PC e una Steam Deck.

1. Installa Hoard e accedi, oppure puntalo verso [il tuo server](/guides/self-host-hoard).
2. Apri la **Libreria**. Palworld viene rilevato dalla tua libreria Steam e dal database comunitario dei salvataggi.
3. Gioca. Quando esci, la prima versione compare nella cronologia.

Se fai da host in co-op, è questa la macchina che conta: fai il backup dell'host e il mondo condiviso è coperto. Per riportare indietro un mondo, [ripristina una versione precedente](/guides/restore-a-game-save).

<!-- faq -->

## Domande frequenti

### Dove sono i salvataggi di Palworld su Steam Deck?

Nel prefisso Proton: `~/.local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames/`, nella cartella col tuo ID Steam.

### Dove viene salvato il mio personaggio nel mondo di un amico?

Sul PC dell'host, nella cartella `Players` di quel mondo. Il tuo PC non tiene una copia dei mondi ospitati da altri.

### Quale file è il mio mondo?

`Level.sav`, ma fai il backup dell'intera cartella del mondo: i file dei giocatori e il mondo vanno insieme.

### Palworld fa il backup del mio mondo da solo?

Tiene backup automatici nella cartella `backup` del mondo. Stanno sullo stesso disco del salvataggio, quindi proteggono da un salvataggio rovinato, non dalla perdita del disco.
