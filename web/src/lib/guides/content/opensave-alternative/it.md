---
title: "Alternativa a OpenSave: diretto tra macchine o con un server tuo"
description: "OpenSave sincronizza i salvataggi direttamente tra i tuoi PC, senza nulla in mezzo. Hoard sincronizza attraverso un server — il nostro o uno tuo — e tiene una cronologia versionata. Uno sguardo onesto su quando vince ciascun approccio."
order: 8
updated: 2026-09-01
---

I due strumenti risolvono lo stesso problema e non sono d'accordo sull'architettura, che è l'unica cosa che valga la pena confrontare. Questa pagina mette i due approcci uno accanto all'altro, compresi i casi in cui l'altro è la risposta migliore.

## La differenza vera: diretto o con un server

**OpenSave** è peer-to-peer. Le tue macchine si parlano direttamente e in mezzo non c'è nulla. Nessun account e nessuno spazio da pagare, e in opzione può replicare una copia su un cloud che hai già.

**Hoard** sincronizza attraverso un server. Quel server è Hoard Cloud, gestito da noi, oppure `hoard-server` sul tuo PC o sul tuo NAS. Il salvataggio sale quando smetti di giocare e scende quando un'altra macchina lo chiede.

Tutto il resto discende da questa singola scelta.

## Cosa ti dà un server

- **L'altra macchina non deve essere accesa.** Finisci sul fisso, il portatile resta chiuso una settimana, e all'apertura l'ultimo salvataggio è lì ad aspettare. Il peer-to-peer vuole entrambi i capi svegli nello stesso momento: ottimo alla scrivania, scomodo con una portatile che prendi in mano due volte al mese.
- **Una cronologia, non solo l'ultimo stato.** Ogni sessione diventa una versione a cui tornare. È la parte che conta il giorno in cui una mod ti mangia il mondo o un salvataggio finisce scritto a metà: una sincronizzazione diretta copia fedelmente il file rotto sull'altro PC.
- **Una copia che sopravvive all'hardware.** Che entrambi i PC muoiano nella stessa casa non è uno scenario esotico. Un salvataggio esistito solo su quelle due macchine muore con loro.
- **Niente da sistemare sulla rete.** Nessun NAT da attraversare, nessuna porta da aprire, nessun vincolo di stare sulla stessa LAN.

## Cosa ti dà il peer-to-peer

Per essere onesti con l'altra parte:

- **Nessuno spazio da pagare, mai.** Non c'è quota da esaurire perché non c'è un archivio. Il piano gratuito di Hoard è 2 GB, sopra si paga o si fa self-hosting.
- **Niente in mezzo per progetto.** Se l'obiettivo è che un file non tocchi mai il disco di terzi, il trasferimento diretto è la risposta più breve possibile.
- **Niente da mandare avanti.** Nessun server da tenere in piedi, nemmeno il tuo.

Se giochi su due fissi entrambi accesi, non vuoi mai tornare indietro e preferisci non pensare allo spazio, quell'approccio calza perfettamente e Hoard è più macchinario di quanto ti serva.

## La questione privacy, detta con precisione

È qui che i confronti su Hoard di solito sbagliano, quindi siamo esatti: ci sono due modi di usarlo e sono davvero diversi.

- **Hoard Cloud** è l'opzione gestita: accedi e i salvataggi stanno sui nostri server, nell'UE.
- **Il self-hosting è interamente tuo.** Fai girare `hoard-server` sul tuo PC o NAS e i salvataggi vanno dalla tua macchina al tuo disco. **Nessun account con noi, nessuna telemetria verso di noi, nessuna quota e nessun relay**: non passa nulla dai nostri server, perché sul percorso non c'è niente di nostro. Non vediamo un salvataggio, il nome di un gioco o un indirizzo email, perché niente di tutto ciò ci arriva. Se Hoard Cloud chiudesse domani, un'installazione self-hosted continuerebbe uguale.

Quindi "server" non vuol dire "il computer di qualcun altro", a meno che tu non lo scelga. Un Hoard self-hosted tiene i salvataggi su hardware tuo, esattamente come un trasferimento diretto, e in più ti dà la cronologia e il caso della macchina spenta.

## Rilevamento e copertura

Entrambi trovano automaticamente i salvataggi di un catalogo ampio. Hoard legge lo stesso manifest comunitario delle posizioni condiviso dall'ecosistema open source, oltre 20.000 titoli, e ci aggiunge le librerie Steam, i processi in esecuzione e una scansione del disco. Per i giochi Steam traccia `<AppID>/remote/` dentro `userdata` e non la cartella superiore, perché quella contiene `remotecache.vdf` e file di obiettivi e tempo di gioco propri di ogni macchina: sincronizzarli significa vedere un conflitto a ogni avvio. Per i casi insoliti gli indichi tu la cartella.

## Quale usare?

- **Peer-to-peer** se le tue macchine sono accese insieme, non vuoi che lo spazio entri nel discorso e l'ultimo salvataggio è tutto ciò che ti è mai servito.
- **Hoard** se vuoi una cronologia a cui tornare, una macchina che possa restare spenta una settimana e una copia che sopravviva a entrambi i PC, con la scelta tra il nostro cloud e il tuo server.

C'è un [confronto di tutti gli strumenti di sincronizzazione](/guides/game-save-sync-comparison) per il quadro completo, e un [confronto con Ludusavi](/guides/ludusavi-alternative) per il versante dei backup locali.

<!-- faq -->

## Domande frequenti

### Hoard richiede un account?

Per Hoard Cloud sì, perché la sincronizzazione è legata a quello. In self-hosting non c'è alcun account con noi: il tuo server ha i suoi utenti e un token per dispositivo, e non escono dalla tua macchina.

### Hoard può funzionare senza alcun cloud?

Sì. Fai girare `hoard-server` su un PC o un NAS e i salvataggi vanno dalla tua macchina al tuo disco, senza che nulla passi dai nostri server.

### Servono entrambi i PC online nello stesso momento?

No, ed è il vantaggio pratico di passare da un server. Il salvataggio viene caricato quando smetti di giocare e scaricato quando l'altra macchina lo richiede.

### Un trasferimento diretto tiene una cronologia?

Non di per sé: copiare un file su un'altra macchina ti dà lo stato attuale su entrambe. Hoard cattura ogni sessione come una versione, ed è questo a rendere possibile tornare indietro da un salvataggio corrotto.

### Anche Hoard è open source?

Sì, AGPL-3.0, server incluso. Il server self-hosted è lo stesso binario che usiamo noi, non un'edizione ridotta.
