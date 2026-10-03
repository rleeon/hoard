---
title: "Dove sono i salvataggi di Marvel's Spider-Man 2 (PC e Steam Deck)"
description: "Dove Marvel's Spider-Man 2 tiene i salvataggi su PC, cos'è la cartella col numero lungo, la trappola di OneDrive, il percorso su Steam Deck e come farne il backup."
order: 22
updated: 2026-10-02
---

Su PC, Marvel's Spider-Man 2 tiene i salvataggi in `Documenti\Marvel's Spider-Man 2\`, dentro una sottocartella con un numero lungo. Su Steam, quel numero è il tuo ID Steam. Qui sotto trovi cosa significa in pratica, la trappola di OneDrive, il percorso su Steam Deck e come tenere i salvataggi al sicuro.

## Dove Marvel's Spider-Man 2 tiene i salvataggi

- **Windows:** `%USERPROFILE%\Documents\Marvel's Spider-Man 2\<numero lungo>`
- **Steam Deck e Linux** (Proton): `~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/<numero lungo>`

La versione PC è solo per Windows, quindi su Steam Deck gira con Proton e i salvataggi stanno nel prefisso Proton che Steam tiene per il gioco; `2651280` è il suo ID Steam. Se è installato sulla microSD, la cartella `compatdata` è sulla scheda.

Nixxes, lo studio dietro la conversione per PC, descrive la cartella dei salvataggi come "una sottocartella con un numero lungo o una combinazione di lettere e numeri" sotto `Documenti\Marvel's Spider-Man 2\`.

## La cartella col numero lungo

La sottocartella prende il nome dal tuo account: su Steam è il tuo **ID Steam a 64 bit**; la versione Epic usa invece un misto di lettere e numeri. In ogni caso è diversa per ogni account. Due conseguenze:

- Se due persone giocano sullo stesso PC con account Steam diversi, ognuna ha la propria cartella di salvataggio.
- Se copi i salvataggi a mano su un altro PC, mettili nella cartella dell'account Steam di **quella** macchina. In una cartella con un altro ID, il gioco non li vede.

La cartella madre `Marvel's Spider-Man 2` contiene anche il log del gioco e i crash dump (`.log`, `.mdmp`). Non sono salvataggi e non serve farne il backup.

## La trappola di OneDrive

Molti PC Windows reindirizzano `Documenti` su OneDrive. Se è il tuo caso, il percorso reale è `%USERPROFILE%\OneDrive\Documents\Marvel's Spider-Man 2\`, e OneDrive sincronizza la cartella da solo mentre giochi. Ne nascono due problemi: OneDrive può caricare un salvataggio scritto a metà, e "Libera spazio" può trasformare il salvataggio in un segnaposto solo online. Se qui ti affidi a OneDrive, imposta la cartella su **Mantieni sempre su questo dispositivo**.

## Spider-Man 2 ha i salvataggi nel cloud?

Sì, Steam Cloud, che tiene allineati gli ultimi salvataggi tra le macchine con lo stesso account Steam. Non tiene le versioni precedenti: se un salvataggio si rompe, si sincronizza quello rotto.

## Backup a mano

1. Chiudi completamente il gioco.
2. Copia la cartella `Marvel's Spider-Man 2` da `Documenti` in un posto sicuro.
3. Per ripristinare, chiudi il gioco e ricopia la cartella col numero lungo nello stesso posto, con lo stesso account Steam.

## Backup e sync automatici con Hoard

[Hoard](/download) fa il backup della cartella ogni volta che smetti di giocare e tiene tutte le versioni. La sincronizza anche tra i tuoi PC e una Steam Deck, così il gioco riprende da dove l'hai lasciato su entrambi.

1. Installa Hoard e accedi, oppure puntalo verso [il tuo server](/guides/self-host-hoard).
2. Apri la **Libreria** e controlla che la cartella mostrata per Spider-Man 2 sia quella sotto `Documenti` (o `OneDrive\Documents`). Se punta altrove, cambiala con quella cartella.
3. Gioca. Quando esci, la prima versione compare nella cronologia.

Se più avanti un salvataggio si rompe, [ripristinare una versione precedente](/guides/restore-a-game-save) lo rimette a posto.

<!-- faq -->

## Domande frequenti

### Cos'è il numero lungo nella cartella dei salvataggi?

Su Steam, il tuo ID Steam a 64 bit; su Epic, l'ID del tuo account. Ogni account ha la propria cartella, e il gioco legge solo quella dell'account connesso.

### Dove sono i salvataggi di Spider-Man 2 su Steam Deck?

Nel prefisso Proton: `~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/`, nella cartella col numero lungo.

### Non trovo la cartella in Documenti. Dov'è?

Guarda in `OneDrive\Documents\Marvel's Spider-Man 2`. Nella maggior parte delle installazioni recenti di Windows, Documenti sta dentro OneDrive.

### Posso copiare i miei salvataggi sul PC di un amico?

I file si copiano, ma vanno nella cartella con l'ID Steam dell'account di quel PC. Se il gioco accetti salvataggi creati con un altro account dipende dal gioco: tieni una copia dell'originale prima di provare.
