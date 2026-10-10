---
title: "Synchroniser ses sauvegardes entre Steam Deck et PC"
description: "Synchronisez automatiquement les sauvegardes du Steam Deck et du PC, jeux hors Steam, émulateurs et jeux sans Steam Cloud compris. Étapes, chemins, pièges."
order: 10
updated: 2026-10-09
---

Pour les jeux Steam compatibles Steam Cloud, votre Deck et votre PC partagent déjà les sauvegardes. Tout le reste a besoin d'un coup de main : les jeux dont le développeur n'a jamais activé Steam Cloud, les jeux Epic et GOG lancés avec Heroic, les émulateurs et tout ce que vous avez ajouté comme jeu non-Steam. Hoard s'occupe de tout cela automatiquement. Quand vous quittez un jeu sur une machine, il sauvegarde la partie, et l'autre machine la récupère, avec chaque version précédente conservée au cas où quelque chose tourne mal.

## Ce que Steam Cloud fait déjà sur le Deck

Si un jeu gère Steam Cloud, Steam envoie la sauvegarde quand vous quittez et la télécharge quand vous lancez le jeu sur une autre machine. La page du magasin indique si un jeu en dispose, et vous pouvez la désactiver jeu par jeu dans **Propriétés → Général**.

Les trous sont toujours les mêmes :

- **Les jeux qui ne l'ont pas.** C'est le choix du développeur, jeu par jeu, et beaucoup de jeux PC ne l'ont jamais activé.
- **Tout ce qui est hors de Steam.** Heroic, Lutris, les émulateurs, un jeu installé à la main.
- **Aucun retour en arrière.** Steam garde la sauvegarde actuelle, pas un historique. Si une sauvegarde abîmée se synchronise, la bonne disparaît sur les deux machines.

Plus de détails dans le guide [alternative à Steam Cloud](/guides/steam-cloud-alternative).

## Où le Deck range vos sauvegardes

Le Deck lance les jeux Windows avec Proton, donc le même jeu ne sauvegarde pas au même endroit que sur votre PC :

