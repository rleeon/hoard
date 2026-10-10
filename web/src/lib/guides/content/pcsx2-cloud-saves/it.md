---
title: "Salvataggi cloud per PCSX2: sincronizzare le memory card PS2 tra PC e Steam Deck"
description: "PCSX2 non ha salvataggi cloud. Sincronizza le memory card PS2 tra PC e Steam Deck in automatico, con cronologia: percorsi, card a cartella e trappole."
order: 16
updated: 2026-10-09
---

PCSX2 non sincronizza i salvataggi da solo: i tuoi progressi PS2 vivono in file di memory card su una sola macchina, e l'altra non ne sa mai niente. Hoard li sincronizza in automatico. Quando chiudi PCSX2, salva le tue memory card, le scarica sugli altri PC e sullo Steam Deck e conserva ogni versione, così un salvataggio sbagliato non ti costa mai un'intera partita.

## Dove PCSX2 tiene i tuoi salvataggi

PCSX2 salva come una vera PS2: sulle memory card. Di default ce ne sono due, `Mcd001.ps2` e `Mcd002.ps2`, da 8 MB ciascuna, in una cartella `memcards`. Una sola card contiene i salvataggi di tutti i giochi che ci hai giocato.

- **Windows:** `Documents\PCSX2\memcards`. In modalità portatile, la cartella sta accanto al programma.
- **Linux:** `~/.config/PCSX2/memcards`.
- **Steam Deck** (il Flatpak di Discover): `~/.var/app/net.pcsx2.PCSX2/config/PCSX2/memcards`. Con EmuDeck, il collegamento sotto `~/Emulation/saves/pcsx2` punta a una di queste.
- **Mac:** `~/Library/Application Support/PCSX2/memcards`.

In PCSX2, **Settings → Memory Cards** mostra la cartella che usa davvero e quale card è in ogni slot. Hoard trova da solo le cartelle di Windows, Linux e Steam Deck; su un Mac o in un'installazione portatile, indica una volta la cartella `memcards`.

## Card a file e card a cartella

PCSX2 sa creare due tipi di memory card, e quando sincronizzi la scelta conta più di quanto sembri.

- **Una card a file** (`.ps2`) è un unico file da 8 MB con dentro i salvataggi di tutti i giochi. Salvi in un gioco qualsiasi e cambia l'intero file, quindi ogni nuova versione sono gli 8 MB completi.
- **Una card a cartella** è una cartella invece di un file, con ogni salvataggio nella sua sottocartella. Salvi in un gioco e cambiano solo i suoi file, quindi le versioni restano leggere e la cronologia mostra quale salvataggio si è mosso.

Puoi creare l'una o l'altra da **Settings → Memory Cards**. Qualunque scegli, usa **lo stesso tipo, gli stessi nomi e gli stessi slot** su ogni macchina. Una card a file sul desktop e una a cartella sul Deck sono due card diverse, e ogni macchina penserà che il salvataggio dell'altra non esista.

## Come funziona la sincronizzazione ogni giorno

Giochi sul desktop e chiudi PCSX2. Hoard aspetta che PCSX2 sia uscito e che le card smettano di cambiare, poi carica la nuova versione. Più tardi prendi lo Steam Deck. Appena è online, Hoard scarica le card più recenti, e quando avvii PCSX2 il tuo salvataggio è lì. Chiudilo sul Deck e succede la stessa cosa al contrario.

Nessuna delle due macchine deve essere accesa insieme all'altra. Le card aspettano sul server finché l'altra macchina non le chiede.

## Trappole da conoscere

- **Chiudi PCSX2, non solo il gioco.** Hoard salva quando l'emulatore è uscito, quindi non copia mai una card a metà scrittura. Su un Deck, sospendere non vale come chiudere.
- **Stesso disco, stessa regione.** Le versioni PAL e NTSC di un gioco hanno seriali diversi (per esempio SLES e SLUS) e non vedono i salvataggi l'una dell'altra. Usa la stessa immagine del disco ovunque.
- **Gli stati sono un'altra cosa.** Gli stati (file `.p2s` in `sstates`) sono istantanee dell'emulatore e spesso non si caricano in un'altra versione di PCSX2. Hoard sincronizza le memory card; se vuoi che viaggino anche gli stati, aggiungi la cartella `sstates` come elemento a parte e tieni PCSX2 alla stessa versione su ogni macchina.
- **Una versione è l'intera card.** Ripristinare una versione precedente rimette l'intera card com'era, con tutti i suoi giochi. Hoard ti mostra cosa cambierà prima di confermare, e la card attuale viene copiata prima, quindi un ripristino si può sempre annullare.

## Configurazione

1. Installa Hoard su ogni macchina e accedi con lo stesso account.
2. Nella **Libreria**, aggiungi PCSX2 dall'elenco degli emulatori.
3. Controlla che tutte le macchine usino lo stesso tipo di card, gli stessi nomi e gli stessi slot.
4. Gioca, chiudi PCSX2 e continua sull'altra macchina.

Preferisci tenere tutto a casa? Avvia `hoard-server` sul tuo PC o NAS e punta lì tutte le macchine. Nessun account con noi, nessuna telemetria verso di noi, niente che passi dai nostri server. Vedi [come ospitare Hoard da solo](/guides/self-host-hoard). Per gli altri emulatori, vedi [salvataggi degli emulatori](/guides/back-up-emulator-saves).

<!-- faq -->

## Domande frequenti

### PCSX2 ha i salvataggi nel cloud?

No. PCSX2 scrive le memory card in una cartella locale e lascia a te la sincronizzazione. Hoard è un modo per farla in automatico, con in più una cronologia delle versioni.

### Posso sincronizzare PCSX2 tra un PC e uno Steam Deck?

Sì. Installa Hoard su entrambi con lo stesso account. Hoard sa dove PCSX2 tiene le card su Windows e nel Flatpak dello Steam Deck, e le abbina tra le macchine.

### Card a file o card a cartella?

Per sincronizzare va meglio la card a cartella: vengono caricati solo i salvataggi cambiati, e la cronologia mostra quale gioco si è mosso. Funzionano entrambe, purché tutte le macchine usino la stessa.

### Hoard sincronizza gli stati di PCSX2?

Non di default, perché gli stati si rompono tra una versione di PCSX2 e l'altra. Aggiungi a mano la cartella `sstates` se li vuoi, e tieni PCSX2 alla stessa versione ovunque.

### Ripristinare una vecchia versione riporta indietro tutti i giochi della card?

Sì. Una versione è l'intera card. Hoard mostra prima cosa cambierà, e conserva come versione anche la card che sostituisci.

### Funziona con gli emulatori PS2 su Android?

Oggi no. Hoard funziona su Windows, macOS, Linux e Steam Deck.
