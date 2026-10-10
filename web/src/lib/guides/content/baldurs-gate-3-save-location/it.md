---
title: "Dove sono i salvataggi di Baldur's Gate 3 (PC e Steam Deck)"
description: "Dove Baldur's Gate 3 tiene i salvataggi su Windows, Steam Deck e Mac, cosa è salvataggio e cosa sono mod o impostazioni, la modalità Onore, backup e sync."
order: 23
updated: 2026-10-09
---

Su Windows, Baldur's Gate 3 tiene i salvataggi in `%LOCALAPPDATA%\Larian Studios\Baldur's Gate 3\PlayerProfiles\Public\Savegames\Story`, una cartella per salvataggio. È il percorso che Larian indica nella propria FAQ di supporto. Qui sotto trovi i percorsi su Steam Deck e Mac, cosa c'è accanto ai salvataggi, la modalità Onore e come tenere tutto al sicuro e sincronizzato tra PC e Steam Deck.

## Dove Baldur's Gate 3 tiene i salvataggi

- **Windows:** `%LOCALAPPDATA%\Larian Studios\Baldur's Gate 3\PlayerProfiles\Public\Savegames\Story`
- **Steam Deck e Linux** (Proton): `~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story`
- **Mac:** `~/Documents/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story`

Larian ha ritirato la versione Linux nativa, quindi su Steam Deck il gioco gira con Proton e i salvataggi stanno nel prefisso Proton che Steam tiene per lui; `1086940` è l'ID Steam del gioco. Se è installato sulla microSD, la cartella `compatdata` è sulla scheda.

## Cosa è salvataggio e cosa no

Ogni salvataggio è **una cartella** dentro `Story`, con un file `.lsv` e una miniatura. Tutto quello che c'è intorno è altro:

- **`Mods`** (sotto `Baldur's Gate 3`) contiene i file delle mod.
- **`modsettings.lsx`** (sotto `PlayerProfiles\Public`) è l'elenco delle mod attive e il loro ordine di caricamento.
- **Le impostazioni**, come grafica e comandi, sono file di configurazione accanto al profilo, non parte di un salvataggio.

La trappola sono le mod. Un salvataggio creato con delle mod si aspetta le stesse mod attive al caricamento. Se sposti un salvataggio con mod su un altro PC, porta con te anche l'elenco delle mod, altrimenti il gioco segnala mod mancanti e il salvataggio potrebbe non caricarsi come previsto.

## La modalità Onore

La modalità Onore ha un unico salvataggio che il gioco sovrascrive mentre giochi, e se il tuo gruppo cade la partita in Onore è finita (puoi continuare in modalità Personalizzata, senza l'Onore). Fare il backup di quel salvataggio è una tua scelta: una copia prima di uno scontro difficile è tecnicamente una via di ritorno, e alcuni la vogliono proprio dopo un crash o un bug, mentre altri lo considerano barare. Uno strumento di backup tiene comunque le versioni; se ne ripristini mai una, è una questione tra te e i dadi.

## Baldur's Gate 3 ha i salvataggi nel cloud?

Sì. Su Steam usa Steam Cloud, che tiene allineati gli ultimi salvataggi tra le macchine con lo stesso account. Tiene solo lo stato attuale: se un salvataggio si corrompe o l'aggiornamento di una mod lo rompe, è quella la versione che si sincronizza.

## Backup a mano

1. Chiudi completamente il gioco.
2. Copia l'intera cartella `PlayerProfiles` dal percorso qui sopra (contiene `Savegames` e `modsettings.lsx`).
3. Per ripristinare, chiudi il gioco e ricopiala.

## Backup e sync automatici con Hoard

[Hoard](/download) fa il backup della cartella ogni volta che smetti di giocare e tiene tutte le versioni, così un salvataggio rotto dall'aggiornamento di una mod o da una patch è a un ripristino di distanza. Tiene anche la cartella sincronizzata tra i tuoi PC e una Steam Deck.

1. Installa Hoard e accedi, oppure puntalo verso [il tuo server](/guides/self-host-hoard).
2. Apri la **Libreria**. Baldur's Gate 3 viene rilevato dalla tua libreria Steam e dal database comunitario dei salvataggi.
3. Gioca. Quando esci, la prima versione compare nella cronologia.

Ogni sessione aggiunge una versione con tutti i salvataggi dentro. Per tornare indietro, apri la cronologia e [ripristina una versione precedente](/guides/restore-a-game-save); quello che hai ora sul PC viene salvato prima, quindi provare una versione vecchia non è mai un viaggio di sola andata.

<!-- faq -->

## Domande frequenti

### Dove sono i salvataggi di Baldur's Gate 3 su Steam Deck?

Nel prefisso Proton del gioco: `~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story`.

### Posso spostare un salvataggio con mod su un altro PC?

Sì, purché l'altro PC abbia le stesse mod installate e attive nello stesso ordine. Copia `modsettings.lsx` insieme al salvataggio e installa gli stessi file delle mod.

### Posso fare il backup di un salvataggio in modalità Onore?

Il salvataggio è una cartella normale, quindi sì, qualsiasi strumento di backup può copiarlo. Se ripristinarlo rispetti lo spirito della modalità, lo decidi tu.

### Perché il mio salvataggio dice che mancano delle mod?

È stato creato con mod che ora non sono attive. Riattiva le stesse mod, nello stesso ordine, e si caricherà normalmente.
