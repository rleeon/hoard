---
title: "Sauvegarde corrompue ? Comment la récupérer"
description: "Une sauvegarde illisible n'est pas toujours perdue. Où trouver une copie saine (backups du jeu, Steam Cloud, Windows, OneDrive) et comment ne plus être coincé."
order: 12
updated: 2026-10-09
---

Une sauvegarde qui ne se charge plus est rarement perdue pour de bon. La plupart du temps, une copie saine existe quelque part : un backup fait par le jeu lui-même, la copie de Steam Cloud, une version précédente gardée par Windows ou OneDrive, ou la sauvegarde d'un autre PC. Mais l'ordre compte, car une mauvaise manipulation peut écraser la bonne copie avec la mauvaise. Commencez ici.

## D'abord : stop, et copiez le dossier

1. **Fermez le jeu**, et ne commencez pas de nouvelle partie dans cet emplacement. Chaque sauvegarde à partir de maintenant peut chasser une copie plus ancienne.
2. **Copiez tout le dossier de sauvegarde** sur votre bureau ou une clé USB. Tout ce que vous tenterez ensuite pourra être annulé. Si vous ne savez pas où est le dossier, voir [où les jeux PC rangent leurs sauvegardes](/guides/where-are-pc-game-saves-stored).
3. **Mettez en pause tout ce qui synchronise ce dossier.** Steam Cloud (jeu par jeu, dans **Propriétés → Général**), OneDrive, Syncthing. Sinon le fichier abîmé peut partir vers le seul endroit qui a encore une bonne copie.

## Vérifiez qu'elle est vraiment corrompue

Certaines choses ressemblent à une corruption sans en être :

- **Le jeu a été mis à jour** et les anciennes sauvegardes ne se chargent plus, ou ont besoin d'un correctif. Consultez les actualités ou le forum du jeu.
- **Des mods manquent.** Les jeux Bethesda en particulier signalent les plugins manquants et peuvent refuser une sauvegarde qui les utilisait. Réinstallez d'abord les mods.
- **Vous êtes sur un autre compte.** Certains jeux rangent les sauvegardes sous l'ID de votre compte Steam ou Ubisoft, donc un autre compte voit un emplacement vide.
- **Le fichier n'est disponible qu'en ligne.** Avec OneDrive, une sauvegarde avec une icône de nuage a été retirée du disque pour libérer de l'espace. Faites un clic droit et choisissez **Toujours conserver sur cet appareil**.

Une sauvegarde de **0 Ko**, ou bien plus petite que ses voisines, est vraiment cassée : l'écriture a été interrompue en plein milieu.

## Où peut se trouver une copie saine

Procédez dans l'ordre. Les premières pistes sont plus rapides et plus susceptibles de marcher.

### 1. Les backups du jeu lui-même

Beaucoup de jeux gardent une copie de secours sans le dire. Cherchez dans le dossier de sauvegarde des fichiers se terminant par `.bak`, `_old` ou `.backup`, et des emplacements de sauvegarde automatique supplémentaires. Quelques cas connus :

- **Elden Ring** écrit `ER0000.sl2.bak` à côté de la sauvegarde.
- **Stardew Valley** garde une copie `_old` de chaque ferme, qui correspond au jour de jeu précédent.
- **Terraria** garde des fichiers `.bak` pour les personnages et les mondes.
- **Minecraft Java** garde `level.dat_old` dans chaque monde.

Pour en utiliser un, mettez le fichier abîmé de côté (vous avez déjà copié le dossier), puis renommez le backup avec le nom du fichier d'origine.

### 2. Steam Cloud

Steam Cloud garde la copie **la plus récente**, pas un historique. Il n'aide que si la sauvegarde s'est cassée après le dernier envoi, par exemple parce que le jeu a planté et que le mauvais fichier n'a jamais été synchronisé. Vous pouvez voir et télécharger ce que Steam garde pour chaque jeu sur [store.steampowered.com/account/remotestorage](https://store.steampowered.com/account/remotestorage), puis remettre le fichier en place à la main.

### 3. Les versions précédentes de Windows

Faites un clic droit sur le dossier de sauvegarde, ouvrez **Propriétés → Versions précédentes**. Si l'Historique des fichiers ou la Protection du système était activé pour ce lecteur, d'anciennes copies du dossier apparaissent ici, et vous pouvez les ouvrir pour reprendre les fichiers dont vous avez besoin. Si la liste est vide, aucun des deux n'était activé.

### 4. L'historique des versions de OneDrive

