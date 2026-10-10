---
title: "Comment sauvegarder et synchroniser les sauvegardes d'émulateur (RetroArch, Dolphin, PCSX2)"
description: "Sauvegardez et synchronisez vos sauvegardes d'émulateur entre PC et Steam Deck : RetroArch, Dolphin, PCSX2, DuckStation, avec historique et emplacements."
order: 6
updated: 2026-10-09
---

Les sauvegardes d'émulateur se perdent facilement : fichiers de sauvegarde et save states vivent dans des dossiers éparpillés, et une réinstallation ou un nouveau PC peut effacer des années de progression. Hoard les sauvegarde automatiquement et les garde synchronisés entre vos machines, Steam Deck compris.

## Émulateurs pris en charge par Hoard

Hoard gère les fichiers de sauvegarde d'émulateur standard (`.srm`, `.sav`, cartes mémoire, dossiers de sauvegarde par jeu) et les save states. Il sait d'office où ces émulateurs rangent leurs sauvegardes :

- **Sony :** PCSX2 (PS2), DuckStation (PS1), PPSSPP (PSP), RPCS3 (PS3), shadPS4 (PS4), Vita3K (PS Vita)
- **Nintendo :** Dolphin (GameCube / Wii), Cemu (Wii U), Ryujinx, yuzu, Eden, Suyu, Citron et Sudachi (Switch), Citra / Azahar (3DS), melonDS (DS), mGBA (GBA), Project64 (N64)
- **Autres :** RetroArch (multisystème), xemu (Xbox), Flycast (Dreamcast)

Comme Hoard localise les dossiers de sauvegarde avec la même base de données communautaire que Ludusavi, beaucoup de chemins sont détectés automatiquement. Pour un emplacement personnalisé, vous pouvez indiquer un dossier à la main.

## Configurer les sauvegardes d'émulateur

1. **Installez Hoard** pour Windows, macOS ou Linux et connectez-vous.
2. Ouvrez la **Bibliothèque** et ajoutez votre émulateur, ou ajoutez son dossier de sauvegardes/états manuellement si vous avez changé l'emplacement par défaut.
3. Gardez le **mode automatique** activé. Hoard sauvegarde après chaque session et conserve un historique versionné.
4. Installez Hoard sur vos autres PC avec le même compte pour synchroniser ces sauvegardes partout — voir [synchroniser ses parties entre plusieurs PC](/guides/sync-game-saves-across-pcs).

## Ludusavi pour les émulateurs ?

Ludusavi peut aussi sauvegarder localement les parties d'émulateur, et c'est une excellente option gratuite pour cela. Si vous voulez en plus que ces sauvegardes se synchronisent automatiquement entre machines et gardent un historique de versions dans le cloud sans configurer Rclone, c'est là que Hoard aide — lisez la [comparaison complète Ludusavi vs Hoard](/guides/ludusavi-alternative).

## Sauvegardes cloud pour chaque émulateur

Aucun des émulateurs autonomes ci-dessous ne synchronise seul les sauvegardes entre machines : ce sont de simples fichiers sur votre disque. C'est une bonne nouvelle, car n'importe quel outil qui surveille le bon dossier peut les transporter. Voici où chacun les range. « Steam Deck » désigne la version Flatpak installée depuis la boutique Discover.

### Sauvegardes cloud PCSX2 (PS2)

PCSX2 écrit les cartes mémoire (fichiers `.ps2`) dans `memcards/` :

- Windows : `Documents\PCSX2\memcards`
- Linux : `~/.config/PCSX2/memcards`
- Steam Deck : `~/.var/app/net.pcsx2.PCSX2/config/PCSX2/memcards`

Une carte mémoire contient les sauvegardes de tous les jeux joués dessus, elle voyage donc d'un bloc : restaurer une version antérieure remet toute la carte en arrière, pas un seul jeu.

Le guide complet, avec cartes fichier et cartes dossier : [sauvegardes cloud pour PCSX2](/guides/pcsx2-cloud-saves).

### Sauvegardes cloud Dolphin (GameCube et Wii)

Les sauvegardes GameCube vivent sous `GC/` (images de carte mémoire ou un dossier par carte), celles de Wii dans la NAND émulée sous `Wii/` :

- Windows : `Documents\Dolphin Emulator\GC` et `\Wii`
- Linux : `~/.local/share/dolphin-emu/GC` et `/Wii`
- Steam Deck : `~/.var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu/`

Le guide complet, avec les dossiers GCI et la mémoire de la Wii : [sauvegardes cloud pour Dolphin](/guides/dolphin-cloud-saves).

