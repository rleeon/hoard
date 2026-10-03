---
title: "Emplacement des sauvegardes de Baldur's Gate 3 (PC et Steam Deck)"
description: "Où Baldur's Gate 3 range ses sauvegardes sous Windows, sur Steam Deck et sur Mac, ce qui est sauvegarde et ce qui est mod ou réglage, le mode Honneur, les backups."
order: 23
updated: 2026-10-02
---

Sous Windows, Baldur's Gate 3 range ses sauvegardes dans `%LOCALAPPDATA%\Larian Studios\Baldur's Gate 3\PlayerProfiles\Public\Savegames\Story`, un dossier par sauvegarde. C'est le chemin que donne Larian dans sa propre FAQ. Vous trouverez ci-dessous les chemins sur Steam Deck et Mac, ce qui se trouve à côté des sauvegardes, le mode Honneur et comment tout garder sauvegardé.

## Où Baldur's Gate 3 range ses sauvegardes

- **Windows :** `%LOCALAPPDATA%\Larian Studios\Baldur's Gate 3\PlayerProfiles\Public\Savegames\Story`
- **Steam Deck et Linux** (Proton) : `~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story`
- **Mac :** `~/Documents/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story`

Larian a retiré la version Linux native : sur Steam Deck, le jeu passe par Proton et les sauvegardes se trouvent dans le préfixe Proton que Steam garde pour lui ; `1086940` est l'identifiant Steam du jeu. S'il est installé sur la carte microSD, le dossier `compatdata` est sur la carte.

## Ce qui est une sauvegarde et ce qui ne l'est pas

Chaque sauvegarde est **un dossier** dans `Story`, avec un fichier `.lsv` et une miniature. Tout ce qui l'entoure est autre chose :

- **`Mods`** (sous `Baldur's Gate 3`) contient les fichiers des mods.
- **`modsettings.lsx`** (sous `PlayerProfiles\Public`) est la liste des mods activés et leur ordre de chargement.
- **Les réglages**, comme les graphismes et les commandes, sont des fichiers de configuration à côté du profil, pas une partie d'une sauvegarde.

Le piège, ce sont les mods. Une sauvegarde créée avec des mods attend les mêmes mods actifs au chargement. Si vous déplacez une sauvegarde moddée sur un autre PC, emportez la liste des mods, sinon le jeu signale des mods manquants et la sauvegarde risque de ne pas se charger comme prévu.

## Le mode Honneur

Le mode Honneur n'a qu'une seule sauvegarde, que le jeu écrase au fil de la partie, et si votre groupe tombe, la partie Honneur est terminée (vous pouvez continuer en mode Personnalisé, sans l'Honneur). Sauvegarder ce fichier, c'est votre choix : une copie avant un combat difficile est techniquement un retour en arrière, et certains la veulent justement après un plantage ou un bug, tandis que d'autres y voient une triche. Un outil de sauvegarde garde les versions de toute façon ; en restaurer une, c'est entre vous et les dés.

## Baldur's Gate 3 a-t-il des sauvegardes cloud ?

Oui. Sur Steam, il utilise Steam Cloud, qui garde les dernières sauvegardes synchronisées entre les machines d'un même compte. Il ne conserve que l'état actuel : si une sauvegarde est corrompue ou cassée par la mise à jour d'un mod, c'est cette version qui se synchronise.

## Sauvegarder à la main

1. Fermez complètement le jeu.
2. Copiez tout le dossier `PlayerProfiles` depuis le chemin ci-dessus (il contient `Savegames` et `modsettings.lsx`).
3. Pour restaurer, fermez le jeu et recopiez-le.

## Sauvegarde et synchro automatiques avec Hoard

[Hoard](/download) sauvegarde le dossier à chaque fois que vous arrêtez de jouer et garde toutes les versions : une sauvegarde cassée par la mise à jour d'un mod ou un patch n'est qu'à une restauration. Il garde aussi le dossier synchronisé entre vos PC et un Steam Deck.

1. Installez Hoard et connectez-vous, ou pointez-le vers [votre propre serveur](/guides/self-host-hoard).
2. Ouvrez la **Bibliothèque**. Baldur's Gate 3 est détecté à partir de votre bibliothèque Steam et de la base de données communautaire des sauvegardes.
3. Jouez. En quittant, la première version apparaît dans l'historique.

Chaque session ajoute une version contenant toutes les sauvegardes. Pour revenir en arrière, ouvrez l'historique et [restaurez une version antérieure](/guides/restore-a-game-save) ; ce qui se trouve sur votre PC est sauvegardé avant, donc essayer une ancienne version n'est jamais un aller simple.

<!-- faq -->

## Questions fréquentes

### Où sont les sauvegardes de Baldur's Gate 3 sur Steam Deck ?

Dans le préfixe Proton du jeu : `~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story`.

### Puis-je déplacer une sauvegarde moddée sur un autre PC ?

Oui, à condition que l'autre PC ait les mêmes mods installés et activés dans le même ordre. Copiez `modsettings.lsx` avec la sauvegarde et installez les mêmes fichiers de mods.

### Puis-je sauvegarder une partie en mode Honneur ?

La sauvegarde est un dossier ordinaire, donc oui, n'importe quel outil peut la copier. Que la restaurer respecte l'esprit du mode, c'est à vous de voir.

### Pourquoi ma sauvegarde signale-t-elle des mods manquants ?

Elle a été créée avec des mods qui ne sont plus actifs. Réactivez les mêmes mods, dans le même ordre, et elle se charge normalement.
