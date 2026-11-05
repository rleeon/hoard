---
title: "Comparatif de synchronisation des sauvegardes : Hoard face à Ludusavi, Syncthing, OpenSave et les autres"
description: "Hoard, Ludusavi, Syncthing, OpenSave, GameSave Manager, SaveState, EmuSync et d'autres comparés : les points forts de chacun et un tableau."
order: 4
updated: 2026-11-05
---

Steam Cloud ne couvre que les jeux achetés sur Steam, et seulement quand le développeur a pris la peine de l'activer. Émulateurs, GOG, Epic, itch.io, jeux hors Steam, tout ce qui est moddé : rien de tout ça n'est couvert. Si vous jouez sur plusieurs machines, un fixe et un Steam Deck par exemple, vous finissez par copier des dossiers à la main en espérant avoir pris le plus récent.

Plusieurs outils règlent le problème, et ils ne font pas tous la même chose. Certains font des sauvegardes locales, d'autres répliquent des dossiers entre appareils, d'autres envoient vers un cloud. Cette page les passe en revue et dit ce que chacun fait vraiment bien. Hoard est mon projet, donc il arrive en dernier, avec ce qu'il ne fait pas, et un tableau lisible sans croire un mot du texte.

## Ludusavi

Le plus connu, et à juste titre. Ludusavi (de mtkennerly) est un outil de sauvegarde gratuit et open source, avec interface et ligne de commande, bâti sur le manifeste communautaire des emplacements de sauvegardes qui couvre des dizaines de milliers de jeux — le même manifeste qu'utilisent presque tous les outils d'ici, Hoard compris. Il conserve des sauvegardes locales versionnées et peut les pousser vers votre propre cloud via Rclone.

**Le meilleur si :** vous voulez des sauvegardes locales, le contrôle total et aucun serveur nulle part. C'est le choix le plus sûr de la liste, et il est gratuit.

**Là où il s'arrête :** la synchronisation entre machines, c'est vous qui l'assemblez. Planifier une sauvegarde, configurer un remote Rclone, puis penser à restaurer sur l'autre PC *avant* de jouer. Ça marche, mais rien ne vous empêche d'oublier la dernière étape.

## Syncthing

Pas du tout un outil de jeu : un miroir de dossiers pair-à-pair généraliste, et très bon. Vous lui désignez un dossier de sauvegardes et il apparaît sur vos autres appareils.

**Le meilleur si :** vous l'utilisez déjà et vous voulez les fichiers à deux endroits sans cloud entre les deux.

**Là où il s'arrête :** il réplique, il ne photographie pas. Une sauvegarde corrompue atteint tous les appareils en quelques secondes, exactement aussi vite qu'une bonne. Son versionnage est par fichier, sans notion de session de jeu, donc « revenir à mardi soir » se reconstruit à la main. Deux machines qui ont joué hors ligne vous donnent des fichiers de conflit, pas une fusion.

## OpenSave

Synchronisation pair-à-pair conçue spécifiquement pour les sauvegardes, en Go, sous licence MIT, pour Windows, Linux, Steam Deck et, depuis la version 2.4, macOS. Pas de compte, pas de serveur : les appareils s'appairent entre eux et se synchronisent en réseau local ou via un code de salon sur un relais. Chaque changement devient un instantané, il y a des branches pour les parties parallèles, les conflits se résolvent par lignage de synchronisation plutôt que par horloge, et seuls les blocs modifiés circulent. Il peut, en option, répliquer vers Drive, Dropbox, OneDrive ou WebDAV.

**Le meilleur si :** vous refusez d'avoir un compte et vos appareils sont allumés en même temps assez souvent.

**Là où il s'arrête :** pair-à-pair veut dire que la sauvegarde ne vit que sur vos appareils. Si le Deck qui détenait la seule copie récente meurt et que la réplication n'a jamais été configurée, c'est terminé. Les deux appareils doivent tourner pour qu'une synchronisation ait lieu.

## OpenCloudSaves

Une interface multiplateforme qui synchronise vos dossiers de sauvegardes vers un cloud que vous payez déjà — OneDrive, Google Drive, Dropbox, Nextcloud — avec Rclone en dessous.

**Le meilleur si :** vous voulez vos sauvegardes dans un espace de stockage que vous avez déjà, avec une interface plutôt que des fichiers de configuration Rclone.

