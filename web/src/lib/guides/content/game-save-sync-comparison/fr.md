---
title: "Comparatif de synchronisation des sauvegardes : Hoard face à Ludusavi, Syncthing, OpenSave et les autres"
description: "Comparatif honnête des outils qui sauvegardent et synchronisent les parties PC — Ludusavi, Syncthing, OpenSave, OpenCloudSaves, Game Backup Monitor, Aletheia, SaveSync et Hoard — avec un tableau et une section sur les points faibles de Hoard."
order: 4
updated: 2026-09-01
---

Steam Cloud ne couvre que les jeux achetés sur Steam, et seulement quand le développeur a pris la peine de l'activer. Émulateurs, GOG, Epic, itch.io, jeux hors Steam, tout ce qui est moddé : rien de tout ça n'est couvert. Si vous jouez sur plusieurs machines, un fixe et un Steam Deck par exemple, vous finissez par copier des dossiers à la main en espérant avoir pris le plus récent.

Plusieurs outils règlent le problème, et ils ne font pas tous la même chose. Certains font des sauvegardes locales, d'autres répliquent des dossiers entre appareils, d'autres envoient vers un cloud. Cette page les passe en revue et dit ce que chacun fait vraiment bien. Hoard est mon projet, donc la partie honnête arrive à la fin : une section sur les points faibles de Hoard, et un tableau lisible sans croire un mot du texte.

## Ludusavi

Le plus connu, et à juste titre. Ludusavi (de mtkennerly) est un outil de sauvegarde gratuit et open source, avec interface et ligne de commande, bâti sur le manifeste communautaire des emplacements de sauvegardes qui couvre des dizaines de milliers de jeux — le même manifeste qu'utilisent presque tous les outils d'ici, Hoard compris. Il conserve des sauvegardes locales versionnées et peut les pousser vers votre propre cloud via Rclone.

**Le meilleur si :** vous voulez des sauvegardes locales, le contrôle total et aucun serveur nulle part. C'est le choix le plus sûr de la liste, et il est gratuit.

**Là où il s'arrête :** la synchronisation entre machines, c'est vous qui l'assemblez. Planifier une sauvegarde, configurer un remote Rclone, puis penser à restaurer sur l'autre PC *avant* de jouer. Ça marche, mais rien ne vous empêche d'oublier la dernière étape.

## Syncthing

Pas du tout un outil de jeu : un miroir de dossiers pair-à-pair généraliste, et très bon. Vous lui désignez un dossier de sauvegardes et il apparaît sur vos autres appareils.

**Le meilleur si :** vous l'utilisez déjà et vous voulez les fichiers à deux endroits sans cloud entre les deux.

**Là où il s'arrête :** il réplique, il ne photographie pas. Une sauvegarde corrompue atteint tous les appareils en quelques secondes, exactement aussi vite qu'une bonne. Son versionnage est par fichier, sans notion de session de jeu, donc « revenir à mardi soir » se reconstruit à la main. Deux machines qui ont joué hors ligne vous donnent des fichiers de conflit, pas une fusion.

## OpenSave

Synchronisation pair-à-pair conçue spécifiquement pour les sauvegardes, en Go, sous licence MIT, pour Windows, Linux et Steam Deck. Pas de compte, pas de serveur : les appareils s'appairent entre eux et se synchronisent en réseau local ou via un code de salon sur un relais. Chaque changement devient un instantané, il y a des branches pour les parties parallèles, les conflits se résolvent par lignage de synchronisation plutôt que par horloge, et seuls les blocs modifiés circulent. Il peut, en option, répliquer vers Drive, Dropbox, OneDrive ou WebDAV.

**Le meilleur si :** vous refusez d'avoir un compte et vos appareils sont allumés en même temps assez souvent.

**Là où il s'arrête :** pair-à-pair veut dire que la sauvegarde ne vit que sur vos appareils. Si le Deck qui détenait la seule copie récente meurt et que la réplication n'a jamais été configurée, c'est terminé. Les deux appareils doivent tourner pour qu'une synchronisation ait lieu, et il n'y a pas de version macOS.

