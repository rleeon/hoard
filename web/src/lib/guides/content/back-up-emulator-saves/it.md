---
title: "Come fare il backup e sincronizzare i salvataggi degli emulatori (RetroArch, Dolphin, PCSX2)"
description: "Backup e sync dei salvataggi degli emulatori tra PC e Steam Deck: RetroArch, Dolphin, PCSX2, DuckStation e altri, con cronologia e percorsi di ognuno."
order: 6
updated: 2026-10-09
---

I salvataggi degli emulatori si perdono facilmente: file di salvataggio e save state vivono in cartelle sparse, e una reinstallazione o un PC nuovo possono cancellare anni di progressi. Hoard ne fa il backup in automatico e li tiene sincronizzati tra le tue macchine, Steam Deck compresa.

## Emulatori con cui funziona Hoard

Hoard gestisce i file di salvataggio standard degli emulatori (`.srm`, `.sav`, memory card, cartelle di salvataggio per gioco) e i save state. Sa già dove salvano questi emulatori:

- **Sony:** PCSX2 (PS2), DuckStation (PS1), PPSSPP (PSP), RPCS3 (PS3), shadPS4 (PS4), Vita3K (PS Vita)
- **Nintendo:** Dolphin (GameCube / Wii), Cemu (Wii U), Ryujinx, yuzu, Eden, Suyu, Citron e Sudachi (Switch), Citra / Azahar (3DS), melonDS (DS), mGBA (GBA), Project64 (N64)
- **Altri:** RetroArch (multisistema), xemu (Xbox), Flycast (Dreamcast)

Dato che Hoard trova le cartelle di salvataggio con lo stesso database comunitario usato da Ludusavi, molti percorsi vengono rilevati in automatico. Per qualsiasi percorso personalizzato puoi indicare una cartella a mano.

## Imposta i backup dei salvataggi degli emulatori

1. **Installa Hoard** per Windows, macOS o Linux e accedi.
2. Apri la **Libreria** e aggiungi il tuo emulatore, oppure aggiungi manualmente la sua cartella di salvataggi/stati se hai cambiato la posizione predefinita.
3. Tieni attiva la **modalità automatica**. Hoard fa il backup dopo ogni sessione e conserva una cronologia versionata.
4. Installa Hoard sugli altri PC con lo stesso account per sincronizzare quei salvataggi ovunque — vedi [sincronizzare i salvataggi tra più PC](/guides/sync-game-saves-across-pcs).

## Ludusavi per gli emulatori?

Anche Ludusavi può fare il backup locale dei salvataggi degli emulatori, ed è un'ottima opzione gratuita per questo. Se vuoi anche che quei salvataggi si sincronizzino in automatico tra le macchine e mantengano una cronologia delle versioni nel cloud senza configurare Rclone, è lì che aiuta Hoard — leggi il [confronto completo tra Ludusavi e Hoard](/guides/ludusavi-alternative).

## Salvataggi nel cloud per ogni emulatore

Nessuno degli emulatori standalone qui sotto sincronizza da solo i salvataggi tra macchine: sono semplici file sul tuo disco. È una buona notizia, perché qualsiasi strumento che osservi la cartella giusta può portarli con sé. Ecco dove li tiene ciascuno. "Steam Deck" indica la versione Flatpak installata dallo store Discover.

### Salvataggi nel cloud di PCSX2 (PS2)

PCSX2 scrive le memory card (file `.ps2`) in `memcards/`:

- Windows: `Documenti\PCSX2\memcards`
- Linux: `~/.config/PCSX2/memcards`
- Steam Deck: `~/.var/app/net.pcsx2.PCSX2/config/PCSX2/memcards`

Una memory card contiene i salvataggi di tutti i giochi usati su di essa, quindi viaggia come un pezzo unico: ripristinare una versione precedente riporta indietro l'intera card, non un singolo gioco.

La guida completa, con card a file e a cartella: [salvataggi cloud per PCSX2](/guides/pcsx2-cloud-saves).

### Salvataggi nel cloud di Dolphin (GameCube e Wii)

I salvataggi GameCube stanno in `GC/` (immagini di memory card o una cartella per card), quelli Wii nella NAND emulata in `Wii/`:

- Windows: `Documenti\Dolphin Emulator\GC` e `\Wii`
- Linux: `~/.local/share/dolphin-emu/GC` e `/Wii`
- Steam Deck: `~/.var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu/`

