---
title: "Syncthing pour les sauvegardes de jeux : ce qui marche et ce qui casse"
description: "Syncthing est un excellent outil de synchronisation généraliste, mais les sauvegardes de jeux brisent trois de ses hypothèses. Ce qui déraille, comment les gens contournent, et quand un outil qui connaît les sauvegardes vaut mieux."
order: 9
updated: 2026-09-01
---

Syncthing est la réponse vers laquelle beaucoup se tournent d'abord, et à raison : gratuit, open source, pair-à-pair, et ça marche. Mais les sauvegardes de jeux brisent trois hypothèses sur lesquelles repose un synchroniseur généraliste, et les échecs sont silencieux. Ce guide parle de ce qui déraille vraiment, et du moment où un outil qui sait ce qu'est une sauvegarde devient utile.

## Pourquoi on y arrive

C'est un très bon logiciel. Pas de compte, pas d'abonnement, vos fichiers ne dorment jamais sur le disque d'une entreprise, et il synchronise n'importe quoi : documents, photos, un dossier de sauvegardes. Si vous l'utilisez déjà pour autre chose, ajouter un dossier vous coûte trente secondes. C'est un argument réel, et pour certains montages c'est le bon.

## Les trois choses qui cassent

**Il synchronise pendant que le jeu tourne.** Syncthing réagit à la modification d'un fichier, ce qui est le comportement correct pour un document. Un jeu écrit sa sauvegarde en pleine session, parfois en plusieurs passes, et un fichier attrapé en cours d'écriture se propage à moitié fini. L'autre machine se retrouve avec une sauvegarde que le jeu peut refuser de charger.

**Les conflits deviennent des fichiers, pas des décisions.** Quand les deux machines modifient la même sauvegarde, Syncthing fait le choix sûr et garde les deux, en renommant l'une en `truc.sync-conflict-20260901-143022-ABCDEFG.sav`. Rien n'est perdu, mais le jeu ignore ce qu'est ce fichier, et vous voilà à comparer des horodatages dans un explorateur pour décider quel après-midi de jeu garder. Répétez quelques fois et le dossier se remplit de fichiers de conflit que personne n'ose supprimer.

**Le versionnage est par fichier, pas par session.** Syncthing peut garder d'anciennes copies dans `.stversions`, ce qui vaut mieux que rien. Mais une sauvegarde est souvent plusieurs fichiers qui n'ont de sens qu'ensemble, et restaurer signifie retrouver à la main le bon horodatage pour chacun. Il n'y a pas de « remets ce jeu comme il était mardi ».

Et un quatrième, propre à Steam : pointez-le sur `userdata/<UserID>/<AppID>/` au lieu du dossier `remote/` à l'intérieur, et vous synchronisez aussi `remotecache.vdf` ainsi que des fichiers de succès et de temps de jeu qui **doivent** différer d'une machine à l'autre. Chaque lancement ressemble alors à un conflit alors qu'aucune sauvegarde n'a bougé. C'est la raison la plus fréquente pour laquelle un montage maison entre Steam Deck et PC de bureau paraît cassé.

## Ce que vous finissez par construire

Rien de tout cela n'est insoluble. On s'en sort avec des motifs d'exclusion par jeu, une politique de versionnage, et l'habitude de fermer le jeu et d'attendre avant de toucher l'autre PC. Ça marche, et c'est un entretien qui vous appartient pour toujours : un nouveau jeu, ce sont de nouveaux chemins, et le jour où vous oubliez d'attendre est le jour où vous l'apprenez.

## Ce que fait à la place un outil qui connaît les sauvegardes

Hoard capture **après que vous avez arrêté de jouer**, une fois le dossier calmé : un instantané n'est donc jamais un fichier à moitié écrit. Chaque capture est une version de la sauvegarde entière, pas de fichiers isolés, donc restaurer se fait en un clic et remet tout ensemble. Il sait quel dossier appartient à quel jeu — il lit le même manifeste communautaire d'emplacements que partage l'écosystème open source, couvrant plus de 20 000 titres — donc aucun chemin à maintenir, et il suit `<AppID>/remote/` plutôt que le dossier au-dessus.

## Quand Syncthing est la meilleure réponse

Pour être juste :

- **Vous l'utilisez déjà**, et ajouter un dossier est gratuit.
- **Vous voulez du pair-à-pair sans aucun serveur**, pas même le vôtre.
- **Vous synchronisez bien plus que des sauvegardes** et préférez un seul outil pour tout.
- **Vous ne revenez jamais en arrière.** Si la dernière sauvegarde vous a toujours suffi, un historique de versions est une mécanique que vous n'utiliserez pas.

## Utiliser les deux

Ils cohabitent sans se battre, et c'est un montage raisonnable : le synchroniseur généraliste s'occupe de vos documents et du reste, un outil qui connaît les sauvegardes s'occupe des dossiers de sauvegarde. La seule règle : ne pointez pas les deux sur le même dossier — deux programmes qui écrivent les mêmes fichiers, c'est fabriquer exactement les conflits que vous vouliez éviter.

## Sans nos serveurs non plus

Si une partie de l'attrait est que rien ne touche le disque d'une entreprise, Hoard se prête au même usage : `hoard-server` sur votre PC ou votre NAS, et vos sauvegardes vont de votre machine à votre disque. **Aucun compte chez nous, aucune télémétrie vers nous, aucun relais** : rien ne passe par nos serveurs, puisque rien de chez nous n'est sur le chemin. Voir [comment auto-héberger Hoard](/guides/self-host-hoard).

Le même binaire, la même détection, le même historique. La seule chose qui change, c'est à qui appartient le stockage. Il existe aussi une [comparaison complète des outils de synchro](/guides/game-save-sync-comparison).

<!-- faq -->

## Questions fréquentes

### Syncthing peut-il synchroniser des sauvegardes de jeux ?

Oui, et pour les cas simples il le fait très bien. Les ennuis commencent avec les jeux qui écrivent pendant que vous jouez, les sauvegardes faites de plusieurs fichiers, et tout montage où les deux machines sont modifiées entre deux synchros.

### Que sont les fichiers .sync-conflict dans mon dossier de sauvegardes ?

C'est le synchroniseur qui garde les deux versions après un conflit au lieu d'en choisir une. Rien n'est perdu, mais le jeu ne sait pas les lire, et décider laquelle garder est un travail manuel à chaque fois.

### Pourquoi ma sauvegarde Steam entre-t-elle en conflit à chaque lancement ?

Presque toujours parce que le dossier synchronisé est celui au-dessus de `remote/`. Il contient `remotecache.vdf` et des fichiers de succès et de temps de jeu qui diffèrent légitimement selon la machine : les deux bouts ne seront jamais d'accord.

### Dois-je fermer le jeu avant de synchroniser ?

Avec un synchroniseur généraliste, oui : c'est l'habitude qui évite les sauvegardes à moitié écrites. Un outil qui connaît les sauvegardes attend tout seul que le dossier se calme.

### Puis-je continuer à utiliser les deux ?

Oui. Ne les pointez simplement pas sur le même dossier, sinon ils se disputeront les mêmes fichiers.
