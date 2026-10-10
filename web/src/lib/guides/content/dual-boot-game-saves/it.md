---
title: "Sincronizzare i salvataggi tra Windows e Linux su un PC in dual boot"
description: "Un PC, due sistemi, due cartelle di salvataggio. Sincronizza i salvataggi tra Windows e Linux in automatico, perché una cartella NTFS condivisa si rompe."
order: 18
updated: 2026-10-09
---

Su un PC in dual boot, lo stesso gioco tiene due salvataggi separati: uno nella tua cartella utente di Windows e uno dentro un prefisso Proton su Linux. Steam Cloud li collega nei giochi che lo supportano; tutto il resto si separa la prima volta che cambi sistema. Hoard li tiene sincronizzati in automatico. Installalo su entrambi i sistemi con lo stesso account, e il salvataggio di ogni gioco ti segue qualunque sistema avvii.

## Perché un gioco ha due salvataggi

Il disco è lo stesso, ma le cartelle dei salvataggi no:

- **Su Windows**, un gioco scrive in `Documents`, `Saved Games` o in una delle cartelle `AppData` sotto `C:\Users\<tu>`.
- **Su Linux**, lo stesso gioco Windows gira con Proton e scrive nel suo prefisso: `~/.local/share/Steam/steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/`, seguito dallo stesso percorso di Windows.

Due copie di un salvataggio, in due sistemi che non sono mai accesi insieme. Ovunque tu abbia giocato per ultimo, l'altro sistema non lo sa. Hoard cerca in entrambi i posti e li abbina per gioco, così il salvataggio di Windows e quello di Linux diventano due versioni di un'unica cronologia.

## Cosa copre già Steam Cloud

Per i giochi Steam con Steam Cloud, Steam sincronizza da solo il salvataggio tra l'installazione Windows e quella Proton. Lì Hoard aggiunge la cronologia: Steam tiene solo il salvataggio attuale, quindi uno rovinato sostituisce quello buono su entrambi i sistemi. Per i giochi senza Steam Cloud, e per tutto ciò che sta fuori da Steam, Hoard si occupa anche della sincronizzazione.

## Perché non condividere semplicemente una cartella sul disco di Windows?

È la prima idea di quasi tutti: far puntare Linux ai salvataggi della partizione Windows e finita lì. Di solito si rompe in tre modi:

- **Avvio rapido e ibernazione.** Quando Windows si spegne con l'avvio rapido attivo, lascia la sua partizione mezza ibernata, e Linux la monta in sola lettura o si rifiuta. Il tuo gioco non può scrivere il salvataggio.
- **NTFS sotto Proton.** Usare prefissi Proton o librerie Steam da un disco NTFS è una fonte nota di problemi di permessi e nomi dei file. I giochi su Linux stanno meglio su un file system Linux.
- **I collegamenti vengono sostituiti.** Collegare la cartella dei salvataggi di un sistema dentro l'altro funziona finché un gioco, un aggiornamento o una reinstallazione non sostituisce in silenzio il collegamento con una cartella vera.

Lasciare che ogni sistema tenga i salvataggi dove il gioco se li aspetta, e sincronizzarli tra loro, evita tutti e tre i problemi.

## Configurazione

1. **Su Windows**, installa Hoard e accedi.
2. **Su Linux**, installa Hoard dalla [pagina di download](/download) e accedi con lo stesso account.
3. **Avvia una volta ogni gioco Proton su Linux**, così esiste il suo prefisso. Prima non c'è nessuna cartella dove mettere il salvataggio.
4. Controlla la **Libreria** su entrambi i sistemi: devono comparire gli stessi giochi, e Hoard li abbina per gioco.

## La trappola che ha solo il dual boot

Con due PC separati, il salvataggio aspetta sul server finché l'altra macchina non lo chiede. Su un PC in dual boot, l'"altra macchina" è lo stesso computer dopo un riavvio, e questo cambia un'abitudine.

Hoard carica un salvataggio quando il gioco si è chiuso e la cartella è ferma. **Se esci dal gioco e riavvii subito, il caricamento potrebbe non essere ancora avvenuto**, e l'altro sistema parte senza i tuoi ultimi progressi. Si rimetterà in pari la prossima volta che riavvii nel primo, ma nel frattempo potresti aver giocato sul vecchio salvataggio.

Quindi: esci dal gioco, lascia un momento a Hoard, controlla nell'app che il salvataggio sia aggiornato, e poi riavvia.

## Versioni native Linux

Alcuni giochi hanno una versione nativa Linux oltre a quella Windows. Le due non usano sempre lo stesso formato di salvataggio, e alcune li tengono in posti completamente diversi. Se vuoi lo stesso salvataggio su entrambi i sistemi, la strada più sicura è avviare anche su Linux la versione Windows con Proton: in Steam, **Proprietà → Compatibilità** e forza una versione di Proton. Così entrambi i sistemi eseguono lo stesso gioco e scrivono gli stessi file.

## Impostazioni e dispositivi

Le impostazioni grafiche possono essere diverse tra i due sistemi, quindi Hoard salva i file di impostazioni come `graphics.ini` ma non li scrive sopra quelli dell'altro sistema. Se vuoi comunque copiarli, c'è un'opzione apposta al momento del ripristino.

Ogni sistema operativo conta come un dispositivo a sé, quindi un PC in dual boot usa due dei tre dispositivi del piano gratuito. Pro e i server ospitati in proprio non hanno limiti di dispositivi.

Preferisci tenere i salvataggi a casa? Avvia `hoard-server` su un NAS o su un'altra macchina e punta lì entrambi i sistemi. Nessun account con noi, nessuna telemetria verso di noi, niente che passi dai nostri server. Vedi [come ospitare Hoard da solo](/guides/self-host-hoard).

<!-- faq -->

## Domande frequenti

### Steam Cloud sincronizza tra Windows e Linux?

Sì, per i giochi che lo supportano: Steam tiene una copia cloud per account, su qualunque sistema giochi. Non tiene una cronologia, e non copre i giochi senza Steam Cloud né ciò che sta fuori da Steam.

### Posso tenere i salvataggi sulla partizione NTFS condivisa?

Non è consigliato. L'avvio rapido può lasciare la partizione in sola lettura su Linux, e Proton ha problemi noti con NTFS. È più affidabile che ogni sistema tenga i suoi salvataggi al suo posto e sincronizzarli.

### Perché il mio salvataggio non c'era dopo il riavvio?

Molto probabilmente il caricamento non era finito quando hai riavviato. Riavvia nel primo sistema, lascia che Hoard carichi e controlla l'app prima di cambiare di nuovo.

### Un PC in dual boot conta come un solo dispositivo?

No, come due: ogni sistema operativo si registra come dispositivo a sé. Nel piano gratuito sono due su tre; Pro e i server ospitati in proprio non hanno limiti.

### E se un gioco ha una versione nativa Linux?

I suoi salvataggi potrebbero non coincidere con quelli della versione Windows. Per condividere un salvataggio, avvia anche su Linux la versione Windows con Proton.
