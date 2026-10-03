---
title: "Emplacement des sauvegardes de Marvel's Spider-Man 2 (PC et Steam Deck)"
description: "Où Marvel's Spider-Man 2 range ses sauvegardes sur PC, ce qu'est le dossier au long numéro, le piège OneDrive, le chemin sur Steam Deck et comment sauvegarder."
order: 22
updated: 2026-10-02
---

Sur PC, Marvel's Spider-Man 2 range ses sauvegardes dans `Documents\Marvel's Spider-Man 2\`, dans un sous-dossier au long numéro. Sur Steam, ce numéro est votre identifiant Steam. Voici ce que cela implique, le piège OneDrive, le chemin sur Steam Deck et comment garder vos sauvegardes à l'abri.

## Où Marvel's Spider-Man 2 range ses sauvegardes

- **Windows :** `%USERPROFILE%\Documents\Marvel's Spider-Man 2\<long numéro>`
- **Steam Deck et Linux** (Proton) : `~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/<long numéro>`

La version PC n'existe que sous Windows : sur Steam Deck, elle passe par Proton, et les sauvegardes se trouvent dans le préfixe Proton que Steam garde pour le jeu ; `2651280` est son identifiant Steam. S'il est installé sur la carte microSD, le dossier `compatdata` est sur la carte.

Nixxes, le studio derrière le portage PC, décrit le dossier de sauvegarde comme « un sous-dossier avec un long numéro ou une combinaison de lettres et de chiffres » sous `Documents\Marvel's Spider-Man 2\`.

## Le dossier au long numéro

Le sous-dossier porte le nom de votre compte : sur Steam, c'est votre **identifiant Steam 64 bits** ; la version Epic utilise plutôt un mélange de lettres et de chiffres. Dans tous les cas, il diffère pour chaque compte. Deux conséquences :

- Si deux personnes jouent sur le même PC avec des comptes Steam différents, chacune a son propre dossier de sauvegarde.
- Si vous copiez des sauvegardes à la main sur un autre PC, mettez-les dans le dossier du compte Steam de **cette** machine. Dans un dossier avec un autre identifiant, le jeu ne les voit pas.

Le dossier parent `Marvel's Spider-Man 2` contient aussi le journal du jeu et des rapports de plantage (`.log`, `.mdmp`). Ce ne sont pas des sauvegardes et inutile de les sauvegarder.

## Le piège OneDrive

Beaucoup de PC Windows redirigent `Documents` vers OneDrive. Si c'est votre cas, le vrai chemin est `%USERPROFILE%\OneDrive\Documents\Marvel's Spider-Man 2\`, et OneDrive synchronise le dossier de lui-même pendant que vous jouez. Deux problèmes : OneDrive peut envoyer une sauvegarde à moitié écrite, et « Libérer de l'espace » peut transformer la sauvegarde en fichier disponible uniquement en ligne. Si vous comptez sur OneDrive ici, marquez le dossier **Toujours conserver sur cet appareil**.

## Spider-Man 2 a-t-il des sauvegardes cloud ?

Oui, Steam Cloud, qui garde les dernières sauvegardes synchronisées entre les machines d'un même compte Steam. Il ne garde pas les anciennes versions : si une sauvegarde casse, c'est la version cassée qui se synchronise.

## Sauvegarder à la main

1. Fermez complètement le jeu.
2. Copiez le dossier `Marvel's Spider-Man 2` de `Documents` vers un endroit sûr.
3. Pour restaurer, fermez le jeu et recopiez le dossier au long numéro au même endroit, sous le même compte Steam.

## Sauvegarde et synchro automatiques avec Hoard

[Hoard](/download) sauvegarde le dossier à chaque fois que vous arrêtez de jouer et garde toutes les versions. Il le synchronise aussi entre vos PC et un Steam Deck, pour que le jeu reprenne là où vous l'avez laissé sur l'un comme sur l'autre.

1. Installez Hoard et connectez-vous, ou pointez-le vers [votre propre serveur](/guides/self-host-hoard).
2. Ouvrez la **Bibliothèque** et vérifiez que le dossier affiché pour Spider-Man 2 est celui sous `Documents` (ou `OneDrive\Documents`). S'il pointe ailleurs, changez-le pour ce dossier.
3. Jouez. En quittant, la première version apparaît dans l'historique.

Si une sauvegarde tourne mal plus tard, [restaurer une version antérieure](/guides/restore-a-game-save) la remet en place.

<!-- faq -->

## Questions fréquentes

### Qu'est-ce que le long numéro du dossier de sauvegarde ?

Sur Steam, votre identifiant Steam 64 bits ; sur Epic, l'identifiant de votre compte. Chaque compte a son propre dossier, et le jeu ne lit que celui du compte connecté.

### Où sont les sauvegardes de Spider-Man 2 sur Steam Deck ?

Dans le préfixe Proton : `~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/`, dans le dossier au long numéro.

### Je ne trouve pas le dossier dans Documents. Où est-il ?

Regardez dans `OneDrive\Documents\Marvel's Spider-Man 2`. Sur la plupart des installations récentes de Windows, Documents se trouve dans OneDrive.

### Puis-je copier mes sauvegardes sur le PC d'un ami ?

Les fichiers se copient, mais ils vont dans le dossier portant l'identifiant Steam du compte de ce PC. Que le jeu accepte des sauvegardes créées sur un autre compte dépend du jeu : gardez une copie de l'original avant d'essayer.