**Là où il s'arrête :** le projet a été archivé en janvier 2025 et n'est plus maintenu, il ne recevra donc pas de correctifs pour les nouveaux jeux ni pour les mises à jour du système. En plus, pas de déduplication au niveau du contenu. Dix copies d'une sauvegarde de 2 Go, ce sont 20 Go de votre quota Drive, et les clouds de fichiers synchronisent des fichiers, pas des sessions de jeu : vous récupérez l'état du dossier à un instant donné, rien de plus.

## Game Backup Monitor

D'abord Windows, et l'ancêtre de tout ce genre. GBM guette le processus du jeu et, à la fermeture, compresse la sauvegarde avec 7-Zip en gardant un historique numéroté.

**Le meilleur si :** vous êtes sur un seul PC Windows et voulez une archive locale compressée sans y penser.

**Là où il s'arrête :** c'est un outil de sauvegarde, pas de synchronisation. Amener l'archive sur une deuxième machine, c'est votre affaire, et Steam Deck / SteamOS n'est pas son terrain.

## GameSave Manager

Un outil Windows gratuit de longue date, propriétaire, avec sa propre base de jeux. Il sauvegarde et restaure, et sa fonction la plus connue, **Sync & Link**, déplace un dossier de sauvegarde dans un dossier cloud comme Dropbox ou OneDrive et laisse un lien à sa place, pour que le client cloud le garde synchronisé.

**Idéal si :** vous êtes sous Windows, vivez déjà dans Dropbox ou OneDrive et voulez simplement que la sauvegarde y soit.

**Où il s'arrête :** Windows uniquement, et Sync & Link signifie qu'un client cloud généraliste synchronise le dossier actif pendant que le jeu écrit dedans, le même schéma qui rend [Syncthing risqué pour les sauvegardes](/guides/syncthing-game-saves). L'historique est celui que garde votre cloud.

## Game Save Manager (GSM)

Malgré le nom, ce n'est pas le même programme que celui du dessus. GSM, de dyang886, est une application Windows sous GPL-3.0 qui trouve les jeux installés via Steam, Epic, Battle.net et d'autres launchers grâce à la base de PCGamingWiki, garde pour chaque jeu une série de sauvegardes locales qui tourne selon la limite que vous fixez, et emporte tout votre historique sur un nouveau PC dans une archive `.gsmr` exportée. Il existe même un troisième homonyme : le game-save-manager de mcthesw, populaire dans les communautés chinoises, qui sauvegarde de façon programmée les dossiers que vous ajoutez à la main, avec WebDAV pour une copie hors de la machine.

**Le meilleur si :** vous jouez sur un seul PC Windows et voulez des sauvegardes locales versionnées presque sans réglage, plus un déménagement facile vers une nouvelle machine.

**Là où il s'arrête :** pas de synchro entre appareils : le second PC reçoit vos sauvegardes quand vous exportez une archive et l'importez là-bas. Windows uniquement.

## SaveState

SaveState, de Matteo842, est un gestionnaire de sauvegardes sous GPL-3.0 pour Windows et Linux, avec une application Android à part. Vous déposez le raccourci d'un launcher dessus et il trouve le dossier de sauvegarde, et sa prise en charge des émulateurs va loin : RetroArch, Dolphin, les cartes mémoire de PCSX2, xemu et plus d'une vingtaine d'autres. Les sauvegardes sont des archives `.zip` compressées avec rotation, et une synchro optionnelle les copie vers Google Drive, WebDAV, FTP, un partage réseau ou Git.

**Le meilleur si :** vous voulez un outil de sauvegarde accueillant, avec une large couverture d'émulateurs, et restaurer à la main sur l'autre machine ne vous gêne pas.

**Là où il s'arrête :** il est construit autour d'archives de sauvegarde, pas de la sauvegarde vivante. La synchro déplace ces archives entre machines, mais en remettre une dans le dossier du jeu de l'autre côté reste une restauration que vous lancez vous-même. Pas de macOS.

## EmuSync

EmuSync est un outil de synchro sous GPL-3.0 pour les sauvegardes PC et d'émulateurs, pensé d'abord pour un PC Windows et un Steam Deck. Vous associez l'emplacement des sauvegardes de chaque jeu, sur chaque appareil, à un point de synchro unique dans votre propre Google Drive, Dropbox ou OneDrive, ou dans un dossier partagé, et il synchronise automatiquement ou à la demande. Un plugin Decky affiche l'état en mode Jeu.

