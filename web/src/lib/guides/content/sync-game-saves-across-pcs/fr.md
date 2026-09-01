---
title: "Comment synchroniser vos parties entre plusieurs PC"
description: "Jouez au même jeu sur votre fixe et votre portable sans perdre votre progression. Synchronisez vos parties entre PC automatiquement avec Hoard — une synchro cloud gérée, sans configurer Ludusavi et Rclone à la main."
order: 2
updated: 2026-09-01
---

Si vous jouez sur plus d'un ordinateur — un fixe à la maison et un portable en déplacement — Hoard garde vos sauvegardes synchronisées pour que vous repreniez toujours là où vous en étiez.

## Comment fonctionne la synchronisation

Hoard sauvegarde chaque partie vers votre cloud et récupère la dernière version sur vos autres machines. Quand vous finissez de jouer sur un PC, la sauvegarde la plus récente vous attend sur le suivant.

## Configurer la synchronisation

1. Installez **Hoard** sur chaque PC où vous jouez (Windows, macOS ou Linux).
2. Connectez-vous avec le **même compte** sur chaque machine, ou reliez-les au même serveur auto-hébergé.
3. Ajoutez les mêmes jeux à votre **Bibliothèque** sur chaque PC. Hoard les associe par jeu, donc une sauvegarde faite sur l'un apparaît sur les autres.
4. Gardez le **mode automatique** activé. Hoard envoie après que vous jouez et télécharge la dernière version avant que vous commenciez.

## Vous venez de Ludusavi ?

Ludusavi est un excellent outil open source pour sauvegarder et restaurer des parties en local, et il peut envoyer ces sauvegardes vers un cloud que vous configurez vous-même avec Rclone. Mais la synchro entre appareils, vous la montez à la main : planifier la sauvegarde, configurer le distant, puis restaurer sur l'autre PC avant de jouer.

Hoard transforme cela en synchro gérée. Il utilise les mêmes données communautaires d'emplacements que Ludusavi pour trouver vos sauvegardes, puis envoie après chaque session et télécharge la dernière version avant la suivante — sur chaque PC de votre compte, avec un historique versionné dans le cloud. Pas de distants Rclone, pas de scripts. Et comme Ludusavi, Hoard est open source et peut être auto-hébergé. Voir la [comparaison complète avec Ludusavi](/guides/ludusavi-alternative).

## Éviter les conflits

Hoard gère les conflits : il compare les dates de modification et conserve une copie locale de toute sauvegarde remplacée, donc une synchro ne détruit jamais la progression en silence. Si un jeu tourne encore ou qu'une sauvegarde a été modifiée il y a quelques minutes, Hoard attend.

## Steam Deck et PC de bureau

Le montage à deux machines le plus courant est aussi celui qui casse le plus souvent quand on le fait à la main, et presque toujours pour la même raison.

Sous Windows, la sauvegarde d'un jeu peut se trouver dans `Documents\My Games\…` ou dans le `userdata` de Steam. Sur un Steam Deck, ce même jeu Windows tourne via Proton : sa sauvegarde vit donc dans un préfixe de compatibilité, `steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…`. Deux chemins très différents, un seul jeu, une seule progression. Hoard lit les préfixes Proton comme les emplacements natifs et rapproche ce qu'il trouve par jeu : la sauvegarde du Deck et celle du bureau deviennent deux versions d'un même historique au lieu de deux dossiers sans rapport.

Le détail dont tout dépend : pour les jeux Steam, Hoard suit `<AppID>/remote/` dans `userdata`, et **non** le dossier au-dessus. Le dossier parent contient aussi `remotecache.vdf` ainsi que des fichiers de succès et de temps de jeu propres à chaque machine, qui doivent différer entre votre Deck et votre bureau. Synchronisez le parent et chaque lancement ressemble à un conflit alors qu'aucune sauvegarde n'a bougé. Cette seule erreur suffit à faire paraître cassés la plupart des montages maison Deck ↔ PC.

## Les jeux que Steam Cloud ne couvre pas

Si tous vos jeux géraient Steam Cloud, vous n'auriez besoin de rien de tout cela. En pratique :

