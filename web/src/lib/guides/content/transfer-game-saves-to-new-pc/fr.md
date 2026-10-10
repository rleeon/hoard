---
title: "Comment transférer ses sauvegardes de jeux sur un nouveau PC"
description: "Nouveau PC ou Windows réinstallé ? Emportez toutes vos sauvegardes : ce que couvre Steam Cloud, la méthode manuelle, la méthode automatique et les pièges."
order: 13
updated: 2026-10-09
---

Les jeux Steam Cloud reviennent tout seuls quand vous vous connectez sur le nouveau PC. Tout le reste est un dossier que vous devez emporter vous-même : jeux sans sauvegarde cloud, émulateurs, tout ce qui est hors de Steam. Vous pouvez copier ces dossiers à la main, ou laisser Hoard les sauvegarder sur l'ancien PC et remettre chacun à sa place sur le nouveau. Voici les deux méthodes, et les pièges qui coûtent leurs sauvegardes aux gens.

## Avant d'effacer quoi que ce soit

- **Faites la liste de ce à quoi vous jouez**, y compris les jeux que vous n'avez pas touchés depuis des mois. Ce sont ceux qu'on oublie.
- **Vérifiez quels jeux ont des sauvegardes cloud.** Sur Steam, la page du magasin l'indique, et **Propriétés → Général** montre si c'est activé. Epic et GOG l'indiquent aussi jeu par jeu.
- **Sauvegardez le reste, et idéalement tout.** Les sauvegardes cloud gardent une seule copie, la dernière. Si elle est abîmée, elle l'est partout.

## La méthode manuelle

1. **Trouvez le dossier de chaque jeu.** La plupart sont dans `Documents\My Games`, `Saved Games` ou les dossiers `AppData` (`Roaming`, `Local`, `LocalLow`). La liste complète est dans [où les jeux PC rangent leurs sauvegardes](/guides/where-are-pc-game-saves-stored).
2. **Copiez-les sur un disque externe**, en gardant l'arborescence. Prenez aussi tout le dossier `userdata` de Steam : il est léger et couvre les jeux qui sauvegardent via Steam sans avoir Steam Cloud activé.
3. **Sur le nouveau PC, installez d'abord le jeu.** S'il doit créer ses dossiers, lancez-le une fois et quittez au menu principal. Ne commencez pas de nouvelle partie.
4. **Copiez les sauvegardes à leur place** et lancez le jeu. Vérifiez que votre progression est là avant d'effacer quoi que ce soit sur l'ancien disque.

Ça marche. L'inconvénient, c'est que c'est une copie unique : il faut penser à chaque dossier, et si vous continuez à jouer sur l'ancien PC, les deux divergent à partir de ce jour.

## Les pièges

- **Sauvegardes liées à un compte.** Certains jeux mettent l'ID de votre compte dans le nom du dossier ou dans la sauvegarde : Elden Ring range les sauvegardes sous votre SteamID, les jeux Ubisoft sous votre ID Ubisoft. Même compte sur les deux PC : aucun souci. Un autre compte : le jeu voit un emplacement vide.
- **OneDrive a déplacé Documents.** Si un PC sauvegarde `Documents` avec OneDrive et l'autre non, le « même » dossier se trouve à deux endroits différents. Faites un clic droit sur `Documents`, ouvrez **Propriétés → Emplacement** pour voir où il est vraiment. Plus d'infos dans [OneDrive et les sauvegardes de jeux](/guides/onedrive-game-saves).
- **Versions du jeu.** Une sauvegarde d'une version plus récente peut ne pas se charger dans une plus ancienne. Mettez le jeu à jour sur le nouveau PC avant de copier.
- **Mods.** Une sauvegarde moddée (surtout dans les jeux Bethesda) peut refuser de se charger sans les mêmes mods. Réinstallez-les d'abord.
- **De Windows vers un Steam Deck ou Linux.** La sauvegarde va dans le préfixe Proton du jeu, qui n'existe qu'après un premier lancement. Voir [synchroniser ses sauvegardes entre Steam Deck et PC](/guides/sync-saves-steam-deck-pc).