La guida completa, con cartelle GCI e la memoria della Wii: [salvataggi cloud per Dolphin](/guides/dolphin-cloud-saves).

### Salvataggi nel cloud di DuckStation (PS1)

DuckStation tiene le memory card in `memcards/` e, di default, crea una card separata per ogni gioco, cosa che si sposa benissimo con la sincronizzazione:

- Windows: `Documenti\DuckStation\memcards` (le versioni recenti usano `%LOCALAPPDATA%\DuckStation\memcards`)
- Linux: `~/.local/share/duckstation/memcards`
- Steam Deck: `~/.var/app/org.duckstation.DuckStation/`, sotto `data/` o `config/`

### Sincronizzare i salvataggi di RetroArch

RetroArch separa `saves/` (i salvataggi dei giochi) da `states/` (i save state). Hoard segue la cartella dei salvataggi; aggiungi `states/` come voce a parte se giochi con gli stati:

- Windows: `%APPDATA%\RetroArch`, oppure accanto a `retroarch.exe` in un'installazione portable
- Linux: `~/.config/retroarch`
- Steam Deck: `~/.var/app/org.libretro.RetroArch/config/retroarch`, oppure `~/Emulation/saves/retroarch` se l'hai configurato con EmuDeck

RetroArch ha anche un Cloud Sync integrato che parla con un server WebDAV fornito da te. È una scelta sensata se usi solo RetroArch e hai già WebDAV. Hoard non ha bisogno di WebDAV, tiene una cronologia delle versioni da ripristinare e copre anche gli emulatori standalone.

### PPSSPP (PSP)

I salvataggi vanno in `PSP/SAVEDATA`, gli stati in `PSP/PPSSPP_STATE`:

- Windows: `Documenti\PPSSPP\PSP\SAVEDATA`, oppure `memstick\PSP\SAVEDATA` accanto all'eseguibile in un'installazione portable
- Linux: `~/.config/ppsspp/PSP/SAVEDATA`
- Steam Deck: `~/.var/app/org.ppsspp.PPSSPP/config/ppsspp/PSP/SAVEDATA`

### RPCS3 (PS3)

I salvataggi stanno in `dev_hdd0/home/00000001/savedata`, dentro la cartella di RPCS3 su Windows e sotto `~/.config/rpcs3/` su Linux e Steam Deck.

### Emulatori Switch: Ryujinx, yuzu, Eden, Suyu, Citron, Sudachi

Ryujinx salva in `bis/user/save` (sotto `%APPDATA%\Ryujinx` o `~/.config/Ryujinx`). La famiglia yuzu usa `nand/user/save` nella propria cartella in `%APPDATA%` o `~/.local/share`.

Qui c'è una trappola. L'albero in stile yuzu è `save/<account>/<profilo>/<id-gioco>/`, e l'ID del profilo viene generato al primo avvio dell'emulatore, quindi è diverso in ogni installazione. Se sincronizzi l'intera cartella `save/` tra due macchine, ciascuna si ritrova il profilo dell'altra accanto al proprio, e nessun gioco vede i progressi dell'altro. Hoard scende invece fino alla cartella di ogni singolo gioco, così lo stesso titolo combacia tra le macchine qualunque sia il nome del profilo.

### Citra e Azahar (3DS)

I salvataggi stanno in profondità sotto `sdmc/Nintendo 3DS/<id0>/<id1>/title/…`, e `id0`/`id1` derivano dalle chiavi della console emulata, quindi cambiano anch'essi a ogni installazione. Hoard li gestisce come l'albero Switch: una voce per gioco, abbinata tra le macchine.

### Il resto

- **Cemu (Wii U):** `mlc01/usr/save`, sotto `%APPDATA%\Cemu` o `~/.local/share/Cemu`.
- **shadPS4 (PS4):** `savedata`, sotto `%APPDATA%\shadPS4` o `~/.local/share/shadPS4`.
- **Vita3K (PS Vita):** `ux0/user/00/savedata` nella sua cartella dati.
- **mGBA, melonDS e la maggior parte degli emulatori dell'era delle cartucce:** un `.sav` accanto alla ROM, salvo diversa impostazione. Aggiungi a mano i salvataggi della cartella delle ROM.

## Salvataggi degli emulatori su Steam Deck

