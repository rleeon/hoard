---
title: "Comment auto-héberger Hoard avec Docker (self-hosted)"
description: "Lancez votre propre serveur Hoard en quelques minutes avec Docker Compose. Open source, gratuit, sur votre matériel : un cloud entièrement auto-hébergé pour vos sauvegardes de jeux, sans compte ni quota."
order: 0
featured: true
updated: 2026-09-03
---

Hoard est open source et auto-hébergeable. Au lieu d'utiliser Hoard Cloud, vous pouvez exécuter le même `hoard-server` sur votre propre machine et y connecter chaque appareil — sans compte, sans quota au-delà du disque que vous lui donnez. Ce guide met un serveur en route avec Docker en quelques minutes.

## Pourquoi auto-héberger Hoard

- **Maîtrise totale.** Vos sauvegardes vivent sur du matériel que vous contrôlez, pas sur le cloud d'un autre.
- **Aucun quota.** L'espace n'est limité que par votre propre disque.
- **Même app, mêmes fonctions.** L'historique versionné et la synchro en arrière-plan fonctionnent comme avec Hoard Cloud — seul le backend change.
- **Open source.** Vous pouvez lire, auditer et modifier le serveur.

C'est la différence clé avec des outils comme [Ludusavi](/guides/ludusavi-alternative) : Ludusavi est excellent pour les sauvegardes locales et le cloud « apportez le vôtre » via Rclone, mais c'est à vous de câbler la synchro. Hoard vous donne un serveur de synchro géré que vous lancez une fois et auquel chaque appareil se connecte.

## Ce que l'auto-hébergement veut dire pour vos données

Autant le dire franchement, car c'est là que presque toutes les comparaisons se trompent au sujet de Hoard.

**Hoard Cloud** est l'option gérée : vous vous connectez, et vos sauvegardes se trouvent sur nos serveurs, dans l'UE.

**Un Hoard auto-hébergé est entièrement le vôtre.** Vos appareils parlent à votre serveur et à rien d'autre. Il n'y a **aucun compte chez nous, aucune télémétrie vers nous, aucun quota et aucun relais** : rien ne passe par nos serveurs, puisque rien de chez nous n'est sur le chemin. Nous ne pouvons voir ni une sauvegarde, ni un nom de jeu, ni une adresse e-mail, pour la simple raison que rien de tout cela ne nous parvient. Si Hoard Cloud fermait demain, votre installation continuerait à l'identique.

Une précision, pour être exact : votre serveur a bel et bien ses propres accès — l'utilisateur que vous créez plus bas, et un jeton par appareil. Ils sont à vous, sur votre machine, dans votre base. Ce qui n'existe pas, c'est un compte chez nous.

## Ce qu'il vous faut

- Une machine qui reste allumée (serveur maison, NAS exécutant Docker ou petit VPS).
- Docker et Docker Compose installés.
- Éventuellement un nom de domaine et un reverse proxy pour le HTTPS (recommandé au-delà de votre réseau local).

## Installation avec Docker Compose

Clonez le dépôt, créez une configuration depuis l'exemple et démarrez la pile :

```sh
git clone https://github.com/rleeon/hoard.git && cd hoard
mkdir -p deploy/docker/config
cp deploy/config.toml.example deploy/docker/config/config.toml
$EDITOR deploy/docker/config/config.toml      # Use nano or vim or something lol

cd deploy/docker
docker compose up -d
docker compose logs -f                         # wait for "listening"
```

Attendez que les logs indiquent que le serveur écoute. Les données vivent dans un volume Docker nommé (`hoard-data`) — sauvegardez-le comme n'importe quel volume. Le conteneur écoute en interne sur le port `12421` ; choisissez un autre port hôte avec `HOARD_PORT=9000 docker compose up -d`.

## Créez votre utilisateur et un jeton d'appareil

Le serveur n'a pas d'écran d'inscription — vous créez les utilisateurs en ligne de commande :

```sh
docker compose exec server hoard-admin --config /etc/hoard/config.toml \
    user create alice --admin --password 'CHANGE_ME'
docker compose exec server hoard-admin --config /etc/hoard/config.toml \
    token create alice --device 'desktop'
```

Le jeton n'est affiché qu'une fois et **ne peut pas être récupéré ensuite**, copiez-le maintenant.

## Connectez l'application de bureau

Installez l'[app de bureau Hoard](/download) sur chaque machine. Dans l'assistant, choisissez **Self-Host**, puis collez l'URL de votre serveur et le jeton que vous venez de créer. Ensuite, le comportement est identique à Hoard Cloud : détection des jeux, sauvegarde automatique et historique versionné. Voir [synchroniser ses sauvegardes entre PC](/guides/sync-game-saves-across-pcs) pour l'usage quotidien.

