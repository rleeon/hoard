---
title: "Emplacement des sauvegardes d'Elden Ring (PC et Steam Deck)"
description: "Où Elden Ring range ER0000.sl2 sur PC et Steam Deck, ce qu'il contient, Seamless Co-op et Nightreign, et comment sauvegarder tous vos personnages."
order: 26
updated: 2026-10-18
---

Sur PC, Elden Ring range ses sauvegardes dans `%APPDATA%\EldenRing\<votre identifiant Steam>`, dans un seul fichier, `ER0000.sl2`, qui contient tous les personnages que vous avez créés, jusqu'à dix. Vous trouverez ci-dessous le chemin sur Steam Deck, ce qu'il y a d'autre dans le dossier, où Seamless Co-op et Nightreign rangent leurs sauvegardes, et comment garder le tout sauvegardé et synchronisé entre votre PC et votre Steam Deck.

## Où Elden Ring range ses sauvegardes

- **Windows (Steam) :** `%APPDATA%\EldenRing\<votre identifiant Steam>\ER0000.sl2`
- **Steam Deck et Linux** (Proton) : `~/.local/share/Steam/steamapps/compatdata/1245620/pfx/drive_c/users/steamuser/AppData/Roaming/EldenRing/<votre identifiant Steam>/`

`%APPDATA%` correspond à `C:\Users\<vous>\AppData\Roaming` : collez `%APPDATA%\EldenRing` dans la barre d'adresse de l'Explorateur de fichiers pour y aller directement. Le dossier de l'identifiant Steam est un long numéro lié à votre compte Steam. Sur Steam Deck, le jeu passe par Proton, donc la sauvegarde est dans le préfixe Proton que Steam garde pour lui ; `1245620` est l'identifiant Steam du jeu.

## Ce que contient le dossier de sauvegarde

- **`ER0000.sl2`** est votre progression : tous les personnages, leur équipement et où ils en sont dans le monde, Shadow of the Erdtree compris.
- **`ER0000.sl2.bak`** est une copie de secours que le jeu garde à côté.
- **`ER0000.co2`**, si vous en voyez un, appartient au mod Seamless Co-op (voir plus bas).

Les réglages graphiques ne sont pas dans ce dossier : ils se trouvent un niveau au-dessus, dans `%APPDATA%\EldenRing\GraphicsConfig.xml`.

Deux choses découlent du fonctionnement de ce fichier :

- **Les personnages ne se sauvegardent ni ne se restaurent un par un.** Restaurer un ancien `ER0000.sl2` ramène tous les personnages à ce moment-là, pas seulement celui que vous vouliez réparer.
- **La sauvegarde est liée à votre compte Steam.** Votre identifiant Steam est écrit dans le fichier, et le jeu refuse de charger une sauvegarde créée sur un autre compte. Déplacer votre propre sauvegarde entre vos propres PC et une Steam Deck fonctionne sans souci.

## Seamless Co-op et Nightreign

### Seamless Co-op

Le mod Seamless Co-op ne touche pas à `ER0000.sl2` : il garde sa propre sauvegarde, `ER0000.co2`, dans le même dossier, donc vos personnages avec et sans mod sont des fichiers distincts. La communauté du mod déconseille de copier un `.co2` par-dessus la sauvegarde normale pour jouer en ligne.

### Elden Ring Nightreign

Nightreign est un jeu à part, avec son propre dossier : `%APPDATA%\Nightreign\<votre identifiant Steam>\NR0000.sl2` sous Windows, et le même chemin dans son propre préfixe Proton sur Steam Deck (`compatdata/2622380`). Il ne touche jamais à vos personnages d'Elden Ring.

## Elden Ring a-t-il des sauvegardes cloud ?

Oui, via Steam Cloud, qui garde `ER0000.sl2` à jour entre les machines d'un même compte. Il ne conserve que le dernier état, sans versions antérieures où revenir, et le `.bak` du jeu est sur le même disque que la sauvegarde : un disque mort emporte les deux.

## Sauvegarder à la main