### Sauvegardes cloud DuckStation (PS1)

DuckStation range les cartes mémoire dans `memcards/` et crée par défaut une carte distincte pour chaque jeu, ce qui se prête très bien à la synchronisation :

- Windows : `Documents\DuckStation\memcards` (les versions récentes utilisent `%LOCALAPPDATA%\DuckStation\memcards`)
- Linux : `~/.local/share/duckstation/memcards`
- Steam Deck : `~/.var/app/org.duckstation.DuckStation/`, sous `data/` ou `config/`

### Synchroniser les sauvegardes RetroArch

RetroArch sépare `saves/` (les sauvegardes du jeu) de `states/` (les save states). Hoard suit le dossier des sauvegardes ; ajoutez `states/` comme entrée à part si vous jouez avec des états :

- Windows : `%APPDATA%\RetroArch`, ou à côté de `retroarch.exe` pour une installation portable
- Linux : `~/.config/retroarch`
- Steam Deck : `~/.var/app/org.libretro.RetroArch/config/retroarch`, ou `~/Emulation/saves/retroarch` si vous l'avez installé avec EmuDeck

RetroArch possède aussi un Cloud Sync intégré qui dialogue avec un serveur WebDAV que vous fournissez. C'est un choix raisonnable si vous n'utilisez que RetroArch et avez déjà un WebDAV. Hoard n'a pas besoin de WebDAV, garde un historique de versions à restaurer et couvre aussi les émulateurs autonomes.

### PPSSPP (PSP)

Les sauvegardes vont dans `PSP/SAVEDATA`, les états dans `PSP/PPSSPP_STATE` :

- Windows : `Documents\PPSSPP\PSP\SAVEDATA`, ou `memstick\PSP\SAVEDATA` à côté de l'exécutable pour une installation portable
- Linux : `~/.config/ppsspp/PSP/SAVEDATA`
- Steam Deck : `~/.var/app/org.ppsspp.PPSSPP/config/ppsspp/PSP/SAVEDATA`

### RPCS3 (PS3)

Les sauvegardes vivent dans `dev_hdd0/home/00000001/savedata`, dans le dossier de RPCS3 sous Windows et sous `~/.config/rpcs3/` sous Linux et sur Steam Deck.

### Émulateurs Switch : Ryujinx, yuzu, Eden, Suyu, Citron, Sudachi

Ryujinx range les sauvegardes dans `bis/user/save` (sous `%APPDATA%\Ryujinx` ou `~/.config/Ryujinx`). La famille yuzu utilise `nand/user/save` dans son propre dossier sous `%APPDATA%` ou `~/.local/share`.

Il y a un piège. L'arborescence de type yuzu est `save/<compte>/<profil>/<id-du-jeu>/`, et l'identifiant de profil est généré au premier lancement de l'émulateur : il diffère donc à chaque installation. Synchronisez tout le dossier `save/` entre deux machines et chacune se retrouve avec le profil de l'autre à côté du sien, sans qu'aucun jeu ne voie la progression de l'autre. Hoard descend plutôt jusqu'au dossier propre à chaque jeu, pour que le même titre corresponde d'une machine à l'autre, quel que soit le nom du profil.

### Citra et Azahar (3DS)

Les sauvegardes sont enfouies sous `sdmc/Nintendo 3DS/<id0>/<id1>/title/…`, et `id0`/`id1` dérivent des clés de la console émulée : ils diffèrent donc aussi à chaque installation. Hoard procède comme pour la Switch : une entrée par jeu, associée entre machines.

### Les autres

- **Cemu (Wii U) :** `mlc01/usr/save`, sous `%APPDATA%\Cemu` ou `~/.local/share/Cemu`.
- **shadPS4 (PS4) :** `savedata`, sous `%APPDATA%\shadPS4` ou `~/.local/share/shadPS4`.
- **Vita3K (PS Vita) :** `ux0/user/00/savedata` dans son dossier de données.
- **mGBA, melonDS et la plupart des émulateurs de l'ère cartouche :** un `.sav` à côté de la ROM, sauf réglage contraire. Ajoutez à la main les sauvegardes du dossier des ROM.

## Sauvegardes d'émulateur sur Steam Deck

Sur Steam Deck, les émulateurs viennent généralement de Flatpak : leurs dossiers se trouvent donc sous `~/.var/app/<id>/` plutôt que dans les habituels `~/.config` ou `~/.local/share`. EmuDeck regroupe tout sous `~/Emulation/saves/`, un dossier par émulateur. Dans tous les cas, ajoutez le dossier une fois et Hoard le surveille.

