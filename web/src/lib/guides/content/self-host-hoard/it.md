---
title: "Come self-hostare Hoard con Docker"
description: "Avvia il tuo server Hoard in pochi minuti con Docker Compose. Open source, gratuito, sul tuo hardware: un cloud completamente self-hosted per i salvataggi dei giochi, senza account né limiti di spazio."
order: 0
featured: true
updated: 2026-09-29
---

Hoard è open source e self-hostabile. Invece di usare Hoard Cloud, puoi eseguire lo stesso `hoard-server` sulla tua macchina e puntarci ogni dispositivo — senza account e senza limiti di spazio oltre al disco che gli dai. Questa guida mette in piedi un server con Docker in pochi minuti.

## Perché self-hostare Hoard

- **Controllo totale.** I tuoi salvataggi vivono su hardware che controlli tu, non sul cloud altrui.
- **Nessun limite.** Lo spazio è limitato solo dal tuo disco.
- **Stessa app, stesse funzioni.** Cronologia versionata e sync in background funzionano come con Hoard Cloud — cambia solo il backend.
- **Open source.** Puoi leggere, verificare e modificare il server.

È la differenza chiave rispetto a strumenti come [Ludusavi](/guides/ludusavi-alternative): Ludusavi è ottimo per i backup locali e per il cloud «porta il tuo» tramite Rclone, ma la sincronizzazione la configuri tu. Hoard ti dà un server di sync gestito che avvii una volta e a cui si collega ogni dispositivo.

## Cosa significa il self-hosting per i tuoi dati

Vale la pena dirlo chiaramente, perché è il punto su cui quasi tutti i confronti sbagliano riguardo a Hoard.

**Hoard Cloud** è l'opzione gestita: accedi e i tuoi salvataggi stanno sui nostri server, nell'UE.

**Un Hoard self-hosted è interamente tuo.** I tuoi dispositivi parlano con il tuo server e con nient'altro. **Nessun account con noi, nessuna telemetria verso di noi, nessuna quota e nessun relay**: non passa nulla dai nostri server, perché sul percorso non c'è niente di nostro. Non possiamo vedere un salvataggio, il nome di un gioco o un indirizzo email, per il semplice motivo che niente di tutto ciò ci arriva. Se Hoard Cloud chiudesse domani, la tua installazione andrebbe avanti identica.

Una precisazione, per essere esatti: il tuo server ha eccome i suoi accessi — l'utente che crei più sotto e un token per dispositivo. Sono tuoi, sulla tua macchina, nel tuo database. Quello che non esiste è un account con noi.

## Cosa ti serve

- Una macchina sempre accesa (un server domestico, un NAS che esegue Docker o un piccolo VPS).
- Docker e Docker Compose installati (su un NAS Synology, il pacchetto Container Manager).
- Facoltativamente un dominio e un reverse proxy per l'HTTPS (consigliato per tutto ciò che esce dalla rete locale).

## Installazione con Docker Compose

Non serve clonare il repository. Il server è un'immagine già pronta (`ghcr.io/rleeon/hoard`, amd64 e arm64), e l'unico file da scaricare è il suo `docker-compose.yml`:

```sh
mkdir hoard && cd hoard
curl -O https://raw.githubusercontent.com/rleeon/hoard/main/deploy/docker/docker-compose.yml

# Used only on the first start: they create that admin and print a device token in the log, once
HOARD_ADMIN_USERNAME=alice HOARD_ADMIN_PASSWORD='CHANGE_ME' docker compose up -d
docker compose logs -f server                  # wait for "listening", then copy the token
```

Al primo avvio il container scrive un `config.toml` funzionante in `./config/`, accanto al file compose, quindi con un semplice `docker compose` non c'è niente da preparare. I dati vivono in un volume Docker (`hoard-data`): eseguine il backup come per qualsiasi volume. Il container ascolta internamente sulla porta `12421`; usa un'altra porta host con `HOARD_PORT=9000 docker compose up -d`.

Preferisci leggere la configurazione prima che parta qualcosa, o compilare l'immagine da te? La [guida al self-hosting nel repository](https://github.com/rleeon/hoard/blob/main/SELF-HOST_GUIDE.md) spiega come clonarlo.

### Su un NAS Synology (Container Manager)

Container Manager non ha una riga di comando da cui passare quelle due variabili, e si rifiuta di avviare un progetto finché manca una cartella montata: è l'errore `Bind mount failed: '…/config' does not exist`. Quattro passaggi risolvono entrambe le cose:

