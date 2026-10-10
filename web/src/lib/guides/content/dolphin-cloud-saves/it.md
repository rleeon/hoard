---
title: "Salvataggi cloud per Dolphin: sincronizzare GameCube e Wii tra PC e Steam Deck"
description: "Dolphin non ha salvataggi cloud. Sincronizza i salvataggi GameCube e Wii tra PC e Steam Deck in automatico, con cronologia: percorsi, tipi di card e trappole."
order: 17
updated: 2026-10-09
---

Dolphin non sincronizza i salvataggi tra macchine: le tue memory card GameCube e la tua Wii emulata vivono in una cartella su un solo PC. Hoard li sincronizza in automatico. Quando chiudi Dolphin, salva i tuoi salvataggi GameCube e Wii, li scarica sugli altri PC e sullo Steam Deck e conserva ogni versione, così puoi sempre tornare indietro.

## Dove Dolphin tiene i tuoi salvataggi

Tutto sta nella cartella utente di Dolphin. Il modo più rapido per trovarla è **File → Open User Folder** dentro Dolphin. Lì dentro:

- `GC` contiene le memory card GameCube.
- `Wii` è la memoria interna della Wii emulata, salvataggi compresi.
- `StateSaves` contiene gli stati.

Dove si trova quella cartella:

- **Windows:** `Documents\Dolphin Emulator`. Le installazioni più recenti possono usare `%APPDATA%\Dolphin Emulator`, e un'installazione portatile tiene una cartella `User` accanto a `Dolphin.exe`.
- **Linux:** `~/.local/share/dolphin-emu`.
- **Steam Deck** (il Flatpak di Discover): `~/.var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu`.
- **Mac:** `~/Library/Application Support/Dolphin`.

Hoard trova da solo le cartelle di `Documents`, Linux e Steam Deck. Per `%APPDATA%`, un'installazione portatile o un Mac, indica una volta le cartelle `GC` e `Wii`.

## GameCube: file di card o cartelle GCI

In **Options → Configuration → GameCube**, ogni slot di memory card può essere una di due cose:

- **Un file di memory card**, un'immagine grezza come `MemoryCardA.USA.raw` con dentro i salvataggi di tutti i giochi. Ogni salvataggio riscrive l'intero file.
- **Una cartella GCI**, dove ogni salvataggio è un file `.gci` a sé, in una cartella come `GC/USA/Card A`. È nuovo solo il salvataggio che è cambiato, quindi le versioni restano leggere e facili da leggere.

Per sincronizzare vanno meglio le cartelle GCI. In ogni caso, **usa la stessa impostazione su ogni macchina**: un file di card su un PC e una cartella GCI sull'altro, e ognuno vede una card vuota. Se devi spostare salvataggi da un tipo all'altro, **Tools → Memory Card Manager** di Dolphin importa ed esporta file `.gci`.

Le card sono anche separate **per regione** (USA, EUR, JAP). Una copia PAL e una NTSC dello stesso gioco non vedono i salvataggi l'una dell'altra, quindi usa la stessa immagine del disco ovunque.

## Wii: la memoria della console emulata

I salvataggi Wii stanno dentro la cartella `Wii`, sotto `Wii/title/00010000/<ID del gioco>/data` per i giochi su disco. Quella cartella è tutta la memoria della console emulata: salvataggi, Mii, impostazioni di sistema e i canali che hai installato. Hoard la salva come un unico elemento, quindi ripristinare una versione rimette la memoria della console com'era in quel momento. Prima di confermare, Hoard mostra cosa cambierà, e i tuoi file attuali vengono copiati prima.

Se vuoi spostare a mano un solo salvataggio Wii, Dolphin può esportarlo: clic destro sul gioco nell'elenco e **Export Wii Save**.

## Come funziona la sincronizzazione ogni giorno

Giochi sul desktop e chiudi Dolphin. Hoard aspetta che Dolphin sia uscito e che le cartelle siano ferme, poi carica la nuova versione. Più tardi prendi lo Steam Deck; appena è online, Hoard scarica i salvataggi più recenti. Chiudi Dolphin sul Deck e succede la stessa cosa al contrario. Nessuna delle due macchine deve essere accesa insieme all'altra.

## Trappole da conoscere

- **Chiudi Dolphin, non solo il gioco.** Hoard salva quando l'emulatore è uscito. Su un Deck, sospendere non vale come chiudere.
- **Gli stati sono fragili.** Gli stati di Dolphin si rompono spesso tra una versione e l'altra. Hoard sincronizza i veri salvataggi; se vuoi anche `StateSaves`, aggiungila come elemento a parte e tieni Dolphin alla stessa versione ovunque.
- **Percorsi personalizzati.** Se hai cambiato la radice della NAND Wii o il percorso delle cartelle GCI in **Options → Configuration → Paths**, indica a Hoard quelle cartelle.

## Configurazione

1. Installa Hoard su ogni macchina e accedi con lo stesso account.
2. Nella **Libreria**, aggiungi Dolphin dall'elenco degli emulatori.
3. Usa la stessa impostazione delle card e la stessa regione su ogni macchina.
4. Gioca, chiudi Dolphin e continua sull'altra macchina.

Preferisci tenere tutto a casa? Avvia `hoard-server` sul tuo PC o NAS e punta lì tutte le macchine. Nessun account con noi, nessuna telemetria verso di noi, niente che passi dai nostri server. Vedi [come ospitare Hoard da solo](/guides/self-host-hoard). Per gli altri emulatori, vedi [salvataggi degli emulatori](/guides/back-up-emulator-saves).

<!-- faq -->

## Domande frequenti

### Dolphin ha i salvataggi nel cloud?

No. Dolphin tiene i salvataggi in una cartella locale e lascia a te la sincronizzazione. Hoard è un modo per sincronizzarli in automatico, con in più una cronologia delle versioni.

### Posso sincronizzare Dolphin tra un PC e uno Steam Deck?

Sì. Installa Hoard su entrambi con lo stesso account. Hoard sa dove Dolphin tiene i salvataggi su Windows, Linux e nel Flatpak dello Steam Deck, e li abbina tra le macchine.

### File di card o cartella GCI?

Per sincronizzare, la cartella GCI: ogni salvataggio è un file a sé, quindi le versioni sono leggere e mostrano quale gioco è cambiato. Qualunque scegli, usa la stessa su ogni macchina.

### Sincronizza anche i salvataggi Wii?

Sì. La cartella `Wii` contiene la memoria della console emulata, salvataggi compresi, e Hoard la salva e la sincronizza come le card GameCube.

### Hoard sincronizza gli stati di Dolphin?

Non di default, perché gli stati si rompono tra una versione di Dolphin e l'altra. Aggiungi a mano la cartella `StateSaves` se li vuoi.

### Funziona con Dolphin su Android?

Oggi no. Hoard funziona su Windows, macOS, Linux e Steam Deck.