Su Steam Deck gli emulatori arrivano di solito da Flatpak, quindi le loro cartelle stanno sotto `~/.var/app/<id>/` invece che nei soliti `~/.config` o `~/.local/share`. EmuDeck raccoglie tutto sotto `~/Emulation/saves/`, una cartella per emulatore. In ogni caso aggiungi la cartella una volta e Hoard la tiene d'occhio.

Ciò che conta su una portatile: il motore di Hoard gira come servizio in background, quindi fa il backup quando esci da un gioco in modalità Gioco, senza finestre aperte. Riprendi la Deck dopo una sessione sul fisso e il salvataggio è già lì.

## Salvataggio e save state non sono la stessa cosa

Vale la pena distinguerli, perché si comportano diversamente quando viaggiano:

- Un **salvataggio** (`.srm`, una memory card, una cartella `SAVEDATA`) è il salvataggio del gioco stesso, scritto dalla console emulata. Passa tra macchine e versioni dell'emulatore senza problemi.
- Un **save state** è una copia della memoria dell'emulatore. È legato alla build dell'emulatore, spesso al core esatto, quindi uno stato creato da una versione potrebbe non caricarsi in un'altra.

Hoard fa il backup di entrambi. Solo, non stupirti se uno stato da una macchina aggiornata non si apre su una rimasta indietro: tieni gli emulatori alla stessa versione e affidati ai salvataggi per ciò che conta.

## Un emulatore, tanti giochi

Un emulatore è un unico processo che ospita decine di titoli, ed è questo a rendere scomodi i suoi salvataggi per uno strumento che ragiona in termini di "gioco in esecuzione". Hoard tiene separati i titoli invece di trattare l'emulatore come un unico blocco, così ogni gioco ha la propria cronologia invece di un mucchio comune che cambia a ogni avvio. Se un salvataggio si rovina, puoi [tornare a una versione precedente](/guides/restore-a-game-save).

## Salvataggi di emulatore senza passare dai nostri server

Tutto questo funziona allo stesso modo con il tuo server: avvia `hoard-server`, punta l'app lì, e i tuoi salvataggi vanno dalla tua macchina al tuo disco. Nessun account presso di noi, nessuna telemetria verso di noi, niente passa dai nostri server. Vedi [come fare il self-host di Hoard](/guides/self-host-hoard).

## Suggerimento

I save state dipendono da una versione precisa dell'emulatore. Aggiorna gli emulatori in modo coerente su tutti i PC, così uno stato sincronizzato si carica ovunque.

<!-- faq -->

## Domande frequenti

### Hoard fa il backup anche delle mie ROM?

No. Segue le cartelle di salvataggio, non i file di gioco. Le ROM sono grandi, non cambiano e le hai già: non c'è niente da versionare.

### PCSX2, Dolphin o DuckStation hanno salvataggi nel cloud integrati?

No. Scrivono i salvataggi in cartelle locali e lasciano a te la sincronizzazione. Punta uno strumento di sync sulle cartelle elencate sopra e i salvataggi ti seguiranno tra le macchine.

### RetroArch ha una sincronizzazione cloud?

Sì, un Cloud Sync integrato che richiede un server WebDAV gestito o affittato da te. Hoard è l'alternativa se preferisci non configurare WebDAV, vuoi una cronologia delle versioni a cui tornare o giochi anche con emulatori standalone.

### Funziona su Steam Deck in modalità Gioco?

Sì. Il motore gira come servizio in background, quindi i salvataggi vengono copiati quando esci da un gioco, senza finestre aperte. Le cartelle Flatpak ed EmuDeck funzionano come tutte le altre.

### Il mio emulatore è portable. Funziona?

Sì. Aggiungi a mano la cartella accanto all'eseguibile e Hoard la segue come qualsiasi altra posizione di salvataggio. È la configurazione tipica sulle console portatili.

### Posso sincronizzare i save state tra due PC?

Sì, e Hoard lo fa. Che uno stato si carichi dipende dall'avere la stessa versione dell'emulatore su entrambe le macchine, un limite dell'emulatore e non della sincronizzazione. I salvataggi non hanno questo problema.

### Funzionerà con un emulatore che non è nella lista?

Quasi certamente. Il rilevamento copre in automatico quelli comuni, e qualsiasi altro lo aggiungi indicando a Hoard la sua cartella di salvataggio.

### Il self-host cambia qualcosa per gli emulatori?

No. Stesso rilevamento, stesse versioni, stessa sincronizzazione. Solo lo spazio di archiviazione è tuo.
