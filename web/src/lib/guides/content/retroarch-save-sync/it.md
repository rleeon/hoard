---
title: "Sincronizzare i salvataggi di RetroArch tra PC e Steam Deck"
description: "Sincronizza salvataggi e stati di RetroArch tra PC, Steam Deck e portatile: dove stanno i .srm, Cloud Sync integrato o sync automatico, e le trappole."
order: 14
updated: 2026-10-09
---

RetroArch tiene i salvataggi del gioco come file `.srm` in una cartella `saves` e gli stati in una cartella `states`. Per sincronizzarli tra dispositivi puoi usare il Cloud Sync integrato di RetroArch con un server WebDAV tuo, oppure uno strumento che tenga d'occhio entrambe le cartelle. Hoard fa la seconda cosa in automatico: salva le due cartelle quando chiudi RetroArch, le scarica sulle altre macchine, conserva ogni versione e capisce le installazioni con EmuDeck.

## Dove RetroArch tiene i salvataggi

- **Windows:** `%APPDATA%\RetroArch\saves` e `\states`, oppure `saves` e `states` accanto a `retroarch.exe` se l'hai installato in una cartella sua.
- **Linux:** `~/.config/retroarch/saves`. Il Flatpak usa `~/.var/app/org.libretro.RetroArch/config/retroarch/saves`.
- **Steam Deck con EmuDeck:** `~/Emulation/saves/retroarch/`, dove `saves` e `states` sono collegamenti alle cartelle reali. Hoard legge `retroarch.cfg` per sapere dove puntano davvero.
- **RetroDECK:** `~/retrodeck/saves` e `~/retrodeck/states` di default.
- **Altrove:** **Settings → Directory** mostra le cartelle che RetroArch sta usando davvero.

## Quando RetroArch scrive davvero il salvataggio

Qui casca un sacco di gente. RetroArch tiene il salvataggio del gioco in memoria e scrive il `.srm` solo quando chiudi il gioco o esci da RetroArch, a meno che non sia impostato **Settings → Saving → SaveRAM Autosave Interval**. Fino ad allora sul disco non c'è niente: un crash o una batteria scarica si portano via tutto dall'ultima scrittura, e nessuno strumento di sincronizzazione può spostare un salvataggio che non è stato scritto.

Imposta un intervallo di salvataggio automatico di qualche secondo. E prima di cambiare dispositivo, **chiudi RetroArch**, non solo il gioco: Hoard salva quando RetroArch si è chiuso, quindi non copia mai un salvataggio scritto a metà. Su un Deck, sospendere non vale come chiudere.

## Configura tutti i dispositivi allo stesso modo

- **Opzioni di ordinamento.** **Settings → Saving** può dividere salvataggi e stati in sottocartelle per nome del core o per cartella dei contenuti. Se un dispositivo ordina e l'altro no, il file sincronizzato finisce in una cartella dove RetroArch non guarda. Usa le stesse impostazioni ovunque.
- **Nomi delle ROM.** Il `.srm` prende il nome dalla ROM: `Super Metroid (USA).sfc` salva in `Super Metroid (USA).srm`. Una ROM con un altro nome sull'altro dispositivo non lo trova.
- **Lo stesso core.** Due core per la stessa console non sempre leggono i salvataggi l'uno dell'altro. Scegline uno per sistema e usalo ovunque.
- **Versioni del core, per gli stati.** Uno stato è un'istantanea della memoria del core e spesso non si carica in un'altra versione. I salvataggi normali non hanno questo problema.

Un'altra trappola con gli stati: uno stato contiene la memoria del gioco, salvataggio compreso. Carichi uno stato vecchio e la scrittura successiva del `.srm` riporta indietro quel salvataggio vecchio. Se usi **Auto Load State**, sincronizza anche gli stati, così a viaggiare è il più recente.

## Cloud Sync di RetroArch o Hoard?

Per essere onesti con entrambi:

- **Il Cloud Sync di RetroArch** è integrato e sincronizza salvataggi e stati con un server WebDAV che gestisci o affitti. Funziona anche su Android e iOS, cosa che Hoard oggi non fa. Se usi già Nextcloud, che conserva per conto suo le versioni dei file, va benissimo, ed è la scelta migliore se il telefono fa parte del tuo setup.
- **Hoard** non ha bisogno di un server WebDAV. Salva e sincronizza in automatico, tiene una cronologia di versioni a cui tornare e copre anche gli emulatori standalone e i giochi PC. Tratta l'intera cartella `saves` come un unico elemento, quindi tornare indietro ripristina ogni gioco che contiene così com'era in quel momento. Prima di confermare ti mostra cosa cambierà, e i file attuali vengono copiati prima.

Scegline uno per cartella. Due strumenti che scrivono gli stessi salvataggi sono la ricetta dei conflitti.

## Configurarlo con Hoard

1. Installa Hoard su ogni dispositivo e accedi con lo stesso account.
2. Nella **Libreria**, aggiungi RetroArch. Salvataggi e stati compaiono come due elementi.
3. Allinea le impostazioni qui sopra su tutti i dispositivi.
4. Gioca, chiudi RetroArch e riprendi sull'altro dispositivo.

Preferisci tenere tutto a casa? Avvia `hoard-server` sul tuo PC o NAS: nessun account con noi, nessuna telemetria verso di noi, niente che passi dai nostri server. Vedi [come ospitare Hoard da solo](/guides/self-host-hoard).

<!-- faq -->

## Domande frequenti

### RetroArch ha i salvataggi nel cloud?

Sì, un Cloud Sync integrato che richiede un server WebDAV. Hoard è l'alternativa se non vuoi gestirne uno, vuoi una cronologia delle versioni o usi anche emulatori standalone.

### Perché il mio salvataggio di RetroArch non si è sincronizzato?

Di solito per uno di tre motivi: RetroArch non aveva ancora scritto il `.srm` (era ancora aperto, senza intervallo di salvataggio automatico), i due dispositivi dividono i salvataggi in sottocartelle diverse, oppure le ROM hanno nomi diversi.

### Posso sincronizzare anche gli stati?

Sì. Hoard segue `states` come elemento a parte. Che uno stato si carichi sull'altro dispositivo dipende dall'avere la stessa versione del core su entrambi.

### Funziona con EmuDeck e RetroDECK?

Sì. Hoard legge la configurazione di RetroArch per seguire i collegamenti di EmuDeck fino alle cartelle reali. Con RetroDECK, aggiungi `~/retrodeck/saves` e `~/retrodeck/states` se non vengono rilevate.

### Hoard sincronizza RetroArch su Android?

Oggi no. Hoard funziona su Windows, macOS, Linux e Steam Deck.
