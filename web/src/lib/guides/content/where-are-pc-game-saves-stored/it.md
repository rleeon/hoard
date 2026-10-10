---
title: "Dove vengono salvati i giochi per PC? Tutti i percorsi"
description: "Dove i giochi per PC tengono i salvataggi su Windows, Steam Deck, Linux e Mac, quali launcher aggiungono cartelle e come trovare quelli di ogni gioco."
order: 11
updated: 2026-10-09
---

Non esiste un'unica cartella. Su Windows quasi ogni gioco salva in uno di sei posti: `Documents`, `Saved Games`, una delle tre cartelle `AppData`, la `userdata` di Steam o la sua cartella di installazione. Decidono il motore e lo sviluppatore, non il negozio dove l'hai comprato. Qui trovi tutti i percorsi più comuni, i launcher che aggiungono un livello proprio e un modo rapido per trovare i salvataggi di qualsiasi gioco, anche di uno che nessuno ha documentato.

## Windows: i sei posti soliti

| Cartella | Percorso tipico | Chi la usa |
|---|---|---|
| Documenti | `%USERPROFILE%\Documents\My Games\<Gioco>` | Giochi Bethesda, Rockstar (`Documents\Rockstar Games`), molti vecchi titoli importanti |
| Partite salvate | `%USERPROFILE%\Saved Games\<Editore>\<Gioco>` | Cyberpunk 2077 e una minoranza ostinata |
| AppData\Roaming | `%APPDATA%\<Gioco>` | Elden Ring, Stardew Valley, Minecraft (`.minecraft`), tanti indie |
| AppData\Local | `%LOCALAPPDATA%\<Gioco>\Saved\SaveGames` | Giochi in Unreal Engine (Palworld usa `Pal\Saved\SaveGames`) |
| AppData\LocalLow | `%USERPROFILE%\AppData\LocalLow\<Azienda>\<Gioco>` | Giochi in Unity (Hollow Knight e molti altri) |
| userdata di Steam | `C:\Program Files (x86)\Steam\userdata\<UserID>\<AppID>\remote` | Giochi che usano lo spazio salvataggi di Steam |

E un settimo che non vuole morire: **la cartella di installazione del gioco**, dove molti giochi vecchi e qualche indie scrivono ancora.

Due note pratiche. `AppData` è nascosta, quindi scrivi `%APPDATA%` o `%LOCALAPPDATA%` nella barra degli indirizzi di Esplora file invece di arrivarci a forza di clic. E se OneDrive fa il backup della tua cartella `Documenti`, il percorso reale è `C:\Users\<tu>\OneDrive\Documents`, cosa che sorprende molti. Vedi [OneDrive e i salvataggi dei giochi](/guides/onedrive-game-saves).

## I launcher che aggiungono un livello proprio

La maggior parte dei launcher non decide dove vanno i salvataggi; decide il gioco. Qualche eccezione:

