---
title: "Alternative à SaveSync : synchro de sauvegardes gratuite et open source"
description: "Une alternative gratuite à SaveSync ? Hoard sauvegarde et synchronise les parties de tous vos jeux, pas seulement en coop, sur PC, Linux, macOS et Deck."
order: 29
updated: 2026-11-05
---

SaveSync est une application payante sur Steam, en accès anticipé, pensée pour la coop : elle garde un monde partagé disponible pour le groupe grâce à des entrées privées du Steam Workshop, même quand l'hôte est hors ligne. Si ce que vous voulez, c'est synchroniser gratuitement vos propres sauvegardes, pour tous vos jeux et sur toutes vos machines, [Hoard](/download) fait ça, et il est open source.

## Ce que SaveSync fait bien

- **Les mondes en coop.** Passer un monde Valheim ou Factorio à celui qui héberge ce soir, c'est exactement le travail pour lequel il a été conçu.
- **Steam comme transport.** Les sauvegardes voyagent sous forme d'entrées Workshop privées, non listées et chiffrées, donc aucun autre compte à créer.
- **S'entend avec Steam Cloud.** Il fonctionne à côté de la synchro de Steam au lieu de se battre avec elle.

## Là où Hoard diffère

| | SaveSync | Hoard |
|---|---|---|
| Prix | Payant, sur Steam | Gratuit et open source (AGPL-3.0) |
| Jeux | Une sélection de jeux coop pris en charge | Tous les jeux de la base communautaire, 19 émulateurs et tout dossier que vous ajoutez |
| Stockage | 500 Mo inclus, davantage vendu dans l'application | Hoard One gratuit avec 2 Go et trois appareils, ou votre propre serveur sans quota de notre part |
| Historique | Selon l'application | Toutes les versions, dédupliquées |
| Plateformes | Windows | Windows, Linux, macOS et SteamOS |
| Partager un monde avec des amis | Oui | Non |

La dernière ligne est la plus honnête. Si le but est de faire passer un monde entre amis, SaveSync est fait pour ça et Hoard non. Pour tout le reste, vos propres sauvegardes sur le PC fixe, le portable et le Steam Deck, Hoard couvre plus de jeux et ne coûte rien.

## Mise en place

1. Installez Hoard et connectez-vous à Hoard One, recommandé pour tout le monde, ou pointez-le vers [votre propre serveur](/guides/self-host-hoard).
2. Ouvrez la **Bibliothèque**. Vos jeux sont détectés grâce à vos launchers et à la base communautaire des sauvegardes.
3. Jouez. Chaque sauvegarde ajoute une version, et avec Hoard sur vos autres machines avec le même compte, la plus récente vous y attend. Voir [comment synchroniser ses sauvegardes entre Steam Deck et PC](/guides/sync-saves-steam-deck-pc).

## Hoard One, ou votre propre serveur

Hoard est gratuit dans les deux cas, et l'application est la même :

- **Hoard One**, recommandé pour tout le monde. Rien à installer ni à faire tourner : un serveur géré dans l'UE, gratuit avec 2 Go et trois appareils, sans carte. [Pro](/pricing) ajoute seulement de la place, 100 Go et des appareils illimités, pour le jour où vos sauvegardes ne tiennent plus.
- **Votre propre serveur.** Faites tourner `hoard-server` sur un PC, un NAS ou un Raspberry Pi, et toutes vos machines se synchronisent à travers lui. Pas de compte chez nous, pas de télémétrie vers nous, pas de quota de notre part : rien ne passe par nos serveurs, puisque rien de nous n'est sur le chemin. Voir [comment héberger Hoard vous-même](/guides/self-host-hoard).

Même programme, même détection, même historique. Hoard One garde toutes les versions pour toujours ; sur votre propre serveur, c'est vous qui décidez combien d'historique garder.

<!-- faq -->

## Questions fréquentes

### Existe-t-il une alternative gratuite à SaveSync ?

Hoard. L'application, la ligne de commande et le serveur sont gratuits et open source, et Hoard One, le serveur géré, est gratuit avec 2 Go et trois appareils. Il ne partage pas de mondes entre amis, mais il synchronise vos propres sauvegardes pour tous vos jeux.

### Puis-je envoyer une sauvegarde à un ami avec Hoard ?

Pas en tant que fonction : Hoard garde vos propres sauvegardes synchronisées entre vos propres machines. Vous pouvez exporter vos sauvegardes telles que le jeu les a écrites et envoyer les fichiers vous-même.

### Hoard est-il gratuit ?

Oui. L'application, la ligne de commande et le serveur sont gratuits et open source (AGPL-3.0), et faire tourner votre propre serveur ne coûte que la machine. Hoard One, le serveur géré que nous recommandons à tous, est gratuit lui aussi avec 2 Go et trois appareils ; Pro ajoute de la place pour 1,99 € par mois.

### L'auto-hébergement envoie-t-il quelque chose à Hoard ?

Non. Sur votre propre serveur, il n'y a ni compte chez nous ni télémétrie vers nous : vos sauvegardes, vos utilisateurs et vos journaux restent sur votre machine et ne touchent jamais les nôtres.