1. In File Station, crea una cartella per Hoard (per esempio `docker/hoard`) e, al suo interno, una cartella `config` vuota.
2. In Container Manager, apri **Project** → **Create**, imposta quella cartella come percorso, scegli di creare un `docker-compose.yml` e incolla il contenuto del file (per ottenerlo, apri nel browser l'URL della riga `curl` qui sopra).
3. Nel file incollato, sostituisci `${HOARD_ADMIN_USERNAME:-}` e `${HOARD_ADMIN_PASSWORD:-}` con nome utente e password del tuo amministratore, lasciando il resto di ogni riga com'è, in modo che le due righe diventino:

   ```yaml
   HOARD_ADMIN_USERNAME: alice
   HOARD_ADMIN_PASSWORD: 'CHANGE_ME'
   ```

4. Completa la procedura guidata per avviarlo, poi apri il log di `hoard-server` in **Container**: il token del dispositivo è stampato lì, una sola volta.

Con il file così com'è, i salvataggi vivono nello spazio di archiviazione di Docker, fuori dalle tue cartelle condivise. Per tenerli in una cartella condivisa di cui fai già il backup, crea anche una cartella `data` accanto a `config` e cambia la riga `hoard-data:/var/lib/hoard` in `./data:/var/lib/hoard`.

## Crea il tuo utente e un token dispositivo

Se l'hai avviato con `HOARD_ADMIN_USERNAME` e `HOARD_ADMIN_PASSWORD`, è già fatto: l'utente esiste e il suo token è nel log. Altrimenti creali da riga di comando, perché il server non ha una schermata di registrazione:

```sh
docker compose exec server hoard-admin --config /etc/hoard/config.toml \
    user create alice --admin --password 'CHANGE_ME'
docker compose exec server hoard-admin --config /etc/hoard/config.toml \
    token create alice --device 'desktop'
```

Il token viene mostrato una sola volta e **non può essere recuperato in seguito**, quindi copialo ora. Per ogni dispositivo che aggiungi in seguito non serve un terminale: apri l'indirizzo del server nel browser, accedi al pannello web con quel nome utente e quella password e usa **Utenti** → **Nuovo token**.

## Collega l'app desktop

Installa l'[app desktop di Hoard](/download) su ogni macchina. Nella procedura iniziale scegli **Self-Host**, poi incolla l'URL del server e il token appena creato. Da lì si comporta esattamente come Hoard Cloud: rileva i giochi, salva automaticamente e mantiene la cronologia versionata. Vedi [sincronizzare i salvataggi tra più PC](/guides/sync-game-saves-across-pcs) per l'uso quotidiano.

## Tieni aggiornato il tuo server

Come si aggiorna dipende da come l'hai installato, e sbagliare comando non dà errore: semplicemente non fa nulla. Vale la pena sapere qual è il tuo caso.

**Docker Compose.** Scarica la nuova immagine e ricrea il container. Entrambe le metà, in quest'ordine:

```sh
docker compose pull
docker compose up -d
```

Se ti fermi alla prima, il vecchio container continua a girare intatto: `/v1/health` riporta ancora la versione precedente e l'aggiornamento sembra fallito in silenzio. `git pull` non aggiorna né l'uno né l'altro: quello che gira è l'immagine pubblicata, non la tua copia del repository. Fissa una versione (`ghcr.io/rleeon/hoard:1.1`) al posto di `:latest` se preferisci scegliere tu quando ne arriva una nuova.

**Unraid.** Scheda *Docker* → Hoard → *Apply update* quando compare. Niente da digitare.

**Bare metal (systemd).** `sudo hoard-server upgrade`, poi `sudo systemctl restart hoard-server`. Sostituisce il binario in modo atomico e di proposito non riavvia il servizio da solo, per non troncare una sincronizzazione in corso.

`hoard-server upgrade` vale solo per l'installazione bare metal. Dentro un container si rifiuta di proposito — la sostituzione del binario non sopravvivrebbe al prossimo `docker compose up -d` — e stampa i due comandi qui sopra; esegui `docker compose exec server hoard-server upgrade` se vuoi sentirglielo dire. Le migrazioni del database le applica il server all'avvio, quindi non c'è mai un passaggio separato.

## In produzione

Per tutto ciò che è esposto oltre la rete locale, termina il TLS su un reverse proxy (Caddy, nginx o Traefik). Preferisci il bare metal? Il repository include anche uno script di installazione `systemd` e un comando `hoard-server upgrade` che sostituisce il binario in modo atomico senza interrompere una sync in corso.

## Self-host o Hoard Cloud?

Il self-hosting è ideale se hai già un server e vuoi controllo totale senza limiti. Se preferisci non gestire infrastruttura, [Hoard Cloud](/pricing) ti dà la stessa sincronizzazione gestita da noi, con un piano gratuito per iniziare. In ogni caso app e salvataggi restano portabili: puoi cambiare in seguito.

<!-- faq -->

## Domande frequenti

### Un Hoard self-hosted comunica con voi?

No. L'app desktop parla con l'indirizzo del server che le indichi tu. I tuoi salvataggi, i tuoi utenti e i tuoi log restano sulla tua macchina, e niente di tutto ciò ci arriva.

### Il server self-hosted è lo stesso codice di Hoard Cloud?

Sì, lo stesso binario `hoard-server`, sotto AGPL-3.0. Non c'è una community edition ridotta né funzioni tenute da parte per la versione ospitata.

### Dove finiscono davvero i salvataggi?

Per impostazione predefinita nel volume Docker che assegni al container, sul tuo disco. Se hai già uno storage a oggetti, il server parla anche S3: MinIO, Garage o Backblaze B2 vanno bene come archivio. In ogni caso i tuoi dispositivi parlano soltanto con il tuo server.

### Posso farlo girare su un NAS?

Sì, su qualsiasi NAS che esegua Docker. Su Synology segui i passaggi per Container Manager qui sopra; per Unraid il repository include un template. In entrambi i casi l'immagine scende ai `PUID`/`PGID` che indichi, così le cartelle montate risultano dell'utente giusto e non di root.

### Servono un dominio e HTTPS?

Sulla tua rete locale no. Non appena il server è raggiungibile dall'esterno, mettici davanti un reverse proxy e termina lì il TLS: vanno bene Caddy, nginx o Traefik.

### E se il server è spento quando smetto di giocare?

Lo snapshot viene preso in locale, quindi non si perde nulla. Sale da solo appena il server torna a rispondere.

### Posso iniziare con Hoard Cloud e spostarmi dopo?

Sì, in entrambe le direzioni. Puoi esportare tutto dalla pagina del tuo account, e l'app può essere puntata su un altro server senza reinstallare niente.