## La méthode automatique

Hoard transforme le déménagement en ce qu'il fait tous les jours : sauvegarder sur une machine, restaurer sur une autre.

1. **Sur l'ancien PC**, installez Hoard et connectez-vous. Ouvrez la **Bibliothèque** : Hoard liste les sauvegardes trouvées pour vos jeux, avec la même base communautaire que Ludusavi. Ajoutez ce qui manque en indiquant son dossier.
2. **Vérifiez que chaque jeu a une version** dans son **Historique**. C'est votre filet de sécurité avant d'effacer l'ancien disque.
3. **Sur le nouveau PC**, installez Hoard, connectez-vous avec le même compte et installez vos jeux. Hoard les associe à leurs sauvegardes jeu par jeu et restaure la dernière version dans le dossier attendu par cette machine, même si le chemin change (autre disque, autre nom d'utilisateur, préfixe Proton sur un Deck).
4. **Avant de commencer une nouvelle partie**, laissez Hoard finir de remettre vos sauvegardes. L'application affiche l'état de chaque jeu.

Deux détails rendent cela plus sûr qu'une copie. Les fichiers de réglages comme `graphics.ini` sont sauvegardés mais pas écrits par-dessus ceux du nouveau PC, donc votre nouveau matériel démarre avec des réglages qui lui conviennent (vous pouvez les reprendre à la restauration si les deux machines se ressemblent). Et rien n'est définitif : chaque version reste dans l'historique, une mauvaise restauration s'annule en restaurant la précédente.

Si l'ancien PC reste en service, il continue simplement à se synchroniser avec le nouveau. S'il part pour de bon, retirez-le de vos appareils. L'offre gratuite en couvre trois.

## Sans nos serveurs

Vous pouvez faire tout cela avec votre propre serveur : lancez `hoard-server` sur un PC ou un NAS, pointez les deux machines dessus, et les sauvegardes ne quittent jamais votre domicile. Pas de compte chez nous, pas de télémétrie vers nous. Voir [comment auto-héberger Hoard](/guides/self-host-hoard).

<!-- faq -->

## Questions fréquentes

### Les sauvegardes Steam se transfèrent-elles automatiquement ?

Seulement pour les jeux avec Steam Cloud. Connectez-vous sur le nouveau PC, installez le jeu et la sauvegarde se télécharge. Les jeux sans Steam Cloud demandent de copier leur dossier, ou un outil qui le fait pour vous.

### Puis-je simplement copier tout mon dossier utilisateur ?

Ça marche pour la plupart des sauvegardes, mais ça embarque aussi des gigaoctets de caches, des réglages pensés pour l'ancien matériel et des données d'applications qui peuvent poser problème sur une installation neuve. Copier seulement les dossiers de sauvegarde est plus propre.

### Mes sauvegardes marcheront-elles avec un autre nom d'utilisateur Windows ?

Oui, presque toujours. Les sauvegardes sont rangées par rapport à votre dossier utilisateur, donc le nom dans le chemin n'a pas d'importance. Hoard s'en charge tout seul.

### Puis-je transférer des sauvegardes de Windows vers un Steam Deck ?

Oui. Lancez le jeu une fois sur le Deck pour que son préfixe Proton existe, puis placez la sauvegarde dedans, ou laissez Hoard le faire. Voir [le guide Steam Deck](/guides/sync-saves-steam-deck-pc).

### Dois-je garder l'ancien PC jusqu'à ce que le nouveau soit prêt ?

Avec une copie manuelle, gardez le disque externe jusqu'à avoir vérifié chaque jeu. Avec Hoard, les sauvegardes sont déjà sur le serveur, l'ancien PC peut donc partir dès que chaque jeu affiche une version dans son historique.
