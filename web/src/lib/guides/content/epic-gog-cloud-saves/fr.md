---
title: "Sauvegardes cloud d'Epic et GOG : ce qu'elles couvrent et comment synchroniser le reste"
description: "Le cloud d'Epic et GOG ne marche que pour certains jeux, dans leur launcher, sans historique. Ce qu'il couvre et comment synchroniser le reste."
order: 19.5
updated: 2026-10-09
---

Epic et GOG ont tous les deux des sauvegardes cloud, avec les mêmes limites que Steam : le développeur doit les prendre en charge jeu par jeu, elles ne fonctionnent qu'à travers le launcher de la boutique, et elles gardent la dernière copie plutôt qu'un historique. Voici ce que couvre chacun, où sont les trous, et comment garder tous vos jeux synchronisés entre vos PC et un Steam Deck, peu importe où vous les avez achetés.

## Epic Games Store

Le launcher d'Epic a un interrupteur de sauvegardes cloud dans ses paramètres, et les jeux compatibles se synchronisent par lui quand vous jouez sur un autre PC. La prise en charge se fait jeu par jeu : le développeur doit l'implémenter, et beaucoup de jeux de la boutique ne l'ont jamais fait.

Sous Linux et sur le Steam Deck, il n'y a pas de launcher Epic officiel. Heroic peut synchroniser le cloud d'Epic pour les jeux compatibles, mais il faut l'activer jeu par jeu.

## GOG

GOG Galaxy synchronise les sauvegardes cloud des jeux qui affichent « Cloud saves » parmi leurs fonctionnalités sur la page de la boutique. Deux limites sont propres à GOG :

- **Uniquement via Galaxy.** Les installateurs hors ligne de GOG, la partie sans DRM qui fait son attrait, n'ont aucun cloud. Jouez avec l'installateur et vos sauvegardes restent sur ce PC.
- **Jeu par jeu et plateforme par plateforme.** Un jeu ne se synchronise qu'entre les plateformes prévues par le développeur.

Comme pour Epic, Heroic peut synchroniser le cloud de GOG sous Linux et sur le Steam Deck si vous l'activez jeu par jeu.

## Ubisoft, EA et les autres

Ubisoft Connect et l'application EA ont des sauvegardes cloud pour beaucoup de leurs propres jeux, chacune uniquement dans son launcher. Pour Amazon Games et les petites boutiques, cela varie d'un jeu à l'autre.

## Ce qu'aucun ne fait

- **L'historique.** Chaque launcher garde la sauvegarde actuelle. Si une sauvegarde se casse et se synchronise, la bonne disparaît partout.
- **Entre boutiques.** Le même jeu acheté sur Steam pour un PC et sur GOG pour un autre a deux clouds séparés qui ne se parlent jamais.
- **Tout ce qui est hors du launcher.** Émulateurs, installateurs sans DRM, jeux installés à la main.
- **Les jeux non compatibles.** Si le développeur ne l'a pas implémenté, le launcher n'y peut rien.

## Synchroniser le reste

Hoard travaille par jeu, pas par boutique. Il trouve le dossier de sauvegarde de chaque jeu grâce à une base communautaire qui couvre des milliers de titres, d'où que vienne le jeu, le sauvegarde automatiquement quand vous arrêtez de jouer et le synchronise avec vos autres PC et votre Steam Deck, en gardant chaque version.

Cela couvre les trous ci-dessus :

- **N'importe quel launcher, ou aucun.** Epic, GOG, Galaxy ou l'installateur hors ligne, Heroic sous Linux, un jeu décompressé dans un dossier.
- **Entre boutiques.** La plupart des jeux sauvegardent au même endroit quelle que soit la boutique qui les a vendus, en général dans `AppData` ou `Documents`, donc une installation GOG sur un PC et une installation Steam sur un autre peuvent partager une sauvegarde. Certains ajoutent un dossier à l'ID du compte ou changent de nom selon la boutique ; vérifiez avant de compter dessus.
- **Un historique.** Chaque session est une version à laquelle vous pouvez revenir.

Là où le cloud d'un launcher synchronise déjà un jeu, laissez-le faire. Hoard ajoute l'historique, et synchronise tout le reste.

Si vous préférez n'utiliser le cloud de personne, lancez `hoard-server` sur votre propre PC ou NAS. Pas de compte chez nous, pas de télémétrie vers nous, rien ne passe par nos serveurs. Voir [comment auto-héberger Hoard](/guides/self-host-hoard).

<!-- faq -->

## Questions fréquentes

### Epic a-t-il des sauvegardes cloud ?

Oui, pour les jeux dont le développeur les a implémentées, via le launcher d'Epic. Beaucoup de jeux de la boutique ne les prennent pas en charge, et le launcher ne garde aucun historique des sauvegardes précédentes.

### GOG a-t-il des sauvegardes cloud ?

Oui, via GOG Galaxy, pour les jeux qui l'indiquent sur leur page de boutique. Les installateurs hors ligne ne synchronisent rien.

### Epic ou GOG gardent-ils les anciennes versions de mes sauvegardes ?

Non. Les deux ne gardent que la dernière copie. Pour revenir à une sauvegarde antérieure, il faut une copie qui garde les versions.

### Puis-je passer une sauvegarde de la version GOG à la version Steam ?

Souvent, oui : la plupart des jeux sauvegardent dans le même dossier quelle que soit la boutique. Certains ajoutent un dossier à l'ID du compte ou utilisent un autre nom de dossier, alors vérifiez d'abord les chemins.

### Les installateurs hors ligne de GOG synchronisent-ils les sauvegardes ?

Pas via GOG, puisque son cloud ne fonctionne que dans Galaxy. Hoard les synchronise comme n'importe quel autre jeu, car il suit le dossier de sauvegarde et non le launcher.
