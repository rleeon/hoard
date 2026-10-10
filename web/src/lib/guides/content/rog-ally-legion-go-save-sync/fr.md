---
title: "Synchroniser ses sauvegardes entre ROG Ally, Legion Go, MSI Claw et son PC"
description: "La ROG Ally, la Legion Go et la MSI Claw sont des PC sous Windows. Synchronisez leurs sauvegardes avec votre PC fixe automatiquement, avec historique."
order: 19
updated: 2026-10-09
---

La ROG Ally, la Legion Go et la MSI Claw tournent sous Windows : pour un jeu, ce sont simplement d'autres PC. C'est justement le problème : votre PC fixe et votre console portable gardent chacun leurs propres sauvegardes. Steam Cloud couvre une partie de votre bibliothèque et le cloud Xbox couvre le Game Pass, mais tout le reste reste sur la machine où vous avez joué. Hoard garde vos sauvegardes synchronisées entre la portable et le PC fixe automatiquement : arrêtez de jouer sur l'une et votre partie vous attend sur l'autre, avec chaque version précédente conservée.

## Ce qui vous suit déjà

- **Les jeux Steam compatibles Steam Cloud** se synchronisent tout seuls.
- **Les jeux Game Pass et de l'application Xbox** utilisent le cloud Xbox, tant que vous jouez à la version Xbox sur les deux machines.
- **Epic, GOG, Ubisoft et EA** ont des sauvegardes cloud pour certains de leurs jeux, dans leurs propres launchers. Voir [les sauvegardes cloud d'Epic et GOG](/guides/epic-gog-cloud-saves).

Ce qui reste : les jeux dont le développeur n'a jamais activé le cloud, les émulateurs, les jeux installés à la main, et tout jeu pour lequel votre PC fixe et votre portable n'utilisent pas le même launcher.

## Mise en place

1. **Sur la portable**, passez sur le bureau Windows, ouvrez [la page de téléchargement](/download) et installez Hoard pour Windows.
2. **Connectez-vous** avec le compte de votre PC fixe, ou pointez l'application vers votre propre serveur.
3. Ouvrez la **Bibliothèque** et vérifiez ce que Hoard a trouvé. Ajoutez ce qui manque en indiquant son dossier, un émulateur par exemple.
4. **Sur le PC fixe**, installez Hoard avec le même compte. Les mêmes jeux s'associent tout seuls.

Le moteur de synchronisation est un service d'arrière-plan qui démarre avec Windows, il continue donc à fonctionner pendant que vous êtes dans Armoury Crate, Legion Space, MSI Center M ou le mode Big Picture de Steam. Pas besoin d'ouvrir la fenêtre de Hoard pour jouer.

## Les pièges des consoles portables

### La veille n'est pas une fermeture

Sur une portable, il est facile d'appuyer sur le bouton d'alimentation et de la ranger avec le jeu ouvert. Hoard ne sauvegarde qu'une fois le jeu fermé, car un jeu en cours peut être en train d'écrire sa sauvegarde, et il ne remplace jamais la sauvegarde d'un jeu en cours. Si vous mettez la portable en veille puis jouez sur le PC fixe, la progression de la portable n'est pas encore envoyée. **Quittez le jeu avant de changer de machine.**

### Les jeux sur la carte microSD

Installer ses jeux sur la carte est courant sur une portable, et cela change rarement quoi que ce soit pour les sauvegardes : la plupart des jeux sauvegardent dans votre dossier utilisateur sur le disque interne, où qu'ils soient installés. L'exception, ce sont les jeux qui sauvegardent à côté de leur dossier d'installation ; si l'un d'eux n'est pas détecté, ajoutez son dossier à la main.

### Écran et réglages

Votre portable tourne en plus basse résolution, avec un GPU plus petit que votre PC fixe. Hoard sauvegarde les fichiers de réglages comme `graphics.ini` avec la partie, mais ne les écrit pas par-dessus ceux de l'autre machine, donc chacune garde des réglages qui lui conviennent. Si vous voulez quand même les copier, une option le permet lors de la restauration.

### Même jeu, boutique différente

Un jeu acheté sur Steam pour le PC fixe et joué via le Game Pass sur la portable, ce sont deux installations différentes, et la version Xbox garde ses sauvegardes dans un format que seule l'application Xbox comprend. Pour partager une sauvegarde, jouez à la version de la même boutique sur les deux.

### SteamOS ou Bazzite au lieu de Windows ?

Votre portable est alors une machine Linux, et les sauvegardes vivent dans des préfixes Proton, exactement comme sur un Steam Deck. Voir [synchroniser ses sauvegardes entre Steam Deck et PC](/guides/sync-saves-steam-deck-pc).

## Sans nos serveurs

Si vous préférez garder vos sauvegardes chez vous, lancez `hoard-server` sur votre PC ou un NAS et pointez les deux machines dessus. Pas de compte chez nous, pas de télémétrie vers nous, rien ne passe par nos serveurs. Voir [comment auto-héberger Hoard](/guides/self-host-hoard).

<!-- faq -->

## Questions fréquentes

### La ROG Ally a-t-elle des sauvegardes cloud ?

Elle a ce qu'a chaque launcher : Steam Cloud, le cloud Xbox, et ceux d'Epic, GOG, Ubisoft ou EA pour les jeux compatibles. Il n'y a pas de synchronisation des sauvegardes pour tout le système. Hoard en ajoute une pour les jeux que ceux-là laissent de côté.

### Hoard fonctionne-t-il avec Armoury Crate ou Legion Space ?

Oui. Le moteur de synchronisation de Hoard est un service d'arrière-plan Windows, indépendant du launcher avec lequel vous lancez vos jeux.

### La portable compte-t-elle comme un appareil ?

Oui. L'offre gratuite couvre trois appareils, donc un PC fixe, un portable et une console portable y tiennent. Pro et les serveurs auto-hébergés n'ont pas de limite d'appareils.

### Et les sauvegardes du Game Pass ?

Laissez-les au cloud Xbox, qui les synchronise entre les installations de l'application Xbox des deux machines. Hoard couvre les jeux qui n'ont pas de cloud à eux.

### Puis-je aussi synchroniser la portable avec un Steam Deck ?

Oui. Hoard tourne sur les deux et associe chaque jeu entre Windows et les préfixes Proton du Deck.
