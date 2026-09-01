---
title: "Comment restaurer une ancienne sauvegarde"
description: "Mauvais choix, fichier corrompu ou envie de repartir de zéro ? Revenez à n'importe quelle version précédente de votre sauvegarde grâce à l'historique cloud de Hoard — y compris des sauvegardes faites avec des outils comme Ludusavi."
order: 3
updated: 2026-09-01
---

Une mauvaise décision en jeu, un fichier corrompu ou un mod qui casse tout — parfois, il faut juste revenir en arrière. Comme Hoard conserve un historique complet des versions de chaque sauvegarde, en restaurer une plus ancienne prend quelques secondes.

## Restaurer une version précédente

1. Ouvrez **Hoard** et allez au jeu dans votre **Bibliothèque**.
2. Ouvrez son onglet **Historique**. Vous verrez chaque sauvegarde avec sa date et sa taille.
3. Choisissez la version voulue et cliquez sur **Restaurer**.
4. Hoard réécrit cet instantané dans le dossier de sauvegarde du jeu. Votre sauvegarde actuelle est d'abord sauvegardée, la restauration est donc réversible.

## Restaurer sur un PC neuf ou réinstallé

1. Installez Hoard et connectez-vous avec votre compte.
2. Ajoutez le jeu à votre Bibliothèque — Hoard trouve la sauvegarde cloud correspondante.
3. Restaurez la dernière version, ou une plus ancienne, et continuez à jouer.

Comme Hoard localise les dossiers de sauvegarde avec la même base communautaire que Ludusavi, il sait où placer une sauvegarde restaurée même sur une installation neuve — sans chasse manuelle au chemin.

## Quand une sauvegarde est corrompue ou qu'un mod l'a cassée

Un jeu qui plante au chargement, un mod qui a réécrit ce qu'il ne fallait pas, une sauvegarde automatique tombée en plein milieu d'une écriture : le remède est le même. Ouvrez l'**Historique** du jeu, choisissez la dernière version d'avant le problème et restaurez-la. Les dates et les tailles suffisent en général à repérer le moment où ça a dérapé — une chute brutale de taille indique souvent une sauvegarde tronquée.

Si vous ne savez pas laquelle est la bonne, restaurez la candidate la plus probable et vérifiez en jeu. Recommencer ne coûte rien, puisque la version que vous venez de remplacer a été conservée elle aussi.

## Ce que fait réellement une restauration

Trois choses à savoir, car ce sont elles qui rendent l'essai sans risque :

1. **Votre sauvegarde actuelle est capturée d'abord.** La restauration est réversible : ce que vous avez remplacé devient une version de l'historique comme une autre.
2. **Seul ce qui manque est téléchargé.** Les fichiers déjà présents avec le bon contenu sont réutilisés tels quels : restaurer une grosse sauvegarde après une petite modification déplace quelques mégaoctets, pas tout le dossier.
3. **Les fichiers propres à cette machine ne sont pas touchés.** La configuration et les journaux voisins de la sauvegarde sont sauvegardés, mais pas réécrits par-dessus vos copies locales : vos touches et vos réglages graphiques survivent à une restauration venue d'un autre PC.

## Restaurer sans passer par nos serveurs

Si vous faites tourner votre propre `hoard-server`, les restaurations fonctionnent exactement pareil, sauf que les versions viennent de votre machine et non de la nôtre. Aucun compte chez nous, aucune télémétrie vers nous, rien qui passe par nos serveurs. Voir [comment auto-héberger Hoard](/guides/self-host-hoard).

## Astuce

Les restaurations ne sont jamais destructrices : la sauvegarde remplacée est d'abord capturée comme nouvelle version, vous pouvez donc toujours annuler une restauration en restaurant l'entrée précédente. Si vous n'aviez que des sauvegardes locales (par exemple avec Ludusavi), passer à Hoard ajoute un historique versionné hors machine, depuis lequel vous pouvez restaurer même après une panne de disque.

<!-- faq -->

## Questions fréquentes

### Une restauration écrase-t-elle ma progression actuelle ?

Seulement après que votre sauvegarde actuelle a été capturée comme nouvelle version. Si vous restaurez la mauvaise, restaurez l'entrée précédente et vous revoilà au point de départ.

### Jusqu'où remonte l'historique ?

Aussi loin que le permet la limite de versions de votre offre, et une version épinglée n'est jamais supprimée pour faire de la place. Sur un serveur auto-hébergé, la seule limite est votre disque.

### Puis-je restaurer sur un PC où le jeu n'est pas encore installé ?

Installez d'abord le jeu pour que son dossier de sauvegarde existe, puis restaurez. Hoard sait où chaque jeu attend ses sauvegardes et écrit l'instantané au bon endroit, sans chasse au chemin.

### Est-ce que ça marche entre Windows et un Steam Deck ?

Oui. Le même jeu range sa sauvegarde à des endroits différents sur chacun — sur le Deck, dans le préfixe Proton — et Hoard écrit la version restaurée là où cette machine l'attend.

### Une restauration est-elle différente sur un serveur auto-hébergé ?

Non. Même application, même historique, même restauration en un clic. Seul le stockage est à vous.