**Le meilleur si :** vous jouez entre un PC Windows et un Deck et voulez vos sauvegardes dans un cloud que vous avez déjà.

**Là où il s'arrête :** le cloud garde l'état actuel, et la sauvegarde et la restauration restent locales à chaque appareil plutôt qu'un historique partagé entre eux. macOS n'est pas testé, et la dernière version date de février 2026.

## Aletheia

L'un des plus récents, sous AGPL, et il attaque précisément ce que les autres couvrent à moitié : les lanceurs. Heroic, itch.io, Lutris, Steam, GOG Galaxy et Xbox, sous Windows, Linux et macOS.

**Le meilleur si :** votre bibliothèque est éparpillée sur des lanceurs que les autres outils détectent mal, en particulier Xbox/Game Pass et Heroic.

**Là où il s'arrête :** projet jeune, au périmètre volontairement étroit. Sauvegarder et restaurer, c'est tout ; il n'y a pas de cloud versionné derrière.

## SaveSync

Le commercial, vendu sur Steam en achat unique, orienté Windows. Sa particularité : il ne vise pas vous-sur-deux-PC mais le coop. Les sauvegardes partent dans des entrées privées et non listées du Steam Workshop pour qu'un ami récupère votre monde Valheim ou Factorio, et il y a aussi une synchronisation en réseau local.

**Le meilleur si :** votre problème est « mon ami héberge et il me faut sa sauvegarde », pas « que mes sauvegardes me suivent ».

**Là où il s'arrête :** code fermé, Windows, dépendant de Steam comme transport, et une liste de jeux coop pris en charge plutôt que tout ce que vous possédez.

## Tachyon

Le nom le plus récent de la liste, en bêta gratuite sous Windows. Tachyon détecte les sauvegardes de plus de 5 000 jeux PC, les synchronise via son propre cloud et conserve un historique de versions avec plusieurs « timelines ».

**Idéal si :** vous êtes sous Windows, voulez quelque chose qui marche tout de suite et qu'une bêta ne vous gêne pas.

**Où il s'arrête :** Windows uniquement pour l'instant (Linux et macOS sont annoncés), donc pas encore de Steam Deck ; pas de code source publié ; impossible d'utiliser son propre serveur. Les tarifs après la bêta ne sont pas annoncés.

## Plugins pour Steam Deck (Decky)

Sur le Deck lui-même, quelques plugins Decky Loader couvrent une partie du travail depuis le mode Jeu. Decky Cloud Save synchronise des dossiers avec OneDrive, Google Drive ou Dropbox via rclone après la fermeture d'un jeu ; son auteur ne le maintient plus et renvoie vers deux successeurs, Decky Ludusavi (une interface Ludusavi pour le mode Jeu) et SDH-GameSync (un fork qui récupère les sauvegardes au lancement de chaque jeu et les envoie à sa fermeture). Steamback garde en local les dix instantanés les plus récents des jeux qui utilisent Steam Cloud, pris quand vous quittez.

**Le meilleur si :** vous jouez surtout sur le Deck et voulez quelque chose que l'on pilote sans quitter le mode Jeu.

**Là où il s'arrête :** ils ne tournent que sur le Deck, et la partie entre appareils repose sur autre chose : rclone et votre cloud, Ludusavi ou Steam Cloud lui-même. Les instantanés de Steamback restent sur le Deck et ne couvrent que les jeux Steam Cloud.

## Une note sur EmuDeck

