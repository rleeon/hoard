---
title: "Emplacement des sauvegardes de Cyberpunk 2077 (PC et Steam Deck)"
description: "Où Cyberpunk 2077 range ses sauvegardes sous Windows, Steam Deck et Mac, ce que contient chaque dossier et comment les sauvegarder et les synchroniser."
order: 20
updated: 2026-10-09
---

Sous Windows, Cyberpunk 2077 range ses sauvegardes dans `%USERPROFILE%\Saved Games\CD Projekt Red\Cyberpunk 2077`, un dossier par sauvegarde. Voilà la réponse courte. La suite couvre les chemins sur Steam Deck et Mac, ce que contient vraiment le dossier et comment le garder sauvegardé et synchronisé entre votre PC et votre Steam Deck.

## Où Cyberpunk 2077 range ses sauvegardes

- **Windows** (Steam, GOG ou Epic) : `%USERPROFILE%\Saved Games\CD Projekt Red\Cyberpunk 2077`
- **Steam Deck et Linux** (Proton) : `~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077`
- **Mac :** `~/Library/Application Support/CD Projekt Red/Cyberpunk 2077/Saves`

Les chemins Windows et Mac sont ceux que donne CD Projekt Red dans ses propres pages d'assistance. Sur Steam Deck, le jeu tourne dans un préfixe Proton, une petite arborescence Windows que Steam garde pour chaque jeu ; `1091500` est l'identifiant Steam de Cyberpunk. Si le jeu est installé sur la carte microSD, cherchez `steamapps/compatdata/1091500` sur la carte.

## Ce que contient le dossier

Cyberpunk n'écrit pas un seul fichier de sauvegarde, mais **un dossier par sauvegarde** : `AutoSave-0`, `AutoSave-1` et ainsi de suite, `ManualSave-0`, `ManualSave-1` et `QuickSave-0`. Chacun contient la sauvegarde elle-même (`sav.dat`) ainsi que la capture et les métadonnées affichées dans le menu de chargement.

Deux conséquences :

- **Sauvegardez le dossier parent, pas une seule sauvegarde.** Copier seulement le `ManualSave` le plus récent laisse de côté les sauvegardes automatiques, qui contiennent souvent la progression la plus récente.
- **Les sauvegardes automatiques tournent.** Le jeu réutilise quelques dossiers `AutoSave` et écrase le plus ancien. Une sauvegarde automatique d'il y a trois heures a généralement déjà disparu, d'où l'intérêt d'un historique hors du jeu.

Les réglages ne sont pas là. Graphismes et commandes vivent dans `UserSettings.json`, sous `%LOCALAPPDATA%\CD Projekt Red\Cyberpunk 2077`, à côté des caches et des journaux. C'est le dossier que beaucoup de gens (et certains outils) sauvegardent par erreur : il ne contient rien dont la perte vous coûterait de la progression.

## Cyberpunk 2077 a-t-il des sauvegardes cloud ?

Oui. La version Steam utilise Steam Cloud, la version GOG le cloud de GOG Galaxy. Les deux gardent le dernier état de vos sauvegardes synchronisé entre les machines d'une même boutique.

Ce qu'aucun des deux ne fait :

- **Garder les anciennes versions.** Si une sauvegarde est corrompue, ou cassée par un mod, le cloud garde aussi la copie cassée.
- **Passer d'une boutique à l'autre.** Steam Cloud et le cloud de GOG ne communiquent pas, alors que les sauvegardes PC de Steam, GOG et Epic se chargent sans problème dans n'importe laquelle si vous copiez le dossier.

## Sauvegarder à la main

1. Fermez complètement le jeu.
2. Copiez tout le dossier `Cyberpunk 2077` depuis le chemin ci-dessus vers une clé USB, un autre disque ou un dossier cloud.
3. Pour restaurer, fermez le jeu et recopiez le dossier en remplaçant l'existant.

Ça marche, mais seulement aussi souvent que vous y pensez, et vous n'avez que la copie de la dernière fois.

## Sauvegarde et synchro automatiques avec Hoard

[Hoard](/download) sauvegarde le dossier à chaque fois que vous arrêtez de jouer et garde toutes les versions : une sauvegarde corrompue ou une sauvegarde automatique écrasée est à un clic. Il synchronise aussi le dossier entre vos PC et un Steam Deck.

1. Installez Hoard et connectez-vous, ou pointez-le vers [votre propre serveur](/guides/self-host-hoard).
2. Ouvrez la **Bibliothèque**. Cyberpunk est détecté à partir de votre bibliothèque Steam et de la base de données communautaire des sauvegardes.
3. Vérifiez que le dossier affiché est celui de `Saved Games\CD Projekt Red\Cyberpunk 2077`. S'il affiche celui d'`AppData\Local`, changez-le : celui-là ne contient que des réglages.
4. Jouez. En quittant, la première version apparaît dans l'historique.

Hoard suit le dossier entier, donc chaque `AutoSave`, `ManualSave` et `QuickSave` entre dans la même version. Avec un Deck et un PC fixe, la version la plus récente vous attend sur celui que vous reprenez — voir [comment fonctionne la synchronisation entre PC](/guides/sync-game-saves-across-pcs).

<!-- faq -->

## Questions fréquentes

### Où sont les sauvegardes de Cyberpunk 2077 sur Steam Deck ?

Dans le préfixe Proton du jeu : `~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077`. Si le jeu est sur la carte microSD, le dossier `compatdata` est sur la carte.

### Puis-je passer mes sauvegardes de Cyberpunk 2077 de GOG à Steam ?

Oui. Les sauvegardes PC sont les mêmes sur Steam, GOG et Epic. Copiez les dossiers de sauvegarde au même chemin dans l'autre installation, jeu fermé, et ils apparaissent dans le menu de chargement.

### Pourquoi y a-t-il autant de dossiers AutoSave ?

Le jeu garde quelques emplacements de sauvegarde automatique et écrase le plus ancien à chaque fois. Ce sont des sauvegardes normales, simplement remplacées toutes seules.

### Pourquoi mon ancienne sauvegarde automatique a-t-elle disparu ?

Parce que son emplacement a été réutilisé. Le jeu n'en garde que quelques-uns. Un outil de sauvegarde qui conserve des versions est le seul moyen de la récupérer une fois écrasée.

### Hoard synchronise-t-il aussi mes réglages ?

Non. Les réglages vivent dans un autre dossier qui ne fait pas partie de la sauvegarde, donc chaque machine garde les siens, ce qui est en général ce que vous voulez : un Deck et un PC fixe ont besoin de réglages graphiques différents. Plus de détails dans [synchroniser ses parties entre PC](/guides/sync-game-saves-across-pcs).
