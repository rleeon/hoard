---
title: "Sauvegardes cloud pour Dolphin : synchroniser GameCube et Wii entre PC et Steam Deck"
description: "Dolphin n'a pas de sauvegardes cloud. Synchronisez vos parties GameCube et Wii entre PC et Steam Deck, avec historique : chemins, cartes, pièges."
order: 17
updated: 2026-10-09
---

Dolphin ne synchronise pas les sauvegardes entre machines : vos cartes mémoire GameCube et votre Wii émulée vivent dans un dossier sur un seul PC. Hoard les synchronise automatiquement. Quand vous fermez Dolphin, il sauvegarde vos parties GameCube et Wii, les récupère sur vos autres PC et votre Steam Deck, et garde chaque version pour que vous puissiez toujours revenir en arrière.

## Où Dolphin range vos sauvegardes

Tout se trouve dans le dossier utilisateur de Dolphin. Le plus rapide pour le trouver est **File → Open User Folder** dans Dolphin. Dedans :

- `GC` contient les cartes mémoire GameCube.
- `Wii` est la mémoire interne de la Wii émulée, sauvegardes comprises.
- `StateSaves` contient les états.

Où se trouve ce dossier :

- **Windows :** `Documents\Dolphin Emulator`. Les installations récentes peuvent utiliser `%APPDATA%\Dolphin Emulator`, et une installation portable garde un dossier `User` à côté de `Dolphin.exe`.
- **Linux :** `~/.local/share/dolphin-emu`.
- **Steam Deck** (le Flatpak de Discover) : `~/.var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu`.
- **Mac :** `~/Library/Application Support/Dolphin`.

Hoard trouve seul les dossiers `Documents`, Linux et Steam Deck. Pour `%APPDATA%`, une installation portable ou un Mac, indiquez une fois les dossiers `GC` et `Wii`.

## GameCube : fichiers de carte ou dossiers GCI

Dans **Options → Configuration → GameCube**, chaque emplacement de carte mémoire peut être l'une de deux choses :

- **Un fichier de carte**, une image brute comme `MemoryCardA.USA.raw` contenant les sauvegardes de tous les jeux. Chaque sauvegarde réécrit tout le fichier.
- **Un dossier GCI**, où chaque sauvegarde est son propre fichier `.gci`, dans un dossier comme `GC/USA/Card A`. Seule la sauvegarde modifiée est nouvelle, donc les versions restent légères et lisibles.

Pour synchroniser, les dossiers GCI conviennent mieux. Dans tous les cas, **utilisez le même réglage sur chaque machine** : un fichier de carte sur un PC et un dossier GCI sur l'autre, et chacun voit une carte vide. Pour passer des sauvegardes d'un type à l'autre, **Tools → Memory Card Manager** de Dolphin importe et exporte des fichiers `.gci`.

Les cartes sont aussi séparées **par région** (USA, EUR, JAP). Une copie PAL et une copie NTSC du même jeu ne voient pas les sauvegardes l'une de l'autre, alors utilisez la même image disque partout.

## Wii : la mémoire de la console émulée

Les sauvegardes Wii se trouvent dans le dossier `Wii`, sous `Wii/title/00010000/<ID du jeu>/data` pour les jeux sur disque. Ce dossier est toute la mémoire de la console émulée : sauvegardes, Mii, réglages système et les chaînes que vous avez installées. Hoard le sauvegarde comme un seul élément, donc restaurer une version remet la mémoire de la console dans l'état de ce moment-là. Avant de confirmer, Hoard montre ce qui va changer, et vos fichiers actuels sont copiés d'abord.

Pour déplacer une seule sauvegarde Wii à la main, Dolphin sait l'exporter : clic droit sur le jeu dans la liste, puis **Export Wii Save**.

## La synchronisation au quotidien

Vous jouez sur le PC fixe et fermez Dolphin. Hoard attend que Dolphin soit fermé et que les dossiers soient au repos, puis envoie la nouvelle version. Plus tard, vous prenez le Steam Deck ; dès qu'il est en ligne, Hoard récupère les sauvegardes plus récentes. Fermez Dolphin sur le Deck et la même chose se produit dans l'autre sens. Aucune machine n'a besoin d'être allumée en même temps que l'autre.

## Les pièges à connaître

- **Fermez Dolphin, pas seulement le jeu.** Hoard sauvegarde une fois l'émulateur fermé. Sur un Deck, la mise en veille ne compte pas comme une fermeture.
- **Les états sont fragiles.** Les états de Dolphin cassent souvent d'une version à l'autre. Hoard synchronise les vraies sauvegardes ; si vous voulez aussi `StateSaves`, ajoutez-le comme élément à part et gardez Dolphin à la même version partout.
- **Chemins personnalisés.** Si vous avez changé la racine de la NAND Wii ou le chemin des dossiers GCI dans **Options → Configuration → Paths**, indiquez ces dossiers à Hoard.

## Mise en place

1. Installez Hoard sur chaque machine et connectez-vous avec le même compte.
2. Dans la **Bibliothèque**, ajoutez Dolphin depuis la liste des émulateurs.
3. Utilisez le même réglage de carte et la même région sur chaque machine.
4. Jouez, fermez Dolphin et continuez sur l'autre machine.

Vous préférez tout garder chez vous ? Lancez `hoard-server` sur votre PC ou NAS et pointez toutes les machines dessus. Pas de compte chez nous, pas de télémétrie vers nous, rien ne passe par nos serveurs. Voir [comment auto-héberger Hoard](/guides/self-host-hoard). Pour les autres émulateurs, voir [sauvegardes d'émulateurs](/guides/back-up-emulator-saves).

<!-- faq -->

## Questions fréquentes

### Dolphin a-t-il des sauvegardes cloud ?

Non. Dolphin garde les sauvegardes dans un dossier local et vous laisse la synchronisation. Hoard est une façon de les synchroniser automatiquement, avec un historique des versions en plus.

### Puis-je synchroniser Dolphin entre un PC et un Steam Deck ?

Oui. Installez Hoard sur les deux avec le même compte. Hoard sait où Dolphin range ses sauvegardes sous Windows, Linux et dans le Flatpak du Steam Deck, et les associe d'une machine à l'autre.

### Fichier de carte ou dossier GCI ?

Pour synchroniser, le dossier GCI : chaque sauvegarde est son propre fichier, donc les versions sont légères et montrent quel jeu a changé. Quel que soit votre choix, utilisez le même sur chaque machine.

### Les sauvegardes Wii sont-elles synchronisées aussi ?

Oui. Le dossier `Wii` contient la mémoire de la console émulée, sauvegardes comprises, et Hoard le sauvegarde et le synchronise comme les cartes GameCube.

### Hoard synchronise-t-il les états de Dolphin ?

Pas par défaut, car les états cassent d'une version de Dolphin à l'autre. Ajoutez le dossier `StateSaves` à la main si vous les voulez.

### Ça marche avec Dolphin sur Android ?

Pas aujourd'hui. Hoard tourne sur Windows, macOS, Linux et Steam Deck.