- **Jeux Windows (Proton) :** `~/.local/share/Steam/steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/`, suivi du chemin Windows habituel : `Documents`, `AppData/Roaming`, `AppData/Local`, `AppData/LocalLow` ou `Saved Games`. L'AppID est le nombre dans l'URL du jeu sur le magasin.
- **Jeux sur la carte microSD :** la carte a son propre `steamapps/compatdata/<AppID>`, avec la même arborescence dedans.
- **Jeux Linux natifs :** en général `~/.local/share/<jeu>` ou `~/.config/<jeu>`. Les jeux Unity utilisent `~/.config/unity3d/<Société>/<Jeu>`.
- **Heroic, Lutris et Bottles :** chacun garde un préfixe Wine par jeu, avec l'arborescence Windows sous `drive_c/users/<votre utilisateur>/` au lieu de `steamuser`.
- **Émulateurs :** EmuDeck les regroupe sous `~/Emulation/saves/`. Voir [sauvegardes d'émulateurs](/guides/back-up-emulator-saves) et [RetroArch](/guides/retroarch-save-sync).

Sur votre PC, le même jeu écrit dans `C:\Users\<vous>\...`. Deux chemins différents pour une seule sauvegarde : voilà pourquoi copier des dossiers à la main tourne mal. Hoard cherche à tous ces endroits et rattache ce qu'il trouve au bon jeu, si bien que la sauvegarde du Deck et celle du PC deviennent deux versions d'un même historique.

## Mise en place

1. Sur le Deck, passez en mode Bureau : **bouton Steam → Alimentation → Passer au bureau**.
2. Ouvrez un navigateur, allez sur [la page de téléchargement](/download) et récupérez **Hoard Setup** pour Linux. Dans le gestionnaire de fichiers, ouvrez les propriétés du fichier, autorisez son exécution comme programme, puis lancez-le.
3. Connectez-vous avec le compte que vous utilisez sur votre PC, ou pointez l'application vers votre propre serveur.
4. Ouvrez la **Bibliothèque** et vérifiez ce que Hoard a trouvé. Ajoutez ce qui manque en indiquant son dossier : un préfixe Heroic, un émulateur, un jeu installé vous-même.
5. Installez Hoard sur votre PC avec le même compte. Les mêmes jeux s'associent tout seuls.
6. Revenez en mode Jeu. Inutile de repasser en mode Bureau.

Hoard Setup place l'application dans votre dossier personnel et le moteur de synchronisation dans un service d'arrière-plan qui démarre avec le Deck. Rien n'est écrit dans le système en lecture seule de SteamOS, donc les mises à jour du système n'y touchent pas.

## À quoi ressemble une journée normale

Vous jouez sur le PC le soir et vous quittez. Hoard attend que le jeu soit fermé et que la sauvegarde ne bouge plus, puis l'envoie. Le lendemain matin, vous prenez le Deck. Dès qu'il est en ligne, Hoard voit la version plus récente et l'écrit dans le préfixe Proton. Vous lancez le jeu et vous continuez. Quand vous quittez sur le Deck, la même chose se produit dans l'autre sens.

Aucune des deux machines n'a besoin d'être allumée en même temps que l'autre. La sauvegarde attend sur le serveur jusqu'à ce que l'autre la demande.

## Les pièges à connaître

### Mettre en veille n'est pas quitter

Le Deck rend très facile d'appuyer sur le bouton d'alimentation et de partir en laissant le jeu ouvert. Hoard ne sauvegarde une partie qu'une fois le jeu fermé, car un jeu en cours peut être en train de l'écrire. Et il ne remplace jamais la sauvegarde d'un jeu en cours. Si vous mettez le Deck en veille puis jouez sur le PC, la progression du Deck n'est pas encore envoyée, et la nouvelle sauvegarde du PC attend que vous fermiez le jeu sur le Deck.

L'habitude qui évite tout cela : **quittez le jeu avant de changer de machine.** Si Proton laisse un processus mort derrière lui après la fermeture, ce qui arrive souvent, Hoard comprend que le jeu n'est plus là et continue.

### Laissez-lui quelques secondes au réveil

Quand le Deck sort de veille, le Wi-Fi met un instant à revenir, et c'est seulement ensuite que Hoard peut chercher une sauvegarde plus récente. Lancez un jeu pendant ces premières secondes et le téléchargement attendra que vous le fermiez. Laissez-lui un moment en ligne avant de jouer.

### La carte microSD

Si un jeu est sur la carte et que la carte n'est pas insérée, Hoard ne télécharge pas de sauvegarde vers un dossier qui n'existe pas. Il attend le retour de la carte.

### Les réglages restent sur chaque machine

Le Deck tourne en 1280×800 sur un GPU de console portable. Votre PC fixe, probablement pas. Hoard sauvegarde les fichiers de réglages comme `graphics.ini` avec la partie, mais ne les écrit pas par-dessus ceux de l'autre machine, donc le Deck garde les siens. Si vous voulez quand même les copier, une option le permet lors de la restauration. Plus d'infos dans [synchroniser ses sauvegardes entre plusieurs PC](/guides/sync-game-saves-across-pcs).

### Le dossier `remote` de Steam

Pour les jeux Steam, la sauvegarde se trouve dans `userdata/<UserID>/<AppID>/remote/`. Le dossier au-dessus contient aussi `remotecache.vdf` et des fichiers de temps de jeu et de succès qui doivent être différents entre le Deck et le PC. Synchronisez le dossier parent à la main et chaque lancement ressemblera à un conflit. Hoard ne suit que `remote/`.

## Steam Cloud et Hoard ensemble

Ils ne se gênent pas. Pour un jeu avec Steam Cloud, laissez Steam continuer à synchroniser. Ce que Hoard ajoute là, c'est l'historique des versions, pour qu'une sauvegarde abîmée sur une machine n'emporte pas votre progression. Pour tous les autres jeux, Hoard assure aussi la synchronisation.

## Sans nos serveurs

Si vous préférez garder vos sauvegardes chez vous, lancez `hoard-server` sur votre PC ou un NAS et pointez le Deck et le PC dessus. Pas de compte chez nous, pas de télémétrie vers nous, rien ne passe par nos serveurs. Voir [comment auto-héberger Hoard](/guides/self-host-hoard).

<!-- faq -->

## Questions fréquentes

### Hoard fonctionne-t-il en mode Jeu ?

Oui. Le moteur de synchronisation est un service d'arrière-plan qui démarre avec le Deck, il sauvegarde et restaure donc sans aucune fenêtre ouverte. Le mode Bureau ne sert qu'à l'installer et à ajouter des dossiers à la main.

### Une mise à jour de SteamOS va-t-elle le supprimer ?

Non. Tout ce que Hoard installe se trouve dans votre dossier personnel, auquel les mises à jour de SteamOS ne touchent pas.

### Synchronise-t-il les jeux de Heroic, Lutris ou EmuDeck ?

Oui. Hoard regarde dans les préfixes de Heroic, Lutris et Bottles et dans les dossiers d'EmuDeck. Si un jeu n'est pas détecté, indiquez une fois son dossier de sauvegarde et il sera suivi comme les autres.

### Et si j'ai joué sur les deux sans synchroniser ?

Hoard n'écrase jamais à l'aveugle. Il compare les versions, garde une copie de ce qu'il remplace, et chaque version précédente reste dans l'historique. Il ne peut pas fusionner deux sessions de jeu différentes en une seule sauvegarde (rien ne le peut), mais vous pouvez choisir laquelle garder.

### Le Deck compte-t-il comme un appareil ?

Oui. L'offre gratuite couvre trois appareils, donc un PC, un portable et un Deck y tiennent. Pro et les serveurs auto-hébergés n'ont pas de limite d'appareils.

### Puis-je plutôt utiliser la version en ligne de commande sur le Deck ?

Oui. La commande `hoard` lance le même moteur sans fenêtre, ce que certains préfèrent sur une console portable. Voir [la page CLI](/cli).