EmuDeck revient dans ces discussions, et ce n'est pas un concurrent au sens habituel : c'est un installateur et configurateur d'émulateurs pour Steam Deck, et la synchronisation qu'il propose est un confort greffé sur cette mission (Rclone vers un cloud de fichiers, pour les sauvegardes d'émulateurs uniquement). Il recoupe les outils ci-dessus sans être de la même nature : EmuDeck installe vos émulateurs, les outils d'ici veillent sur les sauvegardes de toute la bibliothèque. Beaucoup font tourner EmuDeck à côté de l'un d'eux, et c'est une configuration sensée, pas une redondance.

## Hoard

Hoard sauvegarde chaque partie dès que le jeu a fini de l'écrire et la garde synchronisée sur toutes les machines où vous jouez. Le moteur tourne en service d'arrière-plan, `hoardd`, sans fenêtre, il continue donc de fonctionner en mode Jeu de SteamOS. Il attend que le dossier soit calme quelques secondes, donc il n'envoie jamais un fichier à moitié écrit ; il fait une copie de plus quand vous quittez ; et il ne remplace jamais la sauvegarde d'un jeu en cours : ce qui arrive d'une autre machine attend que vous fermiez le jeu.

- **L'autre machine peut être éteinte.** La sauvegarde la plus récente attend sur le serveur, donc le Deck que vous reprenez la semaine prochaine la reçoit sans que le PC soit allumé. Les outils pair-à-pair ont besoin des deux appareils éveillés en même temps.
- **Toutes les versions, conservées.** Chaque sauvegarde est une version à laquelle revenir depuis l'application ou avec `hoard restore`, y compris après une panne de disque ou une installation neuve. Hoard One garde toutes les versions pour de bon ; sur votre propre serveur, c'est vous qui décidez combien d'historique garder.
- **Seul ce qui change voyage.** Les envois ne transmettent que les fichiers modifiés, compressés en zstd, et les versions partagent tout ce qui ne change pas : dix versions d'une sauvegarde de 2 Go qui change peu occupent environ 2 Go, pas 20.
- **SHA-256 à l'envoi et au retour.** La corruption est détectée avant de pouvoir écraser une bonne sauvegarde, et Hoard garde une copie de tout ce qu'il remplace.
- **Gratuit et open source, avec votre propre serveur si vous le voulez.** L'application, la CLI et le serveur sont sous AGPL-3.0. Hoard One, le serveur géré que nous recommandons à tout le monde, est gratuit avec 2 Go et trois appareils, sans rien à installer. Ou lancez `hoard-server` avec Docker sur un PC, un NAS ou un Raspberry Pi, sur disque local ou n'importe quel stockage compatible S3 : aucun compte chez nous, aucune télémétrie vers nous, aucun quota de notre part.
- **Windows, Linux, macOS et SteamOS**, avec la même application de bureau sur les quatre et une CLI pour les serveurs et les machines sans écran.
- **Une détection qui couvre toute la bibliothèque :** Steam, Epic, GOG, préfixes Proton et Wine, des milliers de jeux grâce à la base communautaire des sauvegardes, et des préréglages pour 19 émulateurs, de PCSX2, Dolphin et RPCS3 à Ryujinx, Cemu, DuckStation et RetroArch.

## Le détail qui décide de la synchro Steam Deck ↔ PC

Bon à savoir quel que soit l'outil choisi. La sauvegarde cloud d'un jeu Steam vit dans `<AppID>/remote/`, et le dossier *au-dessus* contient `remotecache.vdf`, l'état des succès, les statistiques et les compteurs de temps de jeu — autant de choses qui diffèrent légitimement entre votre Deck et votre fixe.

Synchronisez le dossier parent et vous obtenez un conflit permanent entre deux machines qui n'ont jamais été en désaccord sur une seule sauvegarde. Hoard suit `remote/`, pas le dossier parent. N'importe quel outil auquel vous désignez un dossier à la main peut faire pareil, et c'est la première chose à vérifier quand une configuration de synchronisation signale des conflits sans raison visible.

## Ce que Hoard ne fait pas

- **Synchroniser sans serveur.** Hoard synchronise toujours par un serveur : Hoard One si vous préférez ne rien faire tourner, ou le vôtre. C'est justement ce qui permet à l'autre appareil d'être éteint, et ce qui laisse une copie qui survit aux deux machines. Si vous ne voulez de serveur en aucun cas, OpenSave, Syncthing ou Ludusavi sont faits pour ça.
- **Partager des mondes en coop.** Passer un monde à un ami, c'est le travail de SaveSync ; Hoard s'occupe de vos propres sauvegardes.
- **Fusionner deux sauvegardes qui ont divergé.** Aucun outil ne peut fusionner deux parties en une. Hoard garde les deux comme versions, sur toutes vos machines, et vous choisissez.

## La distinction entre Hoard One et l'auto-hébergement

Les comparaisons sur Hoard confondent presque toujours les deux, et le résultat est faux. Autant le dire clairement :

- **Hoard One** est l'option gérée, celle que nous recommandons à tous : vous vous connectez, et vos sauvegardes sont stockées sur nos serveurs, dans l'UE.
- **Un Hoard auto-hébergé est entièrement le vôtre.** Vous faites tourner `hoard-server` sur votre PC ou votre NAS, et vos sauvegardes vont de votre machine à votre disque. Il n'y a **aucun compte chez nous, aucune télémétrie vers nous, aucun quota et aucun relais** : rien ne passe par nos serveurs, puisque rien de chez nous n'est sur le chemin. Nous ne voyons ni sauvegarde, ni nom de jeu, ni adresse e-mail, car rien de cela ne nous parvient. Si Hoard One fermait demain, une installation auto-hébergée continuerait à l'identique.

Le même code, la même détection. Ce qui change, c'est à qui appartient le stockage, plus un réglage par défaut : un serveur auto-hébergé élague les anciennes versions avec le temps, sauf si vous le désactivez. Et pour être exact sur un point : votre serveur a bien ses propres accès — un utilisateur et un jeton par appareil — mais ils vivent dans votre base, pas dans la nôtre.

## Le tableau

| Outil | Synchro automatique | Fonctionne avec l'autre appareil éteint | Où vivent les sauvegardes | Historique | Plateformes | Licence |
|---|---|---|---|---|---|---|
| **Hoard** | Oui, à chaque sauvegarde du jeu | Oui | Hoard One ou votre serveur (disque ou compatible S3) | Toutes les versions, dédupliquées | Win · Linux · macOS · Deck | AGPL-3.0, gratuit |
| **Ludusavi** | Manuelle, ou Rclone que vous câblez | Seulement avec un distant Rclone | Local, plus votre distant Rclone | Sauvegardes locales versionnées | Win · Linux · macOS | MIT, gratuit |
| **Syncthing** | Oui, miroir continu | Non, les deux doivent être allumés | Vos appareils uniquement | Versionnage par fichier | Partout | MPL-2.0, gratuit |
| **OpenSave** | Oui, pair-à-pair | Non, sauf si vous configurez un miroir cloud | Vos appareils, miroir cloud en option | Instantanés et branches | Win · Linux · macOS · Deck | MIT, gratuit |
| **OpenCloudSaves** | Via votre cloud | Oui, via le cloud | OneDrive / Drive / Dropbox / Nextcloud | Ce que garde le cloud | Win · Linux · macOS | Open source, archivé |
| **Game Backup Monitor** | Non | Pas de synchro | Archives 7-Zip locales | Sauvegardes numérotées | Windows · Linux | GPL-3.0, gratuit |
| **GameSave Manager** | Via votre cloud (Sync & Link) | Oui, via le cloud | Local, plus Dropbox / OneDrive | Ce que garde le cloud | Windows | Gratuit, code fermé |
| **Game Save Manager (GSM)** | Non, export et import | Pas de synchro | Sauvegardes locales | Sauvegardes locales tournantes | Windows | GPL-3.0, gratuit |
| **SaveState** | En option, des archives | Oui, via votre stockage | Zips locaux, plus Drive / WebDAV / FTP / SMB / Git | Archives tournantes | Windows · Linux | GPL-3.0, gratuit |
| **EmuSync** | Oui, via votre cloud | Oui, via le cloud | Drive / Dropbox / OneDrive ou un dossier partagé | Sauvegardes locales | Windows · Linux · Deck | GPL-3.0, gratuit |
| **Aletheia** | Sauvegarde et restauration par launcher | Pas de synchro | Votre stockage | Sauvegardes | Win · Linux · macOS | AGPL-3.0 |
| **SaveSync** | Oui, et avec des amis | Oui, via Steam | Entrées privées du Steam Workshop | Selon l'application | Windows | Payant, code fermé |
| **Tachyon** | Oui, via son propre cloud | Oui | Le cloud de Tachyon | Versions et chronologies | Windows (bêta) | Bêta gratuite, sans code publié |

## Alors lequel

Si vous voulez que les sauvegardes et la synchro se fassent toutes seules, entre PC et Steam Deck, avec toutes les versions gardées, l'autre machine libre de rester éteinte et la possibilité que tout vous appartienne, c'est Hoard. [Téléchargez-le](/download), connectez-vous à Hoard One, ou lisez d'abord [comment l'auto-héberger avec Docker](/guides/self-host-hoard). C'est gratuit dans les deux cas.