## OpenCloudSaves

Une interface multiplateforme qui synchronise vos dossiers de sauvegardes vers un cloud que vous payez déjà — OneDrive, Google Drive, Dropbox, Nextcloud — avec Rclone en dessous.

**Le meilleur si :** vous voulez vos sauvegardes dans un espace de stockage que vous avez déjà, avec une interface plutôt que des fichiers de configuration Rclone.

**Là où il s'arrête :** pas de déduplication au niveau du contenu. Dix copies d'une sauvegarde de 2 Go, ce sont 20 Go de votre quota Drive, et les clouds de fichiers synchronisent des fichiers, pas des sessions de jeu : vous récupérez l'état du dossier à un instant donné, rien de plus.

## Game Backup Monitor

D'abord Windows, et l'ancêtre de tout ce genre. GBM guette le processus du jeu et, à la fermeture, compresse la sauvegarde avec 7-Zip en gardant un historique numéroté.

**Le meilleur si :** vous êtes sur un seul PC Windows et voulez une archive locale compressée sans y penser.

**Là où il s'arrête :** c'est un outil de sauvegarde, pas de synchronisation. Amener l'archive sur une deuxième machine, c'est votre affaire, et Steam Deck / SteamOS n'est pas son terrain.

## Aletheia

Le plus récent du lot, sous AGPL, et il attaque précisément ce que les autres couvrent à moitié : les lanceurs. Heroic, itch.io, Lutris, Steam, GOG Galaxy et Xbox, sous Windows, Linux et macOS.

**Le meilleur si :** votre bibliothèque est éparpillée sur des lanceurs que les autres outils détectent mal, en particulier Xbox/Game Pass et Heroic.

**Là où il s'arrête :** projet jeune, au périmètre volontairement étroit. Sauvegarder et restaurer, c'est tout ; il n'y a pas de cloud versionné derrière.

## SaveSync

Le commercial, vendu sur Steam en achat unique, orienté Windows. Sa particularité : il ne vise pas vous-sur-deux-PC mais le coop. Les sauvegardes partent dans des entrées privées et non listées du Steam Workshop pour qu'un ami récupère votre monde Valheim ou Factorio, et il y a aussi une synchronisation en réseau local.

**Le meilleur si :** votre problème est « mon ami héberge et il me faut sa sauvegarde », pas « que mes sauvegardes me suivent ».

**Là où il s'arrête :** code fermé, Windows, dépendant de Steam comme transport, et une liste de jeux coop pris en charge plutôt que tout ce que vous possédez.

## Une note sur EmuDeck

