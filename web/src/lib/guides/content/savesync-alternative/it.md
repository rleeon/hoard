---
title: "Alternativa a SaveSync: sincronizzazione dei salvataggi gratis e open source"
description: "Cerchi un'alternativa gratuita a SaveSync? Hoard salva e sincronizza i salvataggi di ogni gioco, non solo co-op, su PC, Linux, macOS e Steam Deck."
order: 29
updated: 2026-11-05
---

SaveSync è un'app a pagamento su Steam, in accesso anticipato, pensata per il co-op: tiene un mondo condiviso disponibile per il gruppo tramite voci private dello Steam Workshop, anche quando l'host è offline. Se quello che vuoi è sincronizzare gratis i tuoi salvataggi, per ogni gioco a cui giochi e su ogni macchina che hai, [Hoard](/download) fa questo, ed è open source.

## Cosa fa bene SaveSync

- **Mondi co-op.** Passare un mondo di Valheim o Factorio a chi ospita stasera è esattamente il lavoro per cui è nato.
- **Steam come trasporto.** I salvataggi viaggiano come voci private, non elencate e cifrate del Workshop, quindi non serve un altro account.
- **Va d'accordo con Steam Cloud.** Lavora accanto alla sincronizzazione di Steam invece di combatterla.

## Dove Hoard è diverso

| | SaveSync | Hoard |
|---|---|---|
| Prezzo | A pagamento, su Steam | Gratis e open source (AGPL-3.0) |
| Giochi | Una serie di giochi co-op supportati | Ogni gioco del database comunitario, 19 emulatori e qualsiasi cartella che aggiungi |
| Spazio | 500 MB inclusi, altro in vendita nell'app | Hoard One gratis con 2 GB e tre dispositivi, o il tuo server senza quote nostre |
| Cronologia | Secondo l'app | Ogni versione, deduplicata |
| Piattaforme | Windows | Windows, Linux, macOS e SteamOS |
| Condividere un mondo con gli amici | Sì | No |

L'ultima riga è quella onesta. Se lo scopo è passare un mondo tra amici, SaveSync è fatto per questo e Hoard no. Per tutto il resto, i tuoi salvataggi sul fisso, sul portatile e sullo Steam Deck, Hoard copre più giochi e non costa nulla.

## Come iniziare

1. Installa Hoard e accedi a Hoard One, consigliato a tutti, oppure puntalo sul [tuo server](/guides/self-host-hoard).
2. Apri la **Libreria**. I tuoi giochi vengono rilevati dai launcher e dal database comunitario dei salvataggi.
3. Gioca. Ogni salvataggio aggiunge una versione, e con Hoard sulle altre macchine con lo stesso account, il più recente ti aspetta lì. Vedi [come sincronizzare i salvataggi tra Steam Deck e PC](/guides/sync-saves-steam-deck-pc).

## Hoard One, o il tuo server

Hoard è gratuito in entrambi i casi, e l'app è la stessa:

- **Hoard One**, consigliato per tutti. Niente da configurare né da tenere acceso: un server gestito nell'UE, gratuito con 2 GB e tre dispositivi, senza carta. [Pro](/pricing) aggiunge solo spazio, 100 GB e dispositivi illimitati, per quando i tuoi salvataggi non ci stanno più.
- **Il tuo server.** Fai girare `hoard-server` su un PC, un NAS o un Raspberry Pi e tutte le tue macchine si sincronizzano attraverso quel server. Nessun account da noi, nessuna telemetria verso di noi e nessuna quota nostra: niente passa dai nostri server, perché sul percorso non c'è niente di nostro. Vedi [come fare il self-hosting di Hoard](/guides/self-host-hoard).

Stesso programma, stesso rilevamento, stessa cronologia. Hoard One conserva ogni versione per sempre; sul tuo server decidi tu quanta cronologia tenere.

<!-- faq -->

## Domande frequenti

### Esiste un'alternativa gratuita a SaveSync?

Hoard. App, riga di comando e server sono gratis e open source, e Hoard One, il server gestito, è gratis con 2 GB e tre dispositivi. Non condivide mondi tra amici, ma sincronizza i tuoi salvataggi per ogni gioco.

### Posso mandare un salvataggio a un amico con Hoard?

Non come funzione: Hoard tiene sincronizzati i tuoi salvataggi tra le tue macchine. Puoi esportare i salvataggi così come li ha scritti il gioco e mandare tu i file.

### Hoard è gratuito?

Sì. L'app, la riga di comando e il server sono gratuiti e open source (AGPL-3.0), e far girare il tuo server non costa altro che la macchina. Hoard One, il server gestito che consigliamo a tutti, è gratuito anche lui con 2 GB e tre dispositivi; Pro aggiunge spazio per 1,99 € al mese.

### Il self-hosting invia qualcosa a Hoard?

No. Sul tuo server non c'è nessun account da noi e nessuna telemetria verso di noi: i tuoi salvataggi, i tuoi utenti e i tuoi log restano sulla tua macchina e non toccano mai i nostri.
