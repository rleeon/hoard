---
title: "Come sincronizzare i salvataggi tra Steam Deck e PC"
description: "Sincronizza in automatico i salvataggi di Steam Deck e PC, anche giochi fuori da Steam, emulatori e senza Steam Cloud. Configurazione, percorsi, trappole."
order: 10
updated: 2026-10-09
---

Per i giochi Steam con Steam Cloud, il tuo Deck e il tuo PC condividono già i salvataggi. Tutto il resto ha bisogno di una mano: i giochi in cui lo sviluppatore non ha mai attivato Steam Cloud, i giochi Epic e GOG avviati con Heroic, gli emulatori e qualsiasi cosa tu abbia aggiunto come gioco non Steam. Hoard li copre tutti in automatico. Quando chiudi un gioco su una macchina, salva la partita, e l'altra macchina la scarica, con ogni versione precedente conservata nel caso qualcosa vada storto.

## Cosa fa già Steam Cloud sul Deck

Se un gioco supporta Steam Cloud, Steam carica il salvataggio quando esci e lo scarica quando avvii il gioco su un'altra macchina. Dalla pagina del negozio puoi vedere se un gioco ce l'ha, e puoi disattivarlo gioco per gioco in **Proprietà → Generale**.

I buchi sono i soliti:

- **Giochi che non ce l'hanno.** Lo decide lo sviluppatore, gioco per gioco, e molti giochi PC non l'hanno mai attivato.
- **Tutto quello che sta fuori da Steam.** Heroic, Lutris, emulatori, un gioco installato a mano.
- **Niente marcia indietro.** Steam tiene il salvataggio attuale, non una cronologia. Se si sincronizza un salvataggio rovinato, quello buono sparisce su entrambe le macchine.

Ne parliamo meglio nella guida [alternativa a Steam Cloud](/guides/steam-cloud-alternative).

## Dove il Deck tiene i tuoi salvataggi

Il Deck avvia i giochi Windows con Proton, quindi lo stesso gioco salva in un posto diverso rispetto al tuo PC:

- **Giochi Windows (Proton):** `~/.local/share/Steam/steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/`, seguito dal solito percorso di Windows: `Documents`, `AppData/Roaming`, `AppData/Local`, `AppData/LocalLow` o `Saved Games`. L'AppID è il numero nell'URL del gioco sul negozio.
- **Giochi sulla microSD:** la scheda ha il suo `steamapps/compatdata/<AppID>`, con lo stesso albero dentro.
- **Giochi nativi Linux:** di solito `~/.local/share/<gioco>` o `~/.config/<gioco>`. I giochi Unity usano `~/.config/unity3d/<Azienda>/<Gioco>`.
- **Heroic, Lutris e Bottles:** ognuno tiene un prefisso Wine per gioco, con l'albero di Windows sotto `drive_c/users/<il tuo utente>/` invece di `steamuser`.
- **Emulatori:** EmuDeck li raccoglie sotto `~/Emulation/saves/`. Vedi [salvataggi degli emulatori](/guides/back-up-emulator-saves) e [RetroArch](/guides/retroarch-save-sync).

Sul tuo PC, lo stesso gioco scrive in `C:\Users\<tu>\...`. Due percorsi diversi per un solo salvataggio: ecco perché copiare cartelle a mano finisce male. Hoard cerca in tutti questi posti e abbina ciò che trova al gioco giusto, così il salvataggio del Deck e quello del PC diventano due versioni di un'unica cronologia.

## Configurazione

1. Sul Deck, passa alla modalità desktop: **tasto Steam → Spegni → Passa al desktop**.
2. Apri un browser, vai alla [pagina di download](/download) e scarica **Hoard Setup** per Linux. Nel file manager, apri le proprietà del file, consentine l'esecuzione come programma e avvialo.
3. Accedi con lo stesso account che usi sul PC, oppure punta l'app al tuo server.
4. Apri la **Libreria** e controlla cosa ha trovato Hoard. Aggiungi quello che manca indicando la sua cartella: un prefisso di Heroic, un emulatore, un gioco installato da te.
5. Installa Hoard sul PC con lo stesso account. Gli stessi giochi si abbinano da soli.
6. Torna alla modalità gioco. Non serve tornare al desktop.

Hoard Setup mette l'app nella tua cartella home e il motore di sincronizzazione in un servizio in background che parte con il Deck. Non scrive nulla nel sistema in sola lettura di SteamOS, quindi gli aggiornamenti di sistema non lo toccano.

## Com'è una giornata normale

La sera giochi sul PC ed esci. Hoard aspetta che il gioco sia chiuso e che il salvataggio smetta di cambiare, poi lo carica. La mattina dopo prendi il Deck. Appena è online, Hoard vede la versione più recente e la scrive nel prefisso di Proton. Avvii il gioco e continui. Quando esci sul Deck, succede la stessa cosa al contrario.

