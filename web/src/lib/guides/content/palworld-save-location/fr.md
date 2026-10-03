---
title: "Emplacement des sauvegardes de Palworld (PC et Steam Deck)"
description: "Où Palworld range ses mondes sur PC et Steam Deck, à quoi sert chaque fichier, comment marchent les mondes en coop et comment sauvegarder ou transférer vos parties."
order: 24
updated: 2026-10-02
---

Sur PC (Steam), Palworld range ses sauvegardes dans `%LOCALAPPDATA%\Pal\Saved\SaveGames\<votre identifiant Steam>`, avec un dossier par monde. C'est le chemin que donne Pocketpair dans sa FAQ officielle. Vous trouverez ci-dessous le chemin sur Steam Deck, le rôle de chaque fichier, ce que change la coop et comment garder vos mondes sauvegardés.

## Où Palworld range ses sauvegardes

- **Windows, version Steam :** `%LOCALAPPDATA%\Pal\Saved\SaveGames\<votre identifiant Steam>\<identifiant du monde>`
- **Steam Deck et Linux** (Proton) : `~/.local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames/`

Le dossier de l'identifiant Steam est un long numéro lié à votre compte Steam. À l'intérieur, chaque monde créé a son propre dossier au long nom hexadécimal. Sur Steam Deck, le jeu passe par Proton, donc les sauvegardes sont dans le préfixe Proton que Steam garde pour lui ; `1623730` est l'identifiant Steam du jeu.

La version de l'application Xbox / Game Pass range ses sauvegardes ailleurs, dans un emplacement empaqueté, et ce ne sont pas les mêmes fichiers que ceux que vous copieriez entre installations Steam.

## Ce que contient le dossier d'un monde

- **`Level.sav`** est le monde lui-même : votre base, la carte, les Pals qui y sont placés.
- **`LevelMeta.sav`** contient le nom et le résumé du monde pour le menu.
- **`Players\`** contient un `.sav` par joueur passé dans ce monde.
- **`LocalData.sav`** et **`WorldOption.sav`** contiennent des données locales et les réglages du monde.
- **`backup\`** contient les sauvegardes automatiques du monde faites par le jeu.

Les réglages sont ailleurs : `Pal\Saved\Config\Windows\GameUserSettings.ini` pour les graphismes et les commandes, `Pal\Saved\Logs` pour les journaux. Aucun ne fait partie de votre progression.

Pour sauvegarder, prenez **tout le dossier du monde**, pas seulement `Level.sav`. Le monde et les fichiers des joueurs vont ensemble, et en restaurer un sans l'autre, c'est ainsi que les personnages se retrouvent décalés par rapport à leur monde.

## Coop et serveurs dédiés

En coop, **le monde vit sur le PC de l'hôte**. Votre personnage dans ce monde est un fichier du dossier `Players` de l'hôte, pas sur votre machine. Si l'hôte perd sa sauvegarde, la progression de tous dans ce monde disparaît avec. Sur un serveur dédié, le monde vit sur le serveur.

Pour un monde partagé, c'est donc le dossier de l'hôte qui doit être sauvegardé.

## Palworld a-t-il des sauvegardes cloud ?

La version Steam utilise Steam Cloud, qui garde le dernier état de vos mondes synchronisé entre les machines d'un même compte. Il ne garde pas les anciennes versions, et le dossier `backup\` du jeu est sur le même disque que la sauvegarde : un disque mort emporte les deux.

## Sauvegarder à la main

1. Fermez complètement le jeu.
2. Copiez votre dossier d'identifiant Steam depuis `SaveGames` (il contient tous vos mondes) vers un endroit sûr.
3. Pour restaurer, fermez le jeu et recopiez-le au même endroit.

## Sauvegarde et synchro automatiques avec Hoard

[Hoard](/download) sauvegarde vos mondes à chaque fois que vous arrêtez de jouer et garde toutes les versions hors de la machine : un monde corrompu ou un disque perdu n'est pas la fin d'une base construite pendant des semaines. Il synchronise aussi les mondes entre vos PC et un Steam Deck.

1. Installez Hoard et connectez-vous, ou pointez-le vers [votre propre serveur](/guides/self-host-hoard).
2. Ouvrez la **Bibliothèque**. Palworld est détecté à partir de votre bibliothèque Steam et de la base de données communautaire des sauvegardes.
3. Jouez. En quittant, la première version apparaît dans l'historique.

Si vous hébergez en coop, c'est cette machine qui compte : sauvegardez l'hôte, et le monde partagé est couvert. Pour ramener un monde en arrière, [restaurez une version antérieure](/guides/restore-a-game-save).

<!-- faq -->

## Questions fréquentes

### Où sont les sauvegardes de Palworld sur Steam Deck ?

Dans le préfixe Proton : `~/.local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames/`, dans le dossier à votre identifiant Steam.

### Où est sauvegardé mon personnage dans le monde d'un ami ?

Sur le PC de l'hôte, dans le dossier `Players` de ce monde. Votre PC ne garde pas de copie des mondes hébergés par quelqu'un d'autre.

### Quel fichier est mon monde ?

`Level.sav`, mais sauvegardez tout le dossier du monde : les fichiers des joueurs et le monde vont ensemble.

### Palworld sauvegarde-t-il mon monde tout seul ?

Il garde des sauvegardes automatiques dans le dossier `backup` du monde. Elles sont sur le même disque que la sauvegarde : elles protègent d'une mauvaise sauvegarde, pas de la perte du disque.
