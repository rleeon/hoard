---
title: "Alternative à OpenCloudSaves : il est archivé, voici quoi utiliser à la place"
description: "OpenCloudSaves est archivé depuis janvier 2025. Hoard est un remplaçant maintenu et open source qui synchronise tout seul et garde chaque version."
order: 28
updated: 2026-11-05
---

OpenCloudSaves a été archivé le 20 janvier 2025. Le code et la dernière version sont toujours là, mais le projet n'accepte plus de changements, donc rien ne le corrigera pour les nouveaux jeux, les nouveaux launchers ou les mises à jour du système. S'il synchronisait vos sauvegardes via un cloud, [Hoard](/download) est un remplaçant gratuit et open source, activement maintenu, qui trouve vos sauvegardes tout seul et les synchronise chaque fois que le jeu en écrit une.

## Ce qu'OpenCloudSaves faisait bien

- **Votre propre cloud.** Les sauvegardes allaient dans le OneDrive, Google Drive, Dropbox ou Nextcloud que vous aviez déjà, via Rclone, avec une vraie interface au lieu de fichiers de configuration.
- **Prudent par défaut.** Il faisait un essai à blanc avant chaque synchro, pour voir ce qui allait changer avant que ça change.
- **Filtres et jeux personnalisés.** Vous pouviez ne synchroniser que certains fichiers, laisser de côté les réglages graphiques et définir des jeux vous-même, même d'une boutique à l'autre.
- **Tous les bureaux.** Windows, macOS et Linux, Steam Deck compris.

## Ce qu'il faut à un remplaçant

- **Quelqu'un qui le maintient.** Les emplacements de sauvegarde bougent et les launchers changent ; un outil de synchro que personne ne met à jour finit par synchroniser le mauvais dossier.
- **Un historique, pas seulement la dernière copie.** Un cloud synchronise des fichiers, donc une sauvegarde corrompue atteint chaque machine aussi fidèlement qu'une bonne.
- **Pas de fichiers à moitié écrits.** Une synchro généraliste peut attraper une sauvegarde pendant que le jeu l'écrit encore ; le [guide OneDrive](/guides/onedrive-game-saves) montre ce que ça casse.

## Hoard comme remplaçant

| | OpenCloudSaves | Hoard |
|---|---|---|
| Maintenu | Archivé depuis janvier 2025 | Oui, open source (AGPL-3.0) |
| Trouver les sauvegardes | Définitions de sauvegardes, plus les dossiers que vous ajoutez | Automatique : base communautaire, Steam, Epic, GOG, préfixes Proton et Wine, 19 émulateurs |
| Quand il synchronise | Quand une synchro est lancée, après un essai à blanc | À chaque sauvegarde du jeu, et une fois de plus quand vous quittez |
| Où vivent les sauvegardes | Votre cloud | Hoard One, ou votre propre serveur sur disque local ou n'importe quel stockage compatible S3 |
| Historique | Ce que garde le cloud | Toutes les versions, dédupliquées |
| Dix versions d'une sauvegarde de 2 Go | Environ 20 Go de quota cloud | Environ 2 Go, car les versions partagent ce qui n'a pas changé |
| Plateformes | Windows, macOS, Linux | Windows, Linux, macOS et SteamOS |

La seule chose que Hoard ne fera pas, c'est écrire dans votre Google Drive ou votre OneDrive. Il synchronise par un serveur, et c'est ce qui lui donne l'historique et permet à l'autre machine d'être éteinte. Si vous vouliez surtout un stockage que vous contrôlez plutôt que ce cloud précis, un Hoard auto-hébergé écrit sur un NAS, un disque local ou n'importe quel bucket compatible S3 comme Backblaze B2. S'il faut absolument ce cloud, Ludusavi avec un distant Rclone est l'option maintenue la plus proche.

## Passer à Hoard

1. Avant de toucher à quoi que ce soit, copiez vos dossiers de sauvegarde en lieu sûr.
2. Arrêtez la synchro de ces dossiers dans OpenCloudSaves, pour que deux outils n'écrivent jamais dans la même sauvegarde.
3. Installez Hoard et connectez-vous à Hoard One, recommandé pour tout le monde, ou pointez-le vers [votre propre serveur](/guides/self-host-hoard).
4. Ouvrez la **Bibliothèque**. Hoard trouve vos jeux ; ajoutez à la main tout dossier personnalisé. La première version apparaît dans l'historique dès que le jeu sauvegarde.
5. Installez Hoard sur vos autres machines avec le même compte.

## Hoard One, ou votre propre serveur

Hoard est gratuit dans les deux cas, et l'application est la même :

- **Hoard One**, recommandé pour tout le monde. Rien à installer ni à faire tourner : un serveur géré dans l'UE, gratuit avec 2 Go et trois appareils, sans carte. [Pro](/pricing) ajoute seulement de la place, 100 Go et des appareils illimités, pour le jour où vos sauvegardes ne tiennent plus.
- **Votre propre serveur.** Faites tourner `hoard-server` sur un PC, un NAS ou un Raspberry Pi, et toutes vos machines se synchronisent à travers lui. Pas de compte chez nous, pas de télémétrie vers nous, pas de quota de notre part : rien ne passe par nos serveurs, puisque rien de nous n'est sur le chemin. Voir [comment héberger Hoard vous-même](/guides/self-host-hoard).

Même programme, même détection, même historique. Hoard One garde toutes les versions pour toujours ; sur votre propre serveur, c'est vous qui décidez combien d'historique garder.

<!-- faq -->

## Questions fréquentes

### OpenCloudSaves est-il encore sûr à utiliser ?

La dernière version reste disponible, mais le projet est archivé : aucun correctif pour les nouveaux jeux ou les changements du système. Il est aussi sorti en bêta, et son auteur demandait de sauvegarder ses parties avant de l'utiliser. Si vous restez dessus, gardez une copie manuelle.

### Hoard peut-il stocker les sauvegardes dans Google Drive ou OneDrive ?

Non. Hoard synchronise par un serveur : Hoard One, ou le vôtre sur disque local ou stockage compatible S3. Votre dossier `Documents` peut rester dans OneDrive ; Hoard demande à Windows où il se trouve vraiment et y trouve les sauvegardes.

### Hoard est-il gratuit ?

Oui. L'application, la ligne de commande et le serveur sont gratuits et open source (AGPL-3.0), et faire tourner votre propre serveur ne coûte que la machine. Hoard One, le serveur géré que nous recommandons à tous, est gratuit lui aussi avec 2 Go et trois appareils ; Pro ajoute de la place pour 1,99 € par mois.

### L'auto-hébergement envoie-t-il quelque chose à Hoard ?

Non. Sur votre propre serveur, il n'y a ni compte chez nous ni télémétrie vers nous : vos sauvegardes, vos utilisateurs et vos journaux restent sur votre machine et ne touchent jamais les nôtres.
