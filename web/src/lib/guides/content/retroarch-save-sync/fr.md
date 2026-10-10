---
title: "Synchroniser les sauvegardes RetroArch entre PC et Steam Deck"
description: "Synchronisez les sauvegardes et états RetroArch entre PC, Steam Deck et portable : où sont les .srm, Cloud Sync intégré ou synchro automatique, et les pièges."
order: 14
updated: 2026-10-09
---

RetroArch enregistre les sauvegardes du jeu sous forme de fichiers `.srm` dans un dossier `saves`, et les états dans un dossier `states`. Pour les synchroniser entre appareils, vous pouvez utiliser le Cloud Sync intégré de RetroArch avec un serveur WebDAV que vous fournissez, ou un outil qui surveille les deux dossiers. Hoard fait la seconde chose automatiquement : il sauvegarde les deux dossiers quand vous quittez RetroArch, les récupère sur vos autres machines, garde chaque version et comprend les installations EmuDeck.

## Où RetroArch range les sauvegardes

- **Windows :** `%APPDATA%\RetroArch\saves` et `\states`, ou `saves` et `states` à côté de `retroarch.exe` si vous l'avez installé dans son propre dossier.
- **Linux :** `~/.config/retroarch/saves`. Le Flatpak utilise `~/.var/app/org.libretro.RetroArch/config/retroarch/saves`.
- **Steam Deck avec EmuDeck :** `~/Emulation/saves/retroarch/`, où `saves` et `states` sont des liens vers les vrais dossiers. Hoard lit `retroarch.cfg` pour savoir où ils pointent vraiment.
- **RetroDECK :** `~/retrodeck/saves` et `~/retrodeck/states` par défaut.
- **Ailleurs :** **Settings → Directory** montre les dossiers que RetroArch utilise réellement.

## Quand RetroArch écrit vraiment la sauvegarde

C'est là que beaucoup se font avoir. RetroArch garde la sauvegarde du jeu en mémoire et n'écrit le `.srm` que quand vous fermez le jeu ou quittez RetroArch, sauf si **Settings → Saving → SaveRAM Autosave Interval** est réglé. Jusque-là, rien n'est sur le disque : un plantage ou une batterie à plat fait perdre tout ce qui s'est passé depuis la dernière écriture, et aucun outil de synchronisation ne peut déplacer une sauvegarde qui n'a pas été écrite.

Réglez un intervalle de sauvegarde automatique de quelques secondes. Et avant de changer d'appareil, **quittez RetroArch**, pas seulement le jeu : Hoard sauvegarde une fois RetroArch fermé, il ne copie donc jamais une sauvegarde à moitié écrite. Sur un Deck, la mise en veille ne compte pas comme une fermeture.

## Réglez tous les appareils de la même façon

- **Options de tri.** **Settings → Saving** peut ranger sauvegardes et états dans des sous-dossiers par nom de cœur ou par dossier de contenu. Si un appareil trie et l'autre non, le fichier synchronisé atterrit dans un dossier où RetroArch ne regarde pas. Utilisez les mêmes réglages partout.
- **Noms des ROM.** Le `.srm` porte le nom de la ROM : `Super Metroid (USA).sfc` sauvegarde dans `Super Metroid (USA).srm`. Une ROM nommée autrement sur l'autre appareil ne le trouvera pas.
- **Le même cœur.** Deux cœurs pour la même console ne lisent pas toujours les sauvegardes l'un de l'autre. Choisissez-en un par système et utilisez-le partout.
- **Versions des cœurs, pour les états.** Un état est un instantané de la mémoire du cœur et ne se charge souvent pas dans une autre version. Les sauvegardes classiques n'ont pas ce problème.

Un autre piège avec les états : un état contient la mémoire du jeu, sauvegarde incluse. Chargez un vieil état et la prochaine écriture du `.srm` ramène cette ancienne sauvegarde. Si vous utilisez **Auto Load State**, synchronisez aussi les états, pour que ce soit le plus récent qui voyage.

## Cloud Sync de RetroArch ou Hoard ?

Pour être juste avec les deux :

- **Le Cloud Sync de RetroArch** est intégré et synchronise sauvegardes et états avec un serveur WebDAV que vous gérez ou louez. Il marche aussi sur Android et iOS, ce que Hoard ne fait pas aujourd'hui. Si vous utilisez déjà Nextcloud, qui garde lui-même des versions des fichiers, il convient bien, et c'est le meilleur choix si votre téléphone fait partie de votre installation.
- **Hoard** n'a pas besoin de serveur WebDAV. Il sauvegarde et synchronise automatiquement, garde un historique de versions où revenir, et couvre aussi vos émulateurs autonomes et vos jeux PC. Il traite le dossier `saves` entier comme un seul élément, donc revenir en arrière restaure chaque jeu qu'il contient tel qu'il était à ce moment-là. Avant de confirmer, il montre ce qui va changer, et vos fichiers actuels sont copiés d'abord.

Choisissez-en un par dossier. Deux outils qui écrivent les mêmes sauvegardes, c'est la recette des conflits.

## Mise en place avec Hoard

1. Installez Hoard sur chaque appareil et connectez-vous avec le même compte.
2. Dans la **Bibliothèque**, ajoutez RetroArch. Les sauvegardes et les états apparaissent comme deux éléments.
3. Alignez les réglages ci-dessus sur tous les appareils.
4. Jouez, quittez RetroArch et reprenez sur l'autre appareil.

Vous préférez tout garder chez vous ? Lancez `hoard-server` sur votre PC ou NAS : pas de compte chez nous, pas de télémétrie vers nous, rien ne passe par nos serveurs. Voir [comment auto-héberger Hoard](/guides/self-host-hoard).

<!-- faq -->

## Questions fréquentes

### RetroArch a-t-il des sauvegardes cloud ?

Oui, un Cloud Sync intégré qui demande un serveur WebDAV. Hoard est l'alternative si vous ne voulez pas en gérer un, si vous voulez un historique de versions ou si vous utilisez aussi des émulateurs autonomes.

### Pourquoi ma sauvegarde RetroArch ne s'est-elle pas synchronisée ?

En général pour l'une de trois raisons : RetroArch n'avait pas encore écrit le `.srm` (il était encore ouvert, sans intervalle de sauvegarde automatique), les deux appareils rangent les sauvegardes dans des sous-dossiers différents, ou les ROM n'ont pas le même nom.

### Puis-je aussi synchroniser les états ?

Oui. Hoard suit `states` comme un élément à part. Qu'un état se charge sur l'autre appareil dépend de la même version du cœur des deux côtés.

### Ça marche avec EmuDeck et RetroDECK ?

Oui. Hoard lit la configuration de RetroArch pour suivre les liens d'EmuDeck jusqu'aux vrais dossiers. Pour RetroDECK, ajoutez `~/retrodeck/saves` et `~/retrodeck/states` s'ils ne sont pas détectés.

### Hoard synchronise-t-il RetroArch sur Android ?

Pas aujourd'hui. Hoard tourne sur Windows, macOS, Linux et Steam Deck.
