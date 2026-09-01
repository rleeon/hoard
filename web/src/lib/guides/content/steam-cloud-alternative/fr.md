---
title: "Alternative à Steam Cloud : sauvegardez les parties que Steam ignore"
description: "Steam Cloud ne couvre que les jeux Steam dont le développeur l'a activé, et ne garde aucun historique. Hoard sauvegarde tous vos jeux, quelle que soit la boutique, avec un historique versionné où revenir — dans le cloud ou sur votre propre serveur."
order: 7
updated: 2026-09-01
---

Steam Cloud fait très bien le travail précis qu'il fait, et la plupart des gens en découvrent les limites le jour où ils perdent quelque chose. Ce guide explique où sont ces limites, et quoi faire des jeux qui restent en dehors.

## Ce que Steam Cloud couvre réellement

Steam Cloud synchronise le dossier d'un jeu quand **le développeur l'a configuré** : soit en déclarant les fichiers à synchroniser, soit en appelant l'API Steam depuis le jeu. C'est tout le modèle, et trois conséquences en découlent :

- Ça ne marche que pour des jeux achetés et lancés via Steam.
- Que ça marche ou non est la décision du développeur, jeu par jeu, parfois par plateforme.
- Chaque jeu a son propre quota de stockage, fixé par ce développeur.

Quand ça marche, c'est invisible et excellent : vous fermez le jeu sur un PC, vous l'ouvrez sur un autre, votre progression est là.

## Là où ça vous laisse exposé

- **Tout ce qui n'est pas un jeu Steam.** GOG, Epic, itch, Battle.net, l'application Xbox, les émulateurs, tout ce qui est installé à la main. Steam ignore leur existence.
- **Les jeux Steam où ça n'a jamais été activé.** Beaucoup de titres, surtout anciens ou modestes, ne l'ont tout simplement pas. La fiche boutique le dit, mais personne ne vérifie avant de lancer une partie de 60 heures.
- **Il n'y a pas de retour en arrière.** C'est le point majeur. Steam conserve l'état actuel de votre sauvegarde, pas son histoire. Fichier corrompu, mod qui dévore votre monde, bonne sauvegarde écrasée par une mauvaise : la copie du cloud est déjà la mauvaise. Vous pouvez consulter les fichiers que Steam détient pour un jeu, mais il n'y a aucune version antérieure à restaurer.
- **La boîte de dialogue de conflit.** Quand Steam estime que local et distant divergent, il vous demande de choisir avec guère plus que deux horodatages. Mauvais choix, l'autre copie a disparu.

## Ce que Hoard ajoute

Hoard surveille le dossier dans lequel le jeu écrit vraiment et capture **une nouvelle version chaque fois que vous arrêtez de jouer** :

- **La provenance du jeu lui est égale.** Steam, GOG, Epic, itch, émulateurs, ou un dossier que vous lui désignez.
- **Toutes les versions sont conservées** : se remettre d'une sauvegarde corrompue ou d'une mauvaise décision coûte deux clics, pas une partie entière.
- **Il synchronise entre vos machines**, Steam Deck et PC de bureau compris.
- **Rien n'est détruit en silence.** La sauvegarde remplacée est capturée d'abord : même une restauration malheureuse est réversible.

Les instantanés sont stockés par empreinte de contenu : dix versions d'une sauvegarde de 2 Go coûtent environ 2 Go, pas 20 — c'est ce qui rend viable de garder tout l'historique.

## Utiliser les deux ensemble

Ils ne se marchent pas dessus, et vous n'avez pas à choisir. Pour un jeu Steam qui gère le cloud, laissez Steam synchroniser ce qu'il synchronise déjà ; l'apport de Hoard est l'historique — précisément ce que Steam ne garde pas. Pour tout le reste, Hoard assure aussi la synchro.

Un détail qui compte si vous avez un Steam Deck en plus d'un fixe : Hoard suit `<AppID>/remote/` dans `userdata`, et non le dossier au-dessus, car le parent contient `remotecache.vdf` et des fichiers de succès et de temps de jeu propres à chaque machine. C'est la distinction qu'une synchro maison rate le plus souvent, et la raison pour laquelle ces montages semblent en conflit à chaque lancement.

## Quand Steam Cloud suffit

Disons-le franchement : si tous vos jeux sont des jeux Steam avec support cloud, que vous jouez sur un seul PC et que vous n'avez jamais eu besoin d'annuler une sauvegarde, Steam Cloud fait le travail et vous n'avez besoin de rien d'autre. Ce qui justifie d'ajouter Hoard, c'est l'historique de versions, les jeux hors Steam et les machines que Steam Cloud n'atteint pas.

## Sans le cloud de personne

Si l'attrait est de ne dépendre d'aucune plateforme, Hoard tourne entièrement sur votre matériel : `hoard-server` sur un PC ou un NAS, et vos sauvegardes vont de votre machine à votre disque. **Aucun compte chez nous, aucune télémétrie vers nous, aucun relais** : rien ne passe par nos serveurs, puisque rien de chez nous n'est sur le chemin. Voir [comment auto-héberger Hoard](/guides/self-host-hoard).

Le même programme, la même détection, le même historique. La seule chose qui change, c'est à qui appartient le stockage.

<!-- faq -->

## Questions fréquentes

### Hoard remplace-t-il Steam Cloud ?

Ce n'est pas obligatoire. Steam Cloud garde votre sauvegarde courante synchronisée pour les jeux compatibles ; Hoard ajoute l'historique de versions et couvre les jeux qui ne le sont pas. Faire tourner les deux est courant.

### Steam Cloud peut-il revenir à une sauvegarde plus ancienne ?

Non. Steam conserve l'état actuel des fichiers, pas leur histoire. Une fois qu'une mauvaise sauvegarde est synchronisée, c'est elle qui est dans le cloud. Revenir en arrière exige un outil qui versionne.

### Pourquoi tous mes jeux Steam ne se synchronisent-ils pas ?

Parce que c'est le développeur qui l'active, jeu par jeu et parfois par plateforme. La fiche du jeu mentionne Steam Cloud dans ses fonctionnalités quand c'est pris en charge — et beaucoup de titres ne le sont pas.

### Hoard fonctionne-t-il avec des jeux hors Steam ?

Oui, et c'est une bonne part de son intérêt. Il localise les sauvegardes via une base communautaire couvrant plus de 20 000 titres, toutes boutiques confondues, et vous pouvez lui désigner un dossier à la main pour les cas particuliers.

### Faire tourner les deux crée-t-il des conflits ?

Non. Hoard capture une version après que vous avez arrêté et que le dossier s'est calmé, et n'écrase jamais sans avoir d'abord capturé ce qu'il remplace.

### Puis-je garder mes sauvegardes hors des deux clouds ?

Oui. Auto-hébergez le serveur : vos sauvegardes ne quittent jamais du matériel qui vous appartient, sans compte et sans télémétrie vers qui que ce soit.
