---
title: "Comment sauvegarder et synchroniser les sauvegardes d'émulateur (RetroArch, Dolphin, PCSX2)"
description: "Sauvegardez et synchronisez vos fichiers de sauvegarde et vos save states d'émulateur entre PC — RetroArch, Dolphin, PCSX2, DuckStation et plus — automatiquement avec Hoard."
order: 6
updated: 2026-09-01
---

Les sauvegardes d'émulateur se perdent facilement : fichiers de sauvegarde et save states vivent dans des dossiers éparpillés, et une réinstallation ou un nouveau PC peut effacer des années de progression. Hoard les sauvegarde automatiquement et les garde synchronisées entre machines.

## Émulateurs pris en charge par Hoard

Hoard gère les fichiers de sauvegarde d'émulateur courants (`.srm`, `.sav`, cartes mémoire) et les save states des émulateurs populaires, dont :

- **RetroArch** — sauvegardes et états par cœur
- **Dolphin** (GameCube / Wii) — cartes mémoire et fichiers GCI
- **PCSX2** (PS2) — cartes mémoire
- **DuckStation** (PS1), **PPSSPP** (PSP), **mGBA**, et plus

Comme Hoard localise les dossiers de sauvegarde avec la même base communautaire que celle qui alimente Ludusavi, de nombreux chemins d'émulateur sont détectés automatiquement. Pour tout cas particulier, vous pouvez pointer Hoard vers un dossier à la main.

## Configurer les sauvegardes d'émulateur

1. **Installez Hoard** pour Windows, macOS ou Linux et connectez-vous.
2. Ouvrez la **Bibliothèque** et ajoutez votre émulateur, ou ajoutez son dossier de sauvegardes/états manuellement si vous avez changé l'emplacement par défaut.
3. Gardez le **mode automatique** activé. Hoard sauvegarde après chaque session et conserve un historique versionné.
4. Installez Hoard sur vos autres PC avec le même compte pour synchroniser ces sauvegardes partout — voir [synchroniser vos parties entre PC](/guides/sync-game-saves-across-pcs).

## Ludusavi pour les émulateurs ?

Ludusavi peut aussi sauvegarder les parties d'émulateur en local, et c'est une excellente option gratuite pour cela. Si vous voulez en plus que ces sauvegardes d'émulateur se synchronisent automatiquement entre machines et conservent un historique de versions cloud sans configurer Rclone, c'est là que Hoard aide — lisez la [comparaison complète Ludusavi vs Hoard](/guides/ludusavi-alternative).

## Où chaque émulateur range ses sauvegardes

Bon à savoir, car une installation portable place tout cela ailleurs :

- **RetroArch** — `saves/` et `states/` dans le dossier de configuration : `%APPDATA%\RetroArch` sous Windows, `~/.config/retroarch` sous Linux.
- **Dolphin** — cartes mémoire sous `GC/`, sauvegardes Wii dans la NAND émulée, dans `Documents\Dolphin Emulator` ou `~/.local/share/dolphin-emu`.
- **PCSX2** — `memcards/`, sous `Documents\PCSX2` ou `~/.config/PCSX2`.
- **DuckStation** — `memcards/` et `savestates/` dans son propre dossier de données.
- **PPSSPP** — `PSP/SAVEDATA` pour les sauvegardes et `PSP/PPSSPP_STATE` pour les états.
- **RPCS3** — `dev_hdd0/home/00000001/savedata`.
- **Cemu** — `mlc01/usr/save`.
- **mGBA et la plupart des cores autonomes** — un `.sav` à côté de la ROM, sauf indication contraire.

Une **installation portable** — la norme sur les consoles portables et les clés USB — range tout cela à côté de l'exécutable. Si c'est votre cas, pointez Hoard sur ce dossier et il le suivra comme n'importe quelle sauvegarde.

## Sauvegarde et état sauvegardé, ce n'est pas pareil

La distinction compte, car les deux ne voyagent pas de la même façon :

- Une **sauvegarde** (`.srm`, une carte mémoire, un dossier `SAVEDATA`) est la sauvegarde propre du jeu, écrite par la console émulée. Elle passe d'une machine à l'autre et d'une version d'émulateur à l'autre sans broncher.
- Un **état sauvegardé** est un vidage de la mémoire de l'émulateur. Il est lié à cette version précise, et souvent au core exact : un état écrit par une version peut refuser de se charger dans une autre.

Hoard sauvegarde les deux. Ne soyez simplement pas surpris qu'un état venu d'une machine à jour n'ouvre pas sur une machine restée en arrière : gardez des versions identiques et appuyez-vous sur les sauvegardes classiques pour ce qui compte.

## Un émulateur, beaucoup de jeux

Un émulateur est un seul processus qui héberge des dizaines de titres, et c'est ce qui rend les sauvegardes d'émulateur pénibles pour un outil qui raisonne en « le jeu qui tourne ». Hoard sépare les titres au lieu de traiter l'émulateur entier comme un bloc : chaque jeu a son propre historique, et non un tas commun qui change dès que vous lancez quoi que ce soit.

## Sauvegardes d'émulateur sans passer par nos serveurs

Tout ceci fonctionne à l'identique face à votre propre serveur : lancez `hoard-server`, pointez l'application dessus, et vos sauvegardes vont de votre machine à votre disque. Aucun compte chez nous, aucune télémétrie vers nous, rien qui passe par nos serveurs. Voir [comment auto-héberger Hoard](/guides/self-host-hoard).

## Astuce

Les save states sont liés à une version précise de l'émulateur. Gardez vos émulateurs à jour de façon cohérente sur tous vos PC pour qu'un état synchronisé se charge correctement partout.

<!-- faq -->

## Questions fréquentes

### Hoard sauvegarde-t-il aussi mes ROMs ?

Non. Il suit les dossiers de sauvegarde, pas les fichiers de jeu. Les ROMs sont volumineuses, elles ne changent pas, et vous les avez déjà : il n'y a rien à versionner.

### Mon émulateur est en installation portable. Ça marche ?

Oui. Ajoutez à la main le dossier situé à côté de l'exécutable et Hoard le suivra comme n'importe quel emplacement de sauvegarde. C'est le montage habituel sur les consoles portables.

### Puis-je synchroniser des états sauvegardés entre deux PC ?

Oui, et Hoard le fera. Qu'un état se charge dépend de l'identité des versions d'émulateur sur les deux machines : c'est une limite de l'émulateur, pas de la synchro. Les sauvegardes classiques n'ont pas ce souci.

### Est-ce que ça marchera avec un émulateur absent de la liste ?

Presque certainement. Les plus courants sont détectés automatiquement, et pour le reste il suffit de pointer Hoard sur son dossier de sauvegardes.

### L'auto-hébergement change-t-il quelque chose pour les émulateurs ?

Non. Même détection, mêmes versions, même synchro. Seul le stockage est à vous.
