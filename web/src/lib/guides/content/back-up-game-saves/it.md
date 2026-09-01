---
title: "Come fare il backup dei salvataggi automaticamente"
description: "Imposta backup cloud automatici e versionati dei tuoi salvataggi PC con Hoard — così un crash, una reinstallazione o una mod difettosa non potranno mai cancellare i tuoi progressi."
order: 1
updated: 2026-09-01
---

Perdere un salvataggio significa perdere ore di progressi. Hoard fa il backup dei tuoi salvataggi PC automaticamente e conserva una cronologia completa delle versioni, così puoi sempre tornare indietro.

## Cosa salva Hoard

Hoard rileva le cartelle di salvataggio dei giochi a cui giochi e le copia sul tuo cloud — Hoard Cloud o un server che ospiti tu stesso. Ogni backup è versionato, quindi le copie più vecchie non vengono mai sovrascritte.

Per trovare dove ogni gioco conserva i salvataggi, Hoard usa lo stesso database comunitario di posizioni che alimenta Ludusavi, quindi il rilevamento funziona da subito per migliaia di titoli. La differenza è ciò che succede dopo: invece di lasciare il backup sul disco, Hoard lo versiona automaticamente nel cloud.

## Imposta i backup automatici

1. **Scarica e installa Hoard** per Windows, macOS o Linux dalla pagina di download.
2. Accedi, oppure punta l'app al tuo server self-hosted.
3. Apri la **Libreria**. Hoard cerca i giochi installati ed elenca i salvataggi trovati.
4. Aggiungi i giochi che vuoi proteggere. Hoard individua ogni cartella di salvataggio automaticamente; puoi aggiungere un percorso a mano se un gioco non viene rilevato.
5. Lascia attiva la **modalità automatica**. Hoard sorveglia le cartelle di salvataggio e fa il backup dopo che smetti di giocare.

Da ora ogni sessione viene catturata senza che tu faccia nulla.

## Dove i giochi PC tengono davvero i salvataggi

Non esiste un posto solo, ed è esattamente il motivo per cui uno strumento così esiste. Nella pratica un salvataggio finisce in uno di questi punti:

- **Dentro Steam**, in `userdata/<UserID>/<AppID>/remote/`, la cartella che Steam Cloud sincronizza per conto suo.
- **`Documenti\My Games\…`**, la cosa più simile a una convenzione che Windows abbia.
- **`%APPDATA%`, `%LOCALAPPDATA%` o `LocalLow`**, dove scrive la maggior parte dei giochi Unity e Unreal.
- **`%USERPROFILE%\Saved Games`**, usata da un gruppo più ristretto ma testardo di titoli.
- **La cartella di installazione del gioco**, dove sorprendentemente molti titoli vecchi salvano ancora.
- **Su Linux**, `~/.local/share` o `~/.config` per i giochi nativi, e dentro il prefisso Proton — `steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…` — per quelli Windows.
- **Su macOS**, `~/Library/Application Support`.

Da dove arrivi il gioco conta poco: i titoli GOG, Epic e itch finiscono negli stessi pochi posti, perché a decidere sono il motore e lo sviluppatore, non il negozio.

## Cosa viene salvato e cosa no

Una cartella di salvataggi contiene raramente solo salvataggi, quindi Hoard divide ciò che trova in tre mucchi:

- **I dati di salvataggio** vengono copiati e ripristinati. Quelli sono i tuoi progressi.
- **I file che appartengono a una macchina specifica** — configurazione, log e simili — vengono caricati per far parte del backup, ma mai riscritti sopra la copia di un altro PC. Le tue impostazioni grafiche restano tue.
- **La spazzatura** — cache, dump dei crash, temporanei — viene ignorata, così un backup non si gonfia con roba che non rivorresti mai.

## Quando avviene il backup

Hoard sorveglia la cartella e la cattura **dopo che smetti di giocare**, non mentre il gioco tiene i file aperti. Se il salvataggio è stato scritto pochi secondi fa, aspetta che tutto si calmi: un file in scrittura non è un file da catturare a metà.

Ogni cattura è una versione. Gli snapshot sono archiviati per hash del contenuto, quindi un file invariato viene salvato una volta sola: dieci versioni di un salvataggio da 2 GB occupano circa 2 GB, non 20.

## Backup senza passare dai nostri server

Se preferisci non usare il cloud di nessuno, fai girare `hoard-server` per conto tuo e punta l'app lì. I salvataggi vanno dal tuo PC al tuo disco: nessun account con noi, nessuna telemetria verso di noi e niente che passi dai nostri server. Vedi [come ospitare Hoard da solo](/guides/self-host-hoard).

## Suggerimento: controlla la cronologia

Apri la scheda **Cronologia** di un gioco per vedere ogni backup con data e dimensione. Da lì puoi ripristinare qualsiasi versione precedente con un clic. I tuoi salvataggi viaggiano cifrati, sono archiviati nell'UE, e puoi esportarli o eliminarli quando vuoi.

Usi già uno strumento di backup locale come Ludusavi? Puoi tenerlo — ma se vuoi che quei backup finiscano nel cloud e si sincronizzino tra le macchine senza scriptare Rclone a mano, è esattamente ciò che Hoard automatizza. Vedi [Ludusavi vs Hoard](/guides/ludusavi-alternative) per un confronto equo.

<!-- faq -->

## Domande frequenti

### Hoard fa backup mentre gioco?

No. Aspetta che tu smetta e che la cartella dei salvataggi si calmi, così un backup non è mai un file scritto a metà.

### Quanto spazio occupano i miei salvataggi?

Meno di quanto pensi. Le versioni sono deduplicate per hash del contenuto, quindi occupa spazio nuovo solo ciò che è davvero cambiato tra una sessione e l'altra: quasi tutte le collezioni stanno comode in un paio di gigabyte.

### E se uno dei miei giochi non viene rilevato?

Punta Hoard alla cartella a mano e la traccerà come qualsiasi altra. Il rilevamento copre migliaia di titoli, ma un gioco che salva in un posto insolito, o installato a mano, a volte ha bisogno dell'indizio.

### Fa il backup anche delle mod?

Hoard traccia la cartella dei salvataggi, quindi le mod che stanno altrove non entrano nel backup. È voluto: le mod sono grandi, si riscaricano, e una cartella di mod sincronizzata tra macchine crea più problemi di quanti ne risolva.

### Il self-hosting cambia il funzionamento dei backup?

Per niente. Stesso rilevamento, stesse versioni, stessa cattura automatica. L'unica cosa tua è lo spazio di archiviazione.