Pour des besoins plus étroits : un seul PC Windows et des archives locales, Game Backup Monitor ou GSM. Des sauvegardes locales en gardant tout le contrôle et sans serveur nulle part, Ludusavi. Aucun compte en aucun cas, avec des appareils souvent allumés en même temps, OpenSave ou Syncthing. Des sauvegardes qui doivent vivre dans un dossier Drive que vous payez déjà, Ludusavi avec un distant Rclone, ou EmuSync ; OpenCloudSaves le faisait, mais il est archivé. Partager un monde coop avec des amis, SaveSync.

Si vous venez d'OpenCloudSaves et vouliez surtout vos sauvegardes dans un stockage que vous contrôlez, plutôt que dans Drive en tant que tel, un Hoard auto-hébergé le fait, avec l'historique en plus : il écrit sur disque local ou dans n'importe quel bucket compatible S3, comme Backblaze B2 ou MinIO. Si c'est Ludusavi que vous comparez, il y a une [comparaison plus longue avec Ludusavi](/guides/ludusavi-alternative).

## Comparaisons en tête-à-tête

Chacune va plus loin que la section ci-dessus, y compris sur les points où l'autre outil l'emporte :

- [Hoard face à Ludusavi](/guides/ludusavi-alternative)
- [Hoard comme alternative à Steam Cloud](/guides/steam-cloud-alternative)
- [Synchro pair-à-pair face à un serveur qui vous appartient](/guides/opensave-alternative)
- [Syncthing pour les sauvegardes : ce qui casse](/guides/syncthing-game-saves)
- [Hoard face à GameSave Manager et aux autres Game Save Manager](/guides/game-save-manager-alternative)
- [Remplacer OpenCloudSaves maintenant qu'il est archivé](/guides/opencloudsaves-alternative)
- [Une alternative gratuite à SaveSync](/guides/savesync-alternative)

<!-- faq -->

## Questions fréquentes

### Lequel de ces outils garde un historique de versions ?

Hoard conserve chaque sauvegarde comme une version où revenir, sur toutes vos machines. Ludusavi, GSM et SaveState gardent des sauvegardes locales versionnées ou tournantes, et OpenSave garde des instantanés sur vos appareils. La plupart des autres synchronisent ou copient l'état actuel : une sauvegarde corrompue est donc fidèlement propagée à votre autre machine.

### Lequel fonctionne sans serveur ni compte ?

Ludusavi en sauvegardes locales, et tout outil pair-à-pair. Hoard entre aussi dans cette catégorie si vous l'auto-hébergez : aucun compte chez nous, et rien qui passe par nos serveurs.

### Lequel synchronise quand mon autre appareil est éteint ?

Tout outil qui garde la sauvegarde ailleurs que sur vos appareils : Hoard, via Hoard One ou votre propre serveur, et les outils qui écrivent dans un cloud, comme EmuSync ou Ludusavi avec Rclone. La synchro pair-à-pair (OpenSave sans son miroir, Syncthing) exige les deux appareils allumés en même temps.

### Lequel couvre les jeux absents de Steam ?

Tous les gestionnaires de sauvegardes cités, car ils localisent les fichiers via la même base communautaire et non via une boutique. L'exception est Steam Cloud : il ne couvre que les jeux Steam dont le développeur l'a activé.

### Dois-je n'en choisir qu'un ?

Non, et beaucoup ne le font pas. Un outil de sauvegarde locale et un outil de synchro règlent deux moitiés différentes du problème. La seule règle : ne jamais pointer l'un vers le dossier de sauvegardes de l'autre, sinon vous synchronisez un miroir périmé au lieu de votre sauvegarde réelle.

### Quel est le détail qui casse la plupart des montages maison ?

Synchroniser le dossier situé au-dessus de `<AppID>/remote/` dans le `userdata` de Steam. Le parent contient `remotecache.vdf` et des fichiers de succès et de temps de jeu censés différer d'une machine à l'autre : chaque lancement ressemble alors à un conflit alors qu'aucune sauvegarde n'a bougé.