- **Steam** ha uno spazio salvataggi per gioco in `userdata`. `<UserID>` è un numero legato al tuo account Steam (c'è una cartella per ogni account che ha fatto accesso su quel PC) e `<AppID>` è il numero nell'URL del gioco sul negozio.
- **Ubisoft Connect** salva in `C:\Program Files (x86)\Ubisoft\Ubisoft Game Launcher\savegames\<ID utente>\<ID gioco>`, con numeri al posto dei nomi su entrambi i livelli.
- **L'app Xbox e PC Game Pass** usano `%LOCALAPPDATA%\Packages\<pacchetto>\SystemAppData\wgs`, con file dai nomi casuali che solo l'app Xbox capisce. Lasciali ai salvataggi cloud di Xbox; copiarli a mano raramente funziona.
- **Epic, GOG e l'app EA** di solito lasciano fare al gioco, quindi i loro titoli finiscono nei posti soliti qui sopra. I loro salvataggi cloud, dove ci sono, copiano da lì.

## Il registro, di rado

Alcuni giochi, soprattutto piccoli titoli Unity, tengono i progressi nel registro di Windows, sotto `HKEY_CURRENT_USER\Software\<Azienda>\<Gioco>`, e non in un file. Non c'è niente da copiare in una cartella, e gli strumenti di backup basati sulle cartelle, Hoard compreso, non lo vedono. Se ti serve, esporta quella chiave con `regedit`.

## Steam Deck e Linux

- **I giochi Windows tramite Proton** salvano in un prefisso per gioco: `~/.local/share/Steam/steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/`, poi il percorso di Windows della tabella (`Documents`, `AppData/Roaming` e così via). I giochi su microSD hanno lo stesso albero sotto lo `steamapps/compatdata` della scheda.
- **I giochi nativi Linux** usano `~/.local/share/<gioco>` o `~/.config/<gioco>`. I giochi Unity vanno in `~/.config/unity3d/<Azienda>/<Gioco>`.
- **La userdata di Steam** è in `~/.local/share/Steam/userdata/<UserID>/<AppID>/remote`.
- **Heroic, Lutris e Bottles** tengono un prefisso Wine per gioco. Dentro, l'albero di Windows sta sotto `drive_c/users/<il tuo utente>/`, non sotto `steamuser`.

Il Deck ha una guida tutta sua: [sincronizzare i salvataggi tra Steam Deck e PC](/guides/sync-saves-steam-deck-pc).

## Mac

- **La maggior parte dei giochi:** `~/Library/Application Support/<Gioco>`. I giochi Unity usano `~/Library/Application Support/<Azienda>/<Gioco>`.
- **I giochi del Mac App Store** sono isolati: `~/Library/Containers/<id del bundle>/Data/Library/Application Support/`.

Anche `~/Library` è nascosta. Nel Finder, apri il menu **Vai** tenendo premuto Opzione e compare.

## Come trovare i salvataggi di qualsiasi gioco

Quando un gioco non è in nessun elenco, tre trucchi lo trovano in un paio di minuti:

1. **Cercalo su PCGamingWiki.** Quasi ogni pagina di gioco ha una sezione "Save game data location". È la stessa fonte da cui nascono i database di salvataggi usati da Hoard e Ludusavi.
2. **Guarda cosa cambia.** Salva nel gioco, esci e cerca nella tua cartella utente i file modificati negli ultimi minuti. Su Windows, cerca `datemodified:today` dentro `C:\Users\<tu>` e ordina per data. Su Linux o su un Deck: `find ~ -type f -mmin -5 -not -path '*/.cache/*'`.
3. **Chiedi a Steam.** Per un gioco con Steam Cloud, [store.steampowered.com/account/remotestorage](https://store.steampowered.com/account/remotestorage) elenca i file che Steam tiene per ogni gioco, con nomi e dimensioni. Una volta saputo come si chiama il file, trovare la cartella è facile.

## Oppure lascia che li trovi qualcun altro

Hoard legge lo stesso database della community, che copre migliaia di giochi, e controlla ogni percorso possibile sulla tua macchina: prefissi di Proton, Heroic e Lutris, la `Documenti` di OneDrive, emulatori, installazioni portatili. Quello che trova viene salvato in automatico ogni volta che smetti di giocare, con ogni versione conservata, e tenuto sincronizzato tra i tuoi PC e lo Steam Deck. Quello che gli sfugge lo aggiungi indicando la cartella una volta. Vedi [come fare il backup automatico dei salvataggi](/guides/back-up-game-saves).

Ci sono anche pagine con i percorsi esatti di alcuni giochi famosi: [Cyberpunk 2077](/guides/cyberpunk-2077-save-location), [Baldur's Gate 3](/guides/baldurs-gate-3-save-location), [Palworld](/guides/palworld-save-location), [Crimson Desert](/guides/crimson-desert-save-location) e [Marvel's Spider-Man 2](/guides/spider-man-2-save-location).

<!-- faq -->

## Domande frequenti

### Dove salva Steam i giochi?

Dipende dal gioco. Alcuni usano lo spazio di Steam, `Steam\userdata\<UserID>\<AppID>\remote`; la maggior parte scrive in `Documents`, `AppData` o `Saved Games` come qualsiasi altro gioco, e Steam Cloud li copia da lì.

### Perché non trovo la cartella AppData?

Perché è nascosta. Scrivi `%APPDATA%` (Roaming) o `%LOCALAPPDATA%` (Local) nella barra degli indirizzi di Esplora file o in Esegui (Win + R). `LocalLow` è accanto a `Local`.

### Le versioni Steam, GOG ed Epic salvano nello stesso posto?

Di solito sì, perché decide il gioco e non il negozio. Ci sono eccezioni: alcuni giochi aggiungono una cartella con l'ID del tuo account, e qualche versione da negozio usa un nome di cartella diverso. Controlla prima di copiare salvataggi da una versione all'altra.

### Dove sono i salvataggi dell'app Xbox e di Game Pass?

In `%LOCALAPPDATA%\Packages\<pacchetto>\SystemAppData\wgs`, come file dai nomi casuali che solo l'app Xbox capisce. Li sincronizza il cloud di Xbox; copiarli a mano raramente funziona.

### La mia cartella Documenti è dentro OneDrive. È un problema?

Può esserlo. I giochi seguono la cartella dentro OneDrive, e OneDrive sincronizza i salvataggi mentre i giochi li scrivono. Vedi [OneDrive e i salvataggi dei giochi](/guides/onedrive-game-saves).