Nessuna delle due macchine deve essere accesa insieme all'altra. Il salvataggio aspetta sul server finché l'altra non lo chiede.

## Le trappole da conoscere

### Sospendere non è chiudere

Il Deck rende facilissimo premere il tasto di accensione e andarsene con il gioco ancora aperto. Hoard salva una partita solo quando il gioco è chiuso, perché un gioco in esecuzione potrebbe essere a metà della scrittura. E non sostituisce mai il salvataggio di un gioco aperto. Quindi se sospendi il Deck e poi giochi sul PC, i progressi del Deck non sono ancora caricati, e il nuovo salvataggio del PC aspetta che tu chiuda il gioco sul Deck.

L'abitudine che evita tutto: **chiudi il gioco prima di cambiare macchina.** Se Proton lascia indietro un processo morto dopo la chiusura, cosa che succede spesso, Hoard capisce che il gioco non c'è più e va avanti.

### Dagli qualche secondo al risveglio

Quando il Deck si risveglia, il Wi-Fi impiega un attimo a tornare, e solo allora Hoard può cercare un salvataggio più recente. Se avvii un gioco in quei primi secondi, il download aspetta finché non lo chiudi. Lascialo un momento online prima di giocare.

### La microSD

Se un gioco è sulla scheda e la scheda non è inserita, Hoard non scarica un salvataggio in una cartella che non esiste. Aspetta che la scheda torni.

### Le impostazioni restano su ogni macchina

Il Deck gira a 1280×800 su una GPU da portatile da gioco. Il tuo desktop probabilmente no. Hoard salva i file di impostazioni come `graphics.ini` insieme alla partita, ma non li scrive sopra quelli dell'altra macchina, quindi il Deck tiene i suoi. Se vuoi comunque copiarli, c'è un'opzione apposta al momento del ripristino. Altro in [sincronizzare i salvataggi tra più PC](/guides/sync-game-saves-across-pcs).

### La cartella `remote` di Steam

Per i giochi Steam, il salvataggio sta in `userdata/<UserID>/<AppID>/remote/`. La cartella sopra contiene anche `remotecache.vdf` e file di tempo di gioco e obiettivi che devono essere diversi tra Deck e PC. Sincronizza a mano la cartella superiore e ogni avvio sembrerà un conflitto. Hoard segue solo `remote/`.

## Steam Cloud e Hoard insieme

Non si danno fastidio. Per un gioco con Steam Cloud, lascia che Steam continui a sincronizzarlo. Quello che aggiunge Hoard è la cronologia delle versioni, così un salvataggio rovinato su una macchina non si porta via i tuoi progressi. Per tutti gli altri giochi, Hoard si occupa anche della sincronizzazione.

## Senza i nostri server

Se preferisci tenere i salvataggi a casa, avvia `hoard-server` sul tuo PC o su un NAS e punta lì sia il Deck sia il PC. Nessun account con noi, nessuna telemetria verso di noi, niente che passi dai nostri server. Vedi [come ospitare Hoard da solo](/guides/self-host-hoard).

<!-- faq -->

## Domande frequenti

### Hoard funziona in modalità gioco?

Sì. Il motore di sincronizzazione è un servizio in background che parte con il Deck, quindi salva e ripristina senza nessuna finestra aperta. La modalità desktop serve solo per installarlo e per aggiungere cartelle a mano.

### Un aggiornamento di SteamOS lo rimuove?

No. Tutto ciò che Hoard installa sta nella tua cartella home, che gli aggiornamenti di SteamOS non toccano.

### Sincronizza i giochi di Heroic, Lutris o EmuDeck?

Sì. Hoard cerca dentro i prefissi di Heroic, Lutris e Bottles e nelle cartelle di EmuDeck. Se un gioco non viene rilevato, indica una volta la sua cartella di salvataggio e verrà seguito come gli altri.

### E se ho giocato su entrambi senza sincronizzare?

Hoard non sovrascrive mai alla cieca. Confronta le versioni, tiene una copia di ciò che sostituisce e ogni versione precedente resta nella cronologia. Non può unire due sessioni di gioco diverse in un solo salvataggio (niente può farlo), ma puoi scegliere quale tenere.

### Il Deck conta come dispositivo?

Sì. Il piano gratuito include tre dispositivi, quindi ci stanno un PC, un portatile e un Deck. Pro e i server ospitati in proprio non hanno limiti di dispositivi.

### Posso usare invece la versione da riga di comando sul Deck?

Sì. Il comando `hoard` avvia lo stesso motore senza finestra, e c'è chi lo preferisce su un portatile da gioco. Vedi [la pagina della CLI](/cli).