## Gardez votre serveur à jour

La façon de mettre à jour dépend de la façon dont vous l'avez installé, et se tromper de commande ne produit pas d'erreur : cela ne fait tout simplement rien. Autant savoir laquelle est la vôtre.

**Docker Compose.** Récupérez la nouvelle image et recréez le conteneur. Les deux moitiés, dans cet ordre :

```sh
docker compose pull
docker compose up -d
```

Si vous vous arrêtez à la première, l'ancien conteneur continue de tourner intact : `/v1/health` annonce toujours l'ancienne version et la mise à jour a l'air d'avoir échoué en silence. `git pull` ne met à jour ni l'un ni l'autre — ce qui tourne, c'est l'image publiée, pas votre copie du dépôt. Épinglez une version (`ghcr.io/rleeon/hoard:1.1`) au lieu de `:latest` si vous préférez choisir quand une nouvelle arrive.

**Unraid.** Onglet *Docker* → Hoard → *Apply update* quand une mise à jour est proposée. Rien à taper.

**Bare metal (systemd).** `sudo hoard-server upgrade`, puis `sudo systemctl restart hoard-server`. La commande remplace le binaire de façon atomique et ne redémarre volontairement pas le service elle-même, pour ne pas couper une synchro en cours.

`hoard-server upgrade` ne concerne que l'installation bare metal. Dans un conteneur, elle refuse volontairement — le remplacement du binaire ne survivrait pas au prochain `docker compose up -d` — et affiche les deux commandes ci-dessus ; lancez `docker compose exec server hoard-server upgrade` si vous voulez le constater. Les migrations de base de données sont appliquées par le serveur au démarrage : il n'y a jamais d'étape séparée pour elles.

## En production

Pour tout ce qui dépasse votre réseau local, terminez le TLS sur un reverse proxy (Caddy, nginx ou Traefik). Plutôt bare metal ? Le dépôt fournit aussi un script d'installation `systemd` et une commande `hoard-server upgrade` qui remplace le binaire de façon atomique sans interrompre une synchro en cours.

## Auto-hébergement ou Hoard Cloud ?

L'auto-hébergement est idéal si vous avez déjà un serveur et voulez un contrôle total sans quota. Si vous préférez ne pas gérer d'infrastructure, [Hoard Cloud](/pricing) vous offre la même synchro gérée pour vous, avec une offre gratuite pour démarrer. Dans les deux cas, l'app et vos sauvegardes restent portables — vous pouvez changer plus tard.

<!-- faq -->

## Questions fréquentes

### Un Hoard auto-hébergé communique-t-il avec vous ?

Non. L'application de bureau parle à l'adresse de serveur que vous lui donnez. Vos sauvegardes, vos utilisateurs et vos journaux restent sur votre machine, et rien de tout cela ne nous parvient.

### Le serveur auto-hébergé est-il le même code que Hoard Cloud ?

Oui, le même binaire `hoard-server`, sous AGPL-3.0. Il n'y a pas d'édition communautaire allégée ni de fonction réservée à la version hébergée.

### Où sont réellement stockées les sauvegardes ?

Par défaut dans le volume Docker que vous donnez au conteneur, sur votre propre disque. Si vous avez déjà du stockage objet, le serveur parle aussi S3 : MinIO, Garage ou Backblaze B2 font l'affaire. Dans tous les cas, vos appareils ne parlent qu'à votre serveur.

### Puis-je le faire tourner sur un NAS ?

Oui, sur n'importe quel NAS qui exécute Docker. Le dépôt fournit un modèle Unraid, et l'image bascule sur les `PUID`/`PGID` que vous indiquez, pour que les dossiers montés appartiennent au bon utilisateur plutôt qu'à root.

### Ai-je besoin d'un domaine et de HTTPS ?

Pas sur votre réseau local. Dès que le serveur est joignable de l'extérieur, placez un reverse proxy devant et terminez-y le TLS : Caddy, nginx ou Traefik conviennent.

### Et si mon serveur est éteint quand j'arrête de jouer ?

L'instantané est pris localement, rien n'est perdu. Il s'envoie tout seul dès que le serveur répond à nouveau.

### Puis-je commencer sur Hoard Cloud et migrer plus tard ?

Oui, dans les deux sens. Vous pouvez tout exporter depuis la page de votre compte, et l'application peut pointer vers un autre serveur sans réinstallation.
