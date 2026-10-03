---
title: "Dove sono i salvataggi di Cyberpunk 2077 (PC e Steam Deck)"
description: "Dove Cyberpunk 2077 tiene i salvataggi su Windows, Steam Deck e Mac, cosa contiene ogni cartella e come farne il backup o spostarli da un PC all'altro."
order: 20
updated: 2026-10-02
---

Su Windows, Cyberpunk 2077 tiene i salvataggi in `%USERPROFILE%\Saved Games\CD Projekt Red\Cyberpunk 2077`, una cartella per salvataggio. Questa è la risposta breve. Il resto della pagina copre i percorsi su Steam Deck e Mac, cosa c'è davvero nella cartella e come tenerla sempre al sicuro.

## Dove Cyberpunk 2077 tiene i salvataggi

- **Windows** (Steam, GOG o Epic): `%USERPROFILE%\Saved Games\CD Projekt Red\Cyberpunk 2077`
- **Steam Deck e Linux** (Proton): `~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077`
- **Mac:** `~/Library/Application Support/CD Projekt Red/Cyberpunk 2077/Saves`

I percorsi per Windows e Mac sono quelli indicati da CD Projekt Red nelle proprie pagine di supporto. Su Steam Deck il gioco gira dentro un prefisso Proton, un piccolo albero di cartelle Windows che Steam tiene per ogni gioco; `1091500` è l'ID Steam di Cyberpunk. Se il gioco è installato sulla microSD, cerca `steamapps/compatdata/1091500` sulla scheda.

## Cosa c'è nella cartella

Cyberpunk non scrive un unico file di salvataggio, ma **una cartella per salvataggio**: `AutoSave-0`, `AutoSave-1` e così via, `ManualSave-0`, `ManualSave-1` e `QuickSave-0`. Ognuna contiene il salvataggio vero e proprio (`sav.dat`) più lo screenshot e i metadati mostrati nel menu di caricamento.

Ne seguono due cose:

- **Fai il backup della cartella madre, non di un singolo salvataggio.** Copiare solo l'ultimo `ManualSave` lascia fuori i salvataggi automatici, che spesso hanno i progressi più recenti.
- **I salvataggi automatici ruotano.** Il gioco riusa poche cartelle `AutoSave` e sovrascrive la più vecchia. Un salvataggio automatico di tre ore fa di solito non c'è già più: per questo conviene una cronologia fuori dal gioco.

Le impostazioni non sono qui. Grafica e comandi stanno in `UserSettings.json`, sotto `%LOCALAPPDATA%\CD Projekt Red\Cyberpunk 2077`, accanto a cache e log. È la cartella che molti (e alcuni strumenti) salvano per errore: non contiene nulla la cui perdita ti costi progressi.

## Cyberpunk 2077 ha i salvataggi nel cloud?

Sì. La versione Steam usa Steam Cloud, quella GOG il cloud di GOG Galaxy. Entrambi tengono allineato l'ultimo stato dei salvataggi tra le macchine dello stesso store.

Cosa non fa nessuno dei due:

- **Tenere le versioni precedenti.** Se un salvataggio si corrompe, o una mod lo rompe, anche il cloud tiene la copia rotta.
- **Collegare store diversi.** Steam Cloud e il cloud di GOG non si parlano, anche se i salvataggi PC di Steam, GOG ed Epic si caricano senza problemi in qualsiasi versione copiando la cartella.

## Backup a mano

1. Chiudi completamente il gioco.
2. Copia l'intera cartella `Cyberpunk 2077` dal percorso qui sopra su una chiavetta USB, un altro disco o una cartella cloud.
3. Per ripristinare, chiudi il gioco e ricopia la cartella sostituendo quella esistente.

Funziona, ma solo quanto spesso te ne ricordi, e hai solo la copia dell'ultima volta.

## Backup e sync automatici con Hoard

[Hoard](/download) fa il backup della cartella ogni volta che smetti di giocare e tiene tutte le versioni, così un salvataggio corrotto o un salvataggio automatico ormai sovrascritto è a un clic. Sincronizza anche la cartella tra i tuoi PC e una Steam Deck.

1. Installa Hoard e accedi, oppure puntalo verso [il tuo server](/guides/self-host-hoard).
2. Apri la **Libreria**. Cyberpunk viene rilevato dalla tua libreria Steam e dal database comunitario dei salvataggi.
3. Controlla che la cartella mostrata sia quella in `Saved Games\CD Projekt Red\Cyberpunk 2077`. Se mostra quella in `AppData\Local`, cambiala: contiene solo impostazioni.
4. Gioca. Quando esci, la prima versione compare nella cronologia.

Hoard segue l'intera cartella, quindi ogni `AutoSave`, `ManualSave` e `QuickSave` finisce nella stessa versione. Con una Deck e un fisso, la versione più recente ti aspetta su quello che prendi dopo — vedi [come funziona la sincronizzazione tra PC](/guides/sync-game-saves-across-pcs).

<!-- faq -->

## Domande frequenti

### Dove sono i salvataggi di Cyberpunk 2077 su Steam Deck?

Nel prefisso Proton del gioco: `~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077`. Se il gioco è sulla microSD, la cartella `compatdata` è sulla scheda.

### Posso spostare i salvataggi di Cyberpunk 2077 da GOG a Steam?

Sì. I salvataggi PC sono gli stessi su Steam, GOG ed Epic. Copia le cartelle di salvataggio nello stesso percorso dell'altra installazione a gioco chiuso e compariranno nel menu di caricamento.

### Perché ci sono così tante cartelle AutoSave?

Il gioco tiene alcuni slot di salvataggio automatico e sovrascrive ogni volta il più vecchio. Sono salvataggi normali, solo che vengono sostituiti da soli.

### Perché il mio vecchio salvataggio automatico è sparito?

Perché lo slot in cui si trovava è stato riusato. Il gioco ne tiene solo pochi. Uno strumento di backup con le versioni è l'unico modo per recuperarlo una volta ruotato via.

### Hoard sincronizza anche le mie impostazioni?

No. Le impostazioni stanno in un'altra cartella che non fa parte del salvataggio, quindi ogni macchina tiene le sue, che di solito è proprio ciò che vuoi: una Deck e un fisso hanno bisogno di impostazioni grafiche diverse. Maggiori dettagli in [sincronizzare i salvataggi tra PC](/guides/sync-game-saves-across-pcs).
