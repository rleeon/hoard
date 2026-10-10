---
title: "Sauvegardes cloud pour PCSX2 : synchroniser les cartes mémoire PS2 entre PC et Steam Deck"
description: "PCSX2 n'a pas de sauvegardes cloud. Synchronisez vos cartes mémoire PS2 entre PC et Steam Deck, avec historique : chemins, cartes dossier, pièges."
order: 16
updated: 2026-10-09
---

PCSX2 ne synchronise pas les sauvegardes tout seul : votre progression PS2 vit dans des fichiers de carte mémoire sur une seule machine, et l'autre n'en sait jamais rien. Hoard les synchronise automatiquement. Quand vous fermez PCSX2, il sauvegarde vos cartes mémoire, les récupère sur vos autres PC et votre Steam Deck, et garde chaque version pour qu'une mauvaise sauvegarde ne vous coûte jamais une partie entière.

## Où PCSX2 range vos sauvegardes

PCSX2 sauvegarde comme une vraie PS2 : sur des cartes mémoire. Par défaut il y en a deux, `Mcd001.ps2` et `Mcd002.ps2`, de 8 Mo chacune, dans un dossier `memcards`. Une seule carte contient les sauvegardes de tous les jeux auxquels vous avez joué avec.

- **Windows :** `Documents\PCSX2\memcards`. En mode portable, le dossier se trouve à côté du programme.
- **Linux :** `~/.config/PCSX2/memcards`.
- **Steam Deck** (le Flatpak de Discover) : `~/.var/app/net.pcsx2.PCSX2/config/PCSX2/memcards`. Avec EmuDeck, le lien sous `~/Emulation/saves/pcsx2` pointe vers l'un de ces dossiers.
- **Mac :** `~/Library/Application Support/PCSX2/memcards`.

Dans PCSX2, **Settings → Memory Cards** montre le dossier réellement utilisé et la carte insérée dans chaque emplacement. Hoard trouve seul les dossiers sous Windows, Linux et sur Steam Deck ; sur un Mac ou une installation portable, indiquez une fois le dossier `memcards`.

## Cartes fichier et cartes dossier

PCSX2 sait créer deux types de carte mémoire, et le choix compte plus qu'il n'y paraît quand on synchronise.

- **Une carte fichier** (`.ps2`) est un seul fichier de 8 Mo contenant les sauvegardes de tous les jeux. Sauvegardez dans n'importe quel jeu et tout le fichier change, donc chaque nouvelle version fait les 8 Mo complets.
- **Une carte dossier** est un dossier au lieu d'un fichier, chaque sauvegarde dans son propre sous-dossier. Sauvegardez dans un jeu et seuls ses fichiers changent, donc les versions restent légères et l'historique montre quelle sauvegarde a bougé.

Vous pouvez créer l'une ou l'autre depuis **Settings → Memory Cards**. Quel que soit votre choix, utilisez **le même type, les mêmes noms de carte et les mêmes emplacements** sur chaque machine. Une carte fichier sur le PC fixe et une carte dossier sur le Deck sont deux cartes différentes, et chaque machine croira que la sauvegarde de l'autre n'existe pas.

## La synchronisation au quotidien

Vous jouez sur le PC fixe et fermez PCSX2. Hoard attend que PCSX2 soit fermé et que les cartes ne bougent plus, puis envoie la nouvelle version. Plus tard, vous prenez le Steam Deck. Dès qu'il est en ligne, Hoard récupère les cartes plus récentes, et quand vous lancez PCSX2 votre sauvegarde est là. Fermez-le sur le Deck et la même chose se produit dans l'autre sens.

Aucune machine n'a besoin d'être allumée en même temps que l'autre. Les cartes attendent sur le serveur que l'autre machine les demande.

## Les pièges à connaître

- **Fermez PCSX2, pas seulement le jeu.** Hoard sauvegarde une fois l'émulateur fermé, il ne copie donc jamais une carte en pleine écriture. Sur un Deck, la mise en veille ne compte pas comme une fermeture.
- **Même disque, même région.** Les versions PAL et NTSC d'un jeu ont des numéros de série différents (SLES et SLUS, par exemple) et ne voient pas les sauvegardes l'une de l'autre. Utilisez la même image disque partout.
- **Les états sont autre chose.** Les états (fichiers `.p2s` dans `sstates`) sont des instantanés de l'émulateur et ne se chargent souvent pas dans une autre version de PCSX2. Hoard synchronise les cartes mémoire ; si vous voulez aussi faire voyager les états, ajoutez le dossier `sstates` comme élément à part et gardez PCSX2 à la même version partout.
- **Une version, c'est toute la carte.** Restaurer une version antérieure remet toute la carte dans son état d'alors, avec tous ses jeux. Hoard montre ce qui va changer avant de confirmer, et votre carte actuelle est copiée d'abord, donc une restauration peut toujours être annulée.

## Mise en place

1. Installez Hoard sur chaque machine et connectez-vous avec le même compte.
2. Dans la **Bibliothèque**, ajoutez PCSX2 depuis la liste des émulateurs.
3. Vérifiez que toutes les machines utilisent le même type de carte, les mêmes noms et les mêmes emplacements.
4. Jouez, fermez PCSX2 et continuez sur l'autre machine.

Vous préférez tout garder chez vous ? Lancez `hoard-server` sur votre PC ou NAS et pointez toutes les machines dessus. Pas de compte chez nous, pas de télémétrie vers nous, rien ne passe par nos serveurs. Voir [comment auto-héberger Hoard](/guides/self-host-hoard). Pour les autres émulateurs, voir [sauvegardes d'émulateurs](/guides/back-up-emulator-saves).

<!-- faq -->

## Questions fréquentes

### PCSX2 a-t-il des sauvegardes cloud ?

Non. PCSX2 écrit les cartes mémoire dans un dossier local et vous laisse la synchronisation. Hoard est une façon de la faire automatiquement, avec un historique des versions en plus.

### Puis-je synchroniser PCSX2 entre un PC et un Steam Deck ?

Oui. Installez Hoard sur les deux avec le même compte. Hoard sait où PCSX2 range ses cartes sous Windows et dans le Flatpak du Steam Deck, et les associe d'une machine à l'autre.

### Carte fichier ou carte dossier ?

Pour synchroniser, la carte dossier convient mieux : seules les sauvegardes modifiées sont envoyées, et l'historique montre quel jeu a bougé. Les deux fonctionnent, du moment que toutes les machines utilisent la même.

### Hoard synchronise-t-il les états de PCSX2 ?

Pas par défaut, car les états cassent d'une version de PCSX2 à l'autre. Ajoutez le dossier `sstates` à la main si vous les voulez, et gardez PCSX2 à la même version partout.

### Restaurer une ancienne version ramène-t-il tous les jeux de la carte en arrière ?

Oui. Une version, c'est toute la carte. Hoard montre d'abord ce qui va changer, et garde aussi comme version la carte que vous remplacez.

### Ça marche avec les émulateurs PS2 sur Android ?

Pas aujourd'hui. Hoard tourne sur Windows, macOS, Linux et Steam Deck.
