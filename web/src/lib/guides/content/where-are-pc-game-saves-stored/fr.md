---
title: "Où sont stockées les sauvegardes des jeux PC ? Tous les emplacements"
description: "Où les jeux PC rangent leurs sauvegardes sous Windows, Steam Deck, Linux et Mac, les launchers à part, et comment trouver celles de n'importe quel jeu."
order: 11
updated: 2026-10-09
---

Il n'y a pas un seul dossier. Sous Windows, presque tous les jeux sauvegardent à l'un de six endroits : `Documents`, `Saved Games`, l'un des trois dossiers `AppData`, le `userdata` de Steam ou leur propre dossier d'installation. C'est le moteur et le développeur qui décident, pas la boutique où vous l'avez acheté. Cette page liste tous les emplacements courants, les launchers qui ajoutent leur propre couche, et une méthode rapide pour trouver les sauvegardes de n'importe quel jeu, même un jeu que personne n'a documenté.

## Windows : les six endroits habituels

| Dossier | Chemin typique | Qui l'utilise |
|---|---|---|
| Documents | `%USERPROFILE%\Documents\My Games\<Jeu>` | Jeux Bethesda, Rockstar (`Documents\Rockstar Games`), beaucoup d'anciens gros titres |
| Parties enregistrées | `%USERPROFILE%\Saved Games\<Éditeur>\<Jeu>` | Cyberpunk 2077 et une minorité tenace |
| AppData\Roaming | `%APPDATA%\<Jeu>` | Elden Ring, Stardew Valley, Minecraft (`.minecraft`), beaucoup d'indés |
| AppData\Local | `%LOCALAPPDATA%\<Jeu>\Saved\SaveGames` | Jeux Unreal Engine (Palworld utilise `Pal\Saved\SaveGames`) |
| AppData\LocalLow | `%USERPROFILE%\AppData\LocalLow\<Société>\<Jeu>` | Jeux Unity (Hollow Knight et bien d'autres) |
| userdata de Steam | `C:\Program Files (x86)\Steam\userdata\<UserID>\<AppID>\remote` | Jeux qui utilisent le stockage de sauvegardes de Steam |

Et un septième qui refuse de mourir : **le dossier d'installation du jeu**, où beaucoup d'anciens jeux et quelques indés écrivent encore.

Deux remarques pratiques. `AppData` est caché, alors tapez `%APPDATA%` ou `%LOCALAPPDATA%` dans la barre d'adresse de l'Explorateur au lieu d'y aller à coups de clics. Et si OneDrive sauvegarde votre dossier `Documents`, son vrai chemin est `C:\Users\<vous>\OneDrive\Documents`, ce qui surprend beaucoup de monde. Voir [OneDrive et les sauvegardes de jeux](/guides/onedrive-game-saves).

## Les launchers qui ajoutent leur propre couche

La plupart des launchers ne décident pas où vont les sauvegardes ; c'est le jeu qui décide. Quelques exceptions :

- **Steam** garde une zone de sauvegarde par jeu dans `userdata`. `<UserID>` est un nombre lié à votre compte Steam (il y a un dossier par compte qui s'est connecté sur ce PC) et `<AppID>` est le nombre dans l'URL du jeu sur le magasin.
- **Ubisoft Connect** range les sauvegardes dans `C:\Program Files (x86)\Ubisoft\Ubisoft Game Launcher\savegames\<ID utilisateur>\<ID du jeu>`, avec des nombres au lieu de noms aux deux niveaux.
- **L'application Xbox et le PC Game Pass** utilisent `%LOCALAPPDATA%\Packages\<paquet>\SystemAppData\wgs`, avec des fichiers aux noms aléatoires que seule l'application Xbox comprend. Laissez-les aux sauvegardes cloud Xbox ; les copier à la main fonctionne rarement.
- **Epic, GOG et l'application EA** laissent généralement le jeu décider, donc leurs titres finissent aux endroits habituels ci-dessus. Leurs sauvegardes cloud, quand elles existent, copient depuis là.

## Le registre, rarement

Quelques jeux, surtout de petits titres Unity, gardent la progression dans le registre Windows, sous `HKEY_CURRENT_USER\Software\<Société>\<Jeu>`, plutôt que dans un fichier. Il n'y a alors rien à copier dans un dossier, et les outils de sauvegarde basés sur les dossiers, Hoard compris, ne le voient pas. Si vous en avez besoin, exportez cette clé avec `regedit`.

## Steam Deck et Linux

- **Les jeux Windows via Proton** sauvegardent dans un préfixe par jeu : `~/.local/share/Steam/steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/`, puis le chemin Windows du tableau (`Documents`, `AppData/Roaming`, etc.). Les jeux sur une carte microSD ont la même arborescence sous le `steamapps/compatdata` de la carte.
- **Les jeux Linux natifs** utilisent `~/.local/share/<jeu>` ou `~/.config/<jeu>`. Les jeux Unity vont dans `~/.config/unity3d/<Société>/<Jeu>`.
- **Le userdata de Steam** se trouve dans `~/.local/share/Steam/userdata/<UserID>/<AppID>/remote`.
- **Heroic, Lutris et Bottles** gardent un préfixe Wine par jeu. À l'intérieur, l'arborescence Windows est sous `drive_c/users/<votre utilisateur>/`, pas sous `steamuser`.

Le Deck a son propre guide : [synchroniser ses sauvegardes entre Steam Deck et PC](/guides/sync-saves-steam-deck-pc).

## Mac

- **La plupart des jeux :** `~/Library/Application Support/<Jeu>`. Les jeux Unity utilisent `~/Library/Application Support/<Société>/<Jeu>`.
- **Les jeux du Mac App Store** sont isolés : `~/Library/Containers/<identifiant du paquet>/Data/Library/Application Support/`.

`~/Library` est caché lui aussi. Dans le Finder, ouvrez le menu **Aller** en maintenant la touche Option et il apparaît.

## Trouver les sauvegardes de n'importe quel jeu

Quand un jeu ne figure sur aucune liste, trois astuces le retrouvent en quelques minutes :

1. **Cherchez-le sur PCGamingWiki.** Presque chaque page de jeu a une section « Save game data location ». C'est la même source qui sert à construire les bases de données de sauvegardes utilisées par Hoard et Ludusavi.
2. **Regardez ce qui change.** Sauvegardez dans le jeu, quittez, puis cherchez dans votre dossier utilisateur les fichiers modifiés ces dernières minutes. Sous Windows, cherchez `datemodified:today` dans `C:\Users\<vous>` et triez par date. Sous Linux ou sur un Deck : `find ~ -type f -mmin -5 -not -path '*/.cache/*'`.
3. **Demandez à Steam.** Pour un jeu Steam Cloud, [store.steampowered.com/account/remotestorage](https://store.steampowered.com/account/remotestorage) liste les fichiers que Steam garde pour chaque jeu, avec noms et tailles. Une fois le nom du fichier connu, trouver le dossier est facile.

## Ou laissez quelqu'un les trouver pour vous

Hoard lit cette même base communautaire, qui couvre des milliers de jeux, et vérifie chaque chemin possible sur votre machine : préfixes Proton, Heroic et Lutris, le `Documents` de OneDrive, émulateurs, installations portables. Ce qu'il trouve est sauvegardé automatiquement chaque fois que vous arrêtez de jouer, chaque version est conservée, et tout reste synchronisé entre vos PC et votre Steam Deck. Ce qu'il manque, vous l'ajoutez en indiquant le dossier une fois. Voir [comment sauvegarder ses parties automatiquement](/guides/back-up-game-saves).

Il existe aussi des pages avec les chemins exacts de quelques jeux populaires : [Cyberpunk 2077](/guides/cyberpunk-2077-save-location), [Baldur's Gate 3](/guides/baldurs-gate-3-save-location), [Palworld](/guides/palworld-save-location), [Crimson Desert](/guides/crimson-desert-save-location) et [Marvel's Spider-Man 2](/guides/spider-man-2-save-location).

<!-- faq -->

## Questions fréquentes

### Où Steam range-t-il les sauvegardes ?

Cela dépend du jeu. Certains utilisent la zone propre à Steam, `Steam\userdata\<UserID>\<AppID>\remote` ; la plupart écrivent dans `Documents`, `AppData` ou `Saved Games` comme n'importe quel jeu, et Steam Cloud les copie depuis là.

### Pourquoi je ne trouve pas le dossier AppData ?

Il est caché. Tapez `%APPDATA%` (Roaming) ou `%LOCALAPPDATA%` (Local) dans la barre d'adresse de l'Explorateur ou dans Exécuter (Win + R). `LocalLow` est à côté de `Local`.

### Les versions Steam, GOG et Epic sauvegardent-elles au même endroit ?

En général oui, car c'est le jeu qui décide et non la boutique. Il y a des exceptions : certains jeux ajoutent un dossier au nom de l'ID de votre compte, et quelques versions de boutique utilisent un autre nom de dossier. Vérifiez avant de copier des sauvegardes d'une version à l'autre.

### Où sont les sauvegardes de l'application Xbox et du Game Pass ?

Dans `%LOCALAPPDATA%\Packages\<paquet>\SystemAppData\wgs`, sous forme de fichiers aux noms aléatoires que seule l'application Xbox comprend. Elles sont synchronisées par le cloud Xbox ; les copier à la main fonctionne rarement.

### Mon dossier Documents est dans OneDrive. Est-ce un problème ?

Ça peut l'être. Les jeux suivent le dossier dans OneDrive, et OneDrive synchronise alors les sauvegardes pendant que les jeux les écrivent. Voir [OneDrive et les sauvegardes de jeux](/guides/onedrive-game-saves).