1. Fermez complètement le jeu.
2. Copiez votre dossier d'identifiant Steam depuis `%APPDATA%\EldenRing` vers un endroit sûr. Il contient `ER0000.sl2`, son `.bak` et l'éventuelle sauvegarde Seamless Co-op.
3. Pour restaurer, fermez le jeu et recopiez le dossier au même endroit.

## Sauvegarde et synchro automatiques avec Hoard

[Hoard](/download) est gratuit et open source. Il sauvegarde vos parties d'Elden Ring chaque fois que le jeu en écrit une, puis encore quand vous quittez, garde un historique où revenir et synchronise le dossier entre vos PC et une Steam Deck. Seuls les fichiers modifiés voyagent, compressés, et chaque fichier est vérifié par son empreinte SHA-256 à son retour.

1. Installez Hoard et connectez-vous à Hoard One, recommandé pour tout le monde, ou pointez-le vers [votre propre serveur](/guides/self-host-hoard).
2. Ouvrez la **Bibliothèque**. Elden Ring est détecté à partir de votre bibliothèque Steam et de la base de données communautaire des sauvegardes.
3. Jouez. En quittant, la première version apparaît dans l'historique.

Hoard prend tout le dossier de l'identifiant Steam, donc une sauvegarde Seamless Co-op figure aussi dans chaque version. Chaque version, ce sont tous vos personnages à ce moment-là : pour revenir en arrière, [restaurez une version antérieure](/guides/restore-a-game-save), et les versions plus récentes restent dans l'historique si vous changez d'avis.

## Hoard One, ou votre propre serveur

Hoard est gratuit dans les deux cas, et l'application est la même :

- **Hoard One**, recommandé pour tout le monde. Rien à installer ni à faire tourner : un serveur géré dans l'UE, gratuit avec 2 Go et trois appareils, sans carte. [Pro](/pricing) ajoute seulement de la place, 100 Go et des appareils illimités, pour le jour où vos sauvegardes ne tiennent plus.
- **Votre propre serveur.** Faites tourner `hoard-server` sur un PC, un NAS ou un Raspberry Pi, et toutes vos machines se synchronisent à travers lui. Pas de compte chez nous, pas de télémétrie vers nous, pas de quota de notre part : rien ne passe par nos serveurs, puisque rien de nous n'est sur le chemin. Voir [comment héberger Hoard vous-même](/guides/self-host-hoard).

Même programme, même détection, même historique. Hoard One garde toutes les versions pour toujours ; sur votre propre serveur, c'est vous qui décidez combien d'historique garder.

<!-- faq -->

## Questions fréquentes

### Puis-je utiliser ma sauvegarde d'Elden Ring sur un autre compte Steam ?

Pas telle quelle. Votre identifiant Steam est écrit dans `ER0000.sl2`, et le jeu refuse de charger une sauvegarde créée sur un autre compte. Votre propre sauvegarde, sur votre propre compte, passe sans souci entre vos PC et une Steam Deck.

### Restaurer une ancienne sauvegarde ramène-t-il tous mes personnages en arrière ?

Oui. Tous vos personnages partagent `ER0000.sl2`, donc toute restauration les ramène tous à ce moment-là. Avec Hoard, les versions plus récentes restent dans l'historique : un retour en arrière n'est jamais un aller simple.

### Hoard sauvegarde-t-il les parties Seamless Co-op ?

Oui. Le mod garde `ER0000.co2` dans le même dossier d'identifiant Steam que la sauvegarde normale, et Hoard sauvegarde tout le dossier : les deux sont dans chaque version.

### Hoard est-il gratuit ?

Oui. L'application, la ligne de commande et le serveur sont gratuits et open source (AGPL-3.0), et faire tourner votre propre serveur ne coûte que la machine. Hoard One, le serveur géré que nous recommandons à tous, est gratuit lui aussi avec 2 Go et trois appareils ; Pro ajoute de la place pour 1,99 € par mois.

### L'auto-hébergement envoie-t-il quelque chose à Hoard ?

Non. Sur votre propre serveur, il n'y a ni compte chez nous ni télémétrie vers nous : vos sauvegardes, vos utilisateurs et vos journaux restent sur votre machine et ne touchent jamais les nôtres.