- **Les jeux venus d'ailleurs que Steam.** GOG, Epic, itch, Battle.net, l'application Xbox, et tout ce que vous avez installé à la main.
- **Les jeux Steam où le développeur ne l'a jamais activé**, ou seulement pour une plateforme.
- **Les émulateurs.** RetroArch, Dolphin, PCSX2, RPCS3 et les autres écrivent où bon leur semble, et Steam n'en sait rien.
- **Les jeux qui écrivent hors du dossier surveillé par Steam**, et il y en a plus qu'on ne croit.

Hoard se moque de qui a publié un jeu et d'où il vient : il suit le dossier qui change quand vous jouez.

## Quand deux PC modifient la même sauvegarde

Vous jouez sur le portable sans laisser le fixe finir sa synchro, et voilà le problème classique : deux sauvegardes, toutes deux plus récentes que la dernière version commune.

Hoard n'écrase jamais à l'aveugle. Il compare les dates de modification, conserve une copie locale de ce qu'il remplace, et attend tant qu'un jeu tourne ou que la sauvegarde a été touchée dans les dernières minutes : un fichier en cours d'écriture n'est pas un fichier qu'on veut envoyer à moitié. Toutes les versions antérieures restent dans l'historique cloud : se tromper de version coûte deux clics, pas un week-end.

La limite honnête : **Hoard ne fusionne pas deux sauvegardes divergentes.** Aucun outil ne le peut — un fichier de sauvegarde est opaque, et il n'existe aucune façon correcte de mélanger deux après-midi de jeu différents. Ce que vous obtenez à la place, c'est toutes les versions, sur toutes les machines, et le choix.

## Synchroniser sans passer par nos serveurs

Autant le dire franchement, car c'est le point sur lequel presque toutes les comparaisons se trompent. Il y a deux façons de l'utiliser :

- **Hoard Cloud** est l'option gérée : vous vous connectez, et vos sauvegardes sont stockées sur nos serveurs, dans l'UE.
- **L'auto-hébergement est entièrement le vôtre.** Vous faites tourner `hoard-server` sur votre PC ou votre NAS et vos machines se synchronisent à travers lui. Il n'y a **aucun compte chez nous, aucune télémétrie vers nous, aucun quota et aucun relais** : rien ne passe par nos serveurs, puisque rien de chez nous n'est sur le chemin. Voir [comment auto-héberger Hoard](/guides/self-host-hoard).

Le même programme, la même détection, le même historique de versions. La seule chose qui change, c'est à qui appartient le stockage.

## Astuce

Laissez chaque machine finir de synchroniser avant de lancer un jeu — le tableau de bord affiche l'état en direct, vous savez donc que la dernière sauvegarde est en place.

<!-- faq -->

## Questions fréquentes

### Combien de PC puis-je synchroniser ?

Trois avec l'offre gratuite, un nombre illimité avec Pro, et illimité en auto-hébergement : votre serveur, vos règles.

### Les deux machines doivent-elles être allumées en même temps ?

Non. Votre sauvegarde monte vers le serveur quand vous finissez de jouer et redescend quand l'autre machine la demande : le second PC peut rester éteint une semaine et recevoir quand même la dernière version à l'allumage.

### Et si je joue hors ligne ?

Aucun souci. L'instantané est pris localement quand vous arrêtez de jouer, et il part tout seul dès que la machine retrouve une connexion.

### Est-ce que ça synchronise aussi mes mods et réglages ?

Les sauvegardes, oui. Les fichiers propres à une machine — configuration, journaux et compagnie — sont envoyés pour figurer dans la sauvegarde, mais ne sont pas réécrits par-dessus la copie d'un autre PC : un réglage graphique qui convient à votre fixe est rarement celui que veut votre portable.

### L'auto-hébergement envoie-t-il quoi que ce soit à Hoard ?

Non. En mode auto-hébergé il n'y a aucun compte chez nous ni aucune télémétrie vers nous : vos sauvegardes, vos utilisateurs et vos journaux vivent sur votre propre serveur et ne touchent jamais le nôtre.
