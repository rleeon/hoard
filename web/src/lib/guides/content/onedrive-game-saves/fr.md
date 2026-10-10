---
title: "OneDrive et les sauvegardes de jeux : ce qui casse et comment réparer"
description: "OneDrive a déplacé vos Documents : sauvegardes qui échouent, disparaissent ou apparaissent en double. Pourquoi, comment réparer, et mieux synchroniser."
order: 15
updated: 2026-10-09
---

Sur beaucoup de PC Windows, OneDrive sauvegarde le dossier `Documents`, souvent activé pendant l'installation sans que personne ne le remarque. Les jeux qui sauvegardent dans `Documents`, c'est-à-dire presque tout `My Games`, le suivent dans OneDrive. Et les ennuis commencent : sauvegardes qui échouent, sauvegardes à télécharger avant de pouvoir les charger, et copies portant le nom de votre PC que le jeu ne lit jamais. Voici pourquoi cela arrive et comment y remédier.

## Comment vos sauvegardes ont fini dans OneDrive

La sauvegarde de dossiers de OneDrive déplace `Documents`, `Bureau` et `Images` vers `C:\Users\<vous>\OneDrive\...`. Les jeux demandent à Windows où se trouve `Documents`, donc ils suivent sans rien dire. Pour vérifier, faites un clic droit sur `Documents` et ouvrez **Propriétés → Emplacement** : si le chemin contient `OneDrive`, vos sauvegardes sont dedans.

`AppData` et `Saved Games` ne font pas partie de cette sauvegarde, donc les jeux qui y écrivent ne sont pas concernés.

## Ce qui tourne mal

- **Les écritures se télescopent.** OneDrive envoie les fichiers dès qu'ils changent. Un jeu qui écrit sa sauvegarde au même moment peut trouver le fichier occupé.
- **Des sauvegardes uniquement en ligne.** OneDrive peut libérer de l'espace en gardant des fichiers seulement dans le cloud (l'icône de nuage). Le jeu doit alors télécharger la sauvegarde avant de la charger, et hors ligne il n'y a rien à charger.
- **Des copies en conflit.** Utilisez OneDrive sur deux PC avec le même compte et les deux synchronisent le même `My Games`. Jouez sur les deux avant que l'un ait rattrapé l'autre, et OneDrive garde les deux versions en renommant l'une avec le nom du PC. Le jeu ignore ce fichier.
- **L'espace.** L'offre gratuite fait 5 Go, et certains jeux mettent aussi des mods, des caches ou des enregistrements dans `Documents`.
- **Aucune notion de session de jeu.** OneDrive synchronise fichier par fichier, en pleine partie, et versionne chaque fichier séparément plutôt que la sauvegarde entière.

## Les solutions

### Rapide : garder les sauvegardes sur l'appareil

Faites un clic droit sur `Documents\My Games` (ou le dossier du jeu) et choisissez **Toujours conserver sur cet appareil**. Cela règle le problème des fichiers uniquement en ligne. Cela n'empêche pas OneDrive de synchroniser pendant que vous jouez.

### Propre : arrêter de sauvegarder Documents

Dans OneDrive, ouvrez **Paramètres → Synchroniser et sauvegarder → Gérer la sauvegarde** et désactivez `Documents`. Windows fait de nouveau pointer `Documents` vers le dossier local, mais les fichiers déjà sauvegardés restent dans le dossier OneDrive. Avant de désactiver, marquez-les **Toujours conserver sur cet appareil** pour qu'ils soient vraiment sur le disque. Ensuite, jeux fermés, déplacez les dossiers des jeux dans le `Documents` local, sinon les jeux repartiront de zéro.

## Une meilleure répartition

OneDrive est bon avec les documents. Les sauvegardes de jeux ont besoin d'autre chose : une copie faite une fois le jeu fermé, des versions de la sauvegarde entière plutôt que de fichiers isolés, et une synchronisation avec vos autres PC et un Steam Deck.

C'est ce que fait Hoard. Il trouve vos sauvegardes que `Documents` soit dans OneDrive ou non, parce qu'il demande à Windows où se trouve vraiment le dossier. Il les sauvegarde automatiquement après chaque session, garde chaque version et les synchronise avec vos autres machines.

Une règle : un seul outil doit gérer la synchronisation entre PC. Si OneDrive sauvegarde `Documents` sur plusieurs PC de jeu avec le même compte, il synchronise déjà ces sauvegardes entre eux. Désactivez la sauvegarde de `Documents` sur ces PC et laissez les sauvegardes de jeux à Hoard.

Vous préférez aucun cloud ? Lancez `hoard-server` sur votre PC ou NAS : pas de compte chez nous, pas de télémétrie vers nous, rien ne passe par nos serveurs. Voir [comment auto-héberger Hoard](/guides/self-host-hoard).

<!-- faq -->

## Questions fréquentes

### Faut-il laisser OneDrive sauvegarder mes parties ?

Sur un seul PC, comme sauvegarde, c'est mieux que rien. Pour synchroniser des sauvegardes entre PC, il crée des conflits, parce qu'il ne sait pas quand un jeu est lancé.

### J'ai désactivé la sauvegarde et mes parties ont disparu. Où sont-elles ?

Toujours dans le dossier OneDrive : `C:\Users\<vous>\OneDrive\Documents\My Games`. Fermez vos jeux et remettez-les dans le `Documents` local.

### Que sont les fichiers de sauvegarde avec le nom de mon PC ?

Des copies en conflit de OneDrive. Deux PC ont modifié le même fichier avant d'être synchronisés, et OneDrive a gardé les deux. Déterminez lequel est le plus récent, donnez-lui le nom d'origine et mettez l'autre de côté.

### OneDrive peut-il me rendre une ancienne sauvegarde ?

Parfois. Sur onedrive.com, faites un clic droit sur le fichier et choisissez **Historique des versions**. Cela fonctionne fichier par fichier et seulement pendant une durée limitée. Voir [comment récupérer une sauvegarde corrompue](/guides/recover-corrupted-game-save).

### Hoard fonctionne-t-il si mon dossier Documents est dans OneDrive ?

Oui. Hoard lit l'emplacement que Windows donne à `Documents`, il trouve donc les sauvegardes dans les deux cas.
