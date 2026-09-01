---
title: "Alternative à Ludusavi : synchronisation cloud automatique de vos parties"
description: "Une comparaison équitable entre Ludusavi et Hoard. Ludusavi est un excellent outil open source de sauvegarde locale ; Hoard ajoute une synchro cloud gérée et un historique versionné sur tous vos PC — avec les mêmes données d'emplacement."
order: 5
updated: 2026-09-01
---

Si vous cherchez un moyen de sauvegarder et synchroniser vos parties, vous avez sans doute trouvé **Ludusavi** — et il est excellent. Ce guide est une comparaison honnête pour vous aider à choisir le bon outil, et explique où Hoard s'inscrit si vous voulez une synchro cloud automatique entre machines.

## Ce que Ludusavi fait bien

Ludusavi est un outil gratuit et open source (créé par mtkennerly) pour sauvegarder et restaurer les parties PC sous Windows, macOS et Linux. Il a une interface soignée et une CLI, trouve automatiquement les sauvegardes de milliers de jeux, conserve des sauvegardes locales versionnées, et peut envoyer ces sauvegardes vers un cloud qui vous appartient en configurant **Rclone** (Google Drive, Dropbox et bien d'autres). Si vous voulez un contrôle total et un montage fait main, Ludusavi est un choix fantastique — et entièrement gratuit.

Hoard n'est pas là pour le remplacer. En fait, **Hoard utilise la même base de données communautaire d'emplacements que celle sur laquelle s'appuie Ludusavi** pour localiser où chaque jeu range ses sauvegardes : la qualité de détection est donc équivalente.

## En quoi Hoard est différent

Le point où la plupart bloquent avec tout outil local, c'est la **synchronisation entre appareils**. Avec Ludusavi, vous la faites vous-même : planifier une sauvegarde, configurer un distant Rclone, puis restaurer sur l'autre PC avant de jouer. Ça marche, mais c'est manuel.

Hoard transforme cela en **synchro cloud gérée** :

- **Connectez-vous et c'est parti.** Pas de distants Rclone, pas de scripts. Hoard envoie votre sauvegarde après le jeu et télécharge la dernière version avant que vous commenciez, sur chaque PC de votre compte.
- **Historique versionné dans le cloud.** Chaque sauvegarde est conservée, vous pouvez donc revenir à n'importe quelle sauvegarde antérieure — même après une panne de disque ou une installation neuve.
- **Gestion des conflits.** Hoard compare les horodatages et conserve une copie locale de tout ce qu'il remplace, donc une synchro ne détruit jamais la progression en silence.
- **Toujours open source et auto-hébergeable.** Comme Ludusavi, pas de verrouillage — utilisez Hoard Cloud ou hébergez le serveur vous-même.

## Face à face

| | Ludusavi | Hoard |
|---|---|---|
| Sauvegardes locales | Oui | Oui |
| Détection des sauvegardes | Manifeste communautaire | Le même manifeste, plus les bibliothèques Steam, les processus en cours et un balayage du disque |
| Stockage cloud | Le vôtre, via Rclone | Inclus, ou votre propre serveur |
| Synchro entre PC | Manuelle : sauvegarder ici, restaurer là-bas | Automatique, après avoir joué et avant de commencer |
| Historique des versions | Sauvegardes locales que vous élaguez vous-même | Toutes les versions dans le cloud, dédupliquées par empreinte de contenu |
| Émulateurs | Oui | Oui |
| Interfaces | Application de bureau et CLI | Application de bureau, CLI et surcouche en jeu |
| Prix | Gratuit | Offre gratuite de 2 Go et 3 appareils, Pro au-delà, sans quota en auto-hébergement |
| Licence | MIT | AGPL-3.0 |

## Quand Ludusavi est le meilleur choix

C'est la partie que presque aucune page de comparaison n'inclut. Ludusavi est le meilleur outil quand :

- **Vous ne jouez que sur un seul PC.** La synchro cloud résout alors un problème que vous n'avez pas. Une sauvegarde locale suffit, et Ludusavi les fait très bien.
- **Vous avez déjà un distant Rclone en qui vous avez confiance.** Si votre stockage est configuré et fonctionne, l'avantage principal de Hoard est une étape que vous avez déjà payée.
- **Vous voulez l'utiliser depuis le mode Jeu d'un Steam Deck.** Ludusavi a un plugin Decky : vous pouvez lancer une sauvegarde sans quitter l'interface console.
- **Vous voulez une licence permissive.** Ludusavi est en MIT, Hoard en AGPL-3.0. Si vous comptez bâtir quelque chose par-dessus sans publier le résultat, cette différence compte.
- **Vous ne voulez rien qui tourne en fond.** Auto-héberger Hoard veut dire garder un petit serveur allumé quelque part, même sur le même PC. Ludusavi est une application que vous ouvrez au besoin.

## Passer de Ludusavi à Hoard

Il n'y a pas d'importateur, et c'est volontaire. Les étapes :

1. **Laissez vos sauvegardes Ludusavi exactement où elles sont.** Rien n'est migré ni supprimé. Gardez-les comme filet de sécurité les premières semaines.
2. **Installez Hoard et connectez-vous**, ou pointez-le vers votre propre serveur.
3. **Laissez-le analyser.** Il lit le même manifeste : la liste des jeux détectés devrait vous sembler familière.
4. **Ne pointez pas Hoard vers votre dossier de sauvegardes Ludusavi.** Suivez le dossier dans lequel le jeu écrit lui-même. Un dossier de sauvegardes est une copie qui change selon un horaire et non quand vous jouez, et synchroniser la copie d'une copie, c'est ainsi qu'on finit par restaurer la progression d'hier. Hoard essaie de le repérer tout seul — `hoard doctor` signale un dossier suivi qui ressemble à un miroir de sauvegardes — mais le plus simple est de ne jamais l'ajouter.
5. **Jouez une fois.** En quittant, la première version apparaît dans l'historique.
6. **Recommencez sur le second PC.** Connectez-vous et les versions sont déjà là.

## Deux détails à connaître

**Les sauvegardes Steam sont un dossier plus bas qu'on ne croit.** Pour les jeux Steam, Hoard suit `<AppID>/remote/` dans `userdata`, pas le dossier au-dessus. Le dossier parent contient aussi `remotecache.vdf` ainsi que des fichiers de succès et de temps de jeu, qui diffèrent légitimement d'une machine à l'autre. Synchronisez le parent et chaque lancement ressemble à un conflit alors qu'aucune sauvegarde n'a bougé. C'est la raison la plus fréquente pour laquelle un montage maison entre Steam Deck et PC de bureau finit par se battre contre lui-même.

**Les versions coûtent peu.** Les instantanés sont stockés par empreinte de contenu : un fichier inchangé n'est stocké qu'une fois. Dix versions d'une sauvegarde de 2 Go coûtent environ 2 Go, pas 20 — c'est ce qui rend viable de garder tout l'historique au lieu de l'élaguer.

## Ce que l'auto-hébergement veut vraiment dire

C'est le point sur lequel presque toutes les comparaisons se trompent au sujet de Hoard, autant être précis. Il y a deux façons de l'utiliser, et elles sont réellement différentes :

- **Hoard Cloud** est l'option gérée : vous vous connectez, et vos sauvegardes sont stockées sur nos serveurs, dans l'UE.
- **L'auto-hébergement est entièrement le vôtre.** Vous faites tourner `hoard-server` sur votre PC ou votre NAS, et vos sauvegardes vont de votre machine à votre disque. Il n'y a **aucun compte chez nous, aucune télémétrie vers nous, aucun quota et aucun relais** : rien ne passe par nos serveurs, puisque rien de chez nous n'est sur le chemin. Nous ne pouvons voir ni une sauvegarde, ni un nom de jeu, ni une adresse e-mail, pour la simple raison que rien de tout cela ne nous parvient. Si Hoard Cloud disparaissait demain, une installation auto-hébergée continuerait à l'identique.

Le même programme, la même détection, le même historique de versions. La seule chose qui change, c'est à qui appartient le stockage.

## Lequel choisir ?

- Choisissez **Ludusavi** si vous voulez un outil de sauvegarde gratuit et local et que configurer votre propre cloud avec Rclone ne vous dérange pas.
- Choisissez **Hoard** si vous voulez que la sauvegarde *et* la synchro entre PC fonctionnent toutes seules, avec un historique cloud versionné, tout en gardant l'option de l'auto-hébergement.

Beaucoup commencent avec Ludusavi pour les sauvegardes locales et passent à Hoard dès qu'ils jouent aux mêmes jeux sur plus d'une machine. Si c'est votre cas, voir [comment synchroniser vos parties entre PC](/guides/sync-game-saves-across-pcs) ou simplement [téléchargez Hoard](/download) et connectez-vous. Pour l'ensemble du paysage, il y a une [comparaison de tous les outils de synchro](/guides/game-save-sync-comparison).

<!-- faq -->

## Questions fréquentes

### Puis-je utiliser Ludusavi et Hoard en même temps ?

Oui. Ils lisent les mêmes emplacements et aucun des deux ne verrouille les fichiers. Beaucoup gardent Ludusavi pour les sauvegardes d'archive locales et laissent Hoard gérer la synchro entre machines. La seule règle : ne pointez pas un outil vers le dossier de sauvegardes de l'autre.

### Hoard importe-t-il mes sauvegardes Ludusavi ?

Non, et c'est délibéré. Un dossier de sauvegardes est une copie qui change selon son propre horaire ; le suivre synchroniserait un miroir périmé au lieu de votre sauvegarde réelle. Hoard suit le dossier dans lequel le jeu écrit et démarre son propre historique à votre prochaine session. Gardez l'archive Ludusavi comme filet de sécurité.

### Hoard est-il gratuit ?

Hoard Cloud a une offre gratuite de 2 Go de stockage et 3 appareils, ce qui couvre la plupart des collections ; Pro augmente les deux. Auto-héberger le serveur est gratuit et sans aucun quota. Tout est open source sous AGPL-3.0.

### Hoard fonctionne-t-il sur Steam Deck ?

Oui, sur Steam Deck et sur n'importe quel bureau Linux, ainsi que sous Windows et macOS. Le Deck est précisément le cas qui exige le détail `remote/` ci-dessus, car un Deck et un PC de bureau écrivent des fichiers de succès et de temps de jeu différents à côté de la même sauvegarde.

### Ai-je besoin de Rclone ou d'un compte cloud à moi ?

Non. C'est la principale différence pratique : avec Hoard Cloud, le stockage est déjà en place dès la connexion. Si vous préférez posséder le stockage, faites tourner le serveur vous-même sur un bucket compatible S3 ou un simple dossier de votre machine.

### L'auto-hébergement envoie-t-il quoi que ce soit à Hoard ?

Non. En mode auto-hébergé il n'y a aucun compte chez nous ni aucune télémétrie vers nous : vos sauvegardes, vos utilisateurs et vos journaux vivent sur votre propre serveur et ne touchent jamais le nôtre. C'est tout l'intérêt de ce mode, et c'est pourquoi le serveur est le même binaire open source que celui que nous faisons tourner, pas une version allégée.
