---
title: "Alternative à GameSave Manager : sauvegarde et synchro automatiques, Linux compris"
description: "Une alternative à GameSave Manager ? Hoard sauvegarde chaque partie tout seul, la synchronise entre vos PC et un Steam Deck, et tourne sous Linux et macOS."
order: 27
updated: 2026-11-05
---

Trois programmes différents s'appellent Game Save Manager, et tous les trois sauvegardent des parties sur un seul PC Windows. Si vous voulez des sauvegardes qui se font toutes seules et vous suivent sur vos autres PC, un Steam Deck ou Linux, [Hoard](/download) est gratuit, open source et fait exactement pour ça.

## Quel Game Save Manager vous avez

- **GameSave Manager**, le vétéran à code fermé pour Windows. Il a sa propre base de jeux, sauvegarde dans une seule archive à la demande ou selon un planning, et son **Sync & Link** déplace un dossier de sauvegarde dans Dropbox, OneDrive ou Google Drive en laissant un lien à sa place.
- **Game Save Manager (GSM)**, de dyang886, GPL-3.0, Windows. Il trouve les jeux installés grâce à la base de PCGamingWiki, garde pour chaque jeu une série de sauvegardes locales qui tourne, et emporte votre historique sur un nouveau PC dans une archive `.gsmr`.
- **game-save-manager**, de mcthesw, AGPL-3.0, Windows, populaire dans les communautés chinoises. Il sauvegarde selon un planning les dossiers que vous ajoutez, avec WebDAV pour une copie hors de la machine.

## Ce qu'ils font bien

- **Simples et locaux.** On choisit les jeux, on appuie sur sauvegarder, on obtient une archive. Pas de compte, rien à configurer.
- **Une bonne détection sous Windows.** De grosses bases de jeux trouvent la plupart des jeux installés sans que vous cherchiez les chemins.
- **Un déménagement propre vers un nouveau PC.** Une seule archive exportée emporte tout.

Si vous ne jouez que sur un PC Windows et qu'appuyer sur un bouton vous convient, n'importe lequel fait l'affaire.

## Là où Hoard va plus loin

| | Outils Game Save Manager | Hoard |
|---|---|---|
| Quand il sauvegarde | Quand vous appuyez sur le bouton, ou selon un planning | À chaque sauvegarde du jeu, et une fois de plus quand vous quittez |
| Synchro entre PC | Export et import, ou un dossier cloud via Sync & Link | Automatique : la sauvegarde la plus récente attend sur la machine suivante |
| L'autre PC peut être éteint | Seulement via un dossier cloud | Oui, la sauvegarde attend sur le serveur |
| Historique | Sauvegardes locales, ou ce que garde le cloud | Toutes les versions, dédupliquées, sur toutes vos machines |
| Sauvegardes à moitié écrites | Un client cloud peut synchroniser un dossier lié pendant que le jeu écrit | N'envoie jamais un fichier à moitié écrit et ne remplace jamais la sauvegarde d'un jeu en cours |
| Plateformes | Windows | Windows, Linux, macOS et SteamOS |
| Licence | Code fermé, GPL-3.0 ou AGPL-3.0 | AGPL-3.0, gratuit |

La ligne Linux est celle que les gens cherchent. Aucun des outils Game Save Manager ne tourne sous Linux ni sur un Steam Deck. Hoard si, en natif : la même application sur les quatre systèmes, avec les préfixes Proton et Wine détectés, si bien que la sauvegarde d'un jeu sur votre PC Windows et celle du même jeu sur le Deck deviennent deux versions d'un même historique.

## Passer à Hoard

1. Installez [Hoard](/download) et connectez-vous à Hoard One, recommandé pour tout le monde, ou pointez-le vers [votre propre serveur](/guides/self-host-hoard).
2. Si vous utilisiez **Sync & Link**, remettez d'abord le dossier de sauvegarde à sa place d'origine, pour qu'un client cloud et Hoard ne synchronisent pas les mêmes fichiers.
3. Ouvrez la **Bibliothèque**. Hoard trouve vos jeux grâce à la base communautaire des sauvegardes et à vos launchers ; ajoutez à la main tout dossier qui lui échappe.
4. Jouez. La première version apparaît dans l'historique dès que le jeu sauvegarde. Installez Hoard sur vos autres machines avec le même compte et la [synchro entre PC](/guides/sync-game-saves-across-pcs) se fait toute seule.

Vos anciennes archives Game Save Manager peuvent rester où elles sont, comme copie froide.

## Hoard One, ou votre propre serveur

Hoard est gratuit dans les deux cas, et l'application est la même :

- **Hoard One**, recommandé pour tout le monde. Rien à installer ni à faire tourner : un serveur géré dans l'UE, gratuit avec 2 Go et trois appareils, sans carte. [Pro](/pricing) ajoute seulement de la place, 100 Go et des appareils illimités, pour le jour où vos sauvegardes ne tiennent plus.
- **Votre propre serveur.** Faites tourner `hoard-server` sur un PC, un NAS ou un Raspberry Pi, et toutes vos machines se synchronisent à travers lui. Pas de compte chez nous, pas de télémétrie vers nous, pas de quota de notre part : rien ne passe par nos serveurs, puisque rien de nous n'est sur le chemin. Voir [comment héberger Hoard vous-même](/guides/self-host-hoard).

Même programme, même détection, même historique. Hoard One garde toutes les versions pour toujours ; sur votre propre serveur, c'est vous qui décidez combien d'historique garder.

<!-- faq -->

## Questions fréquentes

### Existe-t-il un GameSave Manager pour Linux ?

Non : GameSave Manager ne tourne que sous Windows, tout comme les deux Game Save Manager open source. Hoard tourne nativement sous Linux et SteamOS, trouve les sauvegardes dans les préfixes Proton et Wine, et les synchronise avec vos PC Windows.

### Ludusavi ou Game Save Manager ?

Ce sont tous deux des outils de sauvegarde locale. Ludusavi est open source, tourne sous Windows, Linux et macOS et s'appuie sur le manifeste communautaire des sauvegardes ; les Game Save Manager ne tournent que sous Windows. Si vous voulez sauvegarde et synchro sans câbler Rclone vous-même, Hoard utilise les mêmes données communautaires et synchronise tout seul. Il existe une [comparaison complète avec Ludusavi](/guides/ludusavi-alternative).

### Puis-je garder mes anciennes sauvegardes ?

Oui. Hoard n'y touche pas. Gardez les archives comme copie froide ; dès la première sauvegarde, l'historique de Hoard prend le relais.

### Hoard est-il gratuit ?

Oui. L'application, la ligne de commande et le serveur sont gratuits et open source (AGPL-3.0), et faire tourner votre propre serveur ne coûte que la machine. Hoard One, le serveur géré que nous recommandons à tous, est gratuit lui aussi avec 2 Go et trois appareils ; Pro ajoute de la place pour 1,99 € par mois.

### L'auto-hébergement envoie-t-il quelque chose à Hoard ?

Non. Sur votre propre serveur, il n'y a ni compte chez nous ni télémétrie vers nous : vos sauvegardes, vos utilisateurs et vos journaux restent sur votre machine et ne touchent jamais les nôtres.
