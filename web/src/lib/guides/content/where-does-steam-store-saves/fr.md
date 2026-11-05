---
title: "Où Steam enregistre-t-il les sauvegardes ? Tous les emplacements sur PC et Steam Deck"
description: "Steam a un dossier userdata, mais la plupart des jeux sauvegardent dans AppData, Documents ou leur dossier. Où chercher sous Windows, Steam Deck et Mac."
order: 30
updated: 2026-11-05
---

Steam a son propre dossier pour les sauvegardes, `userdata\<account-id>\<AppID>\remote` dans le dossier de Steam, mais la plupart des jeux ne s'en servent jamais. Dans la base communautaire des sauvegardes, qui connaît le chemin Windows de 10 937 jeux Steam, seul **1 sur 10** environ y range ses sauvegardes. Les autres écrivent dans AppData, dans Documents ou dans leur propre dossier d'installation, et Steam Cloud, quand le jeu l'a, récupère les fichiers là où ils sont.

## Le dossier de Steam : userdata

- **Windows :** `C:\Program Files (x86)\Steam\userdata\<account-id>\<AppID>\remote`
- **Steam Deck et Linux :** `~/.local/share/Steam/userdata/<account-id>/<AppID>/remote`
- **macOS :** `~/Library/Application Support/Steam/userdata/<account-id>/<AppID>/remote`

`<account-id>` est le numéro que Steam affiche comme votre **code ami** (Amis, puis Ajouter un ami). Il y a un dossier par compte qui s'est connecté sur cette machine. `<AppID>` est le numéro dans l'adresse du jeu sur la boutique : `store.steampowered.com/app/413150`, c'est Stardew Valley.

Le dossier au-dessus de `remote`, celui du `<AppID>` tout court, contient aussi `remotecache.vdf`. C'est la comptabilité de Steam, pas votre sauvegarde, et il est censé différer entre votre PC et votre Deck.

## Où la plupart des jeux sauvegardent vraiment

Compté dans cette même base le 5 novembre 2026, la part des jeux Steam qui utilisent chaque endroit (certains en utilisent plusieurs) :

- **`AppData\LocalLow`**, 23 %, surtout des jeux faits avec Unity : `%USERPROFILE%\AppData\LocalLow\<développeur>\<jeu>`
- **Le dossier d'installation du jeu**, 22 % : `C:\Program Files (x86)\Steam\steamapps\common\<jeu>`
- **`AppData\Local`**, 21 % : `%LOCALAPPDATA%\<développeur ou jeu>`
- **`Documents`**, 15 %, souvent sous `My Games` : `%USERPROFILE%\Documents\My Games\<jeu>`
- **`AppData\Roaming`**, 12 % : `%APPDATA%\<jeu>`
- **Le dossier `userdata` de Steam**, 10 % : voir plus haut
- **`Jeux enregistrés`**, 3 % : `%USERPROFILE%\Saved Games\<développeur>\<jeu>`

## Sur Steam Deck : le préfixe Proton

Les jeux Windows tournent sur le Deck via Proton, et chacun a son propre petit Windows dans `~/.local/share/Steam/steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/`. AppData, Documents et Jeux enregistrés s'y trouvent, donc `%APPDATA%\Jeu` sur un PC devient `compatdata/<AppID>/pfx/drive_c/users/steamuser/AppData/Roaming/Jeu` sur le Deck. Le préfixe apparaît au premier lancement du jeu.

Les jeux avec une version Linux native se passent du préfixe et sauvegardent en général sous `~/.local/share` ou `~/.config`. Le dossier d'installation est `~/.local/share/Steam/steamapps/common/<jeu>`, ou la même arborescence `steamapps` sur la carte microSD si le jeu y est installé.

## Trouver vite les sauvegardes d'un jeu

1. Cherchez le jeu dans la [liste des emplacements de sauvegarde](/saves), ou sur sa page PCGamingWiki, à « Save game data location ».
2. S'il utilise le dossier de Steam, ouvrez `userdata\<account-id>\<AppID>\remote` avec l'AppID de sa page boutique.
3. Sinon, sauvegardez dans le jeu, puis triez `%LOCALAPPDATA%`, `%USERPROFILE%\AppData\LocalLow`, `%APPDATA%` et `Documents` par date de modification. Le dossier qui vient de changer est le bon.

## Ce que garde Steam Cloud

