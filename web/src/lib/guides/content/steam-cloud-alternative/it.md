---
title: "Alternativa a Steam Cloud: salva i salvataggi che Steam non copre"
description: "Steam Cloud copre solo i giochi Steam il cui sviluppatore l'ha attivato, e non tiene una cronologia. Hoard salva ogni gioco a cui giochi, da qualsiasi store, con una cronologia versionata a cui tornare — nel cloud o sul tuo server."
order: 7
updated: 2026-09-01
---

Steam Cloud fa molto bene il compito ristretto che ha, e quasi tutti ne scoprono i limiti proprio il giorno in cui perdono qualcosa. Questa guida spiega dove sono quei limiti e cosa fare con i giochi che restano fuori.

## Cosa copre davvero Steam Cloud

Steam Cloud sincronizza la cartella di un gioco quando **lo sviluppatore l'ha configurato**: dichiarando quali file sincronizzare, oppure chiamando l'API di Steam dall'interno del gioco. È tutto qui, e ne discendono tre cose:

- Funziona solo per giochi comprati e avviati tramite Steam.
- Che funzioni o no lo decide lo sviluppatore, gioco per gioco e a volte per piattaforma.
- Ogni gioco ha la sua quota di spazio, fissata da quello sviluppatore.

Quando funziona è invisibile ed eccellente: chiudi il gioco su un PC, lo apri su un altro, i progressi sono lì.

## Dove ti lascia scoperto

- **Tutto ciò che non è un gioco Steam.** GOG, Epic, itch, Battle.net, l'app Xbox, gli emulatori, qualsiasi cosa installata a mano. Steam non sa che esistono.
- **Giochi Steam dove non è mai stato attivato.** Parecchi titoli, soprattutto vecchi o piccoli, semplicemente non ce l'hanno. La pagina del negozio lo dice, ma nessuno la controlla prima di iniziare una partita da 60 ore.
- **Non si torna indietro.** Questo è il punto grosso. Steam conserva lo stato attuale del salvataggio, non la sua storia. Se il file si corrompe, se una mod ti mangia il mondo o se sovrascrivi un salvataggio buono con uno rotto, la copia nel cloud è già quella rotta. Puoi vedere i file che Steam tiene per un gioco, ma non c'è una versione precedente da ripristinare.
- **La finestra di conflitto.** Quando Steam ritiene che locale e remoto non coincidano, ti chiede di scegliere con poco più di due date davanti. Se sbagli, l'altra copia è persa.

## Cosa aggiunge Hoard

Hoard sorveglia la cartella in cui il gioco scrive davvero e cattura una **nuova versione ogni volta che smetti di giocare**:

- **Non gli importa da dove venga il gioco.** Steam, GOG, Epic, itch, emulatori o una cartella che gli indichi a mano.
- **Tutte le versioni vengono conservate**, quindi rimediare a un salvataggio corrotto o a una scelta sbagliata sono due clic e non una partita persa.
- **Sincronizza tra le tue macchine** allo stesso modo, Steam Deck e desktop inclusi.
- **Niente viene distrutto in silenzio.** Il salvataggio sostituito viene catturato prima, quindi anche un ripristino sbagliato è reversibile.

Gli snapshot sono archiviati per hash del contenuto, così dieci versioni di un salvataggio da 2 GB occupano circa 2 GB e non 20: è questo a rendere pratico conservare tutta la cronologia.

## Usarli insieme

Non litigano, e non devi scegliere. Per un gioco Steam con supporto cloud, lascia che Steam sincronizzi quello che già sincronizza; il contributo di Hoard lì è la cronologia, cioè proprio ciò che Steam non tiene. Per tutto il resto, alla sincronizzazione pensa Hoard.

Un dettaglio che conta se oltre al desktop hai una Steam Deck: Hoard traccia `<AppID>/remote/` dentro `userdata`, non la cartella superiore, perché quella contiene `remotecache.vdf` e file di obiettivi e tempo di gioco propri di ogni macchina. È la distinzione che una sincronizzazione artigianale sbaglia più spesso, ed è il motivo per cui quei setup sembrano andare in conflitto a ogni avvio.

## Quando Steam Cloud basta

Vale la pena dirlo chiaramente: se tutti i giochi a cui giochi sono giochi Steam con supporto cloud, giochi su un solo PC e non hai mai avuto bisogno di annullare un salvataggio, Steam Cloud fa già il suo e non ti serve altro. Ad aggiungere Hoard convincono la cronologia delle versioni, i giochi fuori da Steam e le macchine che Steam Cloud non raggiunge.

## Senza il cloud di nessuno

Se quello che ti attira è non dipendere da nessuna piattaforma, Hoard può girare interamente sul tuo hardware: `hoard-server` su un PC o su un NAS, e i salvataggi vanno dalla tua macchina al tuo disco. **Nessun account con noi, nessuna telemetria verso di noi e nessun relay**: non passa nulla dai nostri server, perché sul percorso non c'è niente di nostro. Vedi [come ospitare Hoard da solo](/guides/self-host-hoard).

Stesso programma, stesso rilevamento, stessa cronologia. L'unica cosa che cambia è di chi è lo spazio di archiviazione.

<!-- faq -->

## Domande frequenti

### Hoard sostituisce Steam Cloud?

Non deve per forza. Steam Cloud tiene sincronizzato il salvataggio attuale per i giochi supportati; Hoard aggiunge la cronologia e copre i giochi che non lo sono. Tenerli entrambi è normale.

### Steam Cloud può tornare a un salvataggio più vecchio?

No. Steam conserva lo stato attuale dei file, non la loro storia. Una volta che un salvataggio rotto è stato sincronizzato, è quello che sta nel cloud. Per tornare indietro serve uno strumento che versiona.

### Perché non tutti i miei giochi Steam si sincronizzano?

Perché è lo sviluppatore ad attivarlo, gioco per gioco e a volte per piattaforma. La pagina del negozio elenca Steam Cloud tra le caratteristiche quando è supportato, e molti titoli semplicemente non lo sono.

### Hoard funziona con giochi non Steam?

Sì, ed è buona parte del punto. Individua i salvataggi tramite un database comunitario che copre oltre 20.000 titoli, da qualsiasi store, e per i casi insoliti puoi indicargli la cartella a mano.

### Usarli entrambi crea conflitti?

No. Hoard cattura una versione dopo che hai smesso e la cartella si è calmata, e non sovrascrive mai senza aver prima catturato ciò che sostituisce.

### Posso tenere i salvataggi fuori da entrambi i cloud?

Sì. Ospita il server da solo: i salvataggi non lasciano mai hardware tuo, senza account e senza telemetria verso nessuno.