Si OneDrive sauvegarde votre dossier `Documents`, beaucoup de sauvegardes de jeux s'y trouvent sans que vous le sachiez. Sur onedrive.com, faites un clic droit sur le fichier et choisissez **Historique des versions** pour télécharger une version antérieure. OneDrive les garde pendant une durée limitée, et les fichiers supprimés passent aussi un moment dans sa corbeille.

### 5. Vos autres machines

Vous avez joué récemment sur un portable ou un Steam Deck ? Sa copie peut être antérieure au problème. Récupérez-la avant que cette machine ne synchronise la mauvaise.

### 6. Si le fichier a été supprimé, pas abîmé

Regardez d'abord dans la Corbeille. Ensuite, un outil de récupération de fichiers peut le retrouver, à condition de ne plus écrire sur ce lecteur. Chaque installation et chaque téléchargement réduisent les chances.

## Quand rien ne ressort

Pour quelques jeux populaires, la communauté propose des éditeurs de sauvegardes ou des outils de réparation capables de reconstruire un fichier abîmé : cherchez le nom du jeu avec « save repair ». Sinon, la réponse honnête est que la seule copie qui compte est celle faite avant le problème.

## Pourquoi les sauvegardes se corrompent

- **Un plantage ou une coupure de courant pendant l'écriture.** Le jeu était en train de sauvegarder quand il s'est arrêté.
- **Un disque plein.** Le jeu n'a pas pu finir d'écrire et a laissé un fichier tronqué.
- **Un outil de synchronisation l'a copiée en pleine écriture**, ou deux PC ont modifié la même sauvegarde et une copie l'a emporté.
- **Un mod** a écrit quelque chose que le jeu ne sait pas relire.
- **Un disque qui lâche**, ce qui se voit généralement sur d'autres fichiers aussi.

## Ne plus jamais être coincé

Toutes les récupérations ci-dessus dépendent de la chance : que le jeu ait fait un backup, ou que Steam n'ait pas encore synchronisé. Une sauvegarde versionnée retire la chance de l'équation. C'est ce que fait Hoard : il sauvegarde chaque partie automatiquement après que vous avez arrêté de jouer, une fois le dossier au repos, donc une copie n'est jamais un fichier à moitié écrit. Chaque version est conservée. Quand quelque chose casse, vous ouvrez l'**Historique** du jeu et restaurez la dernière bonne version en un clic ; votre sauvegarde actuelle est copiée d'abord, donc même cela peut être annulé. Et comme Hoard garde aussi vos sauvegardes synchronisées, la copie de votre portable ou de votre Steam Deck n'est jamais une vieille copie oubliée : toutes vos machines partagent le même historique.

Une astuce pour repérer le moment où ça a cassé : une chute soudaine de taille entre deux versions signale généralement une sauvegarde tronquée. Plus d'infos dans [comment restaurer une ancienne sauvegarde](/guides/restore-a-game-save).

Si vous préférez garder les backups chez vous, lancez `hoard-server` sur votre propre PC ou NAS. Pas de compte chez nous, pas de télémétrie vers nous, rien ne passe par nos serveurs. Voir [comment auto-héberger Hoard](/guides/self-host-hoard).

<!-- faq -->

## Questions fréquentes

### Peut-on réparer une sauvegarde corrompue ?

Rarement sur place. Quelques jeux ont des outils de réparation communautaires, mais le plus souvent récupérer veut dire trouver une copie plus ancienne : le backup du jeu, Steam Cloud, Windows ou OneDrive, ou un autre PC.

### Steam Cloud garde-t-il les anciennes versions de mes sauvegardes ?

Non. Il ne garde que le fichier actuel. Si une sauvegarde abîmée a déjà été envoyée, Steam Cloud a lui aussi la version abîmée.

### Vérifier l'intégrité des fichiers du jeu répare-t-il une sauvegarde corrompue ?

Non. La vérification compare les fichiers du jeu avec ceux de Steam, pas vos sauvegardes. Elle aide si le jeu lui-même est endommagé, mais ne ramène pas votre progression.

### Pourquoi ma sauvegarde fait-elle 0 Ko ?

Le jeu a commencé à l'écrire sans jamais finir : un plantage, une coupure de courant ou un disque plein. Cherchez à côté un fichier `.bak` ou `_old`, ou une version antérieure ailleurs.

### Comment éviter que ça se reproduise ?

Avec des sauvegardes versionnées faites quand le jeu n'est pas lancé. Hoard le fait automatiquement après chaque session et garde chaque version, vous pouvez donc revenir à n'importe laquelle.
