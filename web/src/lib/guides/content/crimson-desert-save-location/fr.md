---
title: "Emplacement des sauvegardes de Crimson Desert (PC et Steam Deck)"
description: "Où Crimson Desert range ses sauvegardes sous Windows, sur Steam Deck et sur Mac, quel dossier les contient vraiment et comment les sauvegarder ou les transférer."
order: 21
updated: 2026-10-02
---

Sous Windows, Crimson Desert range ses sauvegardes dans `%LOCALAPPDATA%\Pearl Abyss\CD\save`. C'est le dossier qu'indique Pearl Abyss dans sa propre FAQ. Vous trouverez ci-dessous les chemins sur Steam Deck et Mac, ce qu'il contient et comment le garder sauvegardé.

## Où Crimson Desert range ses sauvegardes

- **Windows :** `%LOCALAPPDATA%\Pearl Abyss\CD\save` (soit `C:\Users\<vous>\AppData\Local\Pearl Abyss\CD\save`)
- **Steam Deck et Linux** (Proton) : `~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save`
- **Mac, version Steam :** `~/Library/Application Support/Pearl Abyss/CD/save`
- **Mac, version App Store :** `~/Library/Containers/com.pearlabyss.CrimsonDesert/Data/Library/Application Support/Pearl Abyss/CD/save`

Il n'y a pas de version Linux native : sur Steam Deck, le jeu passe par Proton et ses sauvegardes se trouvent dans le préfixe Proton que Steam garde pour lui ; `3321460` est l'identifiant Steam du jeu. S'il est sur la carte microSD, le dossier `compatdata` est sur la carte.

`AppData` est un dossier caché sous Windows. Le plus rapide est de coller `%LOCALAPPDATA%\Pearl Abyss\CD\save` dans la barre d'adresse de l'Explorateur de fichiers.

## Ce que contient le dossier

Dans `save`, il y a deux sous-dossiers. Selon Pearl Abyss, **celui au nom numérique contient les sauvegardes que vous créez en jeu**. Pour sauvegarder, prenez tout le dossier `save` plutôt que de choisir des fichiers : il est léger et vous n'oubliez rien dont le jeu a besoin.

Les versions Steam et App Store sur Mac utilisent des chemins différents. Si vous passez de l'une à l'autre, copiez les sauvegardes à la main une fois.

## Crimson Desert a-t-il des sauvegardes cloud ?

Oui, la version Steam a Steam Cloud, qui garde les dernières sauvegardes synchronisées entre les machines d'un même compte Steam.

Ce qu'il ne fait pas :

- **Garder les anciennes versions.** Steam Cloud conserve l'état actuel. Si une sauvegarde est corrompue, c'est la version corrompue qui se synchronise.
- **Couvrir les autres boutiques.** Une copie Mac App Store et une copie Steam ne partagent pas de cloud.

## Sauvegarder à la main

1. Fermez complètement le jeu.
2. Copiez tout le dossier `save` depuis le chemin ci-dessus vers un endroit sûr : un autre disque, une clé USB, un dossier cloud.
3. Pour restaurer, fermez le jeu et recopiez-le en remplaçant l'existant.

C'est bien pour une copie ponctuelle avant une grosse mise à jour ou une réinstallation. En routine, cela dépend de votre mémoire, et vous n'avez que la dernière copie.

## Sauvegarde et synchro automatiques avec Hoard

[Hoard](/download) sauvegarde le dossier à chaque fois que vous arrêtez de jouer et garde toutes les versions, pour revenir en arrière après une sauvegarde cassée ou un choix regretté. Il garde aussi le dossier synchronisé entre vos PC et un Steam Deck.

1. Installez Hoard et connectez-vous, ou pointez-le vers [votre propre serveur](/guides/self-host-hoard).
2. Ouvrez la **Bibliothèque**. Crimson Desert est détecté à partir de votre bibliothèque Steam et de la base de données communautaire des sauvegardes, au chemin ci-dessus.
3. Jouez. En quittant, la première version apparaît dans l'historique.

Ensuite, chaque session ajoute une version, et la plus récente est sur la machine où vous vous asseyez ensuite. Si une sauvegarde tourne mal, [restaurer une version antérieure](/guides/restore-a-game-save) prend deux clics.

<!-- faq -->

## Questions fréquentes

### Où sont les sauvegardes de Crimson Desert sur Steam Deck ?

Dans le préfixe Proton du jeu : `~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save`. Si le jeu est sur la carte microSD, le dossier `compatdata` est sur la carte.

### Quel sous-dossier contient mes sauvegardes ?

Celui au nom numérique, dans `save`. Sauvegardez quand même tout le dossier `save`, pour ne rien oublier.

### Je ne trouve pas le dossier AppData. Où est-il ?

Il est caché par défaut. Collez `%LOCALAPPDATA%\Pearl Abyss\CD\save` dans la barre d'adresse de l'Explorateur et appuyez sur Entrée, ou affichez les éléments masqués dans le menu Affichage.

### Puis-je jouer sur mon PC fixe et mon Steam Deck avec la même sauvegarde ?

Oui. Steam Cloud le fait pour la dernière sauvegarde sur un même compte Steam. Hoard aussi, et il garde une version par session, pour revenir en arrière si quelque chose casse.
