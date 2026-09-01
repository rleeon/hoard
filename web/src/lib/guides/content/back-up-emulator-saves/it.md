---
title: "Come fare il backup e sincronizzare i salvataggi degli emulatori (RetroArch, Dolphin, PCSX2)"
description: "Fai il backup e sincronizza i file di salvataggio e i save state dei tuoi emulatori tra PC — RetroArch, Dolphin, PCSX2, DuckStation e altri — automaticamente con Hoard."
order: 6
updated: 2026-09-01
---

I salvataggi degli emulatori si perdono facilmente: file di salvataggio e save state vivono in cartelle sparse, e una reinstallazione o un nuovo PC possono cancellare anni di progressi. Hoard ne fa il backup automaticamente e li mantiene sincronizzati tra le macchine.

## Emulatori con cui funziona Hoard

Hoard gestisce i file di salvataggio standard degli emulatori (`.srm`, `.sav`, memory card) e i save state degli emulatori popolari, tra cui:

- **RetroArch** — salvataggi e stati per core
- **Dolphin** (GameCube / Wii) — memory card e file GCI
- **PCSX2** (PS2) — memory card
- **DuckStation** (PS1), **PPSSPP** (PSP), **mGBA** e altri

Poiché Hoard individua le cartelle di salvataggio con lo stesso database comunitario che alimenta Ludusavi, molti percorsi degli emulatori vengono rilevati automaticamente. Per qualsiasi caso personalizzato, puoi puntare Hoard a una cartella a mano.

## Imposta i backup dei salvataggi degli emulatori

1. **Installa Hoard** per Windows, macOS o Linux e accedi.
2. Apri la **Libreria** e aggiungi il tuo emulatore, oppure aggiungi manualmente la sua cartella di salvataggi/stati se hai cambiato la posizione predefinita.
3. Tieni attiva la **modalità automatica**. Hoard fa il backup dopo ogni sessione e conserva una cronologia versionata.
4. Installa Hoard sugli altri PC con lo stesso account per sincronizzare quei salvataggi ovunque — vedi [sincronizzare i salvataggi tra PC](/guides/sync-game-saves-across-pcs).

## Ludusavi per gli emulatori?

Ludusavi può fare il backup dei salvataggi degli emulatori anche in locale, ed è un'ottima opzione gratuita per questo. Se vuoi anche che quei salvataggi degli emulatori si sincronizzino automaticamente tra le macchine e mantengano una cronologia versioni nel cloud senza configurare Rclone, è qui che Hoard aiuta — leggi il [confronto completo Ludusavi vs Hoard](/guides/ludusavi-alternative).

## Dove ogni emulatore tiene i salvataggi

Utile saperlo, perché un'installazione portable mette tutto questo altrove:

- **RetroArch** — `saves/` e `states/` nella cartella di configurazione: `%APPDATA%\RetroArch` su Windows, `~/.config/retroarch` su Linux.
- **Dolphin** — memory card in `GC/`, salvataggi Wii nella NAND emulata, dentro `Documenti\Dolphin Emulator` o `~/.local/share/dolphin-emu`.
- **PCSX2** — `memcards/`, sotto `Documenti\PCSX2` o `~/.config/PCSX2`.
- **DuckStation** — `memcards/` e `savestates/` nella sua cartella dati.
- **PPSSPP** — `PSP/SAVEDATA` per i salvataggi e `PSP/PPSSPP_STATE` per gli stati.
- **RPCS3** — `dev_hdd0/home/00000001/savedata`.
- **Cemu** — `mlc01/usr/save`.
- **mGBA e la maggior parte dei core autonomi** — un `.sav` accanto alla ROM, se non gli hai detto diversamente.

Un'**installazione portable** — la norma su console portatili e chiavette USB — tiene tutto questo accanto all'eseguibile. Se è il tuo caso, punta Hoard a quella cartella e la traccerà come qualsiasi altro salvataggio.

## Salvataggio e save state non sono la stessa cosa

Vale la pena distinguerli, perché viaggiano in modo diverso:

- Un **salvataggio** (`.srm`, una memory card, una cartella `SAVEDATA`) è il salvataggio proprio del gioco, scritto dalla console emulata. Passa da una macchina all'altra e tra versioni di emulatore senza protestare.
- Un **save state** è un dump della memoria dell'emulatore. È legato a quella build, e spesso al core esatto, quindi uno stato scritto da una versione può rifiutarsi di caricare in un'altra.

Hoard salva entrambi. Solo non stupirti se uno stato da una macchina aggiornata non si apre su una rimasta indietro: tieni gli emulatori alla stessa versione e affidati ai salvataggi veri per ciò a cui tieni.

## Un emulatore, tanti giochi

Un emulatore è un solo processo che ospita decine di titoli, ed è questo a rendere scomodi i salvataggi degli emulatori per uno strumento che ragiona per "il gioco in esecuzione". Hoard tiene separati i titoli invece di trattare l'intero emulatore come un unico blocco, così ogni gioco ha la sua cronologia e non un mucchio comune che cambia ogni volta che avvii qualcosa.

## Salvataggi di emulatore senza passare dai nostri server

Tutto questo funziona allo stesso modo contro il tuo server: fai girare `hoard-server`, punta l'app lì, e i salvataggi vanno dalla tua macchina al tuo disco. Nessun account con noi, nessuna telemetria verso di noi, niente attraverso i nostri server. Vedi [come ospitare Hoard da solo](/guides/self-host-hoard).

## Suggerimento

I save state sono legati a una versione specifica dell'emulatore. Mantieni i tuoi emulatori aggiornati in modo coerente su tutti i PC così che uno stato sincronizzato si carichi senza problemi ovunque.

<!-- faq -->

## Domande frequenti

### Hoard salva anche le mie ROM?

No. Traccia le cartelle dei salvataggi, non i file di gioco. Le ROM sono grandi, non cambiano e le hai già: non c'è niente da versionare.

### Il mio emulatore è portable. Funziona lo stesso?

Sì. Aggiungi a mano la cartella accanto all'eseguibile e Hoard la traccerà come qualsiasi altra posizione di salvataggio. È il setup abituale sulle console portatili.

### Posso sincronizzare i save state tra due PC?

Puoi, e Hoard lo farà. Che uno stato si carichi dipende dal fatto che gli emulatori siano alla stessa versione su entrambe le macchine: è un limite dell'emulatore, non della sincronizzazione. I salvataggi veri non hanno questo problema.

### Funzionerà con un emulatore che non è in elenco?

Quasi certamente sì. Il rilevamento copre automaticamente quelli comuni, e qualsiasi altro lo aggiungi puntando Hoard alla sua cartella dei salvataggi.

### Il self-hosting cambia qualcosa per gli emulatori?

No. Stesso rilevamento, stesse versioni, stessa sincronizzazione. L'unica cosa tua è lo spazio di archiviazione.