Ce qui compte sur une console portable : le moteur de Hoard tourne comme un service en arrière-plan, il sauvegarde donc quand vous quittez un jeu en mode Jeu, sans aucune fenêtre ouverte. Reprenez le Deck après une session sur le PC fixe, la sauvegarde est déjà là.

## Sauvegarde et état sauvegardé, ce n'est pas pareil

Mieux vaut les distinguer, car ils ne voyagent pas de la même manière :

- Une **sauvegarde** (`.srm`, une carte mémoire, un dossier `SAVEDATA`) est la sauvegarde propre du jeu, écrite par la console émulée. Elle passe d'une machine et d'une version d'émulateur à l'autre sans problème.
- Un **save state** est une copie de la mémoire de l'émulateur. Il est lié à la version de l'émulateur, souvent au core exact, et un état créé par une version peut refuser de se charger dans une autre.

Hoard sauvegarde les deux. Ne soyez simplement pas surpris si un état venu d'une machine à jour ne s'ouvre pas sur une machine en retard : gardez vos émulateurs sur les mêmes versions et comptez sur les sauvegardes pour ce qui compte.

## Un émulateur, beaucoup de jeux

Un émulateur est un seul processus qui héberge des dizaines de titres, et c'est ce qui rend ses sauvegardes délicates pour un outil qui raisonne en « jeu en cours ». Hoard sépare les titres au lieu de traiter l'émulateur comme un bloc, chaque jeu a donc son propre historique au lieu d'un tas commun qui change à chaque lancement. Si une sauvegarde tourne mal, vous pouvez [revenir à une version antérieure](/guides/restore-a-game-save).

## Sauvegardes d'émulateur sans passer par nos serveurs

Tout ceci fonctionne de la même façon avec votre propre serveur : lancez `hoard-server`, pointez l'application dessus, et vos sauvegardes vont de votre machine à votre disque. Pas de compte chez nous, pas de télémétrie vers nous, rien ne passe par nos serveurs. Voir [comment auto-héberger Hoard](/guides/self-host-hoard).

## Astuce

Les save states dépendent d'une version précise de l'émulateur. Mettez vos émulateurs à jour de façon cohérente sur tous vos PC pour qu'un état synchronisé se charge partout.

<!-- faq -->

## Questions fréquentes

### Hoard sauvegarde-t-il aussi mes ROM ?

Non. Il suit les dossiers de sauvegarde, pas les fichiers de jeu. Les ROM sont lourdes, ne changent pas, et vous les avez déjà : il n'y a rien à versionner.

### PCSX2, Dolphin ou DuckStation ont-ils des sauvegardes cloud intégrées ?

Non. Ils écrivent les sauvegardes dans des dossiers locaux et vous laissent la synchronisation. Pointez un outil de synchro sur les dossiers ci-dessus et vos sauvegardes vous suivront d'une machine à l'autre.

### RetroArch a-t-il une synchro cloud ?

Oui, un Cloud Sync intégré qui nécessite un serveur WebDAV que vous hébergez ou louez. Hoard est l'alternative si vous préférez éviter WebDAV, voulez un historique de versions à restaurer ou jouez aussi sur des émulateurs autonomes.

### Est-ce que ça marche sur Steam Deck en mode Jeu ?

Oui. Le moteur tourne comme un service en arrière-plan : les sauvegardes sont copiées quand vous quittez un jeu, sans fenêtre ouverte. Les dossiers Flatpak et EmuDeck fonctionnent comme n'importe quels autres.

### Mon émulateur est une version portable. Ça fonctionne ?

Oui. Ajoutez à la main le dossier situé à côté de l'exécutable et Hoard le suit comme n'importe quel autre emplacement. C'est la configuration habituelle sur les consoles portables.

### Puis-je synchroniser des save states entre deux PC ?

Oui, et Hoard le fera. Qu'un état se charge dépend de la même version d'émulateur sur les deux machines, une limite de l'émulateur et non de la synchronisation. Les sauvegardes n'ont pas ce problème.

### Est-ce que ça marchera avec un émulateur absent de la liste ?

Très probablement. La détection couvre automatiquement les plus courants, et vous ajoutez les autres en indiquant à Hoard leur dossier de sauvegarde.

### L'auto-hébergement change-t-il quelque chose pour les émulateurs ?

Non. Même détection, mêmes versions, même synchronisation. Seul le stockage vous appartient.
