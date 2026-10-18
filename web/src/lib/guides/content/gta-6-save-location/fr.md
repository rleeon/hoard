---
title: "Emplacement des sauvegardes de GTA 6 : PS5, Xbox et PC (ce qu'on sait)"
description: "GTA 6 sort sur PS5 et Xbox le 19 novembre 2026, encore sans version PC. Où sont les sauvegardes sur console, comment les protéger et à quoi s'attendre sur PC."
order: 25
updated: 2026-10-17
---

GTA 6 sort le **19 novembre 2026, uniquement sur PS5 et Xbox Series X|S**. Il n'y a pas encore de version PC : Rockstar n'a annoncé aucune date, et la seule fenêtre qui circule vient d'un rapport d'octobre 2026, non confirmé, qui parle de début 2027. D'ici là, les sauvegardes de GTA 6 vivent sur la console et dans son cloud, pas dans un dossier que vous pouvez ouvrir. Voici comment ces sauvegardes sont conservées, comment les protéger et où les sauvegardes PC iront très probablement, à en juger par les dernières sorties PC de Rockstar. Dès que la version PC sortira, cette page donnera le vrai chemin.

## Où GTA 6 range ses sauvegardes sur console

- **PS5 :** sur la console. Avec PlayStation Plus, la PS5 envoie aussi les sauvegardes dans son stockage en ligne automatiquement. Sans abonnement, la seule copie est celle de la console.
- **Xbox Series X|S :** sur la console, synchronisées avec le cloud Xbox de votre compte Microsoft. C'est automatique et sans abonnement.

Aucune des deux n'est un dossier que vous pouvez parcourir depuis un ordinateur. La PS5 peut sauvegarder toute la console, sauvegardes de jeu comprises, sur une clé USB, mais cette copie ne se restaure que sur une PS5.

## Puis-je sauvegarder les parties de GTA 6 avec Hoard ?

Pas sur console. Hoard travaille avec des sauvegardes qui sont des fichiers sur un ordinateur : Windows, macOS, Linux et le Steam Deck. Les sauvegardes de console ne quittent jamais le système de la console, donc aucune application PC n'y a accès.

Une fois la version PC sortie, les sauvegardes de GTA 6 seront des fichiers comme pour n'importe quel autre jeu PC, et Hoard pourra les sauvegarder et les garder synchronisées entre vos PC.

## Où les sauvegardes PC seront probablement

Les dernières sorties PC de Rockstar sauvegardent toutes au même endroit, un dossier par jeu dans `Documents\Rockstar Games` :

- **GTA V Legacy :** `%USERPROFILE%\Documents\Rockstar Games\GTA V\Profiles\<ID de profil>`
- **GTA V Enhanced :** `%USERPROFILE%\Documents\Rockstar Games\GTAV Enhanced\Profiles\<ID de profil>`
- **Red Dead Redemption 2 :** `%USERPROFILE%\Documents\Rockstar Games\Red Dead Redemption 2\Profiles\<ID de profil>`

Le dernier dossier porte le nom de votre compte Rockstar, donc chaque compte a le sien. Si GTA 6 suit le même schéma, ses sauvegardes finiront dans `Documents\Rockstar Games\<nom du jeu>\Profiles\<ID de profil>`. **C'est une supposition fondée sur les précédents, pas un chemin confirmé.**

Si votre PC redirige `Documents` vers OneDrive, tout le dossier `Rockstar Games` part avec, dans `%USERPROFILE%\OneDrive\Documents\Rockstar Games`. OneDrive synchronise alors les sauvegardes de lui-même pendant que vous jouez, ce qui pose ses propres problèmes : voir [sauvegardes de jeux et OneDrive](/guides/onedrive-game-saves).

## Le cloud de Rockstar sur PC

Sur PC, GTA V et Red Dead Redemption 2 synchronisent leurs sauvegardes via le Rockstar Games Launcher, pas via Steam Cloud, même si vous les avez achetés sur Steam. Les sauvegardes dans le cloud s'activent jeu par jeu dans les paramètres du launcher. Comme Steam Cloud, il garde les sauvegardes actuelles identiques d'une machine à l'autre ; il ne vous donne pas de versions antérieures où revenir.

Rockstar n'a pas dit comment GTA 6 gérera les sauvegardes sur PC. Utiliser le même cloud du launcher serait logique, mais tant qu'il n'y a pas d'annonce, c'est une hypothèse.

## Préparez la version PC avec Hoard

[Hoard](/download) sauvegarde un dossier de sauvegardes à chaque fois que vous arrêtez de jouer et garde toutes les versions : une sauvegarde corrompue ou une mission que vous préférez refaire n'est qu'à une restauration. Il garde aussi le dossier synchronisé entre vos PC et un Steam Deck, et il fonctionne déjà avec GTA V et Red Dead Redemption 2.

Quand GTA 6 arrivera sur PC :

1. Installez Hoard et connectez-vous, ou pointez-le vers [votre propre serveur](/guides/self-host-hoard).
2. Ouvrez la **Bibliothèque**. Un jeu tout juste sorti peut mettre un moment à entrer dans la base de données communautaire des sauvegardes : si GTA 6 n'y figure pas encore, ajoutez à la main son dossier dans `Documents\Rockstar Games`.
3. Jouez. En quittant, la première version apparaît dans l'historique.

<!-- faq -->

## Questions fréquentes

### GTA 6 sort-il sur PC ?

Pas au lancement. Le 19 novembre 2026, il sort sur PS5 et Xbox Series X|S. Rockstar n'a annoncé aucune date PC ; un rapport d'octobre 2026 évoque début 2027, sans confirmation.

### Où sont les sauvegardes de GTA 6 sur PS5 ?

Sur la console, et dans le stockage en ligne de PlayStation Plus si vous êtes abonné et que l'envoi automatique est activé. Ce ne sont pas des fichiers que vous pouvez ouvrir sur un ordinateur.

### Puis-je transférer ma sauvegarde de GTA 6 de la console au PC ?

Rockstar ne l'a pas dit. Pour GTA V, il a permis de transférer les sauvegardes du mode histoire vers les nouvelles consoles via son propre cloud, mais rien de semblable n'a été annoncé pour GTA 6.

### Le dossier de sauvegardes contient-il la progression en ligne ?

Pas dans les jeux actuels de Rockstar : la progression de GTA Online et de Red Dead Online est stockée sur les serveurs de Rockstar, pas dans le dossier de sauvegardes. Il n'y a rien sur le disque à sauvegarder pour elle.

### Hoard prendra-t-il en charge GTA 6 ?

Sur PC, oui, tant qu'il sauvegarde dans un dossier comme les autres jeux PC de Rockstar. Les sauvegardes de console sont hors de portée de Hoard comme de toute autre application PC.
