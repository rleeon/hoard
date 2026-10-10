---
title: "Synchroniser ses sauvegardes entre Windows et Linux sur un PC en dual boot"
description: "Un PC, deux systèmes, deux dossiers de sauvegarde. Synchronisez vos parties entre Windows et Linux, pourquoi un dossier NTFS partagé casse, et les pièges."
order: 18
updated: 2026-10-09
---

Sur un PC en dual boot, le même jeu garde deux sauvegardes séparées : une dans votre dossier utilisateur Windows, et une dans un préfixe Proton sous Linux. Steam Cloud fait le lien pour les jeux compatibles ; tout le reste se désynchronise dès que vous changez de système. Hoard les garde synchronisées automatiquement. Installez-le sur les deux systèmes avec le même compte, et la sauvegarde de chaque jeu vous suit, quel que soit le système que vous démarrez.

## Pourquoi un jeu a deux sauvegardes

Le disque est partagé, mais pas les dossiers de sauvegarde :

- **Sous Windows**, un jeu écrit dans `Documents`, `Saved Games` ou l'un des dossiers `AppData` sous `C:\Users\<vous>`.
- **Sous Linux**, le même jeu Windows tourne via Proton et écrit dans son propre préfixe : `~/.local/share/Steam/steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/`, suivi du même chemin Windows.

Deux copies d'une sauvegarde, dans deux systèmes qui ne tournent jamais en même temps. Quel que soit celui où vous avez joué en dernier, l'autre n'en sait rien. Hoard cherche aux deux endroits et les rattache au bon jeu, si bien que la sauvegarde Windows et la sauvegarde Linux deviennent deux versions d'un même historique.

## Ce que Steam Cloud couvre déjà

Pour les jeux Steam compatibles Steam Cloud, Steam synchronise tout seul la sauvegarde entre votre installation Windows et votre installation Proton. Là, Hoard apporte l'historique : Steam ne garde que la sauvegarde actuelle, donc une sauvegarde abîmée remplace la bonne sur les deux systèmes. Pour les jeux sans Steam Cloud, et pour tout ce qui est hors de Steam, Hoard assure aussi la synchronisation.

## Pourquoi ne pas simplement partager un dossier sur le disque Windows ?

C'est la première idée de presque tout le monde : faire pointer Linux vers les sauvegardes de la partition Windows et en rester là. Ça casse généralement de trois façons :

- **Démarrage rapide et hibernation.** Quand Windows s'éteint avec le démarrage rapide activé, il laisse sa partition à moitié en hibernation, et Linux la monte en lecture seule ou refuse. Votre jeu ne peut pas écrire sa sauvegarde.
- **NTFS sous Proton.** Faire tourner des préfixes Proton ou des bibliothèques Steam depuis un disque NTFS est une source connue de problèmes de permissions et de noms de fichiers. Les jeux sous Linux sont plus à l'aise sur un système de fichiers Linux.
- **Les liens se font remplacer.** Lier le dossier de sauvegarde d'un système dans l'autre fonctionne jusqu'à ce qu'un jeu, une mise à jour ou une réinstallation remplace discrètement le lien par un vrai dossier.

Laisser chaque système garder ses sauvegardes là où le jeu les attend, et les synchroniser entre eux, évite les trois.

## Mise en place

1. **Sous Windows**, installez Hoard et connectez-vous.
2. **Sous Linux**, installez Hoard depuis [la page de téléchargement](/download) et connectez-vous avec le même compte.
3. **Lancez une fois chaque jeu Proton sous Linux**, pour que son préfixe existe. Avant, il n'y a pas de dossier où mettre la sauvegarde.
4. Vérifiez la **Bibliothèque** sur les deux systèmes : les mêmes jeux doivent apparaître de chaque côté, et Hoard les rattache par jeu.

## Le piège propre au dual boot

Avec deux PC distincts, la sauvegarde attend sur le serveur que l'autre machine la demande. Sur un PC en dual boot, « l'autre machine » est le même ordinateur après un redémarrage, et cela change une habitude.

Hoard envoie une sauvegarde une fois le jeu fermé et le dossier au repos. **Si vous quittez le jeu et redémarrez aussitôt, l'envoi n'a peut-être pas encore eu lieu**, et l'autre système démarre sans votre dernière progression. Il rattrapera la prochaine fois que vous redémarrerez sur le premier, mais d'ici là vous aurez peut-être joué sur l'ancienne sauvegarde.

Donc : quittez le jeu, laissez un instant à Hoard, vérifiez dans l'application que la sauvegarde est à jour, puis redémarrez.

## Versions Linux natives

Certains jeux ont une version Linux native en plus de la version Windows. Les deux n'utilisent pas toujours le même format de sauvegarde, et quelques-unes les rangent à des endroits complètement différents. Pour avoir la même sauvegarde sur les deux systèmes, le plus sûr est de lancer aussi la version Windows via Proton sous Linux : dans Steam, **Propriétés → Compatibilité**, forcez une version de Proton. Les deux systèmes font alors tourner le même jeu et écrivent les mêmes fichiers.

## Réglages et appareils

Les réglages graphiques peuvent différer entre les deux systèmes, donc Hoard sauvegarde les fichiers de réglages comme `graphics.ini` mais ne les écrit pas par-dessus ceux de l'autre système. Si vous voulez quand même les copier, une option le permet lors de la restauration.

Chaque système d'exploitation compte comme un appareil, donc un PC en dual boot utilise deux des trois appareils de l'offre gratuite. Pro et les serveurs auto-hébergés n'ont pas de limite d'appareils.

Vous préférez garder les sauvegardes chez vous ? Lancez `hoard-server` sur un NAS ou une autre machine et pointez les deux systèmes dessus. Pas de compte chez nous, pas de télémétrie vers nous, rien ne passe par nos serveurs. Voir [comment auto-héberger Hoard](/guides/self-host-hoard).

<!-- faq -->

## Questions fréquentes

### Steam Cloud synchronise-t-il entre Windows et Linux ?

Oui, pour les jeux compatibles : Steam garde une copie cloud par compte, quel que soit le système sur lequel vous jouez. Il ne garde aucun historique, et ne couvre ni les jeux sans Steam Cloud ni ce qui est hors de Steam.

### Puis-je garder mes sauvegardes sur la partition NTFS partagée ?

Ce n'est pas recommandé. Le démarrage rapide peut laisser la partition en lecture seule sous Linux, et Proton a des problèmes connus avec NTFS. Il est plus fiable que chaque système garde ses sauvegardes à sa place et de les synchroniser.

### Pourquoi ma sauvegarde n'était-elle pas là après le redémarrage ?

Très probablement, l'envoi n'était pas terminé au moment du redémarrage. Redémarrez sur le premier système, laissez Hoard envoyer la sauvegarde et vérifiez l'application avant de changer à nouveau.

### Un PC en dual boot compte-t-il comme un seul appareil ?

Non, comme deux : chaque système d'exploitation s'enregistre comme son propre appareil. Sur l'offre gratuite, cela fait deux sur trois ; Pro et les serveurs auto-hébergés n'ont pas de limite.

### Et si un jeu a une version Linux native ?

Ses sauvegardes peuvent ne pas correspondre à celles de la version Windows. Pour partager une sauvegarde, lancez aussi la version Windows via Proton sous Linux.