EmuDeck revient dans ces discussions, et ce n'est pas un concurrent au sens habituel : c'est un installateur et configurateur d'émulateurs pour Steam Deck, et la synchronisation qu'il propose est un confort greffé sur cette mission (Rclone vers un cloud de fichiers, pour les sauvegardes d'émulateurs uniquement). Il recoupe les outils ci-dessus sans être de la même nature : EmuDeck installe vos émulateurs, les outils d'ici veillent sur les sauvegardes de toute la bibliothèque. Beaucoup font tourner EmuDeck à côté de l'un d'eux, et c'est une configuration sensée, pas une redondance.

## Hoard

Hoard prend la session de jeu comme unité. Le moteur tourne en service d'arrière-plan — `hoardd`, sans fenêtre, donc il fonctionne en mode jeu de SteamOS —, remarque que vous avez arrêté de jouer, et prend l'instantané à ce moment-là plutôt que de réagir à chaque écriture pendant la partie.

- **Historique versionné par session.** Chaque session est une version vers laquelle revenir, même après une panne de disque ou une réinstallation.
- **Déduplication par empreinte de contenu.** Dix versions d'une sauvegarde de 2 Go coûtent environ 2 Go, pas 20 Go. Les transferts sont compressés en zstd.
- **SHA-256 à la montée et à la descente.** La corruption est détectée avant de pouvoir écraser une bonne sauvegarde. Rien n'est jamais écrasé en silence : c'est tout le principe.
- **Cloud ou auto-hébergé, le même binaire.** Hoard Cloud a une offre gratuite (2 Go, 3 appareils, historique complet). Ou vous lancez `hoard-server` vous-même avec Docker Compose sur n'importe quel stockage compatible S3 — MinIO, Garage, Backblaze B2 — sans compte ni quota. AGPL-3.0.
- **Windows, Linux, macOS**, plus une CLI sans interface pour un Steam Deck ou un serveur.
- **Émulateurs en bêta :** PCSX2, RPCS3, Dolphin, Cemu, Ryujinx, RetroArch, DuckStation, PPSSPP et d'autres en préréglages.

## Le détail qui décide de la synchro Steam Deck ↔ PC

Bon à savoir quel que soit l'outil choisi. La sauvegarde cloud d'un jeu Steam vit dans `<AppID>/remote/`, et le dossier *au-dessus* contient `remotecache.vdf`, l'état des succès, les statistiques et les compteurs de temps de jeu — autant de choses qui diffèrent légitimement entre votre Deck et votre fixe.

Synchronisez le dossier parent et vous obtenez un conflit permanent entre deux machines qui n'ont jamais été en désaccord sur une seule sauvegarde. Hoard suit `remote/`, pas le dossier parent. N'importe quel outil auquel vous désignez un dossier à la main peut faire pareil, et c'est la première chose à vérifier quand une configuration de synchronisation signale des conflits sans raison visible.

## Là où Hoard perd

- **Il veut un serveur.** Compte cloud ou machine à vous, dans les deux cas c'est de l'infrastructure, alors qu'OpenSave ou Ludusavi n'en demandent aucune.
- **La prise en charge des émulateurs est en bêta.** Les installations portables et les manies de chaque émulateur le piègent encore, et Aletheia comme OpenSave couvrent aujourd'hui mieux certains cas particuliers de lanceurs et d'émulateurs.
- **macOS est à peine testé sur du matériel réel.** Ça compile et ça tourne, mais personne n'y a vécu pendant des mois.
- **C'est jeune.** Ludusavi et Game Backup Monitor ont des années de rapports de bugs derrière eux. Pas Hoard, et ça compte pour un logiciel qui garde une partie de 200 heures.
- **Il ne fait pas le partage coop.** Pour passer un monde à un ami, SaveSync est fait pour ça, Hoard non.

## La distinction entre Hoard Cloud et l'auto-hébergement

Les comparaisons sur Hoard confondent presque toujours les deux, et le résultat est faux. Autant le dire clairement :

- **Hoard Cloud** est l'option gérée : vous vous connectez, et vos sauvegardes sont stockées sur nos serveurs, dans l'UE.
- **Un Hoard auto-hébergé est entièrement le vôtre.** Vous faites tourner `hoard-server` sur votre PC ou votre NAS, et vos sauvegardes vont de votre machine à votre disque. Il n'y a **aucun compte chez nous, aucune télémétrie vers nous, aucun quota et aucun relais** : rien ne passe par nos serveurs, puisque rien de chez nous n'est sur le chemin. Nous ne voyons ni sauvegarde, ni nom de jeu, ni adresse e-mail, car rien de cela ne nous parvient. Si Hoard Cloud fermait demain, une installation auto-hébergée continuerait à l'identique.

Le même binaire, la même détection, le même historique. La seule chose qui change, c'est à qui appartient le stockage. Et pour être exact sur un point : votre serveur a bien ses propres accès — un utilisateur et un jeton par appareil — mais ils vivent dans votre base, pas dans la nôtre.

## Le tableau

| Outil | Synchro automatique entre appareils | Où vivent les sauvegardes | Historique | Plateformes | Licence |
|---|---|---|---|---|---|
| **Hoard** | Oui, par session de jeu | Hoard Cloud ou votre serveur (compatible S3) | Versionné par session, dédupliqué | Win · Linux · macOS · Deck | AGPL-3.0, offre gratuite |
| **Ludusavi** | Manuelle, ou Rclone que vous montez | Local, plus votre remote Rclone | Sauvegardes locales versionnées | Win · Linux · macOS | Gratuit, open source |
| **Syncthing** | Oui, miroir continu | Vos appareils seulement | Versionnage par fichier | Tout | Gratuit, open source |
| **OpenSave** | Oui, pair-à-pair | Vos appareils, réplication cloud optionnelle | Instantanés et branches | Win · Linux · Deck | MIT |
| **OpenCloudSaves** | Oui, via votre cloud | OneDrive / Drive / Dropbox / Nextcloud | Ce que garde le cloud | Win · Linux · macOS | Gratuit, open source |
| **Game Backup Monitor** | Non | Archives 7-Zip locales | Sauvegardes numérotées | Windows | Gratuit, open source |
| **Aletheia** | Sauvegarde et restauration par lanceur | Votre stockage | Sauvegardes | Win · Linux · macOS | AGPL-3.0 |
| **SaveSync** | Oui, et avec des amis | Entrées privées du Steam Workshop | Selon l'application | Windows | Payant, code fermé |

## Alors lequel

Si vous voulez une seule machine sauvegardée et rien d'autre, prenez Ludusavi ou Game Backup Monitor. Si vous refusez tout compte et que vos appareils sont généralement allumés ensemble, OpenSave. Si vos sauvegardes doivent atterrir dans un dossier Drive que vous payez déjà, OpenCloudSaves. Si vous partagez un monde coop avec des amis, SaveSync.

Si vous voulez que la sauvegarde *et* la synchronisation entre PC et Steam Deck se fassent toutes seules, avec une version par session où revenir et la possibilité de tout auto-héberger, c'est à ça que sert Hoard. [Téléchargez-le](/download), ou lisez d'abord [comment l'auto-héberger avec Docker](/guides/self-host-hoard). Il y a aussi un [comparatif détaillé avec Ludusavi](/guides/ludusavi-alternative) si c'est celui que vous mettez dans la balance.

<!-- faq -->

## Questions fréquentes

### Lequel de ces outils garde un historique de versions ?

Hoard conserve chaque session comme une version où revenir. Ludusavi garde des sauvegardes locales versionnées. La plupart des autres synchronisent ou copient l'état actuel : une sauvegarde corrompue est donc fidèlement propagée à votre autre machine.

### Lequel fonctionne sans serveur ni compte ?

Ludusavi en sauvegardes locales, et tout outil pair-à-pair. Hoard entre aussi dans cette catégorie si vous l'auto-hébergez : aucun compte chez nous, et rien qui passe par nos serveurs.

### Lequel couvre les jeux absents de Steam ?

Tous les gestionnaires de sauvegardes cités, car ils localisent les fichiers via la même base communautaire et non via une boutique. L'exception est Steam Cloud : il ne couvre que les jeux Steam dont le développeur l'a activé.

### Dois-je n'en choisir qu'un ?

Non, et beaucoup ne le font pas. Un outil de sauvegarde locale et un outil de synchro règlent deux moitiés différentes du problème. La seule règle : ne jamais pointer l'un vers le dossier de sauvegardes de l'autre, sinon vous synchronisez un miroir périmé au lieu de votre sauvegarde réelle.

### Quel est le détail qui casse la plupart des montages maison ?

Synchroniser le dossier situé au-dessus de `<AppID>/remote/` dans le `userdata` de Steam. Le parent contient `remotecache.vdf` et des fichiers de succès et de temps de jeu censés différer d'une machine à l'autre : chaque lancement ressemble alors à un conflit alors qu'aucune sauvegarde n'a bougé.