Steam Cloud garde la dernière sauvegarde sur votre compte Steam, pour les jeux dont le développeur l'a activé. La [page de stockage distant](https://store.steampowered.com/account/remotestorage) de Steam liste chaque jeu ayant des fichiers dans votre Steam Cloud et vous permet de les télécharger.

Il garde le dernier état, pas un historique, donc une sauvegarde corrompue se synchronise aussi fidèlement qu'une bonne, et il ne couvre que les jeux Steam : rien de GOG, d'Epic, d'un émulateur ou d'un raccourci hors Steam. Plus de détails dans le [guide sur l'alternative à Steam Cloud](/guides/steam-cloud-alternative).

## Sauvegarde et synchro automatiques avec Hoard

[Hoard](/download) est gratuit et open source. Il trouve les sauvegardes de chaque jeu là où le jeu les a mises, userdata, AppData, Documents, le dossier d'installation ou un préfixe Proton sur le Deck, les sauvegarde chaque fois que le jeu en écrit une puis encore quand vous quittez, garde un historique où revenir et les synchronise entre vos PC et un Steam Deck. Dans le dossier de Steam, il suit `<AppID>/remote/` et laisse `remotecache.vdf` tranquille, pour que votre PC et votre Deck n'aient jamais l'air en désaccord.

1. Installez Hoard et connectez-vous à Hoard One, recommandé pour tout le monde, ou pointez-le vers [votre propre serveur](/guides/self-host-hoard).
2. Ouvrez la **Bibliothèque**. Vos jeux Steam sont détectés à partir de votre bibliothèque Steam et de la base de données communautaire des sauvegardes.
3. Jouez. En quittant, la première version apparaît dans l'historique.

## Hoard One, ou votre propre serveur

Hoard est gratuit dans les deux cas, et l'application est la même :

- **Hoard One**, recommandé pour tout le monde. Rien à installer ni à faire tourner : un serveur géré dans l'UE, gratuit avec 2 Go et trois appareils, sans carte. [Pro](/pricing) ajoute seulement de la place, 100 Go et des appareils illimités, pour le jour où vos sauvegardes ne tiennent plus.
- **Votre propre serveur.** Faites tourner `hoard-server` sur un PC, un NAS ou un Raspberry Pi, et toutes vos machines se synchronisent à travers lui. Pas de compte chez nous, pas de télémétrie vers nous, pas de quota de notre part : rien ne passe par nos serveurs, puisque rien de nous n'est sur le chemin. Voir [comment héberger Hoard vous-même](/guides/self-host-hoard).

Même programme, même détection, même historique. Hoard One garde toutes les versions pour toujours ; sur votre propre serveur, c'est vous qui décidez combien d'historique garder.

<!-- faq -->

## Questions fréquentes

### Où se trouve le dossier userdata de Steam ?

Dans le dossier de Steam : `C:\Program Files (x86)\Steam\userdata` sous Windows si Steam est à son emplacement par défaut, `~/.local/share/Steam/userdata` sur un Steam Deck ou sous Linux, et `~/Library/Application Support/Steam/userdata` sur un Mac.

### Désinstaller un jeu supprime-t-il ses sauvegardes ?

Les sauvegardes du dossier d'installation partent avec lui. Celles d'AppData, de Documents ou de userdata restent en général. Sur le Deck, désinstaller peut aussi supprimer le préfixe Proton du jeu, et avec lui toutes les sauvegardes qu'il contient : sauvegardez-le avant.

### Puis-je copier à la main des sauvegardes Steam vers un autre PC ?

Oui. Fermez le jeu, copiez le dossier de sauvegarde au même endroit sur l'autre PC et lancez le jeu là-bas. S'il utilise Steam Cloud, Steam peut vous demander quelle copie garder au prochain lancement ; choisissez la plus récente.

### Hoard est-il gratuit ?

Oui. L'application, la ligne de commande et le serveur sont gratuits et open source (AGPL-3.0), et faire tourner votre propre serveur ne coûte que la machine. Hoard One, le serveur géré que nous recommandons à tous, est gratuit lui aussi avec 2 Go et trois appareils ; Pro ajoute de la place pour 1,99 € par mois.

### L'auto-hébergement envoie-t-il quelque chose à Hoard ?

Non. Sur votre propre serveur, il n'y a ni compte chez nous ni télémétrie vers nous : vos sauvegardes, vos utilisateurs et vos journaux restent sur votre machine et ne touchent jamais les nôtres.
