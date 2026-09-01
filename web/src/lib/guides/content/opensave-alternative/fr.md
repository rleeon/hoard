---
title: "Alternative à OpenSave : direct entre machines ou serveur qui vous appartient"
description: "OpenSave synchronise les parties directement entre vos PC, sans rien au milieu. Hoard passe par un serveur — le nôtre ou le vôtre — et garde un historique versionné. Un regard honnête sur les cas où chaque approche l'emporte."
order: 8
updated: 2026-09-01
---

Les deux outils résolvent le même problème et divergent sur l'architecture, et c'est bien la seule chose qui mérite comparaison. Cette page met les deux approches côte à côte, y compris les cas où l'autre est la meilleure réponse.

## La vraie différence : direct ou via un serveur

**OpenSave** est pair-à-pair. Vos machines se parlent directement, sans rien entre elles. Pas de compte, pas de stockage à payer, et la possibilité de refléter une copie vers un cloud que vous avez déjà.

**Hoard** synchronise via un serveur. Ce serveur est soit Hoard Cloud, géré par nous, soit `hoard-server` sur votre propre PC ou NAS. Votre sauvegarde monte quand vous arrêtez de jouer et redescend quand une autre machine la demande.

Tout le reste découle de ce seul choix.

## Ce qu'un serveur vous apporte

- **L'autre machine n'a pas besoin d'être allumée.** Vous finissez sur le fixe, le portable reste fermé une semaine, et la dernière sauvegarde attend quand vous l'ouvrez. Le pair-à-pair exige les deux bouts éveillés en même temps : parfait à un bureau, pénible avec une console portable que vous sortez deux fois par mois.
- **Un historique de versions, pas seulement le dernier état.** Chaque session devient une version où revenir. C'est ce qui compte le jour où un mod dévore votre monde ou qu'une sauvegarde s'écrit à moitié : une synchro directe recopie fidèlement le fichier cassé sur l'autre PC.
- **Une copie qui survit au matériel.** Que vos deux PC meurent dans le même appartement n'a rien d'exotique. Une sauvegarde qui n'a existé que sur ces deux machines meurt avec elles.
- **Rien à préparer côté réseau.** Pas de NAT à traverser, pas de port à ouvrir, pas de condition d'être sur le même réseau local.

## Ce que le pair-à-pair vous apporte

Pour être juste avec l'autre camp :

- **Jamais de stockage à payer.** Aucun quota à atteindre, puisqu'il n'y a pas d'espace de stockage. L'offre gratuite de Hoard, c'est 2 Go ; au-delà, vous payez ou vous auto-hébergez.
- **Rien au milieu, par construction.** Si l'objectif est qu'un fichier ne touche jamais le disque d'un tiers, le transfert direct est la réponse la plus courte possible.
- **Rien à faire tourner.** Aucun serveur à maintenir, pas même le vôtre.

Si vous jouez sur deux fixes allumés tous les deux, que vous ne voulez jamais revenir en arrière et que le stockage ne doit pas entrer dans l'équation, cette approche convient parfaitement et Hoard est plus de machinerie qu'il n'en faut.

## La question de la vie privée, précisément

C'est là que les comparaisons de Hoard se trompent d'habitude, alors soyons exacts : il y a deux façons de faire tourner Hoard, et elles sont réellement différentes.

- **Hoard Cloud** est l'option gérée : vous vous connectez, et vos sauvegardes sont sur nos serveurs, dans l'UE.
- **L'auto-hébergement est entièrement le vôtre.** Vous faites tourner `hoard-server` sur votre PC ou votre NAS, et vos sauvegardes vont de votre machine à votre disque. **Aucun compte chez nous, aucune télémétrie vers nous, aucun quota et aucun relais** : rien ne passe par nos serveurs, puisque rien de chez nous n'est sur le chemin. Nous ne voyons ni sauvegarde, ni nom de jeu, ni adresse e-mail, car rien de tout cela ne nous parvient. Si Hoard Cloud fermait demain, une installation auto-hébergée continuerait à l'identique.

Donc « serveur » ne veut pas dire « l'ordinateur de quelqu'un d'autre », sauf si vous le choisissez. Un Hoard auto-hébergé garde vos sauvegardes sur du matériel qui vous appartient, exactement comme un transfert direct, et vous donne en plus l'historique et le cas de la machine éteinte.

## Détection et couverture

Les deux outils trouvent automatiquement les sauvegardes d'un large catalogue. Hoard lit le même manifeste communautaire d'emplacements que partage l'écosystème open source, couvrant plus de 20 000 titres, et y ajoute l'analyse des bibliothèques Steam, les processus en cours et un balayage du disque. Pour les jeux Steam, il suit `<AppID>/remote/` dans `userdata` et non le dossier au-dessus, car le parent contient `remotecache.vdf` et des fichiers de succès et de temps de jeu propres à chaque machine : les synchroniser, et chaque lancement ressemble à un conflit. Pour les cas particuliers, vous lui désignez le dossier.

## Lequel choisir ?

- **Le pair-à-pair** si vos machines sont allumées en même temps, que le stockage ne doit pas entrer en jeu et que la dernière sauvegarde vous a toujours suffi.
- **Hoard** si vous voulez un historique où revenir, une machine qui peut rester éteinte une semaine et une copie qui survive aux deux PC — au choix via notre cloud ou votre propre serveur.

Il existe une [comparaison de tous les outils de synchro](/guides/game-save-sync-comparison) pour le paysage complet, et une [comparaison avec Ludusavi](/guides/ludusavi-alternative) pour le versant sauvegarde locale.

<!-- faq -->

## Questions fréquentes

### Hoard exige-t-il un compte ?

Pour Hoard Cloud, oui : la synchro y est rattachée. En auto-hébergé, aucun compte chez nous ; votre serveur a ses propres utilisateurs et un jeton par appareil, et ils ne quittent jamais votre machine.

### Hoard peut-il fonctionner sans aucun cloud ?

Oui. Faites tourner `hoard-server` sur un PC ou un NAS et vos sauvegardes vont de votre machine à votre disque, sans que rien passe par nos serveurs.

### Les deux PC doivent-ils être en ligne en même temps ?

Non, et c'est l'avantage pratique de passer par un serveur. Votre sauvegarde est envoyée quand vous arrêtez de jouer et téléchargée dès que l'autre machine la réclame.

### Un transfert direct garde-t-il un historique de versions ?

Pas en soi : copier un fichier vers une autre machine vous donne l'état actuel des deux côtés. Hoard capture chaque session comme une version, ce qui rend possible le retour en arrière après une sauvegarde corrompue.

### Hoard est-il open source lui aussi ?

Oui, AGPL-3.0, serveur compris. Le serveur auto-hébergé est le même binaire que celui que nous faisons tourner, pas une édition allégée.
