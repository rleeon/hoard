---
title: "Comment sauvegarder vos parties automatiquement"
description: "Configurez des sauvegardes cloud automatiques et versionnées de vos parties PC avec Hoard — pour qu'un plantage, une réinstallation ou un mod défectueux n'efface jamais votre progression."
order: 1
updated: 2026-09-01
---

Perdre une sauvegarde, c'est perdre des heures de progression. Hoard sauvegarde vos parties PC automatiquement et conserve un historique complet des versions, pour que vous puissiez toujours revenir en arrière.

## Ce que Hoard sauvegarde

Hoard détecte les dossiers de sauvegarde des jeux auxquels vous jouez et les copie vers votre propre cloud — Hoard Cloud ou un serveur que vous hébergez vous-même. Chaque sauvegarde est versionnée, les anciennes copies ne sont donc jamais écrasées.

Pour trouver où chaque jeu range ses sauvegardes, Hoard utilise la même base de données communautaire d'emplacements que celle qui alimente Ludusavi : la détection fonctionne donc d'emblée pour des milliers de titres. La différence, c'est ce qui se passe ensuite : au lieu de laisser la sauvegarde sur votre disque, Hoard la versionne automatiquement dans le cloud.

## Configurer les sauvegardes automatiques

1. **Téléchargez et installez Hoard** pour Windows, macOS ou Linux depuis la page de téléchargement.
2. Connectez-vous, ou pointez l'application vers votre serveur auto-hébergé.
3. Ouvrez la **Bibliothèque**. Hoard recherche les jeux installés et liste les sauvegardes trouvées.
4. Ajoutez les jeux à protéger. Hoard localise chaque dossier de sauvegarde automatiquement ; vous pouvez ajouter un chemin à la main si un jeu n'est pas détecté.
5. Laissez le **mode automatique** activé. Hoard surveille les dossiers de sauvegarde et les sauvegarde après que vous arrêtez de jouer.

Désormais, chaque session est capturée sans que vous ayez à faire quoi que ce soit.

## Où les jeux PC rangent vraiment leurs sauvegardes

Il n'y a pas d'endroit unique, et c'est précisément pour ça qu'un outil comme celui-ci existe. En pratique, une sauvegarde atterrit dans l'un de ces endroits :

- **Dans Steam**, sous `userdata/<UserID>/<AppID>/remote/` — le dossier que Steam Cloud synchronise lui-même.
- **`Documents\My Games\…`**, ce qui se rapproche le plus d'une convention sous Windows.
- **`%APPDATA%`, `%LOCALAPPDATA%` ou `LocalLow`**, où écrivent la plupart des jeux Unity et Unreal.
- **`%USERPROFILE%\Saved Games`**, utilisé par un groupe plus restreint mais tenace de titres.
- **Le dossier d'installation du jeu lui-même**, où un nombre surprenant de titres anciens sauvegardent encore.
- **Sous Linux**, `~/.local/share` ou `~/.config` pour les jeux natifs, et dans le préfixe Proton — `steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…` — pour les jeux Windows.
- **Sous macOS**, `~/Library/Application Support`.

La provenance du jeu ne change presque rien : les titres GOG, Epic et itch atterrissent dans la même poignée d'endroits, car ce sont le moteur et le développeur qui décident, pas la boutique.

## Ce qui est sauvegardé, et ce qui ne l'est pas

Un dossier de sauvegarde ne contient presque jamais que des sauvegardes, alors Hoard trie ce qu'il trouve en trois tas :

- **Les données de sauvegarde** sont sauvegardées et restaurées. C'est votre progression.
- **Les fichiers propres à une machine** — configuration, journaux et compagnie — sont envoyés pour faire partie de la sauvegarde, mais jamais réécrits par-dessus la copie d'un autre PC. Vos réglages graphiques restent les vôtres.
- **Le déchet** — caches, rapports de plantage, fichiers temporaires — est ignoré, pour qu'une sauvegarde n'enfle pas avec ce que vous ne voudriez jamais récupérer.

## Quand la sauvegarde a lieu

Hoard surveille le dossier et le capture **après que vous avez arrêté de jouer**, pas pendant qu'un jeu garde des fichiers ouverts. Si la sauvegarde a été écrite il y a quelques secondes, il attend que le calme revienne : un fichier en cours d'écriture ne mérite pas d'être capturé à moitié.

Chaque capture est une version. Les instantanés sont stockés par empreinte de contenu : un fichier inchangé n'est stocké qu'une fois — dix versions d'une sauvegarde de 2 Go coûtent environ 2 Go, pas 20.

## Sauvegarder sans passer par nos serveurs

Si vous préférez n'utiliser le cloud de personne, faites tourner `hoard-server` vous-même et pointez l'application dessus. Vos sauvegardes vont de votre PC à votre disque : aucun compte chez nous, aucune télémétrie vers nous, et rien qui passe par nos serveurs. Voir [comment auto-héberger Hoard](/guides/self-host-hoard).

## Astuce : consultez votre historique

Ouvrez l'onglet **Historique** d'un jeu pour voir chaque sauvegarde avec sa date et sa taille. De là, vous pouvez restaurer n'importe quelle version précédente en un clic. Vos sauvegardes circulent chiffrées, sont stockées dans l'UE, et vous pouvez les exporter ou les supprimer quand vous voulez.

Vous utilisez déjà un outil de sauvegarde locale comme Ludusavi ? Vous pouvez le garder — mais si vous voulez que ces sauvegardes arrivent dans le cloud et se synchronisent entre vos machines sans scripter Rclone vous-même, c'est précisément ce que Hoard automatise. Voir [Ludusavi vs Hoard](/guides/ludusavi-alternative) pour une comparaison équitable.

<!-- faq -->

## Questions fréquentes

### Hoard sauvegarde-t-il pendant que je joue ?

Non. Il attend que vous ayez arrêté et que le dossier se calme, pour qu'une sauvegarde ne soit jamais un fichier à moitié écrit.

### Quelle place prennent mes sauvegardes ?

Moins qu'on ne croit. Les versions sont dédupliquées par empreinte de contenu : seule la partie réellement modifiée entre deux sessions occupe de la place — la plupart des collections tiennent largement dans quelques gigaoctets.

### Et si l'un de mes jeux n'est pas détecté ?

Pointez Hoard sur le dossier à la main et il le suivra comme les autres. La détection couvre des milliers de titres, mais un jeu qui sauvegarde à un endroit inhabituel, ou que vous avez installé à la main, a parfois besoin de l'indice.

### Est-ce qu'il sauvegarde mes mods ?

Hoard suit le dossier de sauvegarde : les mods rangés ailleurs ne font pas partie de la sauvegarde. C'est volontaire — les mods sont volumineux, ils se retéléchargent, et un dossier de mods synchronisé entre machines crée plus de problèmes qu'il n'en résout.

### L'auto-hébergement change-t-il quelque chose aux sauvegardes ?

Rien du tout. Même détection, mêmes versions, même capture automatique. Seul le stockage est à vous.
