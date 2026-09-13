var qe=Object.defineProperty;var ze=(a,e,n)=>e in a?qe(a,e,{enumerable:!0,configurable:!0,writable:!0,value:n}):a[e]=n;var g=(a,e,n)=>ze(a,typeof e!="symbol"?e+"":e,n);import{L as we,D as He}from"./B3cuW3tw.js";const Ce=`---
title: "So sicherst und synchronisierst du Emulator-Spielstände (RetroArch, Dolphin, PCSX2)"
description: "Sichere und synchronisiere deine Emulator-Speicherdateien und Savestates über mehrere PCs — RetroArch, Dolphin, PCSX2, DuckStation und mehr — automatisch mit Hoard."
order: 6
updated: 2026-09-01
---

Emulator-Stände gehen leicht verloren: Speicherdateien und Savestates liegen in verstreuten Ordnern, und eine Neuinstallation oder ein neuer PC kann Jahre an Fortschritt löschen. Hoard sichert sie automatisch und hält sie über mehrere Geräte synchron.

## Emulatoren, mit denen Hoard funktioniert

Hoard verarbeitet gängige Emulator-Speicherdateien (\`.srm\`, \`.sav\`, Memory Cards) und Savestates der beliebten Emulatoren, darunter:

- **RetroArch** — Stände und Savestates pro Core
- **Dolphin** (GameCube / Wii) — Memory Cards und GCI-Dateien
- **PCSX2** (PS2) — Memory Cards
- **DuckStation** (PS1), **PPSSPP** (PSP), **mGBA** und mehr

Da Hoard Speicherordner mit derselben Community-Datenbank findet, die auch Ludusavi antreibt, werden viele Emulator-Pfade automatisch erkannt. Für alles Eigene kannst du Hoard von Hand auf einen Ordner verweisen.

## Emulator-Backups einrichten

1. **Installiere Hoard** für Windows, macOS oder Linux und melde dich an.
2. Öffne die **Bibliothek** und füge deinen Emulator hinzu, oder ergänze seinen Stände-/Savestate-Ordner manuell, falls du den Standardort geändert hast.
3. Lass den **Automatikmodus** an. Hoard sichert nach jeder Sitzung und führt eine versionierte Historie.
4. Installiere Hoard mit demselben Konto auf deinen anderen PCs, um diese Stände überall zu synchronisieren — siehe [Spielstände über PCs synchronisieren](/guides/sync-game-saves-across-pcs).

## Ludusavi für Emulatoren?

Ludusavi kann Emulator-Stände ebenfalls lokal sichern und ist dafür eine großartige kostenlose Option. Wenn diese Emulator-Stände zusätzlich automatisch zwischen Geräten synchronisieren und eine Cloud-Versionshistorie behalten sollen, ohne Rclone zu konfigurieren, hilft Hoard — lies den vollständigen [Vergleich Ludusavi vs. Hoard](/guides/ludusavi-alternative).

## Wo die einzelnen Emulatoren ihre Stände ablegen

Nützlich zu wissen, denn eine portable Installation legt all das ganz woanders ab:

- **RetroArch** — \`saves/\` und \`states/\` im Konfigurationsordner: \`%APPDATA%\\RetroArch\` unter Windows, \`~/.config/retroarch\` unter Linux.
- **Dolphin** — Memory Cards unter \`GC/\`, Wii-Stände im emulierten NAND, in \`Dokumente\\Dolphin Emulator\` oder \`~/.local/share/dolphin-emu\`.
- **PCSX2** — \`memcards/\`, unter \`Dokumente\\PCSX2\` oder \`~/.config/PCSX2\`.
- **DuckStation** — \`memcards/\` und \`savestates/\` im eigenen Datenordner.
- **PPSSPP** — \`PSP/SAVEDATA\` für Stände, \`PSP/PPSSPP_STATE\` für Savestates.
- **RPCS3** — \`dev_hdd0/home/00000001/savedata\`.
- **Cemu** — \`mlc01/usr/save\`.
- **mGBA und die meisten eigenständigen Cores** — eine \`.sav\` neben der ROM, sofern nicht anders eingestellt.

Eine **portable Installation** — auf Handhelds und USB-Sticks der Normalfall — legt all das stattdessen neben die ausführbare Datei. Wenn das dein Setup ist, richte Hoard auf diesen Ordner, und er wird wie jeder andere Spielstand verfolgt.

## Spielstand und Savestate sind nicht dasselbe

Die Unterscheidung lohnt sich, denn beim Umzug verhalten sie sich verschieden:

- Ein **Spielstand** (\`.srm\`, eine Memory Card, ein \`SAVEDATA\`-Ordner) ist der eigene Stand des Spiels, geschrieben von der emulierten Konsole. Er wandert klaglos zwischen Rechnern und Emulatorversionen.
- Ein **Savestate** ist ein Abbild des Emulatorspeichers. Er hängt an genau diesem Build und oft am exakten Core, ein Savestate der einen Version kann sich in einer anderen also weigern zu laden.

Hoard sichert beides. Wundere dich nur nicht, wenn ein Savestate von einer aktualisierten Maschine auf einer veralteten nicht aufgeht: halte die Emulatorversionen gleich und verlass dich für Wichtiges auf Spielstände.

## Ein Emulator, viele Spiele

Ein Emulator ist ein einzelner Prozess, der Dutzende Titel beherbergt — genau das macht Emulator-Stände schwierig für ein Werkzeug, das in "dem laufenden Spiel" denkt. Hoard hält die Titel auseinander, statt den ganzen Emulator als einen Klumpen zu behandeln, sodass jedes Spiel seine eigene Historie bekommt und nicht einen gemeinsamen Haufen, der sich bei jedem Start von irgendetwas ändert.

## Emulator-Stände ohne unsere Server

All das funktioniert genauso gegen deinen eigenen Server: \`hoard-server\` betreiben, die App darauf richten, und deine Stände gehen von deiner Maschine auf deine Platte. Kein Konto bei uns, keine Telemetrie zu uns, nichts über unsere Server. Siehe [wie du Hoard selbst hostest](/guides/self-host-hoard).

## Tipp

Savestates sind an eine bestimmte Emulator-Version gebunden. Halte deine Emulatoren über alle PCs hinweg einheitlich aktuell, damit ein synchronisierter Savestate überall sauber lädt.

<!-- faq -->

## Häufige Fragen

### Sichert Hoard auch meine ROMs?

Nein. Es verfolgt Speicherordner, keine Spieldateien. ROMs sind groß, sie ändern sich nicht, und du hast sie bereits — da gibt es nichts zu versionieren.

### Mein Emulator ist eine portable Installation. Geht das?

Ja. Füge den Ordner neben der ausführbaren Datei von Hand hinzu, dann verfolgt Hoard ihn wie jeden anderen Speicherort. Auf Handhelds ist das der Normalfall.

### Kann ich Savestates zwischen zwei PCs synchronisieren?

Kannst du, und Hoard tut es. Ob ein Savestate lädt, hängt davon ab, dass die Emulatoren auf beiden Maschinen dieselbe Version haben — eine Grenze des Emulators, nicht der Synchronisierung. Spielstände haben das Problem nicht.

### Klappt es mit einem Emulator, der nicht auf der Liste steht?

Ziemlich sicher ja. Die gängigen werden automatisch erkannt, alles andere fügst du hinzu, indem du Hoard auf seinen Speicherordner richtest.

### Ändert Selbsthosten etwas für Emulatoren?

Nein. Gleiche Erkennung, gleiche Versionen, gleiche Synchronisierung. Nur der Speicher gehört dir.
`,Pe=`---
title: "How to back up and sync emulator saves (RetroArch, Dolphin, PCSX2)"
description: "Back up and sync your emulator save files and save states across PCs — RetroArch, Dolphin, PCSX2, DuckStation and more — automatically with Hoard."
order: 6
updated: 2026-09-01
---

Emulator saves are easy to lose: save files and save states live in scattered folders, and a reinstall or a new PC can wipe years of progress. Hoard backs them up automatically and keeps them in sync across machines.

## Emulators Hoard works with

Hoard handles standard emulator save files (\`.srm\`, \`.sav\`, memory cards) and save states for the popular emulators, including:

- **RetroArch** — per-core saves and states
- **Dolphin** (GameCube / Wii) — memory cards and GCI files
- **PCSX2** (PS2) — memory cards
- **DuckStation** (PS1), **PPSSPP** (PSP), **mGBA**, and more

Because Hoard locates save folders using the same community database that powers Ludusavi, many emulator paths are detected automatically. For anything custom, you can point Hoard at a folder by hand.

## Set up emulator save backups

1. **Install Hoard** for Windows, macOS or Linux and sign in.
2. Open the **Library** and add your emulator, or add its saves/states folder manually if you've changed the default location.
3. Keep **automatic mode** on. Hoard backs up after each session and keeps a versioned history.
4. Install Hoard on your other PCs with the same account to sync those saves everywhere — see [syncing saves across PCs](/guides/sync-game-saves-across-pcs).

## Ludusavi for emulators?

Ludusavi can back up emulator saves locally too, and it's a great free option for that. If you also want those emulator saves to sync automatically between machines and keep a cloud version history without configuring Rclone, that's where Hoard helps — read the full [Ludusavi vs Hoard comparison](/guides/ludusavi-alternative).

## Where each emulator keeps its saves

Useful to know, because a portable install puts all of this somewhere else entirely:

- **RetroArch** — \`saves/\` and \`states/\` under the config folder: \`%APPDATA%\\RetroArch\` on Windows, \`~/.config/retroarch\` on Linux.
- **Dolphin** — memory cards under \`GC/\`, Wii saves in the emulated NAND, inside \`Documents\\Dolphin Emulator\` or \`~/.local/share/dolphin-emu\`.
- **PCSX2** — \`memcards/\`, under \`Documents\\PCSX2\` or \`~/.config/PCSX2\`.
- **DuckStation** — \`memcards/\` and \`savestates/\` in its own data folder.
- **PPSSPP** — \`PSP/SAVEDATA\` for saves and \`PSP/PPSSPP_STATE\` for states.
- **RPCS3** — \`dev_hdd0/home/00000001/savedata\`.
- **Cemu** — \`mlc01/usr/save\`.
- **mGBA and most standalone cores** — a \`.sav\` next to the ROM, unless you told them otherwise.

A **portable install** — the norm on handhelds and USB sticks — keeps every one of those next to the executable instead. If that's your setup, point Hoard at that folder and it tracks it like any other save.

## Save files and save states are not the same thing

Worth separating, because they behave differently when they travel:

- A **save file** (\`.srm\`, a memory card, a \`SAVEDATA\` folder) is the game's own save, written by the emulated console. It moves between machines and between emulator versions without complaint.
- A **save state** is a dump of emulator memory. It's tied to the emulator build, and often to the exact core, so a state written by one version may refuse to load in another.

Hoard backs up both. Just don't be surprised when a state from an updated machine won't open on a stale one — keep your emulators on matching versions, and lean on save files for anything you care about.

## One emulator, many games

An emulator is a single process hosting dozens of titles, which is what makes emulator saves awkward for a tool that thinks in terms of "the running game". Hoard keeps the titles apart rather than treating the whole emulator as one blob, so each game gets its own history instead of a single pile that changes every time you launch anything.

## Emulator saves without our servers

Everything here works the same against your own server: run \`hoard-server\`, point the app at it, and your saves go from your machine to your disk. No account with us, no telemetry to us, nothing through our servers. See [how to self-host Hoard](/guides/self-host-hoard).

## Tip

Save states are tied to a specific emulator version. Keep your emulators updated consistently across PCs so a synced state loads cleanly everywhere.

<!-- faq -->

## Frequently asked questions

### Does Hoard back up my ROMs too?

No. It tracks save folders, not game files. ROMs are large, they don't change, and you already have them — there's nothing to version.

### My emulator is a portable install. Does that work?

Yes. Add the folder next to the executable by hand and Hoard tracks it like any other save location. This is the usual setup on handhelds.

### Can I sync save states between two PCs?

You can, and Hoard will. Whether a state loads depends on the emulators being the same version on both machines, which is an emulator limitation rather than a sync one. Save files don't have that problem.

### Will it work with an emulator that isn't on the list?

Almost certainly. Detection covers the common ones automatically, and anything else you can add by pointing Hoard at its saves folder.

### Does self-hosting change anything for emulators?

No. Same detection, same versions, same sync. Only the storage is yours.
`,De=`---
title: "Cómo hacer copia y sincronizar partidas de emuladores (RetroArch, Dolphin, PCSX2)"
description: "Haz copia y sincroniza los archivos de guardado y los estados guardados de tus emuladores entre PC —RetroArch, Dolphin, PCSX2, DuckStation y más— automáticamente con Hoard."
order: 6
updated: 2026-09-01
---

Las partidas de emulador se pierden con facilidad: los archivos de guardado y los estados guardados viven en carpetas dispersas, y una reinstalación o un PC nuevo pueden borrar años de progreso. Hoard hace la copia automáticamente y los mantiene sincronizados entre equipos.

## Emuladores con los que funciona Hoard

Hoard gestiona los archivos de guardado estándar de emulador (\`.srm\`, \`.sav\`, memory cards) y los estados guardados de los emuladores populares, entre ellos:

- **RetroArch** — guardados y estados por núcleo
- **Dolphin** (GameCube / Wii) — memory cards y archivos GCI
- **PCSX2** (PS2) — memory cards
- **DuckStation** (PS1), **PPSSPP** (PSP), **mGBA** y más

Como Hoard localiza las carpetas de guardado con la misma base de datos comunitaria que utiliza Ludusavi, muchas rutas de emulador se detectan automáticamente. Para cualquier ruta personalizada, puedes apuntar Hoard a una carpeta a mano.

## Configura la copia de partidas de emulador

1. **Instala Hoard** para Windows, macOS o Linux e inicia sesión.
2. Abre la **Biblioteca** y añade tu emulador, o añade manualmente su carpeta de guardados/estados si has cambiado la ubicación por defecto.
3. Mantén el **modo automático** activado. Hoard hace la copia tras cada sesión y guarda un historial versionado.
4. Instala Hoard en tus otros PC con la misma cuenta para sincronizar esas partidas en todas partes; mira [cómo sincronizar partidas entre PC](/guides/sync-game-saves-across-pcs).

## ¿Ludusavi para emuladores?

Ludusavi también puede hacer copia de partidas de emulador en local, y es una gran opción gratuita para eso. Si además quieres que esas partidas de emulador se sincronicen automáticamente entre equipos y mantengan un historial de versiones en la nube sin configurar Rclone, ahí es donde ayuda Hoard; lee la [comparativa completa entre Ludusavi y Hoard](/guides/ludusavi-alternative).

## Dónde guarda sus partidas cada emulador

Conviene saberlo, porque una instalación portable lo coloca todo en otro sitio:

- **RetroArch** — \`saves/\` y \`states/\` dentro de la carpeta de configuración: \`%APPDATA%\\RetroArch\` en Windows, \`~/.config/retroarch\` en Linux.
- **Dolphin** — las memory cards en \`GC/\` y las partidas de Wii en la NAND emulada, dentro de \`Documentos\\Dolphin Emulator\` o \`~/.local/share/dolphin-emu\`.
- **PCSX2** — \`memcards/\`, bajo \`Documentos\\PCSX2\` o \`~/.config/PCSX2\`.
- **DuckStation** — \`memcards/\` y \`savestates/\` en su propia carpeta de datos.
- **PPSSPP** — \`PSP/SAVEDATA\` para las partidas y \`PSP/PPSSPP_STATE\` para los estados.
- **RPCS3** — \`dev_hdd0/home/00000001/savedata\`.
- **Cemu** — \`mlc01/usr/save\`.
- **mGBA y la mayoría de núcleos sueltos** — un \`.sav\` junto a la ROM, salvo que les hayas dicho otra cosa.

Una **instalación portable** — lo normal en consolas de mano y en llaves USB — guarda todo eso junto al ejecutable. Si ése es tu caso, apunta Hoard a esa carpeta y la rastreará como cualquier otra partida.

## Partida guardada y estado guardado no son lo mismo

Vale la pena separarlos, porque se comportan distinto cuando viajan:

- Una **partida guardada** (\`.srm\`, una memory card, una carpeta \`SAVEDATA\`) es el guardado propio del juego, escrito por la consola emulada. Se mueve entre máquinas y entre versiones del emulador sin protestar.
- Un **estado guardado** es un volcado de la memoria del emulador. Está atado a esa compilación, y a menudo al núcleo exacto, así que un estado escrito por una versión puede negarse a cargar en otra.

Hoard copia los dos. Sólo que no te sorprenda que un estado de una máquina actualizada no abra en una que se quedó atrás: mantén los emuladores en versiones iguales y apóyate en las partidas guardadas para lo que te importe.

## Un emulador, muchos juegos

Un emulador es un solo proceso que aloja decenas de títulos, y eso es lo que vuelve incómodas las partidas de emulador para una herramienta que piensa en términos de «el juego que está corriendo». Hoard mantiene los títulos separados en lugar de tratar el emulador entero como un único bulto, así que cada juego tiene su propio historial y no un montón común que cambia cada vez que abres cualquier cosa.

## Partidas de emulador sin pasar por nuestros servidores

Todo esto funciona igual contra tu propio servidor: levanta \`hoard-server\`, apunta la aplicación ahí, y tus partidas van de tu máquina a tu disco. Sin cuenta con nosotros, sin telemetría hacia nosotros, nada por nuestros servidores. Mira [cómo autoalojar Hoard](/guides/self-host-hoard).

## Consejo

Los estados guardados dependen de una versión concreta del emulador. Mantén tus emuladores actualizados de forma coherente entre PC para que un estado sincronizado cargue bien en todas partes.

<!-- faq -->

## Preguntas frecuentes

### ¿Hoard copia también mis ROMs?

No. Rastrea carpetas de partidas, no ficheros de juego. Las ROMs son grandes, no cambian y ya las tienes: no hay nada que versionar.

### Mi emulador es portable. ¿Funciona igual?

Sí. Añade a mano la carpeta que está junto al ejecutable y Hoard la rastreará como cualquier otra ubicación de partidas. Es el montaje habitual en consolas de mano.

### ¿Puedo sincronizar estados guardados entre dos PC?

Puedes, y Hoard lo hará. Que un estado cargue depende de que los emuladores estén en la misma versión en las dos máquinas, y eso es una limitación del emulador, no de la sincronización. Las partidas guardadas no tienen ese problema.

### ¿Funcionará con un emulador que no está en la lista?

Casi seguro que sí. La detección cubre los habituales de forma automática, y cualquier otro lo añades apuntando Hoard a su carpeta de partidas.

### ¿Cambia algo con emuladores si me autoalojo?

No. La misma detección, las mismas versiones, la misma sincronización. Lo único tuyo es el almacenamiento.
`,Le=`---
title: "Comment sauvegarder et synchroniser les sauvegardes d'émulateur (RetroArch, Dolphin, PCSX2)"
description: "Sauvegardez et synchronisez vos fichiers de sauvegarde et vos save states d'émulateur entre PC — RetroArch, Dolphin, PCSX2, DuckStation et plus — automatiquement avec Hoard."
order: 6
updated: 2026-09-01
---

Les sauvegardes d'émulateur se perdent facilement : fichiers de sauvegarde et save states vivent dans des dossiers éparpillés, et une réinstallation ou un nouveau PC peut effacer des années de progression. Hoard les sauvegarde automatiquement et les garde synchronisées entre machines.

## Émulateurs pris en charge par Hoard

Hoard gère les fichiers de sauvegarde d'émulateur courants (\`.srm\`, \`.sav\`, cartes mémoire) et les save states des émulateurs populaires, dont :

- **RetroArch** — sauvegardes et états par cœur
- **Dolphin** (GameCube / Wii) — cartes mémoire et fichiers GCI
- **PCSX2** (PS2) — cartes mémoire
- **DuckStation** (PS1), **PPSSPP** (PSP), **mGBA**, et plus

Comme Hoard localise les dossiers de sauvegarde avec la même base communautaire que celle qui alimente Ludusavi, de nombreux chemins d'émulateur sont détectés automatiquement. Pour tout cas particulier, vous pouvez pointer Hoard vers un dossier à la main.

## Configurer les sauvegardes d'émulateur

1. **Installez Hoard** pour Windows, macOS ou Linux et connectez-vous.
2. Ouvrez la **Bibliothèque** et ajoutez votre émulateur, ou ajoutez son dossier de sauvegardes/états manuellement si vous avez changé l'emplacement par défaut.
3. Gardez le **mode automatique** activé. Hoard sauvegarde après chaque session et conserve un historique versionné.
4. Installez Hoard sur vos autres PC avec le même compte pour synchroniser ces sauvegardes partout — voir [synchroniser vos parties entre PC](/guides/sync-game-saves-across-pcs).

## Ludusavi pour les émulateurs ?

Ludusavi peut aussi sauvegarder les parties d'émulateur en local, et c'est une excellente option gratuite pour cela. Si vous voulez en plus que ces sauvegardes d'émulateur se synchronisent automatiquement entre machines et conservent un historique de versions cloud sans configurer Rclone, c'est là que Hoard aide — lisez la [comparaison complète Ludusavi vs Hoard](/guides/ludusavi-alternative).

## Où chaque émulateur range ses sauvegardes

Bon à savoir, car une installation portable place tout cela ailleurs :

- **RetroArch** — \`saves/\` et \`states/\` dans le dossier de configuration : \`%APPDATA%\\RetroArch\` sous Windows, \`~/.config/retroarch\` sous Linux.
- **Dolphin** — cartes mémoire sous \`GC/\`, sauvegardes Wii dans la NAND émulée, dans \`Documents\\Dolphin Emulator\` ou \`~/.local/share/dolphin-emu\`.
- **PCSX2** — \`memcards/\`, sous \`Documents\\PCSX2\` ou \`~/.config/PCSX2\`.
- **DuckStation** — \`memcards/\` et \`savestates/\` dans son propre dossier de données.
- **PPSSPP** — \`PSP/SAVEDATA\` pour les sauvegardes et \`PSP/PPSSPP_STATE\` pour les états.
- **RPCS3** — \`dev_hdd0/home/00000001/savedata\`.
- **Cemu** — \`mlc01/usr/save\`.
- **mGBA et la plupart des cores autonomes** — un \`.sav\` à côté de la ROM, sauf indication contraire.

Une **installation portable** — la norme sur les consoles portables et les clés USB — range tout cela à côté de l'exécutable. Si c'est votre cas, pointez Hoard sur ce dossier et il le suivra comme n'importe quelle sauvegarde.

## Sauvegarde et état sauvegardé, ce n'est pas pareil

La distinction compte, car les deux ne voyagent pas de la même façon :

- Une **sauvegarde** (\`.srm\`, une carte mémoire, un dossier \`SAVEDATA\`) est la sauvegarde propre du jeu, écrite par la console émulée. Elle passe d'une machine à l'autre et d'une version d'émulateur à l'autre sans broncher.
- Un **état sauvegardé** est un vidage de la mémoire de l'émulateur. Il est lié à cette version précise, et souvent au core exact : un état écrit par une version peut refuser de se charger dans une autre.

Hoard sauvegarde les deux. Ne soyez simplement pas surpris qu'un état venu d'une machine à jour n'ouvre pas sur une machine restée en arrière : gardez des versions identiques et appuyez-vous sur les sauvegardes classiques pour ce qui compte.

## Un émulateur, beaucoup de jeux

Un émulateur est un seul processus qui héberge des dizaines de titres, et c'est ce qui rend les sauvegardes d'émulateur pénibles pour un outil qui raisonne en « le jeu qui tourne ». Hoard sépare les titres au lieu de traiter l'émulateur entier comme un bloc : chaque jeu a son propre historique, et non un tas commun qui change dès que vous lancez quoi que ce soit.

## Sauvegardes d'émulateur sans passer par nos serveurs

Tout ceci fonctionne à l'identique face à votre propre serveur : lancez \`hoard-server\`, pointez l'application dessus, et vos sauvegardes vont de votre machine à votre disque. Aucun compte chez nous, aucune télémétrie vers nous, rien qui passe par nos serveurs. Voir [comment auto-héberger Hoard](/guides/self-host-hoard).

## Astuce

Les save states sont liés à une version précise de l'émulateur. Gardez vos émulateurs à jour de façon cohérente sur tous vos PC pour qu'un état synchronisé se charge correctement partout.

<!-- faq -->

## Questions fréquentes

### Hoard sauvegarde-t-il aussi mes ROMs ?

Non. Il suit les dossiers de sauvegarde, pas les fichiers de jeu. Les ROMs sont volumineuses, elles ne changent pas, et vous les avez déjà : il n'y a rien à versionner.

### Mon émulateur est en installation portable. Ça marche ?

Oui. Ajoutez à la main le dossier situé à côté de l'exécutable et Hoard le suivra comme n'importe quel emplacement de sauvegarde. C'est le montage habituel sur les consoles portables.

### Puis-je synchroniser des états sauvegardés entre deux PC ?

Oui, et Hoard le fera. Qu'un état se charge dépend de l'identité des versions d'émulateur sur les deux machines : c'est une limite de l'émulateur, pas de la synchro. Les sauvegardes classiques n'ont pas ce souci.

### Est-ce que ça marchera avec un émulateur absent de la liste ?

Presque certainement. Les plus courants sont détectés automatiquement, et pour le reste il suffit de pointer Hoard sur son dossier de sauvegardes.

### L'auto-hébergement change-t-il quelque chose pour les émulateurs ?

Non. Même détection, mêmes versions, même synchro. Seul le stockage est à vous.
`,xe=`---
title: "Come fare il backup e sincronizzare i salvataggi degli emulatori (RetroArch, Dolphin, PCSX2)"
description: "Fai il backup e sincronizza i file di salvataggio e i save state dei tuoi emulatori tra PC — RetroArch, Dolphin, PCSX2, DuckStation e altri — automaticamente con Hoard."
order: 6
updated: 2026-09-01
---

I salvataggi degli emulatori si perdono facilmente: file di salvataggio e save state vivono in cartelle sparse, e una reinstallazione o un nuovo PC possono cancellare anni di progressi. Hoard ne fa il backup automaticamente e li mantiene sincronizzati tra le macchine.

## Emulatori con cui funziona Hoard

Hoard gestisce i file di salvataggio standard degli emulatori (\`.srm\`, \`.sav\`, memory card) e i save state degli emulatori popolari, tra cui:

- **RetroArch** — salvataggi e stati per core
- **Dolphin** (GameCube / Wii) — memory card e file GCI
- **PCSX2** (PS2) — memory card
- **DuckStation** (PS1), **PPSSPP** (PSP), **mGBA** e altri

Poiché Hoard individua le cartelle di salvataggio con lo stesso database comunitario che alimenta Ludusavi, molti percorsi degli emulatori vengono rilevati automaticamente. Per qualsiasi caso personalizzato, puoi puntare Hoard a una cartella a mano.

## Imposta i backup dei salvataggi degli emulatori

1. **Installa Hoard** per Windows, macOS o Linux e accedi.
2. Apri la **Libreria** e aggiungi il tuo emulatore, oppure aggiungi manualmente la sua cartella di salvataggi/stati se hai cambiato la posizione predefinita.
3. Tieni attiva la **modalità automatica**. Hoard fa il backup dopo ogni sessione e conserva una cronologia versionata.
4. Installa Hoard sugli altri PC con lo stesso account per sincronizzare quei salvataggi ovunque — vedi [sincronizzare i salvataggi tra PC](/guides/sync-game-saves-across-pcs).

## Ludusavi per gli emulatori?

Ludusavi può fare il backup dei salvataggi degli emulatori anche in locale, ed è un'ottima opzione gratuita per questo. Se vuoi anche che quei salvataggi degli emulatori si sincronizzino automaticamente tra le macchine e mantengano una cronologia versioni nel cloud senza configurare Rclone, è qui che Hoard aiuta — leggi il [confronto completo Ludusavi vs Hoard](/guides/ludusavi-alternative).

## Dove ogni emulatore tiene i salvataggi

Utile saperlo, perché un'installazione portable mette tutto questo altrove:

- **RetroArch** — \`saves/\` e \`states/\` nella cartella di configurazione: \`%APPDATA%\\RetroArch\` su Windows, \`~/.config/retroarch\` su Linux.
- **Dolphin** — memory card in \`GC/\`, salvataggi Wii nella NAND emulata, dentro \`Documenti\\Dolphin Emulator\` o \`~/.local/share/dolphin-emu\`.
- **PCSX2** — \`memcards/\`, sotto \`Documenti\\PCSX2\` o \`~/.config/PCSX2\`.
- **DuckStation** — \`memcards/\` e \`savestates/\` nella sua cartella dati.
- **PPSSPP** — \`PSP/SAVEDATA\` per i salvataggi e \`PSP/PPSSPP_STATE\` per gli stati.
- **RPCS3** — \`dev_hdd0/home/00000001/savedata\`.
- **Cemu** — \`mlc01/usr/save\`.
- **mGBA e la maggior parte dei core autonomi** — un \`.sav\` accanto alla ROM, se non gli hai detto diversamente.

Un'**installazione portable** — la norma su console portatili e chiavette USB — tiene tutto questo accanto all'eseguibile. Se è il tuo caso, punta Hoard a quella cartella e la traccerà come qualsiasi altro salvataggio.

## Salvataggio e save state non sono la stessa cosa

Vale la pena distinguerli, perché viaggiano in modo diverso:

- Un **salvataggio** (\`.srm\`, una memory card, una cartella \`SAVEDATA\`) è il salvataggio proprio del gioco, scritto dalla console emulata. Passa da una macchina all'altra e tra versioni di emulatore senza protestare.
- Un **save state** è un dump della memoria dell'emulatore. È legato a quella build, e spesso al core esatto, quindi uno stato scritto da una versione può rifiutarsi di caricare in un'altra.

Hoard salva entrambi. Solo non stupirti se uno stato da una macchina aggiornata non si apre su una rimasta indietro: tieni gli emulatori alla stessa versione e affidati ai salvataggi veri per ciò a cui tieni.

## Un emulatore, tanti giochi

Un emulatore è un solo processo che ospita decine di titoli, ed è questo a rendere scomodi i salvataggi degli emulatori per uno strumento che ragiona per "il gioco in esecuzione". Hoard tiene separati i titoli invece di trattare l'intero emulatore come un unico blocco, così ogni gioco ha la sua cronologia e non un mucchio comune che cambia ogni volta che avvii qualcosa.

## Salvataggi di emulatore senza passare dai nostri server

Tutto questo funziona allo stesso modo contro il tuo server: fai girare \`hoard-server\`, punta l'app lì, e i salvataggi vanno dalla tua macchina al tuo disco. Nessun account con noi, nessuna telemetria verso di noi, niente attraverso i nostri server. Vedi [come ospitare Hoard da solo](/guides/self-host-hoard).

## Suggerimento

I save state sono legati a una versione specifica dell'emulatore. Mantieni i tuoi emulatori aggiornati in modo coerente su tutti i PC così che uno stato sincronizzato si carichi senza problemi ovunque.

<!-- faq -->

## Domande frequenti

### Hoard salva anche le mie ROM?

No. Traccia le cartelle dei salvataggi, non i file di gioco. Le ROM sono grandi, non cambiano e le hai già: non c'è niente da versionare.

### Il mio emulatore è portable. Funziona lo stesso?

Sì. Aggiungi a mano la cartella accanto all'eseguibile e Hoard la traccerà come qualsiasi altra posizione di salvataggio. È il setup abituale sulle console portatili.

### Posso sincronizzare i save state tra due PC?

Puoi, e Hoard lo farà. Che uno stato si carichi dipende dal fatto che gli emulatori siano alla stessa versione su entrambe le macchine: è un limite dell'emulatore, non della sincronizzazione. I salvataggi veri non hanno questo problema.

### Funzionerà con un emulatore che non è in elenco?

Quasi certamente sì. Il rilevamento copre automaticamente quelli comuni, e qualsiasi altro lo aggiungi puntando Hoard alla sua cartella dei salvataggi.

### Il self-hosting cambia qualcosa per gli emulatori?

No. Stesso rilevamento, stesse versioni, stessa sincronizzazione. L'unica cosa tua è lo spazio di archiviazione.
`,Ae=`---
title: "エミュレーターのセーブをバックアップ・同期する方法（RetroArch、Dolphin、PCSX2）"
description: "エミュレーターのセーブファイルとセーブステートを PC 間でバックアップ・同期。RetroArch、Dolphin、PCSX2、DuckStation などに対応し、Hoard が自動で処理します。"
order: 6
updated: 2026-09-01
---

エミュレーターのセーブは失われやすいものです。セーブファイルやセーブステートは散らばったフォルダーに置かれ、再インストールや新しい PC で何年もの進行が消えることがあります。Hoard はそれらを自動でバックアップし、マシン間で同期し続けます。

## Hoard が対応するエミュレーター

Hoard は一般的なエミュレーターのセーブファイル（\`.srm\`、\`.sav\`、メモリーカード）と、人気エミュレーターのセーブステートを扱います。たとえば：

- **RetroArch** — コアごとのセーブとステート
- **Dolphin**（GameCube / Wii）— メモリーカードと GCI ファイル
- **PCSX2**（PS2）— メモリーカード
- **DuckStation**（PS1）、**PPSSPP**（PSP）、**mGBA** など

Hoard は Ludusavi を支えているのと同じコミュニティデータベースでセーブフォルダーを特定するため、多くのエミュレーターのパスが自動で検出されます。独自の場所については、Hoard を手動でフォルダーに向けられます。

## エミュレーターのセーブバックアップを設定する

1. Windows、macOS、Linux 向けの **Hoard をインストール** し、サインインします。
2. **ライブラリ** を開いてエミュレーターを追加します。既定の場所を変更している場合は、セーブ／ステートのフォルダーを手動で追加します。
3. **自動モード** をオンのままにします。Hoard は各セッション後にバックアップし、世代履歴を保持します。
4. 同じアカウントでほかの PC にも Hoard をインストールすると、それらのセーブをどこでも同期できます――[PC 間でセーブを同期する方法](/guides/sync-game-saves-across-pcs) をご覧ください。

## エミュレーターに Ludusavi？

Ludusavi もエミュレーターのセーブをローカルにバックアップでき、そのための無料の優れた選択肢です。さらに、それらのエミュレーターのセーブをマシン間で自動同期し、Rclone を設定せずにクラウドの世代履歴を保ちたいなら、そこで Hoard が役立ちます――[Ludusavi と Hoard の完全な比較](/guides/ludusavi-alternative) をお読みください。

## エミュレーターごとのセーブの置き場所

知っておくと役に立ちます。ポータブル構成にすると、これらがまるごと別の場所へ移るからです。

- **RetroArch** — 設定フォルダーの下の \`saves/\` と \`states/\`。Windows は \`%APPDATA%\\RetroArch\`、Linux は \`~/.config/retroarch\`。
- **Dolphin** — メモリーカードは \`GC/\`、Wii のセーブはエミュレートされた NAND の中。\`ドキュメント\\Dolphin Emulator\` または \`~/.local/share/dolphin-emu\` の下です。
- **PCSX2** — \`memcards/\`。\`ドキュメント\\PCSX2\` か \`~/.config/PCSX2\` の下。
- **DuckStation** — 自身のデータフォルダー内の \`memcards/\` と \`savestates/\`。
- **PPSSPP** — セーブは \`PSP/SAVEDATA\`、ステートは \`PSP/PPSSPP_STATE\`。
- **RPCS3** — \`dev_hdd0/home/00000001/savedata\`。
- **Cemu** — \`mlc01/usr/save\`。
- **mGBA と多くの単体コア** — 設定を変えていなければ、ROM の隣の \`.sav\`。

**ポータブル構成**（携帯機や USB メモリーでは普通の形）では、これらがすべて実行ファイルの隣に置かれます。その構成なら、そのフォルダーを Hoard に指定すれば、他のセーブと同じように追跡されます。

## セーブデータとセーブステートは別物

移動したときの振る舞いが違うので、分けて考える価値があります。

- **セーブデータ**（\`.srm\`、メモリーカード、\`SAVEDATA\` フォルダー）は、エミュレートされたゲーム機が書いた、ゲーム自身のセーブです。マシン間でも、エミュレーターのバージョン間でも問題なく移せます。
- **セーブステート**はエミュレーターのメモリーのダンプです。そのビルドに、しばしば使用中のコアにまで結びついているため、あるバージョンで作ったステートが別のバージョンでは読み込めないことがあります。

Hoard は両方をバックアップします。ただ、更新済みのマシンで作ったステートが古いマシンで開かなくても驚かないでください。エミュレーターのバージョンを揃え、大事なものはセーブデータのほうを頼りにするのが確実です。

## エミュレーターは 1 つ、ゲームは多数

エミュレーターは数十本のタイトルを抱える単一のプロセスです。「いま動いているゲーム」を基準に考えるツールにとって、エミュレーターのセーブが扱いにくいのはこれが理由です。Hoard はエミュレーター全体をひとかたまりとして扱わず、タイトルごとに分けて扱います。だから各ゲームがそれぞれの履歴を持ち、何かを起動するたびに変化する共通の山にはなりません。

## 当方のサーバーを介さないエミュレーターのバックアップ

ここに書いたことは、自分のサーバーに対してもまったく同じように動きます。\`hoard-server\` を動かし、アプリをそこに向ければ、セーブは自分のマシンから自分のディスクへ移ります。当方のアカウントも、当方へのテレメトリも、当方のサーバーを通るものもありません。[Hoard をセルフホストする方法](/guides/self-host-hoard) を参照してください。

## ヒント

セーブステートは特定のエミュレーターのバージョンに結び付いています。同期したステートがどこでも問題なく読み込まれるよう、すべての PC でエミュレーターのバージョンを揃えて更新しておきましょう。

<!-- faq -->

## よくある質問

### ROM もバックアップされますか？

いいえ。追跡するのはセーブのフォルダーであって、ゲームのファイルではありません。ROM は容量が大きく、変化せず、すでに手元にあります。世代管理する対象がありません。

### エミュレーターがポータブル構成でも使えますか？

はい。実行ファイルの隣にあるフォルダーを手動で追加すれば、他のセーブ場所と同じように追跡されます。携帯機ではこれが普通の構成です。

### セーブステートを 2 台の PC で同期できますか？

できますし、Hoard は同期します。ただしステートが読み込めるかどうかは、両方のマシンでエミュレーターのバージョンが揃っているかによります。これは同期ではなくエミュレーター側の制約です。セーブデータにはこの問題がありません。

### 一覧にないエミュレーターでも動きますか？

ほぼ確実に動きます。よく使われるものは自動的に検出され、それ以外もセーブのフォルダーを指定すれば追加できます。

### セルフホストするとエミュレーター周りは変わりますか？

いいえ。同じ検出、同じ世代、同じ同期です。自分のものになるのは保存先だけです。
`,je=`---
title: "Como fazer backup e sincronizar saves de emuladores (RetroArch, Dolphin, PCSX2)"
description: "Faz backup e sincroniza os ficheiros de save e os save states dos teus emuladores entre PCs — RetroArch, Dolphin, PCSX2, DuckStation e mais — automaticamente com o Hoard."
order: 6
updated: 2026-09-01
---

Os saves de emulador perdem-se com facilidade: ficheiros de save e save states vivem em pastas espalhadas, e uma reinstalação ou um PC novo podem apagar anos de progresso. O Hoard faz-lhes backup automaticamente e mantém-nos sincronizados entre máquinas.

## Emuladores com que o Hoard funciona

O Hoard trata os ficheiros de save padrão de emulador (\`.srm\`, \`.sav\`, memory cards) e os save states dos emuladores populares, incluindo:

- **RetroArch** — saves e estados por core
- **Dolphin** (GameCube / Wii) — memory cards e ficheiros GCI
- **PCSX2** (PS2) — memory cards
- **DuckStation** (PS1), **PPSSPP** (PSP), **mGBA** e mais

Como o Hoard localiza as pastas de save com a mesma base de dados comunitária que alimenta o Ludusavi, muitos caminhos de emulador são detetados automaticamente. Para qualquer caso personalizado, podes apontar o Hoard para uma pasta à mão.

## Configurar backups de saves de emulador

1. **Instala o Hoard** para Windows, macOS ou Linux e inicia sessão.
2. Abre a **Biblioteca** e adiciona o teu emulador, ou adiciona manualmente a sua pasta de saves/estados se mudaste a localização predefinida.
3. Mantém o **modo automático** ligado. O Hoard faz backup depois de cada sessão e guarda um histórico versionado.
4. Instala o Hoard nos teus outros PCs com a mesma conta para sincronizar esses saves em todo o lado — vê [sincronizar saves entre PCs](/guides/sync-game-saves-across-pcs).

## Ludusavi para emuladores?

O Ludusavi também pode fazer backup de saves de emulador localmente, e é uma excelente opção gratuita para isso. Se queres, além disso, que esses saves de emulador sincronizem automaticamente entre máquinas e mantenham um histórico de versões na nuvem sem configurar o Rclone, é aí que o Hoard ajuda — lê a [comparação completa Ludusavi vs Hoard](/guides/ludusavi-alternative).

## Onde cada emulador guarda os saves

Vale a pena saber, porque uma instalação portable põe tudo isto noutro sítio:

- **RetroArch** — \`saves/\` e \`states/\` dentro da pasta de configuração: \`%APPDATA%\\RetroArch\` no Windows, \`~/.config/retroarch\` no Linux.
- **Dolphin** — memory cards em \`GC/\`, saves de Wii na NAND emulada, dentro de \`Documentos\\Dolphin Emulator\` ou \`~/.local/share/dolphin-emu\`.
- **PCSX2** — \`memcards/\`, em \`Documentos\\PCSX2\` ou \`~/.config/PCSX2\`.
- **DuckStation** — \`memcards/\` e \`savestates/\` na sua própria pasta de dados.
- **PPSSPP** — \`PSP/SAVEDATA\` para os saves e \`PSP/PPSSPP_STATE\` para os estados.
- **RPCS3** — \`dev_hdd0/home/00000001/savedata\`.
- **Cemu** — \`mlc01/usr/save\`.
- **mGBA e a maioria dos cores autónomos** — um \`.sav\` ao lado da ROM, salvo se lhes disseste outra coisa.

Uma **instalação portable** — o normal em consolas portáteis e pens USB — guarda tudo isso ao lado do executável. Se é o teu caso, aponta o Hoard para essa pasta e ele segue-a como qualquer outro save.

## Save e save state não são a mesma coisa

Vale a pena separá-los, porque viajam de maneira diferente:

- Um **save** (\`.srm\`, um memory card, uma pasta \`SAVEDATA\`) é o guardado próprio do jogo, escrito pela consola emulada. Passa de máquina para máquina e entre versões de emulador sem se queixar.
- Um **save state** é um despejo da memória do emulador. Está preso àquela build, e muitas vezes ao core exato, por isso um estado escrito por uma versão pode recusar-se a carregar noutra.

O Hoard copia os dois. Só não estranhes que um estado de uma máquina atualizada não abra numa que ficou para trás: mantém os emuladores na mesma versão e apoia-te nos saves normais para o que te importa.

## Um emulador, muitos jogos

Um emulador é um único processo a alojar dezenas de títulos, e é isso que torna os saves de emulador incómodos para uma ferramenta que pensa em "o jogo que está a correr". O Hoard mantém os títulos separados em vez de tratar o emulador inteiro como um bloco só, por isso cada jogo tem o seu histórico e não um monte comum que muda sempre que abres seja o que for.

## Saves de emulador sem passar pelos nossos servidores

Tudo isto funciona igual contra o teu próprio servidor: corre o \`hoard-server\`, aponta a aplicação para lá, e os teus saves vão da tua máquina para o teu disco. Sem conta connosco, sem telemetria para nós, nada pelos nossos servidores. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard).

## Dica

Os save states estão ligados a uma versão específica do emulador. Mantém os teus emuladores atualizados de forma coerente em todos os PCs para que um estado sincronizado carregue bem em todo o lado.

<!-- faq -->

## Perguntas frequentes

### O Hoard também copia as minhas ROMs?

Não. Segue pastas de saves, não ficheiros de jogo. As ROMs são grandes, não mudam e já as tens: não há nada para versionar.

### O meu emulador é portable. Funciona?

Sim. Adiciona à mão a pasta ao lado do executável e o Hoard segue-a como qualquer outra localização de saves. É a montagem habitual nas consolas portáteis.

### Posso sincronizar save states entre dois PCs?

Podes, e o Hoard fá-lo. Que um estado carregue depende de os emuladores estarem na mesma versão nas duas máquinas: é uma limitação do emulador, não da sincronização. Os saves normais não têm esse problema.

### Funciona com um emulador que não está na lista?

Quase de certeza. A deteção cobre os comuns automaticamente, e qualquer outro adicionas apontando o Hoard à sua pasta de saves.

### O self-hosting muda alguma coisa para emuladores?

Não. A mesma deteção, as mesmas versões, a mesma sincronização. Só o armazenamento é teu.
`,Oe=`---
title: "如何备份和同步模拟器存档（RetroArch、Dolphin、PCSX2）"
description: "用 Hoard 在多台 PC 之间自动备份和同步你的模拟器存档文件与即时存档——支持 RetroArch、Dolphin、PCSX2、DuckStation 等。"
order: 6
updated: 2026-09-01
---

模拟器存档很容易丢失：存档文件和即时存档散落在各处的文件夹里，一次重装或换一台新 PC 就可能清除多年的进度。Hoard 会自动备份它们，并在多台机器之间保持同步。

## Hoard 支持的模拟器

Hoard 可处理常见的模拟器存档文件（\`.srm\`、\`.sav\`、记忆卡）以及主流模拟器的即时存档，包括：

- **RetroArch** —— 按核心区分的存档和即时存档
- **Dolphin**（GameCube / Wii）—— 记忆卡和 GCI 文件
- **PCSX2**（PS2）—— 记忆卡
- **DuckStation**（PS1）、**PPSSPP**（PSP）、**mGBA** 等

由于 Hoard 使用与 Ludusavi 相同的社区数据库来定位存档文件夹，许多模拟器路径都会被自动检测。对于任何自定义位置，你都可以手动把 Hoard 指向某个文件夹。

## 设置模拟器存档备份

1. **安装 Hoard**（Windows、macOS 或 Linux）并登录。
2. 打开**库**并添加你的模拟器；如果你更改了默认位置，请手动添加它的存档／即时存档文件夹。
3. 保持**自动模式**开启。Hoard 会在每次会话后备份，并保留版本历史。
4. 用同一账号在你的其他 PC 上安装 Hoard，即可在任何地方同步这些存档——请见[在多台 PC 之间同步存档](/guides/sync-game-saves-across-pcs)。

## 模拟器用 Ludusavi？

Ludusavi 同样可以在本地备份模拟器存档，对此它是一个很好的免费选择。如果你还希望这些模拟器存档在多台机器之间自动同步，并在不配置 Rclone 的情况下保留云端版本历史，那就是 Hoard 能帮上忙的地方——请阅读完整的 [Ludusavi 与 Hoard 对比](/guides/ludusavi-alternative)。

## 各个模拟器把存档放在哪里

值得知道，因为便携版安装会把这些统统换个地方：

- **RetroArch** — 配置目录下的 \`saves/\` 和 \`states/\`：Windows 是 \`%APPDATA%\\RetroArch\`，Linux 是 \`~/.config/retroarch\`。
- **Dolphin** — 记忆卡在 \`GC/\`，Wii 存档在模拟的 NAND 里，位于 \`文档\\Dolphin Emulator\` 或 \`~/.local/share/dolphin-emu\`。
- **PCSX2** — \`memcards/\`，在 \`文档\\PCSX2\` 或 \`~/.config/PCSX2\` 下。
- **DuckStation** — 自身数据目录里的 \`memcards/\` 和 \`savestates/\`。
- **PPSSPP** — 存档在 \`PSP/SAVEDATA\`，即时存档在 \`PSP/PPSSPP_STATE\`。
- **RPCS3** — \`dev_hdd0/home/00000001/savedata\`。
- **Cemu** — \`mlc01/usr/save\`。
- **mGBA 以及大多数独立核心** — 除非你另行设置，否则是 ROM 旁边的 \`.sav\`。

**便携版安装**——掌机和 U 盘上的常态——会把上面这些全部放在可执行文件旁边。如果你是这种情况，把 Hoard 指向那个文件夹，它就会像追踪其他存档一样追踪它。

## 存档文件和即时存档不是一回事

值得区分，因为它们"搬家"时的表现不同：

- **存档文件**（\`.srm\`、记忆卡、\`SAVEDATA\` 目录）是游戏自己的存档，由被模拟的主机写入。它在不同机器之间、不同模拟器版本之间都能顺利迁移。
- **即时存档**是模拟器内存的转储。它绑定在那个构建上，往往还绑定具体的核心，因此某个版本写出的即时存档，换一个版本可能拒绝加载。

Hoard 两者都会备份。只是别惊讶：从已更新的机器带过来的即时存档，在版本落后的机器上可能打不开。让各台机器的模拟器保持同一版本，重要的东西以存档文件为准。

## 一个模拟器，许多游戏

模拟器是一个进程，却承载着几十款游戏，这正是模拟器存档对"以正在运行的游戏为单位"的工具来说很棘手的原因。Hoard 会把游戏彼此分开，而不是把整个模拟器当成一个整体，因此每款游戏都有自己的历史，而不是一个每次启动任何东西都会变化的大杂烩。

## 不经过我们服务器的模拟器备份

这里的一切在你自己的服务器上完全一样：运行 \`hoard-server\`，把应用指向它，你的存档就从你的机器走到你的磁盘。没有我们这边的账号，没有发往我们的遥测，也没有任何东西经过我们的服务器。参见[如何自托管 Hoard](/guides/self-host-hoard)。

## 提示

即时存档与特定的模拟器版本绑定。请在所有 PC 上保持模拟器版本一致地更新，这样同步过来的即时存档才能在各处正常加载。

<!-- faq -->

## 常见问题

### Hoard 也会备份我的 ROM 吗？

不会。它追踪的是存档目录，不是游戏文件。ROM 体积大、不会变化，而且你已经有了——没有什么需要做版本管理。

### 我的模拟器是便携版，能用吗？

能。手动把可执行文件旁边的那个文件夹添加进来，Hoard 就会像对待其他存档位置一样追踪它。掌机上这是常见做法。

### 可以在两台 PC 之间同步即时存档吗？

可以，Hoard 会同步。但即时存档能否加载，取决于两台机器上的模拟器版本是否一致——这是模拟器的限制，不是同步的限制。存档文件没有这个问题。

### 列表之外的模拟器能用吗？

几乎肯定可以。常见的会被自动检测，其他的只要把 Hoard 指向它的存档目录即可。

### 自托管对模拟器有影响吗？

没有。同样的检测、同样的版本、同样的同步。只有存储归你所有。
`,Ge=`---
title: "So sicherst du deine Spielstände automatisch"
description: "Richte automatische, versionierte Cloud-Backups für deine PC-Spielstände mit Hoard ein — damit ein Absturz, eine Neuinstallation oder ein fehlerhafter Mod deinen Fortschritt nie löschen kann."
order: 1
updated: 2026-09-01
---

Ein verlorener Spielstand bedeutet verlorene Stunden an Fortschritt. Hoard sichert deine PC-Spielstände automatisch und führt eine vollständige Versionshistorie, sodass du immer zurückgehen kannst.

## Was Hoard sichert

Hoard erkennt die Speicherordner der Spiele, die du spielst, und kopiert sie in deine eigene Cloud — entweder Hoard Cloud oder einen selbst gehosteten Server. Jedes Backup ist versioniert, ältere Kopien werden also nie überschrieben.

Um zu finden, wo jedes Spiel seine Stände ablegt, nutzt Hoard dieselbe Community-Datenbank für Speicherorte, die auch Ludusavi antreibt — die Erkennung funktioniert also sofort für Tausende von Titeln. Der Unterschied liegt darin, was danach passiert: Statt das Backup auf deiner Festplatte zu belassen, versioniert Hoard es automatisch in der Cloud.

## Automatische Backups einrichten

1. **Lade Hoard herunter und installiere es** für Windows, macOS oder Linux von der Download-Seite.
2. Melde dich an oder richte die App auf deinen selbst gehosteten Server aus.
3. Öffne die **Bibliothek**. Hoard sucht nach installierten Spielen und listet die gefundenen Stände auf.
4. Füge die Spiele hinzu, die du schützen willst. Hoard findet jeden Speicherordner automatisch; du kannst einen Pfad von Hand ergänzen, falls ein Spiel nicht erkannt wird.
5. Lass den **Automatikmodus** an. Hoard überwacht die Speicherordner und sichert sie, nachdem du aufhörst zu spielen.

Ab jetzt wird jede Sitzung erfasst, ohne dass du etwas tun musst.

## Wo PC-Spiele ihre Stände wirklich ablegen

Es gibt keinen einzigen Ort, und genau deshalb existiert so ein Werkzeug. In der Praxis landet ein Spielstand an einer dieser Stellen:

- **In Steam**, unter \`userdata/<UserID>/<AppID>/remote/\` — dem Ordner, den Steam Cloud selbst synchronisiert.
- **\`Dokumente\\My Games\\…\`**, das Nächste, was Windows an Konvention zu bieten hat.
- **\`%APPDATA%\`, \`%LOCALAPPDATA%\` oder \`LocalLow\`**, wo die meisten Unity- und Unreal-Spiele schreiben.
- **\`%USERPROFILE%\\Saved Games\`**, genutzt von einer kleineren, aber hartnäckigen Gruppe von Titeln.
- **Im Installationsordner des Spiels selbst**, wo erstaunlich viele ältere Titel weiterhin speichern.
- **Unter Linux** \`~/.local/share\` oder \`~/.config\` für native Spiele, und im Proton-Prefix — \`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\` — für Windows-Spiele.
- **Unter macOS** \`~/Library/Application Support\`.

Woher das Spiel stammt, spielt kaum eine Rolle: Titel von GOG, Epic und itch landen an derselben Handvoll Orte, denn das entscheiden Engine und Entwickler, nicht der Store.

## Was gesichert wird und was nicht

Ein Speicherordner enthält selten nur Spielstände, deshalb sortiert Hoard, was es findet, auf drei Stapel:

- **Spielstanddaten** werden gesichert und wiederhergestellt. Das ist dein Fortschritt.
- **Dateien, die zu einem bestimmten Rechner gehören** — Konfiguration, Logs und Ähnliches — werden hochgeladen, damit sie Teil des Backups sind, aber nie über die Kopie eines anderen PCs geschrieben. Deine Grafikeinstellungen bleiben deine.
- **Müll** — Caches, Absturzberichte, temporäre Dateien — wird ignoriert, damit ein Backup nicht mit Dingen aufquillt, die du nie zurückhaben willst.

## Wann gesichert wird

Hoard beobachtet den Ordner und sichert ihn, **nachdem du aufgehört hast zu spielen**, nicht während ein Spiel Dateien offen hält. Wurde der Stand vor Sekunden geschrieben, wartet es, bis Ruhe einkehrt: eine Datei im Schreibvorgang ist keine Datei, die man halb sichern will.

Jede Sicherung ist eine Version. Snapshots werden per Inhalts-Hash gespeichert, unveränderte Dateien also nur einmal — zehn Versionen eines 2 GB großen Stands kosten etwa 2 GB, nicht 20.

## Sichern ohne unsere Server

Wenn du lieber niemandes Cloud nutzt, betreibe \`hoard-server\` selbst und richte die App darauf. Deine Stände gehen von deinem PC auf deine Platte: kein Konto bei uns, keine Telemetrie zu uns, und nichts, was über unsere Server läuft. Siehe [wie du Hoard selbst hostest](/guides/self-host-hoard).

## Tipp: Prüfe deine Historie

Öffne den Reiter **Historie** eines Spiels, um jedes Backup mit Datum und Größe zu sehen. Von dort kannst du jede frühere Version mit einem Klick wiederherstellen. Deine Stände werden verschlüsselt übertragen, in der EU gespeichert, und du kannst sie jederzeit exportieren oder löschen.

Nutzt du bereits ein lokales Backup-Tool wie Ludusavi? Du kannst es behalten — aber wenn diese Backups in der Cloud landen und zwischen Geräten synchronisieren sollen, ohne dass du Rclone selbst einrichtest, ist genau das, was Hoard automatisiert. Siehe [Ludusavi vs. Hoard](/guides/ludusavi-alternative) für einen fairen Vergleich.

<!-- faq -->

## Häufige Fragen

### Sichert Hoard, während ich spiele?

Nein. Es wartet, bis du aufhörst und der Speicherordner zur Ruhe kommt, damit ein Backup nie eine halb geschriebene Datei ist.

### Wie viel Platz brauchen meine Spielstände?

Weniger als gedacht. Versionen werden per Inhalts-Hash dedupliziert, neuen Platz belegt also nur, was sich zwischen zwei Sitzungen wirklich geändert hat — die meisten Sammlungen passen bequem in ein paar Gigabyte.

### Was, wenn eines meiner Spiele nicht erkannt wird?

Richte Hoard von Hand auf den Ordner, dann verfolgt es ihn wie jeden anderen. Die Erkennung deckt Tausende Titel ab, aber ein Spiel, das an einer ungewöhnlichen Stelle speichert oder das du von Hand installiert hast, braucht manchmal den Hinweis.

### Sichert es auch meine Mods?

Hoard verfolgt den Speicherordner, Mods an anderer Stelle sind also nicht Teil des Backups. Das ist Absicht: Mods sind groß, sie lassen sich neu herunterladen, und ein zwischen Rechnern synchronisierter Mod-Ordner schafft mehr Probleme, als er löst.

### Ändert Selbsthosten etwas an den Backups?

Überhaupt nicht. Gleiche Erkennung, gleiche Versionen, gleiche automatische Sicherung. Nur der Speicher gehört dir.
`,Ee=`---
title: "How to back up your game saves automatically"
description: "Set up automatic, versioned cloud backups for your PC game saves with Hoard — so a crash, reinstall or bad mod can never wipe your progress."
order: 1
updated: 2026-09-01
---

Losing a save file means losing hours of progress. Hoard backs up your PC game saves automatically and keeps a full version history, so you can always go back.

## What Hoard backs up

Hoard detects the save folders of the games you play and copies them to your own cloud — either Hoard Cloud or a server you host yourself. Every backup is versioned, so older copies are never overwritten.

To find where each game stores its saves, Hoard reads the same community save-location database that powers Ludusavi, so detection works out of the box for thousands of titles. The difference is what happens next: instead of leaving the backup on your disk, Hoard versions it in the cloud automatically.

## Set up automatic backups

1. **Download and install Hoard** for Windows, macOS or Linux from the download page.
2. Sign in, or point the app at your self-hosted server.
3. Open the **Library**. Hoard scans for installed games and lists the saves it finds.
4. Add the games you want to protect. Hoard locates each save folder automatically; you can add a path by hand if a game isn't detected.
5. Leave **automatic mode** on. Hoard watches the save folders and backs them up after you stop playing.

From now on every session is captured without you doing anything.

## Where PC games actually keep their saves

There is no single place, which is the whole reason a tool like this exists. In practice a save ends up in one of these:

- **Inside Steam**, at \`userdata/<UserID>/<AppID>/remote/\` — the folder Steam Cloud itself syncs.
- **\`Documents\\My Games\\…\`**, the closest thing Windows has to a convention.
- **\`%APPDATA%\`, \`%LOCALAPPDATA%\` or \`LocalLow\`** — where most Unity and Unreal games write.
- **\`%USERPROFILE%\\Saved Games\`**, used by a smaller but stubborn set of titles.
- **The game's own install folder**, which is where a surprising number of older titles still save.
- **On Linux**, \`~/.local/share\` or \`~/.config\` for native games, and inside the Proton prefix — \`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\` — for Windows games.
- **On macOS**, \`~/Library/Application Support\`.

Where the game came from barely matters: GOG, Epic and itch titles land in the same handful of places, because it's the engine and the developer that decide, not the launcher.

## What gets backed up, and what doesn't

A save folder is rarely just saves, so Hoard sorts what it finds into three piles:

- **Save data** is backed up and restored. This is your progress.
- **Files that belong to one machine** — configuration, logs, and similar — are uploaded so they're part of the backup, but never written back over another PC's copy. Your graphics settings stay yours.
- **Junk** — caches, crash dumps, temporary files — is ignored, so a backup doesn't balloon with things you'd never want back.

## When a backup happens

Hoard watches the folder and captures it **after you stop playing**, not while a game is holding files open. If the save was written to seconds ago, it waits until things go quiet: a file being written is not a file worth capturing halfway.

Each capture is a version. Snapshots are stored by content hash, so unchanged files are stored once — ten versions of a 2 GB save cost about 2 GB, not 20.

## Backing up without our servers

If you'd rather not use anyone's cloud, run \`hoard-server\` yourself and point the app at it. Your saves go from your PC to your disk: no account with us, no telemetry to us, and nothing passing through our servers. See [how to self-host Hoard](/guides/self-host-hoard).

## Tip: check your history

Open a game's **History** tab to see every backup with its date and size. From there you can restore any previous version in one click. Your saves travel encrypted, are stored in the EU, and you can export or delete them whenever you want.

Already use a local backup tool like Ludusavi? You can keep it — but if you want those backups to land in the cloud and sync between machines without scripting Rclone yourself, that's exactly what Hoard automates. See [Ludusavi vs Hoard](/guides/ludusavi-alternative) for a fair comparison.

<!-- faq -->

## Frequently asked questions

### Does Hoard back up while I'm playing?

No. It waits until you stop and the save folder goes quiet, so a backup is never a half-written file.

### How much space do my saves need?

Less than you'd think. Versions are deduplicated by content hash, so only what actually changed between sessions takes new space — most save collections sit comfortably in a couple of gigabytes.

### What if one of my games isn't detected?

Point Hoard at the folder by hand and it will track it like any other. Detection covers thousands of titles, but a game that saves somewhere unusual, or one you installed by hand, sometimes needs the hint.

### Does it back up my mods?

Hoard tracks the save folder, so mods living elsewhere aren't part of the backup. That's deliberate: mods are large, they're re-downloadable, and a mod folder syncing between machines causes more problems than it solves.

### Does self-hosting change how backups work?

Not at all. Same detection, same versions, same automatic capture. Only the storage is yours.
`,Ie=`---
title: "Cómo hacer copias de seguridad de tus partidas automáticamente"
description: "Configura copias de seguridad automáticas y versionadas en la nube de tus partidas de PC con Hoard, para que un fallo, una reinstalación o un mod problemático nunca borren tu progreso."
order: 1
updated: 2026-09-01
---

Perder una partida guardada significa perder horas de progreso. Hoard hace copias de seguridad de tus partidas de PC automáticamente y guarda un historial completo de versiones, para que siempre puedas volver atrás.

## Qué guarda Hoard

Hoard detecta las carpetas de guardado de los juegos a los que juegas y las copia a tu propia nube: Hoard Cloud o un servidor que alojes tú mismo. Cada copia está versionada, así que las versiones antiguas nunca se sobrescriben.

Para saber dónde guarda cada juego sus partidas, Hoard usa la misma base de datos comunitaria de ubicaciones que utiliza Ludusavi, así que la detección funciona desde el primer momento con miles de títulos. La diferencia está en lo que pasa después: en vez de dejar la copia en tu disco, Hoard la versiona en la nube automáticamente.

## Configura las copias automáticas

1. **Descarga e instala Hoard** para Windows, macOS o Linux desde la página de descargas.
2. Inicia sesión o apunta la app a tu servidor autoalojado.
3. Abre la **Biblioteca**. Hoard busca los juegos instalados y lista las partidas que encuentra.
4. Añade los juegos que quieras proteger. Hoard localiza cada carpeta de guardado automáticamente; puedes añadir una ruta a mano si un juego no se detecta.
5. Deja activado el **modo automático**. Hoard vigila las carpetas de guardado y hace la copia cuando dejas de jugar.

A partir de ahí cada sesión queda guardada sin que hagas nada.

## Dónde guardan realmente sus partidas los juegos de PC

No hay un único sitio, y ése es justo el motivo de que exista una herramienta así. En la práctica, una partida acaba en alguno de estos lugares:

- **Dentro de Steam**, en \`userdata/<UserID>/<AppID>/remote/\`, la carpeta que sincroniza el propio Steam Cloud.
- **\`Documentos\\My Games\\…\`**, lo más parecido a una convención que tiene Windows.
- **\`%APPDATA%\`, \`%LOCALAPPDATA%\` o \`LocalLow\`**, donde escriben la mayoría de juegos de Unity y Unreal.
- **\`%USERPROFILE%\\Saved Games\`**, que usa un grupo más pequeño pero tozudo de títulos.
- **La propia carpeta de instalación del juego**, donde todavía guardan sorprendentes cantidades de títulos antiguos.
- **En Linux**, \`~/.local/share\` o \`~/.config\` para los juegos nativos, y dentro del prefijo de Proton — \`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\` — para los de Windows.
- **En macOS**, \`~/Library/Application Support\`.

De dónde venga el juego importa poco: los de GOG, Epic e itch caen en el mismo puñado de sitios, porque lo deciden el motor y el desarrollador, no la tienda.

## Qué se copia y qué no

Una carpeta de partidas rara vez contiene sólo partidas, así que Hoard reparte lo que encuentra en tres montones:

- **Los datos de partida** se copian y se restauran. Eso es tu progreso.
- **Los ficheros que son de una máquina concreta** — configuración, registros y similares — se suben para que formen parte de la copia, pero nunca se escriben encima de la copia de otro PC. Tus ajustes gráficos siguen siendo tuyos.
- **La basura** — cachés, volcados de fallos, temporales — se ignora, para que una copia no se hinche con cosas que nunca querrías de vuelta.

## Cuándo se hace la copia

Hoard vigila la carpeta y la captura **cuando dejas de jugar**, no mientras el juego tiene los ficheros abiertos. Si la partida se escribió hace unos segundos, espera a que la cosa se calme: un fichero que se está escribiendo no es un fichero que merezca capturarse a medias.

Cada captura es una versión. Las instantáneas se guardan por hash de contenido, así que un fichero que no cambia se almacena una sola vez: diez versiones de una partida de 2 GB ocupan unos 2 GB, no 20.

## Copias sin pasar por nuestros servidores

Si prefieres no usar la nube de nadie, levanta \`hoard-server\` tú mismo y apunta la aplicación ahí. Tus partidas van de tu PC a tu disco: sin cuenta con nosotros, sin telemetría hacia nosotros y sin nada que pase por nuestros servidores. Mira [cómo autoalojar Hoard](/guides/self-host-hoard).

## Consejo: revisa tu historial

Abre la pestaña **Historial** de un juego para ver cada copia con su fecha y tamaño. Desde ahí puedes restaurar cualquier versión anterior con un clic. Tus partidas viajan cifradas, se almacenan en la UE y puedes exportarlas o borrarlas cuando quieras.

¿Ya usas una herramienta de copia local como Ludusavi? Puedes seguir usándola, pero si quieres que esas copias acaben en la nube y se sincronicen entre equipos sin montar Rclone a mano, eso es justo lo que Hoard automatiza. Mira [Ludusavi frente a Hoard](/guides/ludusavi-alternative) para una comparativa justa.

<!-- faq -->

## Preguntas frecuentes

### ¿Hoard hace copias mientras juego?

No. Espera a que salgas y a que la carpeta de partidas se quede quieta, así que una copia nunca es un fichero a medio escribir.

### ¿Cuánto espacio necesitan mis partidas?

Menos del que imaginas. Las versiones se deduplican por hash de contenido, así que sólo ocupa espacio nuevo lo que cambió de verdad entre sesiones: la mayoría de colecciones caben de sobra en un par de gigas.

### ¿Y si uno de mis juegos no se detecta?

Apunta Hoard a la carpeta a mano y la rastreará como cualquier otra. La detección cubre miles de títulos, pero un juego que guarde en un sitio raro, o que hayas instalado a mano, a veces necesita la pista.

### ¿Copia también mis mods?

Hoard rastrea la carpeta de partidas, así que los mods que vivan en otro sitio no entran en la copia. Es deliberado: los mods son grandes, se vuelven a descargar, y una carpeta de mods sincronizándose entre máquinas da más problemas de los que resuelve.

### ¿Cambia algo si me autoalojo?

Nada. La misma detección, las mismas versiones, la misma captura automática. Lo único tuyo es el almacenamiento.
`,Re=`---
title: "Comment sauvegarder vos parties automatiquement"
description: "Configurez des sauvegardes cloud automatiques et versionnées de vos parties PC avec Hoard — pour qu'un plantage, une réinstallation ou un mod défectueux n'efface jamais votre progression."
order: 1
updated: 2026-09-01
---

Perdre une sauvegarde, c'est perdre des heures de progression. Hoard sauvegarde vos parties PC automatiquement et conserve un historique complet des versions, pour que vous puissiez toujours revenir en arrière.

## Ce que Hoard sauvegarde

Hoard détecte les dossiers de sauvegarde des jeux auxquels vous jouez et les copie vers votre propre cloud — Hoard Cloud ou un serveur que vous hébergez vous-même. Chaque sauvegarde est versionnée, les anciennes copies ne sont donc jamais écrasées.

Pour trouver où chaque jeu range ses sauvegardes, Hoard utilise la même base de données communautaire d'emplacements que celle qui alimente Ludusavi : la détection fonctionne donc d'emblée pour des milliers de titres. La différence, c'est ce qui se passe ensuite : au lieu de laisser la sauvegarde sur votre disque, Hoard la versionne automatiquement dans le cloud.

## Configurer les sauvegardes automatiques

1. **Téléchargez et installez Hoard** pour Windows, macOS ou Linux depuis la page de téléchargement.
2. Connectez-vous, ou pointez l'application vers votre serveur auto-hébergé.
3. Ouvrez la **Bibliothèque**. Hoard recherche les jeux installés et liste les sauvegardes trouvées.
4. Ajoutez les jeux à protéger. Hoard localise chaque dossier de sauvegarde automatiquement ; vous pouvez ajouter un chemin à la main si un jeu n'est pas détecté.
5. Laissez le **mode automatique** activé. Hoard surveille les dossiers de sauvegarde et les sauvegarde après que vous arrêtez de jouer.

Désormais, chaque session est capturée sans que vous ayez à faire quoi que ce soit.

## Où les jeux PC rangent vraiment leurs sauvegardes

Il n'y a pas d'endroit unique, et c'est précisément pour ça qu'un outil comme celui-ci existe. En pratique, une sauvegarde atterrit dans l'un de ces endroits :

- **Dans Steam**, sous \`userdata/<UserID>/<AppID>/remote/\` — le dossier que Steam Cloud synchronise lui-même.
- **\`Documents\\My Games\\…\`**, ce qui se rapproche le plus d'une convention sous Windows.
- **\`%APPDATA%\`, \`%LOCALAPPDATA%\` ou \`LocalLow\`**, où écrivent la plupart des jeux Unity et Unreal.
- **\`%USERPROFILE%\\Saved Games\`**, utilisé par un groupe plus restreint mais tenace de titres.
- **Le dossier d'installation du jeu lui-même**, où un nombre surprenant de titres anciens sauvegardent encore.
- **Sous Linux**, \`~/.local/share\` ou \`~/.config\` pour les jeux natifs, et dans le préfixe Proton — \`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\` — pour les jeux Windows.
- **Sous macOS**, \`~/Library/Application Support\`.

La provenance du jeu ne change presque rien : les titres GOG, Epic et itch atterrissent dans la même poignée d'endroits, car ce sont le moteur et le développeur qui décident, pas la boutique.

## Ce qui est sauvegardé, et ce qui ne l'est pas

Un dossier de sauvegarde ne contient presque jamais que des sauvegardes, alors Hoard trie ce qu'il trouve en trois tas :

- **Les données de sauvegarde** sont sauvegardées et restaurées. C'est votre progression.
- **Les fichiers propres à une machine** — configuration, journaux et compagnie — sont envoyés pour faire partie de la sauvegarde, mais jamais réécrits par-dessus la copie d'un autre PC. Vos réglages graphiques restent les vôtres.
- **Le déchet** — caches, rapports de plantage, fichiers temporaires — est ignoré, pour qu'une sauvegarde n'enfle pas avec ce que vous ne voudriez jamais récupérer.

## Quand la sauvegarde a lieu

Hoard surveille le dossier et le capture **après que vous avez arrêté de jouer**, pas pendant qu'un jeu garde des fichiers ouverts. Si la sauvegarde a été écrite il y a quelques secondes, il attend que le calme revienne : un fichier en cours d'écriture ne mérite pas d'être capturé à moitié.

Chaque capture est une version. Les instantanés sont stockés par empreinte de contenu : un fichier inchangé n'est stocké qu'une fois — dix versions d'une sauvegarde de 2 Go coûtent environ 2 Go, pas 20.

## Sauvegarder sans passer par nos serveurs

Si vous préférez n'utiliser le cloud de personne, faites tourner \`hoard-server\` vous-même et pointez l'application dessus. Vos sauvegardes vont de votre PC à votre disque : aucun compte chez nous, aucune télémétrie vers nous, et rien qui passe par nos serveurs. Voir [comment auto-héberger Hoard](/guides/self-host-hoard).

## Astuce : consultez votre historique

Ouvrez l'onglet **Historique** d'un jeu pour voir chaque sauvegarde avec sa date et sa taille. De là, vous pouvez restaurer n'importe quelle version précédente en un clic. Vos sauvegardes circulent chiffrées, sont stockées dans l'UE, et vous pouvez les exporter ou les supprimer quand vous voulez.

Vous utilisez déjà un outil de sauvegarde locale comme Ludusavi ? Vous pouvez le garder — mais si vous voulez que ces sauvegardes arrivent dans le cloud et se synchronisent entre vos machines sans scripter Rclone vous-même, c'est précisément ce que Hoard automatise. Voir [Ludusavi vs Hoard](/guides/ludusavi-alternative) pour une comparaison équitable.

<!-- faq -->

## Questions fréquentes

### Hoard sauvegarde-t-il pendant que je joue ?

Non. Il attend que vous ayez arrêté et que le dossier se calme, pour qu'une sauvegarde ne soit jamais un fichier à moitié écrit.

### Quelle place prennent mes sauvegardes ?

Moins qu'on ne croit. Les versions sont dédupliquées par empreinte de contenu : seule la partie réellement modifiée entre deux sessions occupe de la place — la plupart des collections tiennent largement dans quelques gigaoctets.

### Et si l'un de mes jeux n'est pas détecté ?

Pointez Hoard sur le dossier à la main et il le suivra comme les autres. La détection couvre des milliers de titres, mais un jeu qui sauvegarde à un endroit inhabituel, ou que vous avez installé à la main, a parfois besoin de l'indice.

### Est-ce qu'il sauvegarde mes mods ?

Hoard suit le dossier de sauvegarde : les mods rangés ailleurs ne font pas partie de la sauvegarde. C'est volontaire — les mods sont volumineux, ils se retéléchargent, et un dossier de mods synchronisé entre machines crée plus de problèmes qu'il n'en résout.

### L'auto-hébergement change-t-il quelque chose aux sauvegardes ?

Rien du tout. Même détection, mêmes versions, même capture automatique. Seul le stockage est à vous.
`,Te=`---
title: "Come fare il backup dei salvataggi automaticamente"
description: "Imposta backup cloud automatici e versionati dei tuoi salvataggi PC con Hoard — così un crash, una reinstallazione o una mod difettosa non potranno mai cancellare i tuoi progressi."
order: 1
updated: 2026-09-01
---

Perdere un salvataggio significa perdere ore di progressi. Hoard fa il backup dei tuoi salvataggi PC automaticamente e conserva una cronologia completa delle versioni, così puoi sempre tornare indietro.

## Cosa salva Hoard

Hoard rileva le cartelle di salvataggio dei giochi a cui giochi e le copia sul tuo cloud — Hoard Cloud o un server che ospiti tu stesso. Ogni backup è versionato, quindi le copie più vecchie non vengono mai sovrascritte.

Per trovare dove ogni gioco conserva i salvataggi, Hoard usa lo stesso database comunitario di posizioni che alimenta Ludusavi, quindi il rilevamento funziona da subito per migliaia di titoli. La differenza è ciò che succede dopo: invece di lasciare il backup sul disco, Hoard lo versiona automaticamente nel cloud.

## Imposta i backup automatici

1. **Scarica e installa Hoard** per Windows, macOS o Linux dalla pagina di download.
2. Accedi, oppure punta l'app al tuo server self-hosted.
3. Apri la **Libreria**. Hoard cerca i giochi installati ed elenca i salvataggi trovati.
4. Aggiungi i giochi che vuoi proteggere. Hoard individua ogni cartella di salvataggio automaticamente; puoi aggiungere un percorso a mano se un gioco non viene rilevato.
5. Lascia attiva la **modalità automatica**. Hoard sorveglia le cartelle di salvataggio e fa il backup dopo che smetti di giocare.

Da ora ogni sessione viene catturata senza che tu faccia nulla.

## Dove i giochi PC tengono davvero i salvataggi

Non esiste un posto solo, ed è esattamente il motivo per cui uno strumento così esiste. Nella pratica un salvataggio finisce in uno di questi punti:

- **Dentro Steam**, in \`userdata/<UserID>/<AppID>/remote/\`, la cartella che Steam Cloud sincronizza per conto suo.
- **\`Documenti\\My Games\\…\`**, la cosa più simile a una convenzione che Windows abbia.
- **\`%APPDATA%\`, \`%LOCALAPPDATA%\` o \`LocalLow\`**, dove scrive la maggior parte dei giochi Unity e Unreal.
- **\`%USERPROFILE%\\Saved Games\`**, usata da un gruppo più ristretto ma testardo di titoli.
- **La cartella di installazione del gioco**, dove sorprendentemente molti titoli vecchi salvano ancora.
- **Su Linux**, \`~/.local/share\` o \`~/.config\` per i giochi nativi, e dentro il prefisso Proton — \`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\` — per quelli Windows.
- **Su macOS**, \`~/Library/Application Support\`.

Da dove arrivi il gioco conta poco: i titoli GOG, Epic e itch finiscono negli stessi pochi posti, perché a decidere sono il motore e lo sviluppatore, non il negozio.

## Cosa viene salvato e cosa no

Una cartella di salvataggi contiene raramente solo salvataggi, quindi Hoard divide ciò che trova in tre mucchi:

- **I dati di salvataggio** vengono copiati e ripristinati. Quelli sono i tuoi progressi.
- **I file che appartengono a una macchina specifica** — configurazione, log e simili — vengono caricati per far parte del backup, ma mai riscritti sopra la copia di un altro PC. Le tue impostazioni grafiche restano tue.
- **La spazzatura** — cache, dump dei crash, temporanei — viene ignorata, così un backup non si gonfia con roba che non rivorresti mai.

## Quando avviene il backup

Hoard sorveglia la cartella e la cattura **dopo che smetti di giocare**, non mentre il gioco tiene i file aperti. Se il salvataggio è stato scritto pochi secondi fa, aspetta che tutto si calmi: un file in scrittura non è un file da catturare a metà.

Ogni cattura è una versione. Gli snapshot sono archiviati per hash del contenuto, quindi un file invariato viene salvato una volta sola: dieci versioni di un salvataggio da 2 GB occupano circa 2 GB, non 20.

## Backup senza passare dai nostri server

Se preferisci non usare il cloud di nessuno, fai girare \`hoard-server\` per conto tuo e punta l'app lì. I salvataggi vanno dal tuo PC al tuo disco: nessun account con noi, nessuna telemetria verso di noi e niente che passi dai nostri server. Vedi [come ospitare Hoard da solo](/guides/self-host-hoard).

## Suggerimento: controlla la cronologia

Apri la scheda **Cronologia** di un gioco per vedere ogni backup con data e dimensione. Da lì puoi ripristinare qualsiasi versione precedente con un clic. I tuoi salvataggi viaggiano cifrati, sono archiviati nell'UE, e puoi esportarli o eliminarli quando vuoi.

Usi già uno strumento di backup locale come Ludusavi? Puoi tenerlo — ma se vuoi che quei backup finiscano nel cloud e si sincronizzino tra le macchine senza scriptare Rclone a mano, è esattamente ciò che Hoard automatizza. Vedi [Ludusavi vs Hoard](/guides/ludusavi-alternative) per un confronto equo.

<!-- faq -->

## Domande frequenti

### Hoard fa backup mentre gioco?

No. Aspetta che tu smetta e che la cartella dei salvataggi si calmi, così un backup non è mai un file scritto a metà.

### Quanto spazio occupano i miei salvataggi?

Meno di quanto pensi. Le versioni sono deduplicate per hash del contenuto, quindi occupa spazio nuovo solo ciò che è davvero cambiato tra una sessione e l'altra: quasi tutte le collezioni stanno comode in un paio di gigabyte.

### E se uno dei miei giochi non viene rilevato?

Punta Hoard alla cartella a mano e la traccerà come qualsiasi altra. Il rilevamento copre migliaia di titoli, ma un gioco che salva in un posto insolito, o installato a mano, a volte ha bisogno dell'indizio.

### Fa il backup anche delle mod?

Hoard traccia la cartella dei salvataggi, quindi le mod che stanno altrove non entrano nel backup. È voluto: le mod sono grandi, si riscaricano, e una cartella di mod sincronizzata tra macchine crea più problemi di quanti ne risolva.

### Il self-hosting cambia il funzionamento dei backup?

Per niente. Stesso rilevamento, stesse versioni, stessa cattura automatica. L'unica cosa tua è lo spazio di archiviazione.
`,_e=`---
title: "ゲームのセーブデータを自動でバックアップする方法"
description: "Hoard で PC ゲームのセーブデータを自動かつ世代管理付きでクラウドにバックアップ。クラッシュ・再インストール・不具合のある MOD でも進行データが消える心配はありません。"
order: 1
updated: 2026-09-01
---

セーブデータを失うことは、何時間もの進行を失うことです。Hoard は PC ゲームのセーブデータを自動でバックアップし、完全なバージョン履歴を保持するので、いつでも巻き戻せます。

## Hoard がバックアップするもの

Hoard はプレイしているゲームのセーブフォルダーを検出し、あなた自身のクラウド（Hoard Cloud または自分でホストするサーバー）へコピーします。各バックアップは世代管理されるため、古いコピーが上書きされることはありません。

各ゲームがどこにセーブを保存しているかを見つけるために、Hoard は Ludusavi を支えているのと同じコミュニティのセーブ位置データベースを利用します。そのため数千タイトルで検出がすぐに機能します。違いはその後にあります。バックアップをディスクに残すのではなく、Hoard は自動的にクラウドで世代管理します。

## 自動バックアップを設定する

1. ダウンロードページから Windows、macOS、Linux 向けの **Hoard をダウンロードしてインストール** します。
2. サインインするか、アプリを自分のセルフホストサーバーに向けます。
3. **ライブラリ** を開きます。Hoard がインストール済みのゲームを探し、見つけたセーブを一覧表示します。
4. 保護したいゲームを追加します。Hoard は各セーブフォルダーを自動で特定します。ゲームが検出されない場合は手動でパスを追加できます。
5. **自動モード** をオンのままにします。Hoard はセーブフォルダーを監視し、プレイを終えた後にバックアップします。

これ以降、何もしなくても毎回のセッションが記録されます。

## PC ゲームのセーブは実際どこに置かれるのか

置き場所は 1 か所に決まっていません。こういうツールが必要になる理由が、まさにそこにあります。実際には次のどこかに落ち着きます。

- **Steam の中**、\`userdata/<UserID>/<AppID>/remote/\`。Steam クラウド自身が同期するフォルダーです。
- **\`ドキュメント\\My Games\\…\`**。Windows にある慣習らしきものの中では、いちばん近いもの。
- **\`%APPDATA%\`、\`%LOCALAPPDATA%\`、\`LocalLow\`**。Unity や Unreal のゲームの多くはここに書きます。
- **\`%USERPROFILE%\\Saved Games\`**。数は少ないものの、頑固に使い続けるタイトル群があります。
- **ゲームのインストールフォルダーそのもの**。古いタイトルには、いまだにここへ保存するものが驚くほどあります。
- **Linux** では、ネイティブのゲームは \`~/.local/share\` か \`~/.config\`、Windows 版のゲームは Proton プレフィックスの中、\`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\`。
- **macOS** では \`~/Library/Application Support\`。

ゲームの入手元はほとんど関係ありません。GOG、Epic、itch のタイトルも同じ数か所に落ち着きます。決めているのはストアではなく、エンジンと開発者だからです。

## 何がバックアップされ、何がされないか

セーブフォルダーの中身がセーブだけということはまずないので、Hoard は見つけたものを 3 つに仕分けます。

- **セーブデータ** はバックアップされ、復元されます。これがあなたの進行です。
- **特定のマシンに属するファイル**、つまり設定やログなどはバックアップに含めるためアップロードされますが、他の PC のコピーを上書きすることはありません。グラフィック設定はそのマシンのものであり続けます。
- **ゴミ**、つまりキャッシュやクラッシュダンプ、一時ファイルは無視されます。二度と要らないもので、バックアップが膨らまないようにするためです。

## バックアップが行われるタイミング

Hoard はフォルダーを監視し、**プレイを終えたあと** に取り込みます。ゲームがファイルを開いている最中には行いません。数秒前にセーブが書かれたばかりなら、落ち着くまで待ちます。書き込み中のファイルは、半端な状態で取り込む価値がないからです。

取り込みのたびに 1 つの世代ができます。スナップショットは内容ハッシュで保存されるため、変わっていないファイルは一度だけ保存されます。2 GB のセーブの 10 世代は約 20 GB ではなく約 2 GB です。

## 当方のサーバーを介さないバックアップ

誰のクラウドも使いたくない場合は、\`hoard-server\` を自分で動かし、アプリをそこに向けてください。セーブは自分の PC から自分のディスクへ移ります。当方のアカウントも、当方へのテレメトリも、当方のサーバーを通るものもありません。[Hoard をセルフホストする方法](/guides/self-host-hoard) を参照してください。

## ヒント：履歴を確認する

ゲームの **履歴** タブを開くと、各バックアップを日付とサイズ付きで確認できます。そこからどの過去バージョンもワンクリックで復元できます。セーブは暗号化されて転送され、EU 内に保存され、いつでもエクスポートや削除が可能です。

すでに Ludusavi のようなローカルバックアップツールを使っていますか？ そのまま使い続けても構いません。ただし、それらのバックアップをクラウドに送り、Rclone を自分でスクリプトせずに端末間で同期したいなら、まさにそれを Hoard が自動化します。公平な比較は [Ludusavi と Hoard](/guides/ludusavi-alternative) をご覧ください。

<!-- faq -->

## よくある質問

### プレイ中もバックアップされますか？

いいえ。プレイを終えてセーブフォルダーが静かになるまで待つので、書き込み途中のファイルがバックアップになることはありません。

### セーブにはどれくらいの容量が必要ですか？

思っているより少なくて済みます。世代は内容ハッシュで重複排除されるため、新たに容量を使うのはセッション間で実際に変わった分だけです。多くの場合、数ギガバイトに余裕で収まります。

### 検出されないゲームがある場合は？

そのフォルダーを手動で指定すれば、他と同じように追跡します。検出は数千タイトルをカバーしますが、変わった場所に保存するゲームや、手動でインストールしたものには、ヒントが要ることがあります。

### Mod もバックアップされますか？

Hoard が追跡するのはセーブフォルダーなので、別の場所にある Mod はバックアップに入りません。これは意図的です。Mod は容量が大きく、再ダウンロードでき、マシン間で同期すると解決するより多くの問題を生むからです。

### セルフホストするとバックアップの動きは変わりますか？

まったく変わりません。同じ検出、同じ世代、同じ自動取り込みです。自分のものになるのは保存先だけです。
`,We=`---
title: "Como fazer backup dos teus saves automaticamente"
description: "Configura backups na nuvem automáticos e versionados dos teus saves de PC com o Hoard — para que uma falha, uma reinstalação ou um mod com problemas nunca apaguem o teu progresso."
order: 1
updated: 2026-09-01
---

Perder um save significa perder horas de progresso. O Hoard faz backup dos teus saves de PC automaticamente e guarda um histórico completo de versões, para que possas sempre voltar atrás.

## O que o Hoard guarda

O Hoard deteta as pastas de save dos jogos a que jogas e copia-as para a tua própria nuvem — Hoard Cloud ou um servidor que alojes tu mesmo. Cada backup é versionado, por isso as cópias antigas nunca são sobrescritas.

Para encontrar onde cada jogo guarda os saves, o Hoard usa a mesma base de dados comunitária de localizações que alimenta o Ludusavi, por isso a deteção funciona logo para milhares de títulos. A diferença está no que acontece a seguir: em vez de deixar o backup no teu disco, o Hoard versiona-o automaticamente na nuvem.

## Configurar backups automáticos

1. **Descarrega e instala o Hoard** para Windows, macOS ou Linux a partir da página de download.
2. Inicia sessão, ou aponta a app para o teu servidor self-hosted.
3. Abre a **Biblioteca**. O Hoard procura jogos instalados e lista os saves que encontra.
4. Adiciona os jogos que queres proteger. O Hoard localiza cada pasta de save automaticamente; podes adicionar um caminho à mão se um jogo não for detetado.
5. Deixa o **modo automático** ligado. O Hoard vigia as pastas de save e faz backup quando paras de jogar.

A partir daí cada sessão é capturada sem que faças nada.

## Onde os jogos de PC guardam mesmo os saves

Não há um sítio único, e é exatamente por isso que uma ferramenta destas existe. Na prática, um save acaba num destes lugares:

- **Dentro da Steam**, em \`userdata/<UserID>/<AppID>/remote/\` — a pasta que a própria Steam Cloud sincroniza.
- **\`Documentos\\My Games\\…\`**, o mais parecido com uma convenção que o Windows tem.
- **\`%APPDATA%\`, \`%LOCALAPPDATA%\` ou \`LocalLow\`**, onde escrevem a maioria dos jogos Unity e Unreal.
- **\`%USERPROFILE%\\Saved Games\`**, usada por um grupo menor mas teimoso de títulos.
- **A própria pasta de instalação do jogo**, onde ainda guardam surpreendentemente muitos títulos antigos.
- **No Linux**, \`~/.local/share\` ou \`~/.config\` para jogos nativos, e dentro do prefixo Proton — \`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\` — para os de Windows.
- **No macOS**, \`~/Library/Application Support\`.

De onde veio o jogo pouco importa: os títulos de GOG, Epic e itch caem no mesmo punhado de sítios, porque quem decide é o motor e o programador, não a loja.

## O que é copiado e o que não é

Uma pasta de saves raramente contém só saves, por isso o Hoard separa o que encontra em três montes:

- **Os dados de save** são copiados e restaurados. Isso é o teu progresso.
- **Os ficheiros que pertencem a uma máquina concreta** — configuração, registos e afins — são enviados para fazerem parte da cópia, mas nunca escritos por cima da cópia de outro PC. As tuas definições gráficas continuam tuas.
- **O lixo** — caches, despejos de erro, temporários — é ignorado, para que uma cópia não inche com coisas que nunca quererias de volta.

## Quando é feita a cópia

O Hoard vigia a pasta e captura-a **depois de parares de jogar**, não enquanto o jogo tem ficheiros abertos. Se o save foi escrito há segundos, espera que as coisas acalmem: um ficheiro a ser escrito não é um ficheiro que valha a pena capturar a meio.

Cada captura é uma versão. Os snapshots são guardados por hash de conteúdo, por isso um ficheiro que não muda é guardado uma só vez: dez versões de um save de 2 GB ocupam cerca de 2 GB, não 20.

## Cópias sem passar pelos nossos servidores

Se preferes não usar a nuvem de ninguém, corre o \`hoard-server\` tu mesmo e aponta a aplicação para lá. Os teus saves vão do teu PC para o teu disco: sem conta connosco, sem telemetria para nós e sem nada a passar pelos nossos servidores. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard).

## Dica: verifica o teu histórico

Abre o separador **Histórico** de um jogo para ver cada backup com data e tamanho. A partir daí podes restaurar qualquer versão anterior com um clique. Os teus saves viajam cifrados, são guardados na UE, e podes exportá-los ou apagá-los quando quiseres.

Já usas uma ferramenta de backup local como o Ludusavi? Podes mantê-la — mas se queres que esses backups cheguem à nuvem e sincronizem entre máquinas sem configurares o Rclone tu mesmo, é exatamente isso que o Hoard automatiza. Vê [Ludusavi vs Hoard](/guides/ludusavi-alternative) para uma comparação justa.

<!-- faq -->

## Perguntas frequentes

### O Hoard faz cópias enquanto jogo?

Não. Espera que saias e que a pasta de saves fique quieta, por isso uma cópia nunca é um ficheiro escrito a meio.

### Quanto espaço ocupam os meus saves?

Menos do que imaginas. As versões são desduplicadas por hash de conteúdo, por isso só ocupa espaço novo aquilo que mudou mesmo entre sessões: a maioria das coleções cabe à vontade em dois gigabytes.

### E se um dos meus jogos não for detetado?

Aponta o Hoard para a pasta à mão e ele segue-a como qualquer outra. A deteção cobre milhares de títulos, mas um jogo que guarde num sítio invulgar, ou que tenhas instalado à mão, às vezes precisa da pista.

### Também copia as minhas mods?

O Hoard segue a pasta de saves, por isso mods que vivam noutro sítio não entram na cópia. É de propósito: as mods são grandes, voltam a descarregar-se, e uma pasta de mods a sincronizar entre máquinas dá mais problemas do que resolve.

### O self-hosting muda a forma como as cópias funcionam?

Nada. A mesma deteção, as mesmas versões, a mesma captura automática. Só o armazenamento é teu.
`,Ne=`---
title: "如何自动备份游戏存档"
description: "用 Hoard 为你的 PC 游戏存档设置自动、带版本的云端备份——这样崩溃、重装或有问题的 MOD 都永远不会清除你的进度。"
order: 1
updated: 2026-09-01
---

丢失一个存档就意味着丢失数小时的进度。Hoard 会自动备份你的 PC 游戏存档，并保留完整的版本历史，让你随时都能回退。

## Hoard 备份什么

Hoard 会检测你所玩游戏的存档文件夹，并把它们复制到你自己的云端——Hoard Cloud 或你自行托管的服务器。每个备份都带版本，因此旧的副本永远不会被覆盖。

为了找到每款游戏把存档保存在哪里，Hoard 使用与 Ludusavi 相同的社区存档位置数据库，因此对成千上万款游戏的检测开箱即用。区别在于之后发生的事：Hoard 不会把备份留在你的磁盘上，而是自动在云端进行版本管理。

## 设置自动备份

1. 从下载页面**下载并安装 Hoard**（Windows、macOS 或 Linux）。
2. 登录，或将应用指向你自行托管的服务器。
3. 打开**库**。Hoard 会扫描已安装的游戏，并列出找到的存档。
4. 添加你想保护的游戏。Hoard 会自动定位每个存档文件夹；如果某款游戏未被检测到，你可以手动添加路径。
5. 保持**自动模式**开启。Hoard 会监视存档文件夹，并在你停止游戏后进行备份。

从此每一次游戏会话都会被记录，你无需做任何事。

## PC 游戏的存档究竟放在哪里

并没有统一的位置，而这正是需要这类工具的原因。实际上，存档通常落在下面某个地方：

- **在 Steam 内部**，位于 \`userdata/<UserID>/<AppID>/remote/\`——也就是 Steam 云存档自己同步的那个文件夹。
- **\`文档\\My Games\\…\`**，这是 Windows 上最接近约定俗成的位置。
- **\`%APPDATA%\`、\`%LOCALAPPDATA%\` 或 \`LocalLow\`**，大多数 Unity 和 Unreal 游戏写在这里。
- **\`%USERPROFILE%\\Saved Games\`**，被数量不多但很执着的一批游戏使用。
- **游戏自己的安装目录**，出人意料的是，相当多的老游戏仍然存在那里。
- **在 Linux 上**，原生游戏用 \`~/.local/share\` 或 \`~/.config\`；Windows 游戏则在 Proton 前缀内：\`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\`。
- **在 macOS 上**，\`~/Library/Application Support\`。

游戏从哪儿买的几乎无关紧要：GOG、Epic 和 itch 的游戏同样落在这几个位置，因为决定权在引擎和开发者手里，不在商店。

## 什么会被备份，什么不会

存档文件夹里很少只有存档，所以 Hoard 会把找到的东西分成三类：

- **存档数据**会被备份，也会被还原。这就是你的进度。
- **属于某一台机器的文件**——配置、日志之类——会上传以便进入备份，但绝不会覆盖另一台 PC 上的副本。你的画质设置依然是你的。
- **垃圾**——缓存、崩溃转储、临时文件——会被忽略，免得备份被你永远不想要回的东西撑大。

## 备份发生在什么时候

Hoard 会盯着文件夹，并在**你停止游玩之后**抓取它，而不是在游戏还占着文件的时候。如果存档是几秒前刚写入的，它会等到一切安静下来：正在写入的文件，不值得抓一半。

每次抓取就是一个版本。快照按内容哈希存储，因此未改动的文件只存一份——一个 2 GB 存档的十个版本大约占 2 GB，而不是 20 GB。

## 不经过我们服务器的备份

如果你不想用任何人的云，可以自己运行 \`hoard-server\`，把应用指向它。你的存档从你的 PC 走到你的磁盘：没有我们这边的账号，没有发往我们的遥测，也没有任何东西经过我们的服务器。参见[如何自托管 Hoard](/guides/self-host-hoard)。

## 提示：查看你的历史

打开某款游戏的**历史**标签，即可看到每个备份及其日期和大小。你可以从那里一键还原任何先前版本。你的存档以加密方式传输，存储在欧盟境内，你随时可以导出或删除。

已经在用像 Ludusavi 这样的本地备份工具？你可以继续用——但如果你希望这些备份进入云端并在多台机器之间同步，而无需自己编写 Rclone 脚本，那正是 Hoard 所自动化的。公平对比请见 [Ludusavi 与 Hoard](/guides/ludusavi-alternative)。

<!-- faq -->

## 常见问题

### 我在玩的时候 Hoard 会备份吗？

不会。它会等到你退出、存档文件夹安静下来才动手，所以备份绝不会是一个写到一半的文件。

### 我的存档需要多少空间？

比你想的少。版本按内容哈希去重，因此只有两次游玩之间真正变化的部分才占用新空间——大多数存档收藏放在几个 GB 里绰绰有余。

### 如果某个游戏没被检测到怎么办？

手动把 Hoard 指向那个文件夹，它就会像追踪其他游戏一样追踪它。检测覆盖数千款游戏，但存在不寻常位置、或你手动安装的游戏，有时需要你给个提示。

### 它会备份我的模组吗？

Hoard 追踪的是存档文件夹，所以放在别处的模组不在备份范围内。这是刻意的：模组体积大、可以重新下载，而在多台机器之间同步模组文件夹带来的麻烦多过好处。

### 自托管会改变备份的工作方式吗？

完全不会。同样的检测、同样的版本、同样的自动抓取。只有存储归你所有。
`,Be=`---
title: "Spielstand-Sync im Vergleich: Hoard gegen Ludusavi, Syncthing, OpenSave und die anderen"
description: "Ein ehrlicher Vergleich der Tools, die PC-Spielstände sichern und synchronisieren — Ludusavi, Syncthing, OpenSave, OpenCloudSaves, Game Backup Monitor, Aletheia, SaveSync und Hoard — mit Tabelle und einem Abschnitt darüber, wo Hoard verliert."
order: 4
updated: 2026-09-01
---

Steam Cloud deckt nur Spiele ab, die du bei Steam gekauft hast, und auch nur dann, wenn der Entwickler es eingeschaltet hat. Emulatoren, GOG, Epic, itch.io, Nicht-Steam-Spiele, alles Gemoddete: nichts davon ist dabei. Wer auf mehr als einem Rechner spielt, etwa Desktop und Steam Deck, kopiert am Ende Ordner von Hand und hofft, den neuesten erwischt zu haben.

Mehrere Tools lösen das, und sie tun nicht alle dasselbe. Manche legen lokale Backups an, manche spiegeln Ordner zwischen Geräten, manche laden in eine Cloud. Diese Seite geht sie durch und sagt, worin jedes wirklich gut ist. Hoard ist mein Projekt, deshalb kommt der ehrliche Teil am Schluss: ein Abschnitt darüber, wo Hoard verliert, und eine Tabelle, die man lesen kann, ohne dem Fließtext ein Wort zu glauben.

## Ludusavi

Das bekannteste, und das zu Recht. Ludusavi (von mtkennerly) ist ein kostenloses Open-Source-Backup-Tool mit Oberfläche und CLI, aufgebaut auf dem Community-Manifest der Spielstand-Pfade, das Zehntausende Spiele abdeckt — dasselbe Manifest, das fast alle hier verwenden, Hoard eingeschlossen. Es hält versionierte lokale Backups und kann sie über Rclone in deine eigene Cloud schieben.

**Am besten, wenn:** du lokale Backups, volle Kontrolle und nirgendwo einen Server willst. Die sicherste Wahl dieser Liste, und sie kostet nichts.

**Wo es aufhört:** Sync zwischen Rechnern ist etwas, das du selbst zusammenbaust. Backup planen, Rclone-Remote einrichten, und daran denken, auf dem anderen PC wiederherzustellen, *bevor* du spielst. Das funktioniert, aber nichts hindert dich daran, den letzten Schritt zu vergessen.

## Syncthing

Überhaupt kein Spiele-Tool, sondern ein allgemeiner Peer-to-Peer-Ordnerspiegel, und ein sehr guter. Zeig ihm einen Spielstandordner, und er taucht auf deinen anderen Geräten auf.

**Am besten, wenn:** du es ohnehin betreibst und die Dateien ohne Cloud dazwischen an zwei Orten haben willst.

**Wo es aufhört:** es spiegelt, es fotografiert nicht. Ein kaputter Spielstand erreicht jedes Gerät in Sekunden, genauso schnell wie ein guter. Die Dateiversionierung arbeitet pro Datei und hat keinen Begriff davon, was eine Spielsitzung ist — "zurück auf Dienstagabend" rekonstruierst du also von Hand. Zwei Maschinen, die beide offline gespielt haben, liefern dir Konfliktdateien, keine Zusammenführung.

## OpenSave

Peer-to-peer-Sync, eigens für Spielstände gebaut, in Go, MIT-lizenziert, für Windows, Linux und Steam Deck. Kein Konto, kein Server: Geräte koppeln sich miteinander und synchronisieren über das LAN oder per Raumcode über ein Relay. Jede Änderung wird als Snapshot festgehalten, es gibt Branches für parallele Durchläufe, Konflikte werden über die Sync-Abstammung statt über die Uhrzeit aufgelöst, und übertragen werden nur die geänderten Blöcke. Optional lässt sich zu Drive, Dropbox, OneDrive oder WebDAV spiegeln.

**Am besten, wenn:** du partout kein Konto willst und deine Geräte oft genug gleichzeitig laufen.

**Wo es aufhört:** Peer-to-Peer heißt, der Spielstand lebt nur auf deinen Geräten. Stirbt das Deck mit der einzigen aktuellen Kopie und war die Spiegelung nie eingerichtet, war's das. Für einen Sync müssen beide Geräte laufen, und einen macOS-Build gibt es nicht.

## OpenCloudSaves

Eine plattformübergreifende Oberfläche, die deine Spielstandordner in eine Cloud synchronisiert, für die du ohnehin zahlst — OneDrive, Google Drive, Dropbox, Nextcloud — mit Rclone darunter.

**Am besten, wenn:** du deine Spielstände in einem Speicherkonto haben willst, das du schon hast, mit Oberfläche statt Rclone-Konfigurationsdateien.

**Wo es aufhört:** es gibt keine inhaltsbasierte Deduplizierung. Zehn Kopien eines 2-GB-Spielstands sind 20 GB deines Drive-Kontingents, und Cloud-Laufwerke synchronisieren Dateien, keine Spielsitzungen — du bekommst also zurück, wie der Ordner damals eben aussah.

## Game Backup Monitor

Windows zuerst, und der Ursprung dieses ganzen Genres. GBM wartet auf den Spielprozess und packt den Spielstand beim Beenden mit 7-Zip ein, mit nummerierter Historie.

**Am besten, wenn:** du an einem einzigen Windows-PC sitzt und ein komprimiertes lokales Archiv ohne Nachdenken willst.

**Wo es aufhört:** es ist ein Backup-Tool, kein Sync-Tool. Das Archiv auf eine zweite Maschine zu bekommen, ist dein Problem, und Steam Deck / SteamOS ist nicht sein Zuhause.

## Aletheia

Das jüngste der Runde, AGPL, und es geht genau die Stelle an, die alle anderen halb abdecken: die Launcher. Heroic, itch.io, Lutris, Steam, GOG Galaxy und Xbox, unter Windows, Linux und macOS.

**Am besten, wenn:** deine Bibliothek über Launcher verteilt ist, die andere Tools schlecht erkennen — vor allem Xbox/Game Pass und Heroic.

**Wo es aufhört:** ein junges Projekt mit bewusst engem Zuschnitt. Sichern und Wiederherstellen ist der Funktionsumfang; eine versionierte Cloud steht nicht dahinter.

## SaveSync

Das kommerzielle, auf Steam als Einmalkauf, mit Fokus auf Windows. Sein Kniff: Es zielt gar nicht auf dich-an-zwei-PCs, sondern auf Koop. Spielstände landen in privaten, nicht gelisteten Steam-Workshop-Einträgen, damit ein Freund deine Valheim- oder Factorio-Welt ziehen kann, und LAN-Sync gibt es auch.

**Am besten, wenn:** dein Problem "mein Freund hostet und ich brauche seinen Spielstand" lautet und nicht "meine Spielstände sollen mir folgen".

**Wo es aufhört:** Closed Source, Windows, an Steam als Transportweg gebunden, und eine Liste unterstützter Koop-Spiele statt allem, was du besitzt.

## Eine Anmerkung zu EmuDeck

EmuDeck kommt in diesen Gesprächen auf und ist kein Konkurrent im üblichen Sinn: Es ist ein Installer und Konfigurator für Emulatoren auf dem Steam Deck, und der angebotene Sync ist eine Bequemlichkeit, die an diese Aufgabe angeflanscht ist (Rclone gegen ein Cloud-Laufwerk, nur für Emulator-Spielstände). Es überschneidet sich mit den Tools oben, ohne dasselbe zu sein: EmuDeck richtet deine Emulatoren ein, die Tools hier kümmern sich um die Spielstände der ganzen Bibliothek. Manche betreiben EmuDeck neben einem davon, und das ist ein sinnvolles Setup, kein doppeltes.

## Hoard

Hoard nimmt die Spielsitzung als Einheit. Die Engine läuft als Hintergrunddienst — \`hoardd\`, ohne Fenster, also funktioniert sie im Game Mode von SteamOS —, merkt, dass du aufgehört hast zu spielen, und macht dann den Snapshot, statt mitten im Spiel auf jeden Schreibvorgang zu reagieren.

- **Versionshistorie pro Sitzung.** Jede Sitzung ist eine Version, zu der du zurückkannst, auch nach einem Plattenausfall oder einer Neuinstallation.
- **Deduplizierung über Inhalts-Hashes.** Zehn Versionen eines 2-GB-Spielstands kosten rund 2 GB, nicht 20 GB. Übertragungen sind zstd-komprimiert.
- **SHA-256 beim Hochladen und beim Herunterladen.** Beschädigungen werden erkannt, bevor sie einen guten Spielstand überschreiben können. Nichts wird stillschweigend überschrieben — darum geht es im Kern.
- **Cloud oder selbst gehostet, dasselbe Binary.** Hoard Cloud hat einen kostenlosen Tarif (2 GB, 3 Geräte, volle Historie). Oder du betreibst \`hoard-server\` selbst per Docker Compose gegen beliebigen S3-kompatiblen Speicher — MinIO, Garage, Backblaze B2 — ohne Konto und ohne Kontingent. AGPL-3.0.
- **Windows, Linux, macOS**, dazu eine headless CLI für ein Steam Deck oder einen Server.
- **Emulatoren in der Beta:** PCSX2, RPCS3, Dolphin, Cemu, Ryujinx, RetroArch, DuckStation, PPSSPP und weitere als Voreinstellungen.

## Das Detail, an dem Steam Deck ↔ PC hängt

Gut zu wissen, egal welches Tool du nimmst. Der Cloud-Spielstand eines Steam-Spiels liegt in \`<AppID>/remote/\`, und der Ordner *darüber* enthält \`remotecache.vdf\`, Erfolgsstände, Statistiken und Spielzeitzähler — alles Dinge, die sich zwischen Deck und Desktop berechtigterweise unterscheiden.

Synchronisiere den übergeordneten Ordner, und du hast einen Dauerkonflikt zwischen zwei Maschinen, die sich über keinen einzigen Spielstand uneinig waren. Hoard verfolgt \`remote/\`, nicht den Elternordner. Jedem Tool, dem du einen Ordner von Hand zuweist, kann man dasselbe beibringen — und es ist das Erste, was man prüft, wenn ein Sync-Setup ohne sichtbaren Grund ständig Konflikte meldet.

## Wo Hoard verliert

- **Es will einen Server.** Cloud-Konto oder eigene Kiste, so oder so ist es Infrastruktur, und OpenSave oder Ludusavi brauchen keine.
- **Emulator-Unterstützung ist Beta.** Portable Installationen und die Eigenheiten einzelner Emulatoren erwischen es noch, und Aletheia und OpenSave decken manche Launcher- und Emulator-Sonderfälle heute besser ab.
- **macOS ist auf echter Hardware kaum getestet.** Es baut und läuft, aber niemand hat monatelang darauf gelebt.
- **Es ist jung.** Ludusavi und Game Backup Monitor haben Jahre an Fehlerberichten hinter sich. Hoard nicht, und das zählt bei etwas, das einen 200-Stunden-Spielstand hütet.
- **Es macht kein Koop-Teilen.** Wenn du einem Freund eine Welt geben willst, ist SaveSync dafür gebaut und Hoard nicht.

## Der Unterschied zwischen Hoard Cloud und Selbsthosten

Vergleiche zu Hoard werfen diese beiden fast immer in einen Topf, und das Ergebnis stimmt dann nicht. Deshalb klar gesagt:

- **Hoard Cloud** ist die verwaltete Variante: du meldest dich an, und deine Stände liegen auf unseren Servern in der EU.
- **Ein selbst gehostetes Hoard gehört vollständig dir.** Du betreibst \`hoard-server\` auf deinem PC oder NAS, und deine Stände gehen von deiner Maschine auf deine Platte. Es gibt **kein Konto bei uns, keine Telemetrie zu uns, kein Limit und kein Relay** — nichts läuft über unsere Server, weil nichts von uns im Weg steht. Wir sehen weder Spielstand noch Spieltitel noch E-Mail-Adresse, weil davon nichts bei uns ankommt. Würde Hoard Cloud morgen abgeschaltet, liefe ein selbst gehostetes Setup unverändert weiter.

Dasselbe Binary, dieselbe Erkennung, dieselbe Versionshistorie. Es ändert sich nur, wem der Speicher gehört. Ein Detail der Genauigkeit halber: dein eigener Server hat sehr wohl eigene Zugänge — einen Benutzer und ein Token je Gerät — aber die liegen in deiner Datenbank, nicht in unserer.

## Die Tabelle

| Tool | Automatischer Sync zwischen Geräten | Wo die Spielstände liegen | Historie | Plattformen | Lizenz |
|---|---|---|---|---|---|
| **Hoard** | Ja, pro Spielsitzung | Hoard Cloud oder eigener Server (S3-kompatibel) | Versioniert pro Sitzung, dedupliziert | Win · Linux · macOS · Deck | AGPL-3.0, kostenloser Tarif |
| **Ludusavi** | Manuell, oder Rclone, das du einrichtest | Lokal, plus dein Rclone-Remote | Versionierte lokale Backups | Win · Linux · macOS | Kostenlos, Open Source |
| **Syncthing** | Ja, fortlaufender Spiegel | Nur deine Geräte | Versionierung pro Datei | Alles | Kostenlos, Open Source |
| **OpenSave** | Ja, peer-to-peer | Deine Geräte, optionale Cloud-Spiegelung | Snapshots und Branches | Win · Linux · Deck | MIT |
| **OpenCloudSaves** | Ja, über dein Cloud-Laufwerk | OneDrive / Drive / Dropbox / Nextcloud | Was das Laufwerk aufhebt | Win · Linux · macOS | Kostenlos, Open Source |
| **Game Backup Monitor** | Nein | Lokale 7-Zip-Archive | Nummerierte Backups | Windows | Kostenlos, Open Source |
| **Aletheia** | Sichern und Wiederherstellen pro Launcher | Dein Speicher | Backups | Win · Linux · macOS | AGPL-3.0 |
| **SaveSync** | Ja, auch mit Freunden | Private Steam-Workshop-Einträge | Laut App | Windows | Kostenpflichtig, Closed Source |

## Also welches

Willst du eine Maschine gesichert haben und sonst nichts, nimm Ludusavi oder Game Backup Monitor. Willst du unter keinen Umständen ein Konto und laufen deine Geräte meist gleichzeitig, OpenSave. Sollen die Spielstände in einem Drive-Ordner landen, für den du schon zahlst, OpenCloudSaves. Teilst du eine Koop-Welt mit Freunden, SaveSync.

Willst du, dass Backup *und* Sync zwischen PCs und einem Steam Deck einfach passieren, mit einer Version pro Sitzung, zu der du zurückkannst, und der Option, das Ganze selbst zu hosten, dann ist Hoard dafür da. [Lade es herunter](/download) oder lies vorher, [wie man es mit Docker selbst hostet](/guides/self-host-hoard). Es gibt außerdem einen [ausführlichen Ludusavi-Vergleich](/guides/ludusavi-alternative), falls du genau damit abwägst.

## Direkte Vergleiche

Jeder davon geht tiefer als der Abschnitt oben, samt der Punkte, an denen das andere Werkzeug gewinnt:

- [Hoard gegen Ludusavi](/guides/ludusavi-alternative)
- [Hoard als Steam-Cloud-Alternative](/guides/steam-cloud-alternative)
- [Peer-to-peer gegen einen eigenen Server](/guides/opensave-alternative)
- [Syncthing für Spielstände: was bricht](/guides/syncthing-game-saves)

<!-- faq -->

## Häufige Fragen

### Welches dieser Werkzeuge führt eine Versionshistorie?

Hoard behält jede Sitzung als Version, zu der du zurückkannst. Ludusavi führt versionierte lokale Backups. Die meisten übrigen synchronisieren oder kopieren den aktuellen Zustand — ein beschädigter Spielstand wandert damit getreulich auf die andere Maschine.

### Welches funktioniert ohne Server und ohne Konto?

Ludusavi mit lokalen Backups, und jedes Peer-to-peer-Werkzeug. Hoard zählt ebenfalls dazu, wenn du selbst hostest: kein Konto bei uns, und nichts, was über unsere Server läuft.

### Welches deckt Spiele ab, die nicht auf Steam sind?

Alle Spielstand-Verwalter hier, denn sie finden Stände über dieselbe Community-Datenbank statt über einen Store. Die Ausnahme ist Steam Cloud: sie deckt nur Steam-Spiele ab, deren Entwickler sie aktiviert hat.

### Muss ich mich für eines entscheiden?

Nein, und viele tun es nicht. Ein lokales Backup-Werkzeug und ein Sync-Werkzeug lösen unterschiedliche Hälften des Problems. Die einzige Regel: richte niemals eines auf den Backup-Ordner des anderen, sonst synchronisierst du einen veralteten Spiegel statt deines echten Spielstands.

### Was ist das eine Detail, an dem die meisten Eigenbau-Setups scheitern?

Den Ordner über \`<AppID>/remote/\` in Steams \`userdata\` zu synchronisieren. Der übergeordnete Ordner enthält \`remotecache.vdf\` sowie Dateien für Erfolge und Spielzeit, die sich pro Rechner unterscheiden sollen — jeder Start sieht dann nach einem Konflikt aus, obwohl sich kein Stand bewegt hat.
`,Me=`---
title: "Game save sync compared: Hoard vs Ludusavi, Syncthing, OpenSave and the rest"
description: "An honest comparison of the tools that back up and sync PC game saves — Ludusavi, Syncthing, OpenSave, OpenCloudSaves, Game Backup Monitor, Aletheia, SaveSync and Hoard — with a table, and a section on where Hoard loses."
order: 4
updated: 2026-09-01
---

Steam Cloud only covers games you bought on Steam, and only when the developer bothered to switch it on. Emulators, GOG, Epic, itch.io, non-Steam games, anything modded — none of that is covered. If you play on more than one machine, a desktop and a Steam Deck say, you end up copying folders by hand and hoping you grabbed the newest one.

Several tools fix this, and they don't all do the same thing. Some make local backups, some mirror folders between devices, some upload to a cloud. This page goes through them and says what each one is genuinely best at. Hoard is my project, so the honest part comes at the end: a section on where Hoard loses, and a table you can read without trusting a word of the prose.

## Ludusavi

The best-known one, and deservedly so. Ludusavi (by mtkennerly) is a free, open-source backup tool with a GUI and a CLI, and it's built on the community save-location manifest that covers tens of thousands of games — the same manifest most of the tools here use, Hoard included. It keeps versioned local backups and can push them to your own cloud through Rclone.

**Best if:** you want local backups, full control, and no server anywhere. It's the safest default on this list and costs nothing.

**Where it stops:** cross-machine sync is a thing you assemble. Schedule a backup, configure an Rclone remote, remember to restore on the other PC *before* you play. It works, but nothing stops you forgetting the last step.

## Syncthing

Not a game tool at all — a general-purpose, peer-to-peer folder mirror, and a very good one. Point it at a save folder and it appears on your other devices.

**Best if:** you already run it and you want files in two places with no cloud in between.

**Where it stops:** it mirrors, it doesn't snapshot. A corrupted save reaches every device in seconds, exactly as fast as a good one. Its file versioning is per-file, with no idea what a play session is, so "roll back to how it was on Tuesday night" is something you reconstruct by hand. Two machines that both played offline give you conflict files, not a merge.

## OpenSave

Peer-to-peer sync built specifically for saves, in Go, MIT licensed, for Windows, Linux and Steam Deck. No account, no server: devices pair with each other and sync over the LAN or through a relay room code. It snapshots every change, has branches for parallel playthroughs, resolves conflicts by sync lineage rather than clock timestamps, and transfers only changed blocks. It can optionally mirror to Drive, Dropbox, OneDrive or WebDAV.

**Best if:** you refuse to have an account, and your devices are on together often enough to actually meet.

**Where it stops:** peer-to-peer means the save lives only on your devices. If the Deck holding the only recent copy dies and the mirror was never configured, that's it. Both devices have to be running for a sync to happen, and there's no macOS build.

## OpenCloudSaves

A cross-platform GUI that syncs your save folders into a cloud you already pay for — OneDrive, Google Drive, Dropbox, Nextcloud — using Rclone underneath.

**Best if:** you want your saves in a storage account you already have, with a UI instead of Rclone config files.

**Where it stops:** there's no content-level deduplication. Ten copies of a 2 GB save is 20 GB of your Drive quota, and cloud drives sync files, not play sessions, so what you get back is whatever the folder looked like at the time.

## Game Backup Monitor

Windows-first, and the original of this whole genre. GBM watches for a game process, and when you quit, it compresses the save with 7-Zip and keeps a numbered history.

**Best if:** you're on one Windows PC and want a compressed local archive with zero thinking.

**Where it stops:** it's a backup tool, not a sync tool. Getting the archive onto a second machine is your problem, and Steam Deck / SteamOS is not its home turf.

## Aletheia

The newest of the bunch, AGPL, and it goes after the part everyone else half-covers: launchers. Heroic, itch.io, Lutris, Steam, GOG Galaxy and Xbox, across Windows, Linux and macOS.

**Best if:** your library is spread across launchers that other tools detect badly — especially Xbox/Game Pass and Heroic.

**Where it stops:** it's a young project with a deliberately narrow scope. Backup and restore is the feature set; there's no versioned cloud behind it.

## SaveSync

The commercial one, sold on Steam as a one-time purchase, Windows-focused. Its trick is that it isn't really aimed at you-on-two-PCs — it's aimed at co-op. Saves go into private, unlisted Steam Workshop entries so a friend can pull your Valheim or Factorio world, and there's LAN sync too.

**Best if:** the problem you're solving is "my friend hosts and I need their save", not "my saves follow me".

**Where it stops:** closed source, Windows, tied to Steam as the transport, and a set of supported co-op games rather than everything you own.

## A note on EmuDeck

EmuDeck comes up in these conversations, and it isn't a competitor in the normal sense — it's an emulator installer and configurator for Steam Deck, and the sync it offers is a convenience bolted onto that job (Rclone against a cloud drive, for emulator saves only). It overlaps with the tools above without being the same kind of thing: EmuDeck sets your emulators up, the tools here look after saves for the whole library. People do run EmuDeck alongside one of these, and that's a sensible setup, not a redundant one.

## Hoard

Hoard treats a play session as the unit. The engine runs as a background service — \`hoardd\`, no window, so it works in SteamOS game mode — notices you stopped playing, and takes a snapshot then, instead of reacting to every file write mid-game.

- **Version history per session.** Every session is a version you can roll back to, including after a disk failure or a fresh install.
- **Content-hash deduplication.** Ten versions of a 2 GB save cost about 2 GB, not 20 GB. Transfers are zstd-compressed.
- **SHA-256 on the way up and on the way down.** Corruption is caught before it can overwrite a good save. Nothing is ever silently overwritten — that's the whole design.
- **Cloud or self-hosted, same binary.** Hoard Cloud has a free tier (2 GB, 3 devices, full history). Or run \`hoard-server\` yourself with Docker Compose against any S3-compatible storage — MinIO, Garage, Backblaze B2 — with no account and no quota. AGPL-3.0.
- **Windows, Linux, macOS**, plus a headless CLI for a Steam Deck or a server.
- **Emulators in beta:** PCSX2, RPCS3, Dolphin, Cemu, Ryujinx, RetroArch, DuckStation, PPSSPP and others as presets.

## The detail that decides Steam Deck ↔ PC sync

Worth knowing whichever tool you pick. A Steam game's cloud save lives in \`<AppID>/remote/\`, and the folder *above* it holds \`remotecache.vdf\`, achievement state, stats and playtime counters — all of which legitimately differ between your Deck and your desktop.

Sync the parent folder and you get a permanent conflict between two machines that never disagreed about a single save. Hoard tracks \`remote/\`, not the parent. Any tool pointed at a folder by hand can be told to do the same, and it's the first thing to check when a sync setup keeps flagging conflicts for no visible reason.

## Where Hoard loses

- **It wants a server.** Cloud account or your own box — either way it's infrastructure, and OpenSave or Ludusavi need none.
- **Emulator support is beta.** Portable installs and per-emulator quirks still catch it out; Aletheia and OpenSave cover some launcher/emulator edge cases better today.
- **macOS is barely tested on real hardware.** It builds and it runs, but nobody has lived on it for months.
- **It's young.** Ludusavi and Game Backup Monitor have years of bug reports behind them. Hoard doesn't, and that matters for something guarding a 200-hour save.
- **It doesn't do co-op sharing.** If you want to hand a world to a friend, SaveSync is built for that and Hoard isn't.

## The Hoard Cloud / self-host distinction

Comparisons of Hoard almost always collapse these two into one, and the result is wrong, so it's worth stating plainly:

- **Hoard Cloud** is the managed option: you sign in, and your saves are stored on our servers, in the EU.
- **A self-hosted Hoard is entirely yours.** You run \`hoard-server\` on your own PC or NAS, and your saves go from your machine to your disk. There is **no account with us, no telemetry to us, no quota and no relay** — nothing passes through our servers, because there is nothing of ours in the path. We can't see a save, a game name or an email address, because none of it ever reaches us. If Hoard Cloud shut down tomorrow, a self-hosted setup would carry on unchanged.

Same binary, same detection, same version history. The only thing that changes is who owns the storage. Being exact about one detail: your own server does have logins of its own — a user and a token per device — but they live in your database, not ours.

## The table

| Tool | Automatic sync between devices | Where saves live | History | Platforms | Licence |
|---|---|---|---|---|---|
| **Hoard** | Yes, per play session | Hoard Cloud or your own server (S3-compatible) | Versioned per session, deduplicated | Win · Linux · macOS · Deck | AGPL-3.0, free tier |
| **Ludusavi** | Manual, or Rclone that you wire up | Local, plus your Rclone remote | Versioned local backups | Win · Linux · macOS | Free, open source |
| **Syncthing** | Yes, continuous mirror | Your devices only | Per-file versioning | Everything | Free, open source |
| **OpenSave** | Yes, peer-to-peer | Your devices, optional cloud mirror | Snapshots and branches | Win · Linux · Deck | MIT |
| **OpenCloudSaves** | Yes, via your cloud drive | OneDrive / Drive / Dropbox / Nextcloud | Whatever the drive keeps | Win · Linux · macOS | Free, open source |
| **Game Backup Monitor** | No | Local 7-Zip archives | Numbered backups | Windows | Free, open source |
| **Aletheia** | Backup and restore per launcher | Your storage | Backups | Win · Linux · macOS | AGPL-3.0 |
| **SaveSync** | Yes, and with friends | Private Steam Workshop entries | Per the app | Windows | Paid, closed source |

## So which one

If you want one machine backed up and nothing else, take Ludusavi or Game Backup Monitor. If you want no account under any circumstances and your devices are usually on together, OpenSave. If your saves should be in a Drive folder you already pay for, OpenCloudSaves. If you're sharing a co-op world with friends, SaveSync.

If you want backups *and* automatic sync across PCs and a Steam Deck to just happen, with a version per session you can roll back to and the option to self-host the whole thing, that's what Hoard is for. [Download it](/download), or read [how to self-host it with Docker](/guides/self-host-hoard) first. There's also a longer [Ludusavi comparison](/guides/ludusavi-alternative) if that's the one you're weighing it against.

## One-on-one comparisons

Each of these goes deeper than the section above, including where the other tool wins:

- [Hoard vs Ludusavi](/guides/ludusavi-alternative)
- [Hoard as a Steam Cloud alternative](/guides/steam-cloud-alternative)
- [Peer-to-peer sync vs a server you own](/guides/opensave-alternative)
- [Syncthing for game saves: what breaks](/guides/syncthing-game-saves)

<!-- faq -->

## Frequently asked questions

### Which of these tools keeps a version history?

Hoard keeps every session as a version you can roll back to. Ludusavi keeps versioned local backups. Most of the rest sync or copy the current state, which means a corrupted save is faithfully propagated to your other machine.

### Which one works without any server or account?

Ludusavi with local backups, and any peer-to-peer tool. Hoard also qualifies if you self-host: no account with us, and nothing passing through our servers.

### Which one covers games that aren't on Steam?

All the save-manager tools here do, because they locate saves through the same community database rather than through a store. Steam Cloud is the one that doesn't: it only covers Steam games whose developer enabled it.

### Do I have to pick just one?

No, and plenty of people don't. A local backup tool and a sync tool solve different halves of the problem. The only rule is never to point one tool at another's backup folder, or you end up syncing a stale mirror instead of your live save.

### What's the single detail that breaks most DIY setups?

Syncing the folder above \`<AppID>/remote/\` in Steam's \`userdata\`. The parent holds \`remotecache.vdf\` plus achievement and playtime files that are supposed to differ per machine, so every launch looks like a conflict even though no save moved.
`,Ue=`---
title: "Comparativa de sincronización de partidas: Hoard frente a Ludusavi, Syncthing, OpenSave y las demás"
description: "Comparativa honesta de las herramientas que hacen copia y sincronizan partidas de PC — Ludusavi, Syncthing, OpenSave, OpenCloudSaves, Game Backup Monitor, Aletheia, SaveSync y Hoard — con tabla y un apartado sobre dónde pierde Hoard."
order: 4
updated: 2026-09-01
---

Steam Cloud solo cubre los juegos que compraste en Steam, y solo cuando el desarrollador se molestó en activarlo. Emuladores, GOG, Epic, itch.io, juegos que no son de Steam, cualquier cosa con mods: nada de eso entra. Si juegas en más de un equipo, un sobremesa y una Steam Deck por ejemplo, acabas copiando carpetas a mano y confiando en haber cogido la más reciente.

Hay varias herramientas que resuelven esto y no todas hacen lo mismo. Unas hacen copias locales, otras replican carpetas entre dispositivos, otras suben a una nube. Esta página las repasa y dice en qué es buena de verdad cada una. Hoard es mi proyecto, así que la parte honesta va al final: un apartado sobre dónde pierde Hoard, y una tabla que puedes leer sin fiarte de una sola línea del texto.

## Ludusavi

La más conocida, y con razón. Ludusavi (de mtkennerly) es una herramienta de copia gratuita y open source, con interfaz y con CLI, construida sobre el manifiesto comunitario de ubicaciones de partidas que cubre decenas de miles de juegos: el mismo manifiesto que usan casi todas las de esta lista, Hoard incluido. Guarda copias locales versionadas y puede subirlas a una nube tuya configurando Rclone.

**Mejor si:** quieres copias locales, control total y ningún servidor en ninguna parte. Es la opción más segura de la lista y no cuesta nada.

**Dónde se queda:** la sincronización entre equipos es algo que montas tú. Programas una copia, configuras un remoto de Rclone y te acuerdas de restaurar en el otro PC *antes* de jugar. Funciona, pero nada te impide olvidarte del último paso.

## Syncthing

No es una herramienta de juegos: es un espejo de carpetas peer-to-peer de propósito general, y muy bueno. Le señalas una carpeta de partidas y aparece en tus otros dispositivos.

**Mejor si:** ya lo tienes montado y quieres los ficheros en dos sitios sin nube por medio.

**Dónde se queda:** replica, no fotografía. Una partida corrupta llega a todos los dispositivos en segundos, exactamente igual de rápido que una buena. Su versionado es por fichero, sin noción de qué es una sesión de juego, así que "volver a como estaba el martes por la noche" es algo que reconstruyes a mano. Dos máquinas que jugaron sin conexión te dan ficheros de conflicto, no una fusión.

## OpenSave

Sincronización peer-to-peer hecha específicamente para partidas, en Go, con licencia MIT, para Windows, Linux y Steam Deck. Sin cuenta y sin servidor: los dispositivos se emparejan entre ellos y sincronizan por la red local o a través de un código de sala en un relay. Fotografía cada cambio, tiene ramas para partidas paralelas, resuelve conflictos por linaje de sincronización en vez de por reloj, y transfiere solo los bloques que cambiaron. Opcionalmente puede replicar a Drive, Dropbox, OneDrive o WebDAV.

**Mejor si:** te niegas a tener una cuenta y tus dispositivos coinciden encendidos lo bastante a menudo.

**Dónde se queda:** peer-to-peer significa que la partida vive solo en tus dispositivos. Si muere la Deck que tenía la única copia reciente y nunca configuraste la réplica, se acabó. Los dos dispositivos tienen que estar en marcha para que haya sincronización, y no hay versión para macOS.

## OpenCloudSaves

Una interfaz multiplataforma que sincroniza tus carpetas de partidas contra una nube que ya pagas — OneDrive, Google Drive, Dropbox, Nextcloud — usando Rclone por debajo.

**Mejor si:** quieres tus partidas en una cuenta de almacenamiento que ya tienes, con una interfaz en vez de ficheros de configuración de Rclone.

**Dónde se queda:** no hay deduplicación por contenido. Diez copias de una partida de 2 GB son 20 GB de tu cuota de Drive, y las nubes de disco sincronizan ficheros, no sesiones de juego, así que lo que recuperas es como estuviera la carpeta en ese momento.

## Game Backup Monitor

Primero Windows, y el original de todo este género. GBM vigila el proceso del juego y, cuando sales, comprime la partida con 7-Zip y guarda un historial numerado.

**Mejor si:** estás en un solo PC con Windows y quieres un archivo comprimido local sin pensar en nada.

**Dónde se queda:** es una herramienta de copia, no de sincronización. Llevar el archivo a una segunda máquina es cosa tuya, y Steam Deck / SteamOS no es su terreno.

## Aletheia

La más nueva del grupo, AGPL, y va justo a la parte que las demás cubren a medias: los lanzadores. Heroic, itch.io, Lutris, Steam, GOG Galaxy y Xbox, en Windows, Linux y macOS.

**Mejor si:** tu biblioteca está repartida entre lanzadores que otras herramientas detectan mal, sobre todo Xbox/Game Pass y Heroic.

**Dónde se queda:** es un proyecto joven con un alcance deliberadamente estrecho. Copiar y restaurar es todo el conjunto de funciones; no hay una nube versionada detrás.

## SaveSync

La comercial, se vende en Steam como pago único y está centrada en Windows. Su truco es que no apunta a ti-en-dos-PC, sino al cooperativo: las partidas van a entradas privadas y no listadas del Steam Workshop para que un amigo pueda bajarse tu mundo de Valheim o de Factorio, y además hay sincronización por red local.

**Mejor si:** el problema que resuelves es "mi amigo hospeda y necesito su partida", no "que mis partidas me sigan".

**Dónde se queda:** código cerrado, Windows, atado a Steam como transporte, y una lista de juegos cooperativos soportados en vez de todo lo que tengas.

## Un apunte sobre EmuDeck

EmuDeck sale en estas conversaciones y no es un competidor en el sentido normal: es un instalador y configurador de emuladores para Steam Deck, y la sincronización que ofrece es una comodidad añadida a ese trabajo (Rclone contra una nube de disco, solo para partidas de emulador). Se solapa con las herramientas de arriba sin ser lo mismo: EmuDeck te deja los emuladores montados, y las de aquí cuidan las partidas de toda la biblioteca. Hay gente que usa EmuDeck junto a una de estas, y es un montaje sensato, no redundante.

## Hoard

Hoard toma la sesión de juego como unidad. El motor corre como servicio en segundo plano — \`hoardd\`, sin ventana, así que funciona en el modo juego de SteamOS —, se entera de que has dejado de jugar y hace la instantánea entonces, en vez de reaccionar a cada escritura de fichero en mitad de la partida.

- **Historial versionado por sesión.** Cada sesión es una versión a la que puedes volver, incluso después de un fallo de disco o una instalación limpia.
- **Deduplicación por hash de contenido.** Diez versiones de una partida de 2 GB ocupan unos 2 GB, no 20 GB. Las transferencias van comprimidas con zstd.
- **SHA-256 al subir y al bajar.** La corrupción se detecta antes de que pueda sobrescribir una partida buena. Nada se sobrescribe en silencio: ese es todo el diseño.
- **Nube o autoalojado, el mismo binario.** Hoard Cloud tiene plan gratuito (2 GB, 3 dispositivos, historial completo). O levantas \`hoard-server\` tú mismo con Docker Compose contra cualquier almacenamiento compatible con S3 — MinIO, Garage, Backblaze B2 — sin cuenta y sin cuota. AGPL-3.0.
- **Windows, Linux y macOS**, más una CLI sin interfaz para una Steam Deck o un servidor.
- **Emuladores en beta:** PCSX2, RPCS3, Dolphin, Cemu, Ryujinx, RetroArch, DuckStation, PPSSPP y otros como preajustes.

## El detalle que decide la sincronización Steam Deck ↔ PC

Conviene saberlo elijas la herramienta que elijas. La partida en la nube de un juego de Steam vive en \`<AppID>/remote/\`, y la carpeta de *encima* guarda \`remotecache.vdf\`, el estado de logros, estadísticas y contadores de horas jugadas, cosas que legítimamente son distintas entre tu Deck y tu sobremesa.

Sincroniza la carpeta padre y tendrás un conflicto permanente entre dos máquinas que nunca discreparon sobre una sola partida. Hoard rastrea \`remote/\`, no la carpeta padre. A cualquier herramienta a la que le señales una carpeta a mano se le puede decir lo mismo, y es lo primero que hay que mirar cuando un montaje de sincronización marca conflictos sin motivo aparente.

## Dónde pierde Hoard

- **Quiere un servidor.** Cuenta en la nube o máquina tuya, en cualquier caso es infraestructura, y OpenSave o Ludusavi no necesitan ninguna.
- **El soporte de emuladores está en beta.** Las instalaciones portables y las manías de cada emulador todavía lo pillan, y hoy Aletheia y OpenSave cubren mejor algunos casos raros de lanzadores y emuladores.
- **macOS apenas está probado en hardware real.** Compila y funciona, pero nadie ha vivido ahí durante meses.
- **Es joven.** Ludusavi y Game Backup Monitor llevan años de informes de fallos a la espalda. Hoard no, y eso importa en algo que custodia una partida de 200 horas.
- **No hace cooperativo.** Si quieres pasarle un mundo a un amigo, SaveSync está hecho para eso y Hoard no.

## La distinción entre Hoard Cloud y autoalojarse

Las comparativas sobre Hoard casi siempre funden las dos en una, y el resultado sale mal, así que conviene decirlo claro:

- **Hoard Cloud** es la opción gestionada: inicias sesión y tus partidas se guardan en nuestros servidores, en la UE.
- **Un Hoard autoalojado es tuyo por completo.** Levantas \`hoard-server\` en tu PC o en tu NAS y tus partidas van de tu máquina a tu disco. **No hay cuenta con nosotros, ni telemetría hacia nosotros, ni cupo, ni relé**: no pasa nada por nuestros servidores, porque no hay nada nuestro en el camino. No podemos ver una partida, ni el nombre de un juego, ni un correo, porque nada de eso nos llega. Si Hoard Cloud cerrara mañana, un montaje autoalojado seguiría igual.

El mismo binario, la misma detección, el mismo historial de versiones. Lo único que cambia es de quién es el almacenamiento. Y siendo exactos en un detalle: tu servidor sí tiene sus propios accesos — un usuario y un token por dispositivo — pero viven en tu base de datos, no en la nuestra.

## La tabla

| Herramienta | Sincronización automática entre dispositivos | Dónde viven las partidas | Historial | Plataformas | Licencia |
|---|---|---|---|---|---|
| **Hoard** | Sí, por sesión de juego | Hoard Cloud o tu propio servidor (compatible con S3) | Versionado por sesión, deduplicado | Win · Linux · macOS · Deck | AGPL-3.0, plan gratuito |
| **Ludusavi** | Manual, o Rclone que montas tú | Local, más tu remoto de Rclone | Copias locales versionadas | Win · Linux · macOS | Gratis, open source |
| **Syncthing** | Sí, espejo continuo | Solo tus dispositivos | Versionado por fichero | Todo | Gratis, open source |
| **OpenSave** | Sí, peer-to-peer | Tus dispositivos, réplica opcional en nube | Instantáneas y ramas | Win · Linux · Deck | MIT |
| **OpenCloudSaves** | Sí, vía tu nube de disco | OneDrive / Drive / Dropbox / Nextcloud | Lo que guarde la nube | Win · Linux · macOS | Gratis, open source |
| **Game Backup Monitor** | No | Archivos 7-Zip locales | Copias numeradas | Windows | Gratis, open source |
| **Aletheia** | Copia y restauración por lanzador | Tu almacenamiento | Copias | Win · Linux · macOS | AGPL-3.0 |
| **SaveSync** | Sí, y con amigos | Entradas privadas del Steam Workshop | Según la app | Windows | De pago, código cerrado |

## Entonces cuál

Si quieres una sola máquina respaldada y nada más, coge Ludusavi o Game Backup Monitor. Si no quieres una cuenta bajo ningún concepto y tus dispositivos suelen estar encendidos a la vez, OpenSave. Si tus partidas deben acabar en una carpeta de Drive que ya pagas, OpenCloudSaves. Si compartes un mundo cooperativo con amigos, SaveSync.

Si lo que quieres es que la copia *y* la sincronización entre PC y una Steam Deck pasen solas, con una versión por sesión a la que volver y la opción de autoalojarlo todo, para eso está Hoard. [Descárgalo](/download), o léete antes [cómo autoalojarlo con Docker](/guides/self-host-hoard). También hay una [comparativa larga con Ludusavi](/guides/ludusavi-alternative) si es esa la que estás sopesando.

## Comparativas una a una

Cada una entra más a fondo que el bloque de arriba, incluido dónde gana la otra herramienta:

- [Hoard frente a Ludusavi](/guides/ludusavi-alternative)
- [Hoard como alternativa a Steam Cloud](/guides/steam-cloud-alternative)
- [Sincronización punto a punto frente a un servidor tuyo](/guides/opensave-alternative)
- [Syncthing para partidas: qué se rompe](/guides/syncthing-game-saves)

<!-- faq -->

## Preguntas frecuentes

### ¿Cuál de estas herramientas guarda historial de versiones?

Hoard conserva cada sesión como una versión a la que puedes volver. Ludusavi guarda copias locales versionadas. La mayoría del resto sincroniza o copia el estado actual, lo que significa que una partida corrupta se propaga fielmente a tu otra máquina.

### ¿Cuál funciona sin servidor ni cuenta?

Ludusavi con copias locales, y cualquier herramienta punto a punto. Hoard también entra si te autoalojas: sin cuenta con nosotros y sin nada que pase por nuestros servidores.

### ¿Cuál cubre juegos que no están en Steam?

Todas las herramientas de gestión de partidas de aquí, porque localizan los saves con la misma base de datos comunitaria y no a través de una tienda. La que no lo hace es Steam Cloud: sólo cubre juegos de Steam cuyo desarrollador lo activó.

### ¿Tengo que quedarme con una sola?

No, y mucha gente no lo hace. Una herramienta de copia local y una de sincronización resuelven mitades distintas del problema. La única regla es no apuntar nunca una a la carpeta de copias de la otra, o acabas sincronizando un espejo desfasado en vez de tu partida real.

### ¿Cuál es el detalle que rompe la mayoría de montajes caseros?

Sincronizar la carpeta que está por encima de \`<AppID>/remote/\` en el \`userdata\` de Steam. La padre guarda \`remotecache.vdf\` y ficheros de logros y tiempo jugado que deben ser distintos en cada máquina, así que cada arranque parece un conflicto aunque no se haya movido ninguna partida.
`,Ve=`---
title: "Comparatif de synchronisation des sauvegardes : Hoard face à Ludusavi, Syncthing, OpenSave et les autres"
description: "Comparatif honnête des outils qui sauvegardent et synchronisent les parties PC — Ludusavi, Syncthing, OpenSave, OpenCloudSaves, Game Backup Monitor, Aletheia, SaveSync et Hoard — avec un tableau et une section sur les points faibles de Hoard."
order: 4
updated: 2026-09-01
---

Steam Cloud ne couvre que les jeux achetés sur Steam, et seulement quand le développeur a pris la peine de l'activer. Émulateurs, GOG, Epic, itch.io, jeux hors Steam, tout ce qui est moddé : rien de tout ça n'est couvert. Si vous jouez sur plusieurs machines, un fixe et un Steam Deck par exemple, vous finissez par copier des dossiers à la main en espérant avoir pris le plus récent.

Plusieurs outils règlent le problème, et ils ne font pas tous la même chose. Certains font des sauvegardes locales, d'autres répliquent des dossiers entre appareils, d'autres envoient vers un cloud. Cette page les passe en revue et dit ce que chacun fait vraiment bien. Hoard est mon projet, donc la partie honnête arrive à la fin : une section sur les points faibles de Hoard, et un tableau lisible sans croire un mot du texte.

## Ludusavi

Le plus connu, et à juste titre. Ludusavi (de mtkennerly) est un outil de sauvegarde gratuit et open source, avec interface et ligne de commande, bâti sur le manifeste communautaire des emplacements de sauvegardes qui couvre des dizaines de milliers de jeux — le même manifeste qu'utilisent presque tous les outils d'ici, Hoard compris. Il conserve des sauvegardes locales versionnées et peut les pousser vers votre propre cloud via Rclone.

**Le meilleur si :** vous voulez des sauvegardes locales, le contrôle total et aucun serveur nulle part. C'est le choix le plus sûr de la liste, et il est gratuit.

**Là où il s'arrête :** la synchronisation entre machines, c'est vous qui l'assemblez. Planifier une sauvegarde, configurer un remote Rclone, puis penser à restaurer sur l'autre PC *avant* de jouer. Ça marche, mais rien ne vous empêche d'oublier la dernière étape.

## Syncthing

Pas du tout un outil de jeu : un miroir de dossiers pair-à-pair généraliste, et très bon. Vous lui désignez un dossier de sauvegardes et il apparaît sur vos autres appareils.

**Le meilleur si :** vous l'utilisez déjà et vous voulez les fichiers à deux endroits sans cloud entre les deux.

**Là où il s'arrête :** il réplique, il ne photographie pas. Une sauvegarde corrompue atteint tous les appareils en quelques secondes, exactement aussi vite qu'une bonne. Son versionnage est par fichier, sans notion de session de jeu, donc « revenir à mardi soir » se reconstruit à la main. Deux machines qui ont joué hors ligne vous donnent des fichiers de conflit, pas une fusion.

## OpenSave

Synchronisation pair-à-pair conçue spécifiquement pour les sauvegardes, en Go, sous licence MIT, pour Windows, Linux et Steam Deck. Pas de compte, pas de serveur : les appareils s'appairent entre eux et se synchronisent en réseau local ou via un code de salon sur un relais. Chaque changement devient un instantané, il y a des branches pour les parties parallèles, les conflits se résolvent par lignage de synchronisation plutôt que par horloge, et seuls les blocs modifiés circulent. Il peut, en option, répliquer vers Drive, Dropbox, OneDrive ou WebDAV.

**Le meilleur si :** vous refusez d'avoir un compte et vos appareils sont allumés en même temps assez souvent.

**Là où il s'arrête :** pair-à-pair veut dire que la sauvegarde ne vit que sur vos appareils. Si le Deck qui détenait la seule copie récente meurt et que la réplication n'a jamais été configurée, c'est terminé. Les deux appareils doivent tourner pour qu'une synchronisation ait lieu, et il n'y a pas de version macOS.

## OpenCloudSaves

Une interface multiplateforme qui synchronise vos dossiers de sauvegardes vers un cloud que vous payez déjà — OneDrive, Google Drive, Dropbox, Nextcloud — avec Rclone en dessous.

**Le meilleur si :** vous voulez vos sauvegardes dans un espace de stockage que vous avez déjà, avec une interface plutôt que des fichiers de configuration Rclone.

**Là où il s'arrête :** pas de déduplication au niveau du contenu. Dix copies d'une sauvegarde de 2 Go, ce sont 20 Go de votre quota Drive, et les clouds de fichiers synchronisent des fichiers, pas des sessions de jeu : vous récupérez l'état du dossier à un instant donné, rien de plus.

## Game Backup Monitor

D'abord Windows, et l'ancêtre de tout ce genre. GBM guette le processus du jeu et, à la fermeture, compresse la sauvegarde avec 7-Zip en gardant un historique numéroté.

**Le meilleur si :** vous êtes sur un seul PC Windows et voulez une archive locale compressée sans y penser.

**Là où il s'arrête :** c'est un outil de sauvegarde, pas de synchronisation. Amener l'archive sur une deuxième machine, c'est votre affaire, et Steam Deck / SteamOS n'est pas son terrain.

## Aletheia

Le plus récent du lot, sous AGPL, et il attaque précisément ce que les autres couvrent à moitié : les lanceurs. Heroic, itch.io, Lutris, Steam, GOG Galaxy et Xbox, sous Windows, Linux et macOS.

**Le meilleur si :** votre bibliothèque est éparpillée sur des lanceurs que les autres outils détectent mal, en particulier Xbox/Game Pass et Heroic.

**Là où il s'arrête :** projet jeune, au périmètre volontairement étroit. Sauvegarder et restaurer, c'est tout ; il n'y a pas de cloud versionné derrière.

## SaveSync

Le commercial, vendu sur Steam en achat unique, orienté Windows. Sa particularité : il ne vise pas vous-sur-deux-PC mais le coop. Les sauvegardes partent dans des entrées privées et non listées du Steam Workshop pour qu'un ami récupère votre monde Valheim ou Factorio, et il y a aussi une synchronisation en réseau local.

**Le meilleur si :** votre problème est « mon ami héberge et il me faut sa sauvegarde », pas « que mes sauvegardes me suivent ».

**Là où il s'arrête :** code fermé, Windows, dépendant de Steam comme transport, et une liste de jeux coop pris en charge plutôt que tout ce que vous possédez.

## Une note sur EmuDeck

EmuDeck revient dans ces discussions, et ce n'est pas un concurrent au sens habituel : c'est un installateur et configurateur d'émulateurs pour Steam Deck, et la synchronisation qu'il propose est un confort greffé sur cette mission (Rclone vers un cloud de fichiers, pour les sauvegardes d'émulateurs uniquement). Il recoupe les outils ci-dessus sans être de la même nature : EmuDeck installe vos émulateurs, les outils d'ici veillent sur les sauvegardes de toute la bibliothèque. Beaucoup font tourner EmuDeck à côté de l'un d'eux, et c'est une configuration sensée, pas une redondance.

## Hoard

Hoard prend la session de jeu comme unité. Le moteur tourne en service d'arrière-plan — \`hoardd\`, sans fenêtre, donc il fonctionne en mode jeu de SteamOS —, remarque que vous avez arrêté de jouer, et prend l'instantané à ce moment-là plutôt que de réagir à chaque écriture pendant la partie.

- **Historique versionné par session.** Chaque session est une version vers laquelle revenir, même après une panne de disque ou une réinstallation.
- **Déduplication par empreinte de contenu.** Dix versions d'une sauvegarde de 2 Go coûtent environ 2 Go, pas 20 Go. Les transferts sont compressés en zstd.
- **SHA-256 à la montée et à la descente.** La corruption est détectée avant de pouvoir écraser une bonne sauvegarde. Rien n'est jamais écrasé en silence : c'est tout le principe.
- **Cloud ou auto-hébergé, le même binaire.** Hoard Cloud a une offre gratuite (2 Go, 3 appareils, historique complet). Ou vous lancez \`hoard-server\` vous-même avec Docker Compose sur n'importe quel stockage compatible S3 — MinIO, Garage, Backblaze B2 — sans compte ni quota. AGPL-3.0.
- **Windows, Linux, macOS**, plus une CLI sans interface pour un Steam Deck ou un serveur.
- **Émulateurs en bêta :** PCSX2, RPCS3, Dolphin, Cemu, Ryujinx, RetroArch, DuckStation, PPSSPP et d'autres en préréglages.

## Le détail qui décide de la synchro Steam Deck ↔ PC

Bon à savoir quel que soit l'outil choisi. La sauvegarde cloud d'un jeu Steam vit dans \`<AppID>/remote/\`, et le dossier *au-dessus* contient \`remotecache.vdf\`, l'état des succès, les statistiques et les compteurs de temps de jeu — autant de choses qui diffèrent légitimement entre votre Deck et votre fixe.

Synchronisez le dossier parent et vous obtenez un conflit permanent entre deux machines qui n'ont jamais été en désaccord sur une seule sauvegarde. Hoard suit \`remote/\`, pas le dossier parent. N'importe quel outil auquel vous désignez un dossier à la main peut faire pareil, et c'est la première chose à vérifier quand une configuration de synchronisation signale des conflits sans raison visible.

## Là où Hoard perd

- **Il veut un serveur.** Compte cloud ou machine à vous, dans les deux cas c'est de l'infrastructure, alors qu'OpenSave ou Ludusavi n'en demandent aucune.
- **La prise en charge des émulateurs est en bêta.** Les installations portables et les manies de chaque émulateur le piègent encore, et Aletheia comme OpenSave couvrent aujourd'hui mieux certains cas particuliers de lanceurs et d'émulateurs.
- **macOS est à peine testé sur du matériel réel.** Ça compile et ça tourne, mais personne n'y a vécu pendant des mois.
- **C'est jeune.** Ludusavi et Game Backup Monitor ont des années de rapports de bugs derrière eux. Pas Hoard, et ça compte pour un logiciel qui garde une partie de 200 heures.
- **Il ne fait pas le partage coop.** Pour passer un monde à un ami, SaveSync est fait pour ça, Hoard non.

## La distinction entre Hoard Cloud et l'auto-hébergement

Les comparaisons sur Hoard confondent presque toujours les deux, et le résultat est faux. Autant le dire clairement :

- **Hoard Cloud** est l'option gérée : vous vous connectez, et vos sauvegardes sont stockées sur nos serveurs, dans l'UE.
- **Un Hoard auto-hébergé est entièrement le vôtre.** Vous faites tourner \`hoard-server\` sur votre PC ou votre NAS, et vos sauvegardes vont de votre machine à votre disque. Il n'y a **aucun compte chez nous, aucune télémétrie vers nous, aucun quota et aucun relais** : rien ne passe par nos serveurs, puisque rien de chez nous n'est sur le chemin. Nous ne voyons ni sauvegarde, ni nom de jeu, ni adresse e-mail, car rien de cela ne nous parvient. Si Hoard Cloud fermait demain, une installation auto-hébergée continuerait à l'identique.

Le même binaire, la même détection, le même historique. La seule chose qui change, c'est à qui appartient le stockage. Et pour être exact sur un point : votre serveur a bien ses propres accès — un utilisateur et un jeton par appareil — mais ils vivent dans votre base, pas dans la nôtre.

## Le tableau

| Outil | Synchro automatique entre appareils | Où vivent les sauvegardes | Historique | Plateformes | Licence |
|---|---|---|---|---|---|
| **Hoard** | Oui, par session de jeu | Hoard Cloud ou votre serveur (compatible S3) | Versionné par session, dédupliqué | Win · Linux · macOS · Deck | AGPL-3.0, offre gratuite |
| **Ludusavi** | Manuelle, ou Rclone que vous montez | Local, plus votre remote Rclone | Sauvegardes locales versionnées | Win · Linux · macOS | Gratuit, open source |
| **Syncthing** | Oui, miroir continu | Vos appareils seulement | Versionnage par fichier | Tout | Gratuit, open source |
| **OpenSave** | Oui, pair-à-pair | Vos appareils, réplication cloud optionnelle | Instantanés et branches | Win · Linux · Deck | MIT |
| **OpenCloudSaves** | Oui, via votre cloud | OneDrive / Drive / Dropbox / Nextcloud | Ce que garde le cloud | Win · Linux · macOS | Gratuit, open source |
| **Game Backup Monitor** | Non | Archives 7-Zip locales | Sauvegardes numérotées | Windows | Gratuit, open source |
| **Aletheia** | Sauvegarde et restauration par lanceur | Votre stockage | Sauvegardes | Win · Linux · macOS | AGPL-3.0 |
| **SaveSync** | Oui, et avec des amis | Entrées privées du Steam Workshop | Selon l'application | Windows | Payant, code fermé |

## Alors lequel

Si vous voulez une seule machine sauvegardée et rien d'autre, prenez Ludusavi ou Game Backup Monitor. Si vous refusez tout compte et que vos appareils sont généralement allumés ensemble, OpenSave. Si vos sauvegardes doivent atterrir dans un dossier Drive que vous payez déjà, OpenCloudSaves. Si vous partagez un monde coop avec des amis, SaveSync.

Si vous voulez que la sauvegarde *et* la synchronisation entre PC et Steam Deck se fassent toutes seules, avec une version par session où revenir et la possibilité de tout auto-héberger, c'est à ça que sert Hoard. [Téléchargez-le](/download), ou lisez d'abord [comment l'auto-héberger avec Docker](/guides/self-host-hoard). Il y a aussi un [comparatif détaillé avec Ludusavi](/guides/ludusavi-alternative) si c'est celui que vous mettez dans la balance.

## Comparaisons en tête-à-tête

Chacune va plus loin que la section ci-dessus, y compris sur les points où l'autre outil l'emporte :

- [Hoard face à Ludusavi](/guides/ludusavi-alternative)
- [Hoard comme alternative à Steam Cloud](/guides/steam-cloud-alternative)
- [Synchro pair-à-pair face à un serveur qui vous appartient](/guides/opensave-alternative)
- [Syncthing pour les sauvegardes : ce qui casse](/guides/syncthing-game-saves)

<!-- faq -->

## Questions fréquentes

### Lequel de ces outils garde un historique de versions ?

Hoard conserve chaque session comme une version où revenir. Ludusavi garde des sauvegardes locales versionnées. La plupart des autres synchronisent ou copient l'état actuel : une sauvegarde corrompue est donc fidèlement propagée à votre autre machine.

### Lequel fonctionne sans serveur ni compte ?

Ludusavi en sauvegardes locales, et tout outil pair-à-pair. Hoard entre aussi dans cette catégorie si vous l'auto-hébergez : aucun compte chez nous, et rien qui passe par nos serveurs.

### Lequel couvre les jeux absents de Steam ?

Tous les gestionnaires de sauvegardes cités, car ils localisent les fichiers via la même base communautaire et non via une boutique. L'exception est Steam Cloud : il ne couvre que les jeux Steam dont le développeur l'a activé.

### Dois-je n'en choisir qu'un ?

Non, et beaucoup ne le font pas. Un outil de sauvegarde locale et un outil de synchro règlent deux moitiés différentes du problème. La seule règle : ne jamais pointer l'un vers le dossier de sauvegardes de l'autre, sinon vous synchronisez un miroir périmé au lieu de votre sauvegarde réelle.

### Quel est le détail qui casse la plupart des montages maison ?

Synchroniser le dossier situé au-dessus de \`<AppID>/remote/\` dans le \`userdata\` de Steam. Le parent contient \`remotecache.vdf\` et des fichiers de succès et de temps de jeu censés différer d'une machine à l'autre : chaque lancement ressemble alors à un conflit alors qu'aucune sauvegarde n'a bougé.
`,Fe=`---
title: "Sincronizzazione dei salvataggi a confronto: Hoard contro Ludusavi, Syncthing, OpenSave e le altre"
description: "Confronto onesto degli strumenti che copiano e sincronizzano i salvataggi PC — Ludusavi, Syncthing, OpenSave, OpenCloudSaves, Game Backup Monitor, Aletheia, SaveSync e Hoard — con tabella e una sezione su dove Hoard perde."
order: 4
updated: 2026-09-01
---

Steam Cloud copre solo i giochi comprati su Steam, e solo quando lo sviluppatore si è preso la briga di attivarlo. Emulatori, GOG, Epic, itch.io, giochi non Steam, qualsiasi cosa con mod: niente di tutto questo rientra. Se giochi su più macchine, un fisso e uno Steam Deck per dire, finisci a copiare cartelle a mano sperando di aver preso la più recente.

Diversi strumenti risolvono la cosa, e non fanno tutti lo stesso. Alcuni fanno copie locali, altri replicano cartelle tra dispositivi, altri caricano su un cloud. Questa pagina li passa in rassegna e dice in cosa ciascuno è davvero bravo. Hoard è il mio progetto, quindi la parte onesta arriva alla fine: una sezione su dove Hoard perde, e una tabella che puoi leggere senza credere a una parola del testo.

## Ludusavi

Il più noto, e a ragione. Ludusavi (di mtkennerly) è uno strumento di backup gratuito e open source, con interfaccia e con CLI, costruito sul manifesto comunitario delle posizioni dei salvataggi che copre decine di migliaia di giochi: lo stesso manifesto che usano quasi tutti quelli di questa lista, Hoard compreso. Tiene copie locali versionate e può spingerle su un cloud tuo tramite Rclone.

**Il migliore se:** vuoi copie locali, controllo totale e nessun server da nessuna parte. È la scelta più sicura della lista e non costa nulla.

**Dove si ferma:** la sincronizzazione tra macchine è una cosa che monti tu. Pianifichi un backup, configuri un remote Rclone e ti ricordi di ripristinare sull'altro PC *prima* di giocare. Funziona, ma nulla ti impedisce di dimenticare l'ultimo passo.

## Syncthing

Non è affatto uno strumento per giochi: è uno specchio di cartelle peer-to-peer generico, e molto buono. Gli indichi una cartella di salvataggi e compare sugli altri dispositivi.

**Il migliore se:** lo usi già e vuoi i file in due posti senza cloud in mezzo.

**Dove si ferma:** replica, non fotografa. Un salvataggio corrotto raggiunge ogni dispositivo in pochi secondi, esattamente alla stessa velocità di uno buono. Il versionamento è per file, senza alcuna idea di cosa sia una sessione di gioco, quindi «torna a com'era martedì sera» te lo ricostruisci a mano. Due macchine che hanno giocato entrambe offline ti danno file di conflitto, non una fusione.

## OpenSave

Sincronizzazione peer-to-peer costruita apposta per i salvataggi, in Go, con licenza MIT, per Windows, Linux e Steam Deck. Nessun account, nessun server: i dispositivi si accoppiano tra loro e sincronizzano sulla rete locale o tramite un codice stanza su un relay. Ogni modifica diventa uno snapshot, ci sono i branch per partite parallele, i conflitti si risolvono per lignaggio di sincronizzazione invece che per orologio, e viaggiano solo i blocchi cambiati. Volendo può replicare su Drive, Dropbox, OneDrive o WebDAV.

**Il migliore se:** ti rifiuti di avere un account e i tuoi dispositivi sono accesi insieme abbastanza spesso.

**Dove si ferma:** peer-to-peer vuol dire che il salvataggio vive solo sui tuoi dispositivi. Se muore il Deck con l'unica copia recente e la replica non era configurata, è finita. Per sincronizzare devono essere accesi entrambi i dispositivi, e non c'è una build per macOS.

## OpenCloudSaves

Un'interfaccia multipiattaforma che sincronizza le cartelle dei salvataggi su un cloud che già paghi — OneDrive, Google Drive, Dropbox, Nextcloud — con Rclone sotto.

**Il migliore se:** vuoi i salvataggi in uno spazio di archiviazione che hai già, con un'interfaccia invece dei file di configurazione di Rclone.

**Dove si ferma:** non c'è deduplicazione a livello di contenuto. Dieci copie di un salvataggio da 2 GB sono 20 GB della tua quota Drive, e i cloud di file sincronizzano file, non sessioni di gioco: quel che recuperi è com'era la cartella in quel momento.

## Game Backup Monitor

Prima Windows, e il capostipite di tutto il genere. GBM sorveglia il processo del gioco e, quando esci, comprime il salvataggio con 7-Zip tenendo una cronologia numerata.

**Il migliore se:** sei su un solo PC Windows e vuoi un archivio locale compresso senza pensarci.

**Dove si ferma:** è uno strumento di backup, non di sincronizzazione. Portare l'archivio su una seconda macchina è affare tuo, e Steam Deck / SteamOS non è il suo terreno.

## Aletheia

Il più nuovo del gruppo, AGPL, e va proprio sulla parte che gli altri coprono a metà: i launcher. Heroic, itch.io, Lutris, Steam, GOG Galaxy e Xbox, su Windows, Linux e macOS.

**Il migliore se:** la tua libreria è sparsa tra launcher che gli altri strumenti rilevano male, soprattutto Xbox/Game Pass e Heroic.

**Dove si ferma:** è un progetto giovane con un perimetro volutamente stretto. Copia e ripristino sono tutto il set di funzioni; dietro non c'è un cloud versionato.

## SaveSync

Quello commerciale, venduto su Steam con acquisto unico, centrato su Windows. Il suo trucco è che non punta a te-su-due-PC ma al cooperativo: i salvataggi finiscono in voci private e non elencate dello Steam Workshop così che un amico possa scaricarsi il tuo mondo di Valheim o di Factorio, e c'è anche la sincronizzazione in rete locale.

**Il migliore se:** il problema che risolvi è «ospita il mio amico e mi serve il suo salvataggio», non «che i miei salvataggi mi seguano».

**Dove si ferma:** codice chiuso, Windows, legato a Steam come mezzo di trasporto, e un elenco di giochi cooperativi supportati invece di tutto quello che possiedi.

## Una nota su EmuDeck

EmuDeck salta fuori in queste discussioni e non è un concorrente nel senso normale: è un installatore e configuratore di emulatori per Steam Deck, e la sincronizzazione che offre è una comodità innestata su quel lavoro (Rclone verso un cloud di file, solo per i salvataggi degli emulatori). Si sovrappone agli strumenti qui sopra senza essere la stessa cosa: EmuDeck ti sistema gli emulatori, quelli di qui si occupano dei salvataggi dell'intera libreria. C'è chi usa EmuDeck accanto a uno di questi, ed è una configurazione sensata, non ridondante.

## Hoard

Hoard prende la sessione di gioco come unità. Il motore gira come servizio in background — \`hoardd\`, senza finestra, quindi funziona in modalità gioco su SteamOS —, si accorge che hai smesso di giocare e scatta lo snapshot allora, invece di reagire a ogni scrittura di file mentre giochi.

- **Cronologia versionata per sessione.** Ogni sessione è una versione a cui tornare, anche dopo un guasto al disco o un'installazione pulita.
- **Deduplicazione per hash del contenuto.** Dieci versioni di un salvataggio da 2 GB costano circa 2 GB, non 20 GB. I trasferimenti sono compressi con zstd.
- **SHA-256 in salita e in discesa.** La corruzione viene intercettata prima che possa sovrascrivere un salvataggio buono. Niente viene mai sovrascritto in silenzio: è tutto il senso del progetto.
- **Cloud o self-hosted, lo stesso binario.** Hoard Cloud ha un piano gratuito (2 GB, 3 dispositivi, cronologia completa). Oppure avvii \`hoard-server\` da solo con Docker Compose su qualsiasi archiviazione compatibile S3 — MinIO, Garage, Backblaze B2 — senza account e senza quota. AGPL-3.0.
- **Windows, Linux, macOS**, più una CLI senza interfaccia per uno Steam Deck o un server.
- **Emulatori in beta:** PCSX2, RPCS3, Dolphin, Cemu, Ryujinx, RetroArch, DuckStation, PPSSPP e altri come preimpostazioni.

## Il dettaglio che decide la sincronizzazione Steam Deck ↔ PC

Vale la pena saperlo qualunque strumento tu scelga. Il salvataggio cloud di un gioco Steam vive in \`<AppID>/remote/\`, e la cartella *sopra* contiene \`remotecache.vdf\`, lo stato degli obiettivi, le statistiche e i contatori delle ore giocate: tutte cose che legittimamente differiscono tra il Deck e il fisso.

Sincronizza la cartella padre e ottieni un conflitto permanente tra due macchine che non hanno mai discordato su un solo salvataggio. Hoard traccia \`remote/\`, non la cartella padre. A qualsiasi strumento a cui indichi una cartella a mano si può dire lo stesso, ed è la prima cosa da controllare quando una configurazione di sincronizzazione segnala conflitti senza motivo visibile.

## Dove Hoard perde

- **Vuole un server.** Account cloud o macchina tua, in ogni caso è infrastruttura, mentre OpenSave o Ludusavi non ne richiedono nessuna.
- **Il supporto agli emulatori è in beta.** Le installazioni portatili e le manie dei singoli emulatori lo colgono ancora in fallo, e oggi Aletheia e OpenSave coprono meglio certi casi limite di launcher ed emulatori.
- **macOS è provato pochissimo su hardware vero.** Compila e gira, ma nessuno ci ha vissuto per mesi.
- **È giovane.** Ludusavi e Game Backup Monitor hanno anni di segnalazioni alle spalle. Hoard no, e per qualcosa che custodisce una partita da 200 ore la differenza conta.
- **Non fa condivisione cooperativa.** Se vuoi passare un mondo a un amico, SaveSync è fatto per quello e Hoard no.

## La distinzione tra Hoard Cloud e self-hosting

I confronti su Hoard quasi sempre fondono i due in uno solo, e il risultato è sbagliato. Quindi, chiaramente:

- **Hoard Cloud** è l'opzione gestita: accedi e i tuoi salvataggi stanno sui nostri server, nell'UE.
- **Un Hoard self-hosted è interamente tuo.** Fai girare \`hoard-server\` sul tuo PC o NAS e i salvataggi vanno dalla tua macchina al tuo disco. **Nessun account con noi, nessuna telemetria verso di noi, nessuna quota e nessun relay**: non passa nulla dai nostri server, perché sul percorso non c'è niente di nostro. Non vediamo un salvataggio, il nome di un gioco o un indirizzo email, perché niente di tutto ciò ci arriva. Se Hoard Cloud chiudesse domani, un'installazione self-hosted continuerebbe uguale.

Stesso binario, stesso rilevamento, stessa cronologia. L'unica cosa che cambia è di chi è lo spazio di archiviazione. E per essere esatti su un dettaglio: il tuo server ha eccome i suoi accessi — un utente e un token per dispositivo — ma vivono nel tuo database, non nel nostro.

## La tabella

| Strumento | Sincronizzazione automatica tra dispositivi | Dove vivono i salvataggi | Cronologia | Piattaforme | Licenza |
|---|---|---|---|---|---|
| **Hoard** | Sì, per sessione di gioco | Hoard Cloud o un tuo server (compatibile S3) | Versionata per sessione, deduplicata | Win · Linux · macOS · Deck | AGPL-3.0, piano gratuito |
| **Ludusavi** | Manuale, o Rclone montato da te | Locale, più il tuo remote Rclone | Copie locali versionate | Win · Linux · macOS | Gratis, open source |
| **Syncthing** | Sì, specchio continuo | Solo i tuoi dispositivi | Versionamento per file | Tutto | Gratis, open source |
| **OpenSave** | Sì, peer-to-peer | I tuoi dispositivi, replica cloud opzionale | Snapshot e branch | Win · Linux · Deck | MIT |
| **OpenCloudSaves** | Sì, tramite il tuo cloud | OneDrive / Drive / Dropbox / Nextcloud | Quello che tiene il cloud | Win · Linux · macOS | Gratis, open source |
| **Game Backup Monitor** | No | Archivi 7-Zip locali | Backup numerati | Windows | Gratis, open source |
| **Aletheia** | Copia e ripristino per launcher | Il tuo spazio | Copie | Win · Linux · macOS | AGPL-3.0 |
| **SaveSync** | Sì, e con gli amici | Voci private dello Steam Workshop | Secondo l'app | Windows | A pagamento, codice chiuso |

## Quindi quale

Se vuoi una sola macchina messa al sicuro e nient'altro, prendi Ludusavi o Game Backup Monitor. Se non vuoi un account per nessun motivo e i tuoi dispositivi sono di solito accesi insieme, OpenSave. Se i salvataggi devono finire in una cartella di Drive che già paghi, OpenCloudSaves. Se condividi un mondo cooperativo con gli amici, SaveSync.

Se invece vuoi che copia *e* sincronizzazione tra PC e Steam Deck avvengano da sole, con una versione per sessione a cui tornare e la possibilità di ospitare tutto da te, è per questo che c'è Hoard. [Scaricalo](/download), o leggi prima [come ospitarlo da solo con Docker](/guides/self-host-hoard). C'è anche un [confronto approfondito con Ludusavi](/guides/ludusavi-alternative) se è quello che stai valutando.

## Confronti uno contro uno

Ognuno va più a fondo del blocco qui sopra, compresi i punti in cui vince l'altro strumento:

- [Hoard contro Ludusavi](/guides/ludusavi-alternative)
- [Hoard come alternativa a Steam Cloud](/guides/steam-cloud-alternative)
- [Sincronizzazione peer-to-peer contro un server tuo](/guides/opensave-alternative)
- [Syncthing per i salvataggi: cosa si rompe](/guides/syncthing-game-saves)

<!-- faq -->

## Domande frequenti

### Quale di questi strumenti tiene una cronologia delle versioni?

Hoard conserva ogni sessione come una versione a cui tornare. Ludusavi tiene backup locali versionati. Quasi tutti gli altri sincronizzano o copiano lo stato attuale, quindi un salvataggio corrotto viene propagato fedelmente all'altra macchina.

### Quale funziona senza server né account?

Ludusavi con i backup locali, e qualsiasi strumento peer-to-peer. Ci rientra anche Hoard se fai self-hosting: nessun account con noi e niente che passi dai nostri server.

### Quale copre i giochi che non stanno su Steam?

Tutti i gestori di salvataggi elencati, perché individuano i file tramite lo stesso database comunitario e non attraverso un negozio. L'eccezione è Steam Cloud: copre solo i giochi Steam il cui sviluppatore l'ha attivata.

### Devo sceglierne uno solo?

No, e molti non lo fanno. Uno strumento di backup locale e uno di sincronizzazione risolvono metà diverse del problema. L'unica regola è non puntare mai uno alla cartella di backup dell'altro, o finisci per sincronizzare un mirror vecchio invece del salvataggio reale.

### Qual è il dettaglio che rompe quasi tutti i setup fai-da-te?

Sincronizzare la cartella sopra \`<AppID>/remote/\` dentro \`userdata\` di Steam. Quella superiore contiene \`remotecache.vdf\` e i file di obiettivi e tempo di gioco, che devono differire da macchina a macchina: ogni avvio sembra un conflitto anche se nessun salvataggio si è mosso.
`,Ke=`---
title: "セーブデータ同期ツール比較：Hoard と Ludusavi・Syncthing・OpenSave ほか"
description: "PC のセーブデータをバックアップ・同期するツールの正直な比較。Ludusavi、Syncthing、OpenSave、OpenCloudSaves、Game Backup Monitor、Aletheia、SaveSync、Hoard を一覧表つきで比較し、Hoard が負けている点も書いています。"
order: 4
updated: 2026-09-01
---

Steam クラウドが守ってくれるのは Steam で買ったゲームだけ、しかも開発者が対応をオンにした場合に限られます。エミュレーター、GOG、Epic、itch.io、Steam 以外のゲーム、MOD を入れたもの——どれも対象外です。デスクトップと Steam Deck のように複数の環境で遊んでいると、結局フォルダーを手でコピーして、新しいほうを掴んだと信じるしかなくなります。

これを解決するツールはいくつもありますが、やっていることは同じではありません。ローカルにバックアップを取るもの、端末間でフォルダーをミラーするもの、クラウドへアップロードするもの。このページではそれぞれを見ていき、何が本当に得意なのかを書きます。Hoard は私のプロジェクトなので、正直な部分は最後に置きました。Hoard が負けている点の節と、本文を一切信じなくても読める比較表です。

## Ludusavi

いちばん有名で、それも当然の一本です。Ludusavi（作者は mtkennerly）は GUI と CLI を備えた無料のオープンソースのバックアップツールで、何万本ものゲームのセーブ位置を収録したコミュニティ製マニフェストの上に成り立っています。このページのほとんどのツール（Hoard も含む）が使っているのと同じマニフェストです。ローカルにバージョン付きのバックアップを保持し、Rclone を設定すれば自分のクラウドへ送れます。

**向いているのは：** ローカルのバックアップと完全な制御が欲しくて、サーバーはどこにも置きたくない人。このリストで最も安全な選択で、しかも無料です。

**足りないところ：** 端末間の同期は自分で組み立てるものになります。バックアップを予約し、Rclone のリモートを設定し、遊ぶ*前*に別の PC で復元するのを忘れない。動きはしますが、最後の一手を忘れるのを止めてくれるものは何もありません。

## Syncthing

そもそもゲーム用ではなく、汎用の P2P フォルダーミラーで、しかも良い出来です。セーブフォルダーを指定すれば、ほかの端末にも現れます。

**向いているのは：** すでに動かしていて、クラウドを挟まずにファイルを二か所に置きたい人。

**足りないところ：** ミラーであって、スナップショットではありません。壊れたセーブも、正常なセーブとまったく同じ速さで数秒のうちに全端末へ届きます。ファイル単位のバージョン管理はありますが、プレイセッションという概念はないので、「火曜の夜の状態に戻す」は手作業で組み直すことになります。両方の端末がオフラインで遊んでいれば、返ってくるのは競合ファイルであってマージではありません。

## OpenSave

セーブデータ専用に作られた P2P 同期。Go 製、MIT ライセンス、Windows・Linux・Steam Deck 対応です。アカウントもサーバーも不要で、端末同士をペアリングして LAN 経由、あるいはリレーのルームコード経由で同期します。変更のたびにスナップショットを取り、並行プレイ用のブランチがあり、競合は時計ではなく同期の系譜で解決し、転送は変化したブロックだけ。任意で Drive・Dropbox・OneDrive・WebDAV へのミラーもできます。

**向いているのは：** アカウントは絶対に作りたくなくて、端末が同時に起動している機会が十分にある人。

**足りないところ：** P2P である以上、セーブはあなたの端末の上にしか存在しません。最新のコピーを持っていた Deck が壊れ、ミラーを設定していなければそれで終わりです。同期には両方の端末が動いている必要があり、macOS 版はありません。

## OpenCloudSaves

すでに料金を払っているクラウド——OneDrive、Google Drive、Dropbox、Nextcloud——へセーブフォルダーを同期する、マルチプラットフォームの GUI です。中身は Rclone です。

**向いているのは：** すでに持っているストレージにセーブを置きたくて、Rclone の設定ファイルではなく画面で操作したい人。

**足りないところ：** 内容ベースの重複排除がありません。2 GB のセーブが 10 世代あれば Drive の容量を 20 GB 食いますし、クラウドドライブが同期するのはファイルであってプレイセッションではないので、戻ってくるのは「その時点のフォルダーの姿」だけです。

## Game Backup Monitor

Windows 中心で、このジャンルの元祖です。GBM はゲームのプロセスを見張り、終了した時点でセーブを 7-Zip で圧縮し、連番の履歴として残します。

**向いているのは：** Windows PC 一台で、何も考えずに圧縮済みのローカルアーカイブが欲しい人。

**足りないところ：** バックアップのツールであって同期のツールではありません。アーカイブを二台目に持っていくのは自分の仕事ですし、Steam Deck / SteamOS は得意分野ではありません。

## Aletheia

この中では最も新しく、AGPL。ほかが中途半端にしか押さえていない部分、つまりランチャーを正面から狙っています。Heroic、itch.io、Lutris、Steam、GOG Galaxy、Xbox に、Windows・Linux・macOS 対応。

**向いているのは：** ライブラリが、ほかのツールでは検出しづらいランチャー——とくに Xbox / Game Pass と Heroic——に散らばっている人。

**足りないところ：** 意図的に範囲を絞った若いプロジェクトです。機能はバックアップと復元まで。背後にバージョン管理されたクラウドがあるわけではありません。

## SaveSync

唯一の商用で、Steam で買い切り販売、Windows 中心。特徴は、狙いが「二台の PC を使う自分」ではなく協力プレイにあることです。セーブは非公開・非掲載の Steam ワークショップの項目として保存され、友達があなたの Valheim や Factorio のワールドを持っていけます。LAN 同期もあります。

**向いているのは：** 解決したい問題が「自分のセーブについてきてほしい」ではなく「友達がホストで、その人のセーブが要る」である人。

**足りないところ：** クローズドソース、Windows、転送路として Steam に依存、そして対応するのは所有物すべてではなく協力プレイ向けの対応ゲーム一覧です。

## EmuDeck についての注記

この手の話題では EmuDeck も名前が挙がりますが、通常の意味での競合ではありません。Steam Deck 向けのエミュレーターのインストーラー兼設定ツールであり、備わっている同期はその仕事に付け足された利便機能です（クラウドドライブに対する Rclone、しかもエミュレーターのセーブ限定）。上のツール群と重なる部分はあっても、同じ種類のものではありません。EmuDeck はエミュレーター環境を整えるもの、ここで挙げたものはライブラリ全体のセーブを見守るもの。EmuDeck とどれか一つを併用している人もいて、それは重複ではなく理にかなった構成です。

## Hoard

Hoard はプレイセッションを単位として扱います。エンジンはバックグラウンドサービスとして動き（\`hoardd\`、ウィンドウを持たないので SteamOS のゲームモードでも動作します）、遊び終わったことを検知してからスナップショットを取ります。プレイ中のファイル書き込みに逐一反応するのではありません。

- **セッションごとのバージョン履歴。** どのセッションにも戻れます。ディスク故障のあとでも、クリーンインストールのあとでも。
- **内容ハッシュによる重複排除。** 2 GB のセーブが 10 世代あっても消費はおよそ 2 GB で、20 GB にはなりません。転送は zstd で圧縮されます。
- **アップロード時とダウンロード時の SHA-256。** 破損は、正常なセーブを上書きする前に検出されます。何も黙って上書きされない——設計の核はそこにあります。
- **クラウドでも自己ホストでも、同じバイナリ。** Hoard Cloud には無料プラン（2 GB、3 台、履歴は全部）があります。あるいは \`hoard-server\` を Docker Compose で自分で立て、S3 互換ストレージ（MinIO、Garage、Backblaze B2）に対して動かせば、アカウントも容量制限もありません。AGPL-3.0。
- **Windows・Linux・macOS**、加えて Steam Deck やサーバー向けのヘッドレス CLI。
- **エミュレーターはベータ：** PCSX2、RPCS3、Dolphin、Cemu、Ryujinx、RetroArch、DuckStation、PPSSPP ほかをプリセットで用意。

## Steam Deck ↔ PC の同期を左右する細部

どのツールを選ぶにしても知っておく価値があります。Steam のゲームのクラウドセーブは \`<AppID>/remote/\` にあり、その*一つ上*のフォルダーには \`remotecache.vdf\`、実績の状態、統計、プレイ時間のカウンターが入っています。これらは Deck とデスクトップとで違っていて当たり前のものです。

親フォルダーを同期すれば、セーブについては一度も食い違っていない二台の間で、恒久的な競合が起きます。Hoard が追いかけるのは \`remote/\` であって親フォルダーではありません。フォルダーを手動で指定できるツールなら同じ設定にできますし、同期の構成が理由もなく競合を出し続けるときに最初に確認すべき点でもあります。

## Hoard が負けている点

- **サーバーを欲しがる。** クラウドのアカウントか自前のマシンか、いずれにせよインフラです。OpenSave や Ludusavi はどちらも必要としません。
- **エミュレーター対応はベータ。** ポータブル構成や各エミュレーターの癖にまだ足をすくわれますし、ランチャーやエミュレーターの一部の特殊なケースは今日のところ Aletheia や OpenSave のほうがうまく扱えます。
- **macOS は実機での検証がほとんどない。** ビルドも起動もしますが、何か月も常用した人がいません。
- **歴史が浅い。** Ludusavi や Game Backup Monitor には何年分ものバグ報告が積み上がっています。Hoard にはそれがなく、200 時間のセーブを預かるものとしては軽くない差です。
- **協力プレイの共有はできない。** 友達にワールドを渡したいなら、それは SaveSync のための仕事で、Hoard の仕事ではありません。

## Hoard Cloud とセルフホストの違い

Hoard についての比較は、ほぼ必ずこの 2 つを一緒くたにし、その結果として誤った説明になります。はっきり書いておきます。

- **Hoard Cloud** はマネージドな選択肢です。サインインすると、セーブは EU にある当方のサーバーに保存されます。
- **セルフホストした Hoard は完全にあなたのものです。** 自分の PC や NAS で \`hoard-server\` を動かせば、セーブは自分のマシンから自分のディスクへ移ります。**当方のアカウントも、当方へのテレメトリも、容量制限も、中継もありません。** 経路上に当方のものが何一つないため、当方のサーバーを何も通りません。セーブもゲーム名もメールアドレスも見えません。届かないからです。仮に明日 Hoard Cloud が終了しても、セルフホスト構成はそのまま動き続けます。

同じバイナリ、同じ検出、同じ世代履歴。変わるのは保存先が誰のものかだけです。正確を期して 1 点だけ補うと、あなたのサーバーには確かに自前のログイン、つまりユーザーと端末ごとのトークンがありますが、それらはあなたのデータベースの中にあり、当方のデータベースにはありません。

## 比較表

| ツール | 端末間の自動同期 | セーブの置き場所 | 履歴 | 対応環境 | ライセンス |
|---|---|---|---|---|---|
| **Hoard** | あり（プレイセッション単位） | Hoard Cloud または自前サーバー（S3 互換） | セッション単位のバージョン、重複排除あり | Win · Linux · macOS · Deck | AGPL-3.0、無料プランあり |
| **Ludusavi** | 手動、または自分で組む Rclone | ローカル＋自分の Rclone リモート | バージョン付きローカルバックアップ | Win · Linux · macOS | 無料・オープンソース |
| **Syncthing** | あり（常時ミラー） | 自分の端末のみ | ファイル単位のバージョン | すべて | 無料・オープンソース |
| **OpenSave** | あり（P2P） | 自分の端末、任意でクラウドミラー | スナップショットとブランチ | Win · Linux · Deck | MIT |
| **OpenCloudSaves** | あり（自分のクラウド経由） | OneDrive / Drive / Dropbox / Nextcloud | クラウド側が保持する範囲 | Win · Linux · macOS | 無料・オープンソース |
| **Game Backup Monitor** | なし | ローカルの 7-Zip アーカイブ | 連番バックアップ | Windows | 無料・オープンソース |
| **Aletheia** | ランチャーごとのバックアップと復元 | 自分のストレージ | バックアップ | Win · Linux · macOS | AGPL-3.0 |
| **SaveSync** | あり（友達とも） | 非公開の Steam ワークショップ項目 | アプリの仕様による | Windows | 有料・クローズドソース |

## で、どれを選ぶか

一台だけ守れれば十分なら Ludusavi か Game Backup Monitor。アカウントだけは何があっても作りたくなくて、端末がだいたい同時に起動しているなら OpenSave。すでに料金を払っている Drive のフォルダーにセーブを置きたいなら OpenCloudSaves。友達と協力プレイのワールドを共有したいなら SaveSync。

バックアップ*と*、PC と Steam Deck をまたぐ同期が勝手に行われること、セッションごとに戻れるバージョンがあること、そして全部を自分でホストできる選択肢があること——それを求めるなら Hoard です。[ダウンロード](/download)するか、先に[Docker で自己ホストする方法](/guides/self-host-hoard)を読んでみてください。天秤にかけている相手が Ludusavi なら、[詳しい比較](/guides/ludusavi-alternative)もあります。

## 一対一の比較

以下はそれぞれ、上の節より踏み込んで扱っています。相手のほうが優れている点も含みます。

- [Hoard と Ludusavi](/guides/ludusavi-alternative)
- [Steam クラウドの代替としての Hoard](/guides/steam-cloud-alternative)
- [ピアツーピア同期と自分のサーバー](/guides/opensave-alternative)
- [Syncthing でセーブを同期すると何が壊れるか](/guides/syncthing-game-saves)

<!-- faq -->

## よくある質問

### この中で世代履歴を残すのはどれですか？

Hoard は 1 セッションを 1 世代として残し、そこへ戻れます。Ludusavi は世代管理されたローカルバックアップを保持します。その他の多くは現在の状態を同期・コピーするだけなので、壊れたセーブはそのまま忠実にもう 1 台へ伝わります。

### サーバーもアカウントもなしで使えるのはどれですか？

ローカルバックアップとしての Ludusavi と、ピアツーピアのツール全般です。セルフホストするなら Hoard も該当します。当方のアカウントはなく、当方のサーバーを通るものもありません。

### Steam にないゲームをカバーするのはどれですか？

ここに挙げたセーブ管理ツールはすべてカバーします。ストア経由ではなく、同じコミュニティのデータベースでセーブの場所を突き止めるからです。例外は Steam クラウドで、開発者が有効にした Steam のゲームしか対象になりません。

### 1 つだけ選ばないといけませんか？

いいえ。実際、多くの人は選んでいません。ローカルバックアップのツールと同期のツールは、問題の別々の半分を解いています。唯一の注意は、一方をもう一方のバックアップフォルダーに向けないこと。向けると、実際のセーブではなく古い写しを同期することになります。

### 自作構成がいちばん壊れる原因は何ですか？

Steam の \`userdata\` にある \`<AppID>/remote/\` の 1 つ上のフォルダーを同期することです。上のフォルダーには \`remotecache.vdf\` や、マシンごとに違って当然の実績・プレイ時間のファイルが入っているため、セーブが動いていなくても起動のたびに競合に見えます。
`,Qe=`---
title: "Sincronização de saves comparada: Hoard frente a Ludusavi, Syncthing, OpenSave e as outras"
description: "Comparação honesta das ferramentas que fazem backup e sincronizam saves de PC — Ludusavi, Syncthing, OpenSave, OpenCloudSaves, Game Backup Monitor, Aletheia, SaveSync e Hoard — com tabela e uma secção sobre onde o Hoard perde."
order: 4
updated: 2026-09-01
---

A Steam Cloud só cobre jogos comprados na Steam, e apenas quando o programador se deu ao trabalho de a ligar. Emuladores, GOG, Epic, itch.io, jogos fora da Steam, qualquer coisa com mods: nada disso entra. Se jogas em mais do que uma máquina, um desktop e uma Steam Deck por exemplo, acabas a copiar pastas à mão na esperança de teres apanhado a mais recente.

Há várias ferramentas que resolvem isto, e não fazem todas o mesmo. Umas fazem cópias locais, outras espelham pastas entre dispositivos, outras enviam para uma nuvem. Esta página passa por elas e diz em que é que cada uma é genuinamente boa. O Hoard é o meu projeto, por isso a parte honesta fica no fim: uma secção sobre onde o Hoard perde, e uma tabela que podes ler sem acreditar numa única linha do texto.

## Ludusavi

O mais conhecido, e com razão. O Ludusavi (de mtkennerly) é uma ferramenta de backup gratuita e open source, com interface e com CLI, construída sobre o manifesto comunitário de localizações de saves que cobre dezenas de milhares de jogos — o mesmo manifesto que quase todas as desta lista usam, o Hoard incluído. Guarda cópias locais versionadas e pode enviá-las para uma nuvem tua através do Rclone.

**Melhor se:** queres cópias locais, controlo total e nenhum servidor em lado nenhum. É a escolha mais segura da lista e não custa nada.

**Onde para:** a sincronização entre máquinas é algo que montas tu. Agendar um backup, configurar um remote do Rclone e lembrares-te de restaurar no outro PC *antes* de jogar. Funciona, mas nada te impede de esquecer o último passo.

## Syncthing

Não é sequer uma ferramenta de jogos: é um espelho de pastas peer-to-peer de uso geral, e muito bom. Apontas-lhe uma pasta de saves e ela aparece nos teus outros dispositivos.

**Melhor se:** já o tens a correr e queres os ficheiros em dois sítios sem nuvem pelo meio.

**Onde para:** espelha, não fotografa. Um save corrompido chega a todos os dispositivos em segundos, exatamente à mesma velocidade de um bom. O versionamento é por ficheiro, sem ideia nenhuma do que é uma sessão de jogo, por isso «voltar a como estava na terça à noite» é algo que reconstróis à mão. Duas máquinas que jogaram offline dão-te ficheiros de conflito, não uma fusão.

## OpenSave

Sincronização peer-to-peer feita de propósito para saves, em Go, com licença MIT, para Windows, Linux e Steam Deck. Sem conta e sem servidor: os dispositivos emparelham entre si e sincronizam pela rede local ou através de um código de sala num relay. Cada alteração vira um snapshot, há branches para partidas paralelas, os conflitos resolvem-se por linhagem de sincronização em vez de pelo relógio, e só viajam os blocos que mudaram. Opcionalmente pode espelhar para Drive, Dropbox, OneDrive ou WebDAV.

**Melhor se:** recusas ter uma conta e os teus dispositivos estão ligados ao mesmo tempo com frequência suficiente.

**Onde para:** peer-to-peer significa que o save só vive nos teus dispositivos. Se morre a Deck com a única cópia recente e o espelho nunca foi configurado, acabou. Os dois dispositivos têm de estar ligados para haver sincronização, e não há versão para macOS.

## OpenCloudSaves

Uma interface multiplataforma que sincroniza as tuas pastas de saves para uma nuvem que já pagas — OneDrive, Google Drive, Dropbox, Nextcloud — com o Rclone por baixo.

**Melhor se:** queres os saves numa conta de armazenamento que já tens, com interface em vez de ficheiros de configuração do Rclone.

**Onde para:** não há desduplicação ao nível do conteúdo. Dez cópias de um save de 2 GB são 20 GB da tua quota do Drive, e as nuvens de ficheiros sincronizam ficheiros, não sessões de jogo, por isso o que recuperas é como a pasta estava naquele momento.

## Game Backup Monitor

Windows primeiro, e o original de todo este género. O GBM vigia o processo do jogo e, quando sais, comprime o save com 7-Zip e guarda um histórico numerado.

**Melhor se:** estás num único PC com Windows e queres um arquivo local comprimido sem pensar nisso.

**Onde para:** é uma ferramenta de backup, não de sincronização. Levar o arquivo para uma segunda máquina é problema teu, e a Steam Deck / SteamOS não é o seu terreno.

## Aletheia

O mais recente do grupo, AGPL, e vai exatamente à parte que os outros cobrem pela metade: os launchers. Heroic, itch.io, Lutris, Steam, GOG Galaxy e Xbox, em Windows, Linux e macOS.

**Melhor se:** a tua biblioteca está espalhada por launchers que as outras ferramentas detetam mal, sobretudo Xbox/Game Pass e Heroic.

**Onde para:** é um projeto jovem com um âmbito propositadamente estreito. Fazer cópia e restaurar é todo o conjunto de funcionalidades; não há uma nuvem versionada por trás.

## SaveSync

O comercial, vendido na Steam como compra única, virado para Windows. O truque dele é que não aponta a ti-em-dois-PC, mas ao cooperativo: os saves vão para entradas privadas e não listadas da Steam Workshop para que um amigo possa puxar o teu mundo de Valheim ou de Factorio, e também há sincronização por rede local.

**Melhor se:** o problema que resolves é «o meu amigo aloja e preciso do save dele», não «que os meus saves me sigam».

**Onde para:** código fechado, Windows, preso à Steam como meio de transporte, e uma lista de jogos cooperativos suportados em vez de tudo o que tens.

## Uma nota sobre o EmuDeck

O EmuDeck aparece nestas conversas e não é um concorrente no sentido normal: é um instalador e configurador de emuladores para a Steam Deck, e a sincronização que oferece é uma comodidade acoplada a esse trabalho (Rclone contra uma nuvem de ficheiros, só para saves de emulador). Sobrepõe-se às ferramentas acima sem ser a mesma coisa: o EmuDeck deixa-te os emuladores montados, e as daqui tomam conta dos saves da biblioteca toda. Há quem use o EmuDeck ao lado de uma destas, e é uma montagem sensata, não redundante.

## Hoard

O Hoard toma a sessão de jogo como unidade. O motor corre como serviço em segundo plano — \`hoardd\`, sem janela, por isso funciona no modo de jogo do SteamOS —, dá-se conta de que paraste de jogar e faz o snapshot nessa altura, em vez de reagir a cada escrita de ficheiro a meio da partida.

- **Histórico versionado por sessão.** Cada sessão é uma versão à qual podes voltar, mesmo depois de uma falha de disco ou de uma instalação limpa.
- **Desduplicação por hash de conteúdo.** Dez versões de um save de 2 GB custam cerca de 2 GB, não 20 GB. As transferências vão comprimidas com zstd.
- **SHA-256 à subida e à descida.** A corrupção é apanhada antes de poder sobrescrever um save bom. Nada é sobrescrito em silêncio: é esse o desenho todo.
- **Nuvem ou auto-alojado, o mesmo binário.** O Hoard Cloud tem plano gratuito (2 GB, 3 dispositivos, histórico completo). Ou levantas o \`hoard-server\` tu mesmo com Docker Compose contra qualquer armazenamento compatível com S3 — MinIO, Garage, Backblaze B2 — sem conta e sem quota. AGPL-3.0.
- **Windows, Linux, macOS**, mais uma CLI sem interface para uma Steam Deck ou um servidor.
- **Emuladores em beta:** PCSX2, RPCS3, Dolphin, Cemu, Ryujinx, RetroArch, DuckStation, PPSSPP e outros como predefinições.

## O detalhe que decide a sincronização Steam Deck ↔ PC

Vale a pena saber, escolhas a ferramenta que escolheres. O save na nuvem de um jogo da Steam vive em \`<AppID>/remote/\`, e a pasta *acima* guarda o \`remotecache.vdf\`, o estado das conquistas, estatísticas e contadores de horas jogadas — coisas que legitimamente diferem entre a tua Deck e o teu desktop.

Sincroniza a pasta-mãe e ficas com um conflito permanente entre duas máquinas que nunca discordaram sobre um único save. O Hoard segue \`remote/\`, não a pasta-mãe. A qualquer ferramenta a que apontes uma pasta à mão pode dizer-se o mesmo, e é a primeira coisa a verificar quando uma configuração de sincronização assinala conflitos sem motivo visível.

## Onde o Hoard perde

- **Quer um servidor.** Conta na nuvem ou máquina tua, de qualquer forma é infraestrutura, e o OpenSave ou o Ludusavi não precisam de nenhuma.
- **O suporte a emuladores está em beta.** As instalações portáteis e as manias de cada emulador ainda o apanham, e hoje o Aletheia e o OpenSave cobrem melhor alguns casos limite de launchers e emuladores.
- **O macOS está mal testado em hardware real.** Compila e funciona, mas ninguém viveu lá durante meses.
- **É jovem.** O Ludusavi e o Game Backup Monitor têm anos de relatos de bugs atrás deles. O Hoard não, e isso pesa em algo que guarda um save de 200 horas.
- **Não faz partilha cooperativa.** Se queres passar um mundo a um amigo, o SaveSync foi feito para isso e o Hoard não.

## A distinção entre Hoard Cloud e self-hosting

As comparações sobre o Hoard quase sempre fundem os dois num só, e o resultado sai errado. Por isso, de forma clara:

- **O Hoard Cloud** é a opção gerida: inicias sessão e os teus saves ficam nos nossos servidores, na UE.
- **Um Hoard self-hosted é inteiramente teu.** Corres o \`hoard-server\` no teu PC ou NAS e os saves vão da tua máquina para o teu disco. **Não há conta connosco, nem telemetria para nós, nem quota, nem retransmissão**: não passa nada pelos nossos servidores, porque não há nada nosso no caminho. Não vemos um save, o nome de um jogo ou um email, porque nada disso nos chega. Se o Hoard Cloud fechasse amanhã, uma instalação self-hosted continuaria igual.

O mesmo binário, a mesma deteção, o mesmo histórico. A única coisa que muda é de quem é o armazenamento. E, para ser exato num detalhe: o teu servidor tem sim os seus próprios acessos — um utilizador e um token por dispositivo — mas vivem na tua base de dados, não na nossa.

## A tabela

| Ferramenta | Sincronização automática entre dispositivos | Onde vivem os saves | Histórico | Plataformas | Licença |
|---|---|---|---|---|---|
| **Hoard** | Sim, por sessão de jogo | Hoard Cloud ou servidor teu (compatível com S3) | Versionado por sessão, desduplicado | Win · Linux · macOS · Deck | AGPL-3.0, plano gratuito |
| **Ludusavi** | Manual, ou Rclone montado por ti | Local, mais o teu remote do Rclone | Cópias locais versionadas | Win · Linux · macOS | Grátis, open source |
| **Syncthing** | Sim, espelho contínuo | Só os teus dispositivos | Versionamento por ficheiro | Tudo | Grátis, open source |
| **OpenSave** | Sim, peer-to-peer | Os teus dispositivos, espelho opcional na nuvem | Snapshots e branches | Win · Linux · Deck | MIT |
| **OpenCloudSaves** | Sim, através da tua nuvem | OneDrive / Drive / Dropbox / Nextcloud | O que a nuvem guardar | Win · Linux · macOS | Grátis, open source |
| **Game Backup Monitor** | Não | Arquivos 7-Zip locais | Cópias numeradas | Windows | Grátis, open source |
| **Aletheia** | Cópia e restauro por launcher | O teu armazenamento | Cópias | Win · Linux · macOS | AGPL-3.0 |
| **SaveSync** | Sim, e com amigos | Entradas privadas da Steam Workshop | Conforme a app | Windows | Pago, código fechado |

## Então qual

Se queres uma máquina protegida e mais nada, leva o Ludusavi ou o Game Backup Monitor. Se não queres uma conta em circunstância alguma e os teus dispositivos costumam estar ligados ao mesmo tempo, o OpenSave. Se os saves devem ir parar a uma pasta do Drive que já pagas, o OpenCloudSaves. Se partilhas um mundo cooperativo com amigos, o SaveSync.

Se o que queres é que a cópia *e* a sincronização entre PC e Steam Deck aconteçam sozinhas, com uma versão por sessão à qual voltar e a opção de alojar tudo tu, é para isso que serve o Hoard. [Descarrega-o](/download), ou lê primeiro [como alojá-lo com Docker](/guides/self-host-hoard). Há também uma [comparação longa com o Ludusavi](/guides/ludusavi-alternative) se for essa a que estás a pesar.

## Comparações um para um

Cada uma vai mais fundo do que o bloco acima, incluindo onde a outra ferramenta ganha:

- [Hoard frente ao Ludusavi](/guides/ludusavi-alternative)
- [Hoard como alternativa à Steam Cloud](/guides/steam-cloud-alternative)
- [Sincronização ponto a ponto frente a um servidor teu](/guides/opensave-alternative)
- [Syncthing para saves: o que parte](/guides/syncthing-game-saves)

<!-- faq -->

## Perguntas frequentes

### Qual destas ferramentas guarda histórico de versões?

O Hoard conserva cada sessão como uma versão à qual podes voltar. O Ludusavi guarda cópias locais versionadas. A maioria das restantes sincroniza ou copia o estado atual, o que significa que um save corrompido é propagado fielmente para a outra máquina.

### Qual funciona sem servidor nem conta?

O Ludusavi com cópias locais, e qualquer ferramenta ponto a ponto. O Hoard também entra se fizeres self-hosting: sem conta connosco e sem nada a passar pelos nossos servidores.

### Qual cobre jogos que não estão na Steam?

Todos os gestores de saves aqui listados, porque localizam os ficheiros pela mesma base de dados comunitária e não através de uma loja. A exceção é a Steam Cloud: só cobre jogos da Steam cujo programador a ativou.

### Tenho de escolher só uma?

Não, e muita gente não escolhe. Uma ferramenta de cópia local e uma de sincronização resolvem metades diferentes do problema. A única regra é nunca apontar uma para a pasta de cópias da outra, ou acabas a sincronizar um espelho desatualizado em vez do teu save real.

### Qual é o detalhe que parte a maioria das montagens caseiras?

Sincronizar a pasta acima de \`<AppID>/remote/\` no \`userdata\` da Steam. A de cima guarda \`remotecache.vdf\` e ficheiros de proezas e tempo de jogo que devem ser diferentes em cada máquina, por isso cada arranque parece um conflito mesmo sem nenhum save se ter mexido.
`,$e=`---
title: "游戏存档同步工具对比：Hoard 与 Ludusavi、Syncthing、OpenSave 等"
description: "对备份与同步 PC 游戏存档的各款工具做一次诚实对比——Ludusavi、Syncthing、OpenSave、OpenCloudSaves、Game Backup Monitor、Aletheia、SaveSync 和 Hoard——附对比表，以及 Hoard 输在哪里的一节。"
order: 4
updated: 2026-09-01
---

Steam 云存档只覆盖你在 Steam 上买的游戏，而且还得开发者愿意打开这个开关。模拟器、GOG、Epic、itch.io、非 Steam 的游戏、任何装了 MOD 的东西，统统不在其中。如果你在不止一台机器上玩——比如一台台式机加一台 Steam Deck——最后就是手动复制文件夹，然后祈祷自己拿的是最新的那一份。

有好几款工具在解决这件事，而它们做的并不是同一件事。有的做本地备份，有的在设备之间镜像文件夹，有的上传到云端。这一页把它们逐个过一遍，说清楚每一款真正擅长什么。Hoard 是我自己的项目，所以诚实的部分放在最后：一节讲 Hoard 输在哪里，再加一张即使你一个字都不信正文也能读的表。

## Ludusavi

最有名的一款，而且名副其实。Ludusavi（作者 mtkennerly）是一款免费开源的备份工具，有图形界面也有命令行，建立在收录了数万款游戏存档位置的社区清单之上——本页几乎所有工具用的都是同一份清单，Hoard 也不例外。它保留带版本的本地备份，并且可以通过配置 Rclone 推送到你自己的云。

**适合：** 想要本地备份、完全掌控，并且任何地方都不想有服务器的人。它是这份名单里最稳妥的选择，而且一分钱不花。

**止步之处：** 跨机器同步是要你自己拼出来的。安排备份、配置 Rclone 远端，然后记得在开玩*之前*到另一台电脑上还原。它确实可行，但没有任何东西会阻止你忘掉最后一步。

## Syncthing

它根本不是游戏工具，而是一个通用的点对点文件夹镜像，而且做得很好。把存档文件夹指给它，它就会出现在你其他设备上。

**适合：** 你本来就在用它，并且希望文件同时存在两处、中间不经过任何云。

**止步之处：** 它做的是镜像，不是快照。一个损坏的存档会在几秒内到达每一台设备，速度和一个完好的存档一模一样。它的版本保留是按文件的，完全不知道"一局游戏"是什么概念，所以"回到周二晚上的样子"得靠你手工拼回来。两台机器都离线玩过之后，你拿到的是冲突文件，不是合并结果。

## OpenSave

专为存档而做的点对点同步，用 Go 写成，MIT 许可，支持 Windows、Linux 和 Steam Deck。不需要账号也不需要服务器：设备之间互相配对，通过局域网或中继的房间码同步。每次改动都会生成快照，有分支可以放平行的存档进度，冲突按同步谱系而不是按时钟解决，传输只走变化的数据块。也可以选择镜像到 Drive、Dropbox、OneDrive 或 WebDAV。

**适合：** 说什么都不肯注册账号，而且设备同时开机的机会足够多的人。

**止步之处：** 点对点意味着存档只活在你自己的设备上。如果那台存着唯一一份最新存档的 Deck 坏了，而镜像又从来没配置过，那就到此为止。要同步，两台设备都得开着；另外没有 macOS 版本。

## OpenCloudSaves

一个跨平台的图形界面，把你的存档文件夹同步到你已经在付费的云上——OneDrive、Google Drive、Dropbox、Nextcloud——底层用的是 Rclone。

**适合：** 想把存档放进已经拥有的存储空间，并且宁可点界面也不想写 Rclone 配置文件的人。

**止步之处：** 没有基于内容的去重。一个 2 GB 存档保十份，就是吃掉你 Drive 配额的 20 GB；而且网盘同步的是文件而不是游戏会话，你取回来的只是文件夹当时的样子。

## Game Backup Monitor

以 Windows 为主，也是整个门类的鼻祖。GBM 盯着游戏进程，等你退出时用 7-Zip 压缩存档，并保留一份编号的历史。

**适合：** 只有一台 Windows 电脑，想要一份压缩好的本地归档、完全不用动脑的人。

**止步之处：** 它是备份工具，不是同步工具。把归档弄到第二台机器上是你自己的事，而 Steam Deck / SteamOS 也不是它的主场。

## Aletheia

这一组里最新的一款，AGPL 许可，而且专攻别人都只覆盖了一半的那块：启动器。Heroic、itch.io、Lutris、Steam、GOG Galaxy 和 Xbox，覆盖 Windows、Linux 和 macOS。

**适合：** 游戏库散落在其他工具识别得不好的启动器上，尤其是 Xbox / Game Pass 和 Heroic。

**止步之处：** 这是一个年轻的项目，范围也是刻意收窄的。功能就是备份和还原，背后并没有一个带版本的云。

## SaveSync

唯一的商业产品，在 Steam 上买断制出售，以 Windows 为主。它的巧思在于：它瞄准的并不是"你和你的两台电脑"，而是联机合作。存档会存进私有且不公开列出的 Steam 创意工坊条目，好让朋友把你的《Valheim》或《Factorio》世界拉走；另外也有局域网同步。

**适合：** 你要解决的问题是"朋友开房，我需要他那份存档"，而不是"让我的存档跟着我走"。

**止步之处：** 闭源、限 Windows、把 Steam 当作传输通道，而且支持的是一份联机游戏清单，不是你拥有的一切。

## 关于 EmuDeck 的一点说明

这类讨论里常常会提到 EmuDeck，但它并不是通常意义上的竞品：它是 Steam Deck 上的模拟器安装与配置工具，所提供的同步只是附在这份工作上的便利功能（用 Rclone 对接网盘，而且仅限模拟器存档）。它和上面这些工具有重叠，却不是同一类东西：EmuDeck 负责把模拟器给你装好配好，这里的工具负责照看整个游戏库的存档。确实有人把 EmuDeck 和其中一款搭配着用，那是合理的组合，并不重复。

## Hoard

Hoard 以一次游戏会话作为单位。引擎作为后台服务运行——\`hoardd\`，没有窗口，所以在 SteamOS 的游戏模式下照样工作——它会察觉你已经不玩了，然后在那一刻拍下快照，而不是在游戏进行中对每一次文件写入做出反应。

- **按会话的版本历史。** 每一次会话都是一个可以回退到的版本，哪怕是在硬盘故障或者全新安装之后。
- **基于内容哈希的去重。** 一个 2 GB 存档的十个版本大约只占 2 GB，而不是 20 GB。传输使用 zstd 压缩。
- **上传和下载都做 SHA-256 校验。** 损坏会在覆盖掉一个完好存档之前被抓出来。任何东西都不会被悄悄覆盖——整个设计就是围着这一点转的。
- **云端或自托管，同一个二进制。** Hoard Cloud 有免费额度（2 GB、3 台设备、完整历史）。或者你用 Docker Compose 自己跑 \`hoard-server\`，对接任何兼容 S3 的存储——MinIO、Garage、Backblaze B2——不需要账号，也没有配额。AGPL-3.0。
- **Windows、Linux、macOS**，另外还有一个无界面的命令行版本，适合 Steam Deck 或服务器。
- **模拟器支持处于测试阶段：** PCSX2、RPCS3、Dolphin、Cemu、Ryujinx、RetroArch、DuckStation、PPSSPP 等以预设形式提供。

## 决定 Steam Deck ↔ PC 同步成败的那个细节

不管你最后选哪款工具，这一点都值得知道。Steam 游戏的云存档位于 \`<AppID>/remote/\`，而它*上一层*的文件夹里放着 \`remotecache.vdf\`、成就状态、统计数据和游戏时长计数器——这些东西在你的 Deck 和台式机上本来就应该不一样。

同步父文件夹，你就会在两台从未在任何一个存档上产生分歧的机器之间，制造出永久的冲突。Hoard 跟踪的是 \`remote/\`，不是父文件夹。任何允许你手动指定文件夹的工具都可以照此设置；当一套同步配置莫名其妙地不断报冲突时，这也是第一个该去检查的地方。

## Hoard 输在哪里

- **它需要一台服务器。** 云端账号也好，自己的机器也罢，总归是基础设施；而 OpenSave 或 Ludusavi 一台都不需要。
- **模拟器支持还在测试阶段。** 便携式安装和各家模拟器的怪癖仍然会绊到它，某些启动器和模拟器的边缘情况，今天 Aletheia 和 OpenSave 处理得更好。
- **macOS 几乎没在真机上验证过。** 能编译也能跑，但没有人在上面长期用过几个月。
- **它还年轻。** Ludusavi 和 Game Backup Monitor 背后有好几年的问题反馈积累，Hoard 没有；对于一个要守着 200 小时存档的软件来说，这个差距不轻。
- **它不做联机存档共享。** 想把一个世界递给朋友，那是 SaveSync 的活儿，不是 Hoard 的。

## Hoard Cloud 与自托管的区别

关于 Hoard 的比较几乎总把这两者混为一谈，结论也就跟着错了。所以直说：

- **Hoard Cloud** 是托管方案：你登录，存档保存在我们位于欧盟的服务器上。
- **自托管的 Hoard 完全属于你。** 你在自己的 PC 或 NAS 上运行 \`hoard-server\`，存档从你的机器走到你的磁盘。**没有我们这边的账号，没有发往我们的遥测，没有配额，也没有中转**——不经过我们的任何服务器，因为这条路径上根本没有我们的东西。我们看不到任何存档、游戏名或邮箱地址，因为这些从未到达我们这里。就算 Hoard Cloud 明天关停，自托管的部署照常运行。

同一个二进制、同样的检测、同样的版本历史。唯一变化的是存储归谁所有。有一点要说准确：你自己的服务器确实有它的登录——一个用户和每台设备一个令牌——但它们在你的数据库里，不在我们的。

## 对比表

| 工具 | 设备之间自动同步 | 存档放在哪里 | 历史 | 平台 | 许可 |
|---|---|---|---|---|---|
| **Hoard** | 是，按游戏会话 | Hoard Cloud 或你自己的服务器（兼容 S3） | 按会话版本化，带去重 | Win · Linux · macOS · Deck | AGPL-3.0，有免费额度 |
| **Ludusavi** | 手动，或你自己搭的 Rclone | 本地，外加你的 Rclone 远端 | 带版本的本地备份 | Win · Linux · macOS | 免费开源 |
| **Syncthing** | 是，持续镜像 | 只在你的设备上 | 按文件的版本保留 | 全平台 | 免费开源 |
| **OpenSave** | 是，点对点 | 你的设备，可选云端镜像 | 快照与分支 | Win · Linux · Deck | MIT |
| **OpenCloudSaves** | 是，经由你的网盘 | OneDrive / Drive / Dropbox / Nextcloud | 取决于网盘保留什么 | Win · Linux · macOS | 免费开源 |
| **Game Backup Monitor** | 否 | 本地 7-Zip 归档 | 编号备份 | Windows | 免费开源 |
| **Aletheia** | 按启动器备份与还原 | 你自己的存储 | 备份 | Win · Linux · macOS | AGPL-3.0 |
| **SaveSync** | 是，还能和朋友同步 | 私有的 Steam 创意工坊条目 | 视应用而定 | Windows | 付费闭源 |

## 那么选哪个

如果你只想保住一台机器，别的都不管，选 Ludusavi 或 Game Backup Monitor。如果你无论如何都不想要账号，而且设备通常同时开着，选 OpenSave。如果存档应该落进一个你已经在付费的 Drive 文件夹里，选 OpenCloudSaves。如果你要和朋友共享一个联机世界，选 SaveSync。

如果你想要的是备份*和*跨 PC 与 Steam Deck 的同步自己就发生，每一次会话都有一个可以回退的版本，并且保留把整套东西自托管的选项——那正是 Hoard 存在的理由。[下载它](/download)，或者先读一读[如何用 Docker 自托管](/guides/self-host-hoard)。如果你正在权衡的对手就是 Ludusavi，这里还有一篇[更详细的对比](/guides/ludusavi-alternative)。

## 一对一比较

下面每一篇都比上面的段落更深入，也包括对方胜出的地方：

- [Hoard 与 Ludusavi](/guides/ludusavi-alternative)
- [用 Hoard 替代 Steam 云存档](/guides/steam-cloud-alternative)
- [点对点同步与一台属于你的服务器](/guides/opensave-alternative)
- [用 Syncthing 同步存档会坏在哪里](/guides/syncthing-game-saves)

<!-- faq -->

## 常见问题

### 这些工具里，哪个保留版本历史？

Hoard 把每次游玩留成一个可回退的版本。Ludusavi 保留带版本的本地备份。其余大多数只是同步或复制当前状态，也就是说损坏的存档会被忠实地传到你的另一台机器上。

### 哪个不需要服务器也不需要账号？

用作本地备份的 Ludusavi，以及任何点对点工具。如果你自托管，Hoard 也算在内：没有我们这边的账号，也没有任何东西经过我们的服务器。

### 哪个能覆盖不在 Steam 上的游戏？

这里列出的存档管理工具都可以，因为它们通过同一份社区数据库定位存档，而不是通过商店。例外是 Steam 云存档：它只覆盖开发者启用了它的 Steam 游戏。

### 我必须只选一个吗？

不必，很多人也没有只选一个。本地备份工具和同步工具解决的是问题的不同一半。唯一的原则是：永远不要让其中一个指向另一个的备份文件夹，否则你同步的会是一份过期镜像，而不是真正的存档。

### 让多数自制方案翻车的，是哪个细节？

同步 Steam \`userdata\` 里 \`<AppID>/remote/\` 的上一层文件夹。上一层放着 \`remotecache.vdf\` 以及本就应该因机器而异的成就和游戏时长文件，于是每次启动都像冲突，尽管没有任何存档动过。
`,Xe=`---
title: "Ludusavi-Alternative: automatische Cloud-Synchronisierung für deine Spielstände"
description: "Ein fairer Vergleich von Ludusavi und Hoard. Ludusavi ist ein großartiges Open-Source-Tool für lokale Backups; Hoard ergänzt verwaltete Cloud-Synchronisierung und versionierte Historie über alle deine PCs — mit denselben Speicherort-Daten."
order: 5
updated: 2026-09-01
---

Wenn du nach einer Möglichkeit suchst, deine Spielstände zu sichern und zu synchronisieren, bist du wahrscheinlich auf **Ludusavi** gestoßen — und es ist hervorragend. Diese Anleitung ist ein ehrlicher Vergleich, damit du das richtige Tool wählst, und erklärt, wo Hoard passt, wenn du automatische Cloud-Synchronisierung über mehrere Geräte willst.

## Was Ludusavi gut macht

Ludusavi ist ein kostenloses Open-Source-Tool (von mtkennerly), um PC-Spielstände unter Windows, macOS und Linux zu sichern und wiederherzustellen. Es hat eine aufgeräumte GUI und eine CLI, findet Stände für Tausende Spiele automatisch, führt versionierte lokale Backups und kann diese über **Rclone** in eine eigene Cloud übertragen (Google Drive, Dropbox und viele andere). Wenn du volle Kontrolle und ein Do-it-yourself-Setup willst, ist Ludusavi eine fantastische Wahl — und völlig kostenlos.

Hoard will das nicht ersetzen. Tatsächlich nutzt **Hoard dieselbe Community-Datenbank für Speicherorte, auf die sich auch Ludusavi stützt**, um zu finden, wo jedes Spiel seine Stände ablegt — die Erkennungsqualität ist also gleichwertig.

## Worin sich Hoard unterscheidet

Die Lücke, auf die die meisten bei jedem lokalen Tool stoßen, ist die **Synchronisierung über Geräte hinweg**. Mit Ludusavi machst du das selbst: Backup planen, Rclone-Remote konfigurieren, dann auf dem anderen PC wiederherstellen, bevor du spielst. Das funktioniert, ist aber manuell.

Hoard macht daraus **verwaltete Cloud-Synchronisierung**:

- **Anmelden und loslegen.** Keine Rclone-Remotes, keine Skripte. Hoard lädt deinen Stand nach dem Spielen hoch und vor dem Start die neueste Version herunter, auf jedem PC deines Kontos.
- **Versionierte Historie in der Cloud.** Jedes Backup bleibt erhalten, du kannst also zu jedem früheren Stand zurück — sogar nach einem Festplattenausfall oder einer Neuinstallation.
- **Konfliktbewusst.** Hoard vergleicht Zeitstempel und behält eine lokale Kopie von allem, was es ersetzt, sodass eine Synchronisierung nie stillschweigend Fortschritt zerstört.
- **Weiterhin Open Source und selbst hostbar.** Wie bei Ludusavi gibt es keine Bindung — nutze Hoard Cloud oder hoste den Server selbst.

## Direkter Vergleich

| | Ludusavi | Hoard |
|---|---|---|
| Lokale Backups | Ja | Ja |
| Erkennung der Stände | Community-Manifest | Dasselbe Manifest, dazu Steam-Bibliotheken, laufende Prozesse und ein Dateisystem-Scan |
| Cloud-Speicher | Eigener, über Rclone | Enthalten, oder dein eigener Server |
| Synchronisierung zwischen PCs | Manuell: hier sichern, dort wiederherstellen | Automatisch, nach dem Spielen und vor dem Start |
| Versionshistorie | Lokale Backups, die du selbst aufräumst | Jede Version in der Cloud, dedupliziert per Inhalts-Hash |
| Emulatoren | Ja | Ja |
| Oberflächen | Desktop-App und CLI | Desktop-App, CLI und ein Overlay im Spiel |
| Preis | Kostenlos | Kostenlos mit 2 GB und 3 Geräten, Pro darüber, ohne Limit beim Selbsthosten |
| Lizenz | MIT | AGPL-3.0 |

## Wann Ludusavi die bessere Wahl ist

Das ist der Teil, den die meisten Vergleichsseiten weglassen. Ludusavi ist das bessere Werkzeug, wenn:

- **Du nur an einem PC spielst.** Cloud-Synchronisierung löst dann ein Problem, das du nicht hast. Ein lokales Backup reicht, und darin ist Ludusavi sehr gut.
- **Du bereits ein Rclone-Remote hast, dem du vertraust.** Wenn dein Speicher eingerichtet ist und läuft, ist Hoards Hauptvorteil ein Einrichtungsschritt, den du längst hinter dir hast.
- **Du es im Spielmodus des Steam Deck nutzen willst.** Für Ludusavi gibt es ein Decky-Plugin, du kannst ein Backup also anstoßen, ohne die Konsolenoberfläche zu verlassen.
- **Du eine permissive Lizenz brauchst.** Ludusavi ist MIT, Hoard ist AGPL-3.0. Wenn du etwas darauf aufbauen und das Ergebnis nicht veröffentlichen willst, macht dieser Unterschied viel aus.
- **Du willst nichts laufen haben.** Hoard selbst zu hosten heißt, irgendwo einen kleinen Server am Laufen zu halten, und sei es derselbe PC. Ludusavi ist eine App, die du öffnest, wenn du sie brauchst.

## Von Ludusavi zu Hoard wechseln

Es gibt keinen Import, und das ist Absicht. Die Schritte:

1. **Lass deine Ludusavi-Backups genau dort, wo sie sind.** Es wird nichts migriert und nichts gelöscht. Behalte sie in den ersten Wochen als Sicherheitsnetz.
2. **Installiere Hoard und melde dich an**, oder richte es auf deinen eigenen Server.
3. **Lass es scannen.** Es liest dasselbe Manifest, die Liste der erkannten Spiele sollte dir also bekannt vorkommen.
4. **Richte Hoard nicht auf deinen Ludusavi-Backup-Ordner.** Verfolge den Ordner, in den das Spiel selbst schreibt. Ein Backup-Ordner ist eine Kopie, die sich nach Zeitplan ändert statt beim Spielen, und die Kopie einer Kopie zu synchronisieren ist der Weg, am Ende den Fortschritt von gestern wiederherzustellen. Hoard versucht das selbst zu erkennen — \`hoard doctor\` meldet einen verfolgten Ordner, der wie ein Backup-Spiegel aussieht — aber am einfachsten ist, ihn gar nicht erst aufzunehmen.
5. **Spiel einmal.** Beim Beenden erscheint die erste Version in der Historie.
6. **Wiederhole das am zweiten PC.** Dort anmelden, und die Versionen liegen schon bereit.

## Zwei Details, die man kennen sollte

**Steam-Spielstände liegen einen Ordner tiefer als gedacht.** Bei Steam-Spielen verfolgt Hoard \`<AppID>/remote/\` innerhalb von \`userdata\`, nicht den Ordner darüber. Der übergeordnete Ordner enthält auch \`remotecache.vdf\` sowie Dateien für Erfolge und Spielzeit, und die unterscheiden sich zu Recht von Rechner zu Rechner. Synchronisierst du den übergeordneten Ordner, sieht jeder Start nach einem Konflikt aus, obwohl sich kein einziger Spielstand bewegt hat. Das ist der häufigste Grund, warum ein selbstgebautes Setup zwischen Steam Deck und Desktop gegen sich selbst arbeitet.

**Versionen sind billig.** Snapshots werden per Inhalts-Hash gespeichert, unveränderte Dateien also nur einmal. Zehn Versionen eines 2 GB großen Spielstands kosten etwa 2 GB, nicht 20 — und genau das macht es praktikabel, die komplette Historie zu behalten, statt sie auszudünnen.

## Was Selbsthosten wirklich bedeutet

Genau hier liegen die meisten Vergleiche bei Hoard falsch, deshalb der Punkt im Detail. Es gibt zwei Betriebsarten, und sie unterscheiden sich wirklich:

- **Hoard Cloud** ist die verwaltete Variante: du meldest dich an, und deine Spielstände liegen auf unseren Servern in der EU.
- **Selbsthosten gehört vollständig dir.** Du betreibst \`hoard-server\` auf deinem eigenen PC oder NAS, und deine Stände gehen von deiner Maschine auf deine Platte. Es gibt **kein Konto bei uns, keine Telemetrie zu uns, kein Limit und kein Relay** — nichts läuft über unsere Server, weil nichts von uns im Weg steht. Wir können weder einen Spielstand noch einen Spieltitel noch eine E-Mail-Adresse sehen, schlicht weil davon nichts bei uns ankommt. Verschwände Hoard Cloud morgen, liefe ein selbst gehostetes Setup unverändert weiter.

Dasselbe Programm, dieselbe Erkennung, dieselbe Versionshistorie. Es ändert sich nur, wem der Speicher gehört.

## Was solltest du wählen?

- Wähle **Ludusavi**, wenn du ein kostenloses, lokal orientiertes Backup-Tool willst und gern deine eigene Cloud mit Rclone einrichtest.
- Wähle **Hoard**, wenn Backups *und* automatische Synchronisierung über PCs einfach funktionieren sollen, mit versionierter Cloud-Historie und der Option, selbst zu hosten.

Viele beginnen mit Ludusavi für lokale Backups und wechseln zu Hoard, sobald sie dieselben Spiele auf mehr als einem Gerät spielen. Wenn das auf dich zutrifft, siehe [wie du Spielstände über PCs synchronisierst](/guides/sync-game-saves-across-pcs) oder [lade einfach Hoard herunter](/download) und melde dich an. Einen Blick auf das ganze Feld gibt der [Vergleich aller Sync-Tools](/guides/game-save-sync-comparison).

<!-- faq -->

## Häufige Fragen

### Kann ich Ludusavi und Hoard gleichzeitig nutzen?

Ja. Beide lesen dieselben Speicherorte und keines hält die Dateien geöffnet. Viele behalten Ludusavi für lokale Archiv-Backups und überlassen Hoard die Synchronisierung zwischen Geräten. Die einzige Regel: richte keines der beiden Werkzeuge auf den Backup-Ordner des anderen.

### Importiert Hoard meine Ludusavi-Backups?

Nein, und das ist Absicht. Ein Backup-Ordner ist eine Kopie, die sich nach eigenem Zeitplan ändert; ihn zu verfolgen würde einen veralteten Spiegel synchronisieren statt deines echten Spielstands. Hoard verfolgt den Ordner, in den das Spiel schreibt, und beginnt seine eigene Historie mit deiner nächsten Sitzung. Behalte das Ludusavi-Archiv als Sicherheitsnetz.

### Ist Hoard kostenlos?

Hoard Cloud hat einen kostenlosen Tarif mit 2 GB Speicher und 3 Geräten, was für die meisten Sammlungen reicht; Pro hebt beides an. Den Server selbst zu hosten ist kostenlos und hat überhaupt kein Limit. Alles ist Open Source unter AGPL-3.0.

### Funktioniert Hoard auf dem Steam Deck?

Ja, auf dem Steam Deck und jedem Linux-Desktop, ebenso unter Windows und macOS. Das Deck ist genau der Fall, für den das \`remote/\`-Detail oben wichtig ist: Deck und Desktop schreiben neben demselben Spielstand unterschiedliche Dateien für Erfolge und Spielzeit.

### Brauche ich Rclone oder ein eigenes Cloud-Konto?

Nein. Das ist der wesentliche praktische Unterschied: Bei Hoard Cloud ist der Speicher schon eingerichtet, sobald du dich anmeldest. Wenn dir der Speicher lieber selbst gehört, betreibe den Server selbst gegen einen S3-kompatiblen Bucket oder einen gewöhnlichen Ordner auf deiner eigenen Maschine.

### Sendet Selbsthosten irgendetwas an Hoard?

Nein. Im selbst gehosteten Betrieb gibt es kein Konto bei uns und keine Telemetrie zu uns: deine Spielstände, deine Nutzer und deine Logs liegen auf deinem eigenen Server und berühren unseren nie. Das ist der ganze Sinn dieses Modus, und deshalb ist der Server dasselbe quelloffene Binary, das wir selbst betreiben, und keine abgespeckte Fassung.
`,Ze=`---
title: "Ludusavi alternative: automatic cloud sync for your game saves"
description: "A fair comparison of Ludusavi and Hoard. Ludusavi is a great open-source local backup tool; Hoard adds managed cloud sync and versioned history across all your PCs — using the same save-location data."
order: 5
updated: 2026-09-01
---

If you're looking for a way to back up and sync your game saves, you've probably found **Ludusavi** — and it's excellent. This guide is an honest comparison so you can pick the right tool, and it explains where Hoard fits if you want automatic cloud sync across machines.

## What Ludusavi does well

Ludusavi is a free, open-source tool (made by mtkennerly) for backing up and restoring PC game saves on Windows, macOS and Linux. It has a clean GUI and a CLI, finds saves for thousands of games automatically, keeps versioned local backups, and can push those backups to a cloud you own by configuring **Rclone** (Google Drive, Dropbox, and many others). If you want full control and a do-it-yourself setup, Ludusavi is a fantastic choice — and it's completely free.

Hoard isn't here to replace that. In fact, **Hoard uses the same community save-location database that Ludusavi relies on** to locate where each game stores its saves, so detection quality is on par.

## Where Hoard is different

The gap most people hit with any local-first tool is **syncing across devices**. With Ludusavi you do it yourself: schedule a backup, configure an Rclone remote, then restore on the other PC before you play. That works, but it's manual.

Hoard turns that into **managed cloud sync**:

- **Sign in and go.** No Rclone remotes, no scripts. Hoard uploads your save after you finish playing and downloads the latest before you start, on every PC on your account.
- **Versioned history in the cloud.** Every backup is kept, so you can roll back to any earlier save — even after a disk failure or a fresh install.
- **Conflict-aware.** Hoard compares timestamps and keeps a local copy of anything it replaces, so a sync never silently destroys progress.
- **Still open source and self-hostable.** Like Ludusavi, you're not locked in — run Hoard Cloud or host the server yourself.

## Side by side

| | Ludusavi | Hoard |
|---|---|---|
| Local backups | Yes | Yes |
| Save detection | Community manifest | The same manifest, plus Steam libraries, running processes and a filesystem scan |
| Cloud storage | Bring your own, through Rclone | Included, or your own server |
| Sync between PCs | Manual: back up here, restore there | Automatic, after you stop playing and before you start |
| Version history | Local backups you prune yourself | Every version kept in the cloud, deduplicated by content hash |
| Emulators | Yes | Yes |
| Interfaces | Desktop app and CLI | Desktop app, CLI, and an in-game overlay |
| Price | Free | Free tier of 2 GB and 3 devices, Pro above that, no quota at all if you self-host |
| Licence | MIT | AGPL-3.0 |

## When Ludusavi is the better choice

This is the part most comparison pages skip. Ludusavi is the better tool when:

- **You only play on one PC.** Cloud sync solves a problem you don't have. A local backup is enough, and Ludusavi does local backups very well.
- **You already have an Rclone remote you trust.** If your storage is wired up and working, Hoard's main advantage is a setup step you've already paid for.
- **You want to run it from Game Mode on a Steam Deck.** Ludusavi has a Decky plugin, so you can trigger a backup without leaving the console interface.
- **You want a permissive licence.** Ludusavi is MIT, Hoard is AGPL-3.0. If you intend to build something on top and not publish the result, that difference matters.
- **You don't want anything running.** Self-hosting Hoard means keeping a small server up somewhere, even if it's the same PC. Ludusavi is an app you open when you want it.

## Moving from Ludusavi to Hoard

There's no importer, and that's on purpose. The steps:

1. **Leave your Ludusavi backups exactly where they are.** Nothing is migrated or deleted. Keep them as a safety net for the first few weeks.
2. **Install Hoard and sign in**, or point it at your own server.
3. **Let it scan.** It reads the same manifest, so the list of detected games should look familiar.
4. **Don't point Hoard at your Ludusavi backup folder.** Track the folder the game itself writes to. A backup folder is a copy that changes on a schedule rather than when you play, and syncing a copy of a copy is how you end up restoring yesterday's progress. Hoard tries to catch this on its own — \`hoard doctor\` flags a tracked folder that looks like a backup mirror — but it's easier never to track it.
5. **Play once.** When you quit, the first version appears in the history.
6. **Repeat on the second PC.** Sign in there and the versions are already waiting.

## Two details worth knowing

**Steam saves live one folder deeper than you think.** For Steam games, Hoard tracks \`<AppID>/remote/\` inside \`userdata\`, not the folder above it. The parent also holds \`remotecache.vdf\` and achievement and playtime files, and those legitimately differ from machine to machine. Sync the parent and every launch looks like a conflict even though no save actually moved. It's the most common reason a hand-rolled Steam Deck ↔ desktop setup ends up fighting itself.

**Versions are cheap.** Snapshots are stored by content hash, so unchanged files are stored once. Ten versions of a 2 GB save cost about 2 GB, not 20 — which is what makes keeping the full history practical instead of pruning it.

## What self-hosting actually means

This is the point most comparisons get wrong about Hoard, so it's worth being exact. There are two ways to run it, and they are genuinely different:

- **Hoard Cloud** is the managed option: you sign in, and your saves are stored on our servers, in the EU.
- **Self-hosting is entirely yours.** You run \`hoard-server\` on your own PC or NAS, and your saves go from your machine to your disk. There is **no account with us, no telemetry to us, no quota and no relay** — nothing passes through our servers, because there is nothing of ours in the path. We can't see a save, a game name or an email address, for the simple reason that none of it ever reaches us. If Hoard Cloud disappeared tomorrow, a self-hosted setup would carry on unchanged.

Same program, same detection, same version history. The only thing that changes is who owns the storage.

## Which should you choose?

- Choose **Ludusavi** if you want a free, local-first backup tool and you're happy to wire up your own cloud with Rclone.
- Choose **Hoard** if you want backups *and* automatic sync across PCs to just work, with a versioned cloud history, while keeping the option to self-host.

Many people start with Ludusavi for local backups and move to Hoard once they're playing the same games on more than one machine. If that's you, see [how to sync game saves across PCs](/guides/sync-game-saves-across-pcs) or just [download Hoard](/download) and sign in. For the wider field, there's a [comparison of every save sync tool](/guides/game-save-sync-comparison).

<!-- faq -->

## Frequently asked questions

### Can I use Ludusavi and Hoard at the same time?

Yes. They read the same save locations and neither one holds the files open. Plenty of people keep Ludusavi for local archive backups and let Hoard handle sync between machines. The only rule is not to point either tool at the other's backup folder.

### Does Hoard import my Ludusavi backups?

No, and that's deliberate. A backup folder is a copy that changes on its own schedule, so tracking it would sync a stale mirror instead of your live save. Hoard tracks the folder the game writes to and starts its own history from your next session. Keep the Ludusavi archive as a safety net.

### Is Hoard free?

Hoard Cloud has a free tier with 2 GB of storage and 3 devices, which covers most save collections; Pro raises both. Self-hosting the server is free and has no quota at all. Everything is open source under AGPL-3.0.

### Does Hoard work on Steam Deck?

Yes, on Steam Deck and any Linux desktop, as well as Windows and macOS. The Deck is exactly the case that needs the \`remote/\` detail above, because a Deck and a desktop write different achievement and playtime files next to the same save.

### Do I need Rclone or a cloud account of my own?

No. That's the main practical difference: with Hoard Cloud, storage is already set up when you sign in. If you'd rather own the storage, run the server yourself against an S3-compatible bucket or a plain folder on your own machine.

### Does self-hosting send anything to Hoard?

No. In self-hosted mode there is no account with us and no telemetry to us: your saves, your users and your logs live on your own server and never touch ours. That's the whole point of the mode, and it's why the server is the same open-source binary we run ourselves rather than a cut-down version.
`,Ye=`---
title: "Alternativa a Ludusavi: sincronización automática de partidas en la nube"
description: "Comparativa justa entre Ludusavi y Hoard. Ludusavi es una gran herramienta open source de copia local; Hoard añade sincronización gestionada en la nube e historial versionado entre todos tus PC, usando los mismos datos de ubicación de partidas."
order: 5
updated: 2026-09-01
---

Si buscas una forma de hacer copia y sincronizar tus partidas guardadas, seguramente has encontrado **Ludusavi**, y es excelente. Esta guía es una comparativa honesta para que elijas la herramienta adecuada, y explica dónde encaja Hoard si quieres sincronización automática en la nube entre equipos.

## Qué hace bien Ludusavi

Ludusavi es una herramienta gratuita y open source (creada por mtkennerly) para hacer copias y restaurar partidas de PC en Windows, macOS y Linux. Tiene una interfaz limpia y una CLI, detecta automáticamente las partidas de miles de juegos, guarda copias locales versionadas y puede subir esas copias a una nube tuya configurando **Rclone** (Google Drive, Dropbox y muchas más). Si quieres control total y un montaje a tu medida, Ludusavi es una opción fantástica, y es completamente gratis.

Hoard no viene a reemplazar eso. De hecho, **Hoard usa la misma base de datos comunitaria de ubicación de partidas en la que se apoya Ludusavi** para localizar dónde guarda cada juego sus saves, así que la calidad de detección está a la par.

## En qué se diferencia Hoard

El punto donde la mayoría se atasca con cualquier herramienta local es **sincronizar entre dispositivos**. Con Ludusavi lo haces tú: programas una copia, configuras un remoto de Rclone y luego restauras en el otro PC antes de jugar. Funciona, pero es manual.

Hoard convierte eso en **sincronización gestionada en la nube**:

- **Inicia sesión y listo.** Sin remotos de Rclone, sin scripts. Hoard sube tu partida cuando terminas de jugar y descarga la última antes de empezar, en todos los PC de tu cuenta.
- **Historial versionado en la nube.** Se conserva cada copia, así que puedes volver a cualquier partida anterior, incluso tras un fallo de disco o una instalación limpia.
- **Tiene en cuenta los conflictos.** Hoard compara fechas y guarda una copia local de lo que reemplaza, así que una sincronización nunca destruye progreso en silencio.
- **Sigue siendo open source y autoalojable.** Como Ludusavi, no hay bloqueo: usa Hoard Cloud o aloja el servidor tú mismo.

## Cara a cara

| | Ludusavi | Hoard |
|---|---|---|
| Copias locales | Sí | Sí |
| Detección de partidas | Manifiesto comunitario | El mismo manifiesto, más bibliotecas de Steam, procesos en ejecución y un barrido del disco |
| Almacenamiento en la nube | El tuyo, vía Rclone | Incluido, o tu propio servidor |
| Sincronización entre PC | Manual: copia aquí, restaura allí | Automática, al dejar de jugar y antes de empezar |
| Historial de versiones | Copias locales que podas tú | Todas las versiones en la nube, deduplicadas por hash de contenido |
| Emuladores | Sí | Sí |
| Interfaces | App de escritorio y CLI | App de escritorio, CLI y overlay dentro del juego |
| Precio | Gratis | Plan gratis de 2 GB y 3 dispositivos, Pro por encima, sin cupo si te autoalojas |
| Licencia | MIT | AGPL-3.0 |

## Cuándo Ludusavi es la mejor opción

Ésta es la parte que casi ninguna comparativa incluye. Ludusavi es mejor herramienta cuando:

- **Sólo juegas en un PC.** La sincronización en la nube resuelve un problema que no tienes. Con una copia local basta, y Ludusavi hace copias locales muy bien.
- **Ya tienes un remoto de Rclone que funciona.** Si tu almacenamiento está montado y va fino, la ventaja principal de Hoard es un paso de configuración que tú ya has pagado.
- **Quieres usarlo desde el modo Juego de una Steam Deck.** Ludusavi tiene un plugin de Decky, así que puedes lanzar una copia sin salir de la interfaz de consola.
- **Quieres una licencia permisiva.** Ludusavi es MIT y Hoard es AGPL-3.0. Si piensas construir algo encima y no publicar el resultado, esa diferencia importa.
- **No quieres nada corriendo de fondo.** Autoalojar Hoard implica mantener un servidor en pie, aunque sea en el mismo PC. Ludusavi es una aplicación que abres cuando te hace falta.

## Pasar de Ludusavi a Hoard

No hay importador, y es a propósito. Los pasos:

1. **Deja tus copias de Ludusavi exactamente donde están.** No se migra ni se borra nada. Consérvalas como red de seguridad las primeras semanas.
2. **Instala Hoard e inicia sesión**, o apúntalo a tu propio servidor.
3. **Déjalo escanear.** Lee el mismo manifiesto, así que la lista de juegos detectados debería resultarte familiar.
4. **No apuntes Hoard a la carpeta de copias de Ludusavi.** Rastrea la carpeta en la que escribe el juego. Una carpeta de copias es un duplicado que cambia por horario y no cuando juegas, y sincronizar la copia de una copia es como acabas restaurando el progreso de ayer. Hoard intenta detectarlo solo — \`hoard doctor\` avisa de una carpeta rastreada que parece un espejo de copias — pero es más fácil no rastrearla nunca.
5. **Juega una vez.** Al salir, la primera versión aparece en el historial.
6. **Repite en el segundo PC.** Inicias sesión y las versiones ya están ahí.

## Dos detalles que conviene saber

**Las partidas de Steam viven una carpeta más adentro de lo que parece.** En los juegos de Steam, Hoard rastrea \`<AppID>/remote/\` dentro de \`userdata\`, no la carpeta de encima. La carpeta padre guarda además \`remotecache.vdf\` y ficheros de logros y de tiempo jugado, y ésos son legítimamente distintos en cada máquina. Si sincronizas la padre, cada arranque parece un conflicto aunque no se haya movido ninguna partida. Es el motivo más común de que un montaje casero entre Steam Deck y sobremesa acabe peleándose consigo mismo.

**Las versiones salen baratas.** Las instantáneas se guardan por hash de contenido, así que un fichero que no cambia se almacena una sola vez. Diez versiones de una partida de 2 GB ocupan unos 2 GB, no 20, y eso es lo que hace práctico conservar el historial entero en vez de ir podándolo.

## Qué significa realmente autoalojarse

Es el punto donde casi todas las comparativas se equivocan con Hoard, así que conviene ser exacto. Hay dos formas de usarlo, y son genuinamente distintas:

- **Hoard Cloud** es la opción gestionada: inicias sesión y tus partidas se guardan en nuestros servidores, en la UE.
- **Autoalojarse es tuyo por completo.** Levantas \`hoard-server\` en tu PC o en tu NAS, y tus partidas van de tu máquina a tu disco. **No hay cuenta con nosotros, ni telemetría hacia nosotros, ni cupo, ni relé**: no pasa nada por nuestros servidores, porque no hay nada nuestro en el camino. No podemos ver una partida, ni el nombre de un juego, ni un correo, por la sencilla razón de que nada de eso nos llega. Si Hoard Cloud desapareciera mañana, un montaje autoalojado seguiría funcionando igual.

El mismo programa, la misma detección, el mismo historial de versiones. Lo único que cambia es de quién es el almacenamiento.

## ¿Cuál elegir?

- Elige **Ludusavi** si quieres una herramienta de copia gratuita y local y no te importa montar tu propia nube con Rclone.
- Elige **Hoard** si quieres que la copia *y* la sincronización entre PC funcionen solas, con historial versionado en la nube, sin renunciar a poder autoalojarte.

Mucha gente empieza con Ludusavi para copias locales y pasa a Hoard cuando juega a los mismos juegos en más de un equipo. Si es tu caso, mira [cómo sincronizar partidas entre PC](/guides/sync-game-saves-across-pcs) o simplemente [descarga Hoard](/download) e inicia sesión. Y si quieres el panorama completo, hay una [comparativa de todas las herramientas de sincronización](/guides/game-save-sync-comparison).

<!-- faq -->

## Preguntas frecuentes

### ¿Puedo usar Ludusavi y Hoard a la vez?

Sí. Leen las mismas ubicaciones de partidas y ninguno de los dos bloquea los ficheros. Mucha gente conserva Ludusavi para copias de archivo locales y deja que Hoard se encargue de la sincronización entre equipos. La única regla es no apuntar una herramienta a la carpeta de copias de la otra.

### ¿Hoard importa mis copias de Ludusavi?

No, y es deliberado. Una carpeta de copias es un duplicado que cambia según su propio horario, así que rastrearla sincronizaría un espejo desfasado en lugar de tu partida real. Hoard rastrea la carpeta en la que escribe el juego y arranca su propio historial desde tu siguiente sesión. Guarda el archivo de Ludusavi como red de seguridad.

### ¿Hoard es gratis?

Hoard Cloud tiene un plan gratuito con 2 GB de almacenamiento y 3 dispositivos, que cubre la mayoría de colecciones de partidas; Pro sube ambos. Autoalojar el servidor es gratis y no tiene cupo ninguno. Todo es open source bajo AGPL-3.0.

### ¿Funciona en Steam Deck?

Sí, en Steam Deck y en cualquier escritorio Linux, además de Windows y macOS. La Deck es justo el caso que necesita el detalle de \`remote/\` de más arriba, porque una Deck y un sobremesa escriben ficheros de logros y de tiempo jugado distintos junto a la misma partida.

### ¿Necesito Rclone o una cuenta de nube propia?

No. Ésa es la diferencia práctica principal: con Hoard Cloud el almacenamiento ya está listo al iniciar sesión. Si prefieres ser dueño del almacenamiento, levanta el servidor tú mismo contra un bucket compatible con S3 o una carpeta normal de tu máquina.

### ¿Autoalojarse envía algo a Hoard?

No. En modo autoalojado no hay cuenta con nosotros ni telemetría hacia nosotros: tus partidas, tus usuarios y tus registros viven en tu propio servidor y nunca tocan el nuestro. Ése es el sentido del modo, y por eso el servidor es el mismo binario open source que usamos nosotros y no una versión recortada.
`,Je=`---
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
4. **Ne pointez pas Hoard vers votre dossier de sauvegardes Ludusavi.** Suivez le dossier dans lequel le jeu écrit lui-même. Un dossier de sauvegardes est une copie qui change selon un horaire et non quand vous jouez, et synchroniser la copie d'une copie, c'est ainsi qu'on finit par restaurer la progression d'hier. Hoard essaie de le repérer tout seul — \`hoard doctor\` signale un dossier suivi qui ressemble à un miroir de sauvegardes — mais le plus simple est de ne jamais l'ajouter.
5. **Jouez une fois.** En quittant, la première version apparaît dans l'historique.
6. **Recommencez sur le second PC.** Connectez-vous et les versions sont déjà là.

## Deux détails à connaître

**Les sauvegardes Steam sont un dossier plus bas qu'on ne croit.** Pour les jeux Steam, Hoard suit \`<AppID>/remote/\` dans \`userdata\`, pas le dossier au-dessus. Le dossier parent contient aussi \`remotecache.vdf\` ainsi que des fichiers de succès et de temps de jeu, qui diffèrent légitimement d'une machine à l'autre. Synchronisez le parent et chaque lancement ressemble à un conflit alors qu'aucune sauvegarde n'a bougé. C'est la raison la plus fréquente pour laquelle un montage maison entre Steam Deck et PC de bureau finit par se battre contre lui-même.

**Les versions coûtent peu.** Les instantanés sont stockés par empreinte de contenu : un fichier inchangé n'est stocké qu'une fois. Dix versions d'une sauvegarde de 2 Go coûtent environ 2 Go, pas 20 — c'est ce qui rend viable de garder tout l'historique au lieu de l'élaguer.

## Ce que l'auto-hébergement veut vraiment dire

C'est le point sur lequel presque toutes les comparaisons se trompent au sujet de Hoard, autant être précis. Il y a deux façons de l'utiliser, et elles sont réellement différentes :

- **Hoard Cloud** est l'option gérée : vous vous connectez, et vos sauvegardes sont stockées sur nos serveurs, dans l'UE.
- **L'auto-hébergement est entièrement le vôtre.** Vous faites tourner \`hoard-server\` sur votre PC ou votre NAS, et vos sauvegardes vont de votre machine à votre disque. Il n'y a **aucun compte chez nous, aucune télémétrie vers nous, aucun quota et aucun relais** : rien ne passe par nos serveurs, puisque rien de chez nous n'est sur le chemin. Nous ne pouvons voir ni une sauvegarde, ni un nom de jeu, ni une adresse e-mail, pour la simple raison que rien de tout cela ne nous parvient. Si Hoard Cloud disparaissait demain, une installation auto-hébergée continuerait à l'identique.

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

Oui, sur Steam Deck et sur n'importe quel bureau Linux, ainsi que sous Windows et macOS. Le Deck est précisément le cas qui exige le détail \`remote/\` ci-dessus, car un Deck et un PC de bureau écrivent des fichiers de succès et de temps de jeu différents à côté de la même sauvegarde.

### Ai-je besoin de Rclone ou d'un compte cloud à moi ?

Non. C'est la principale différence pratique : avec Hoard Cloud, le stockage est déjà en place dès la connexion. Si vous préférez posséder le stockage, faites tourner le serveur vous-même sur un bucket compatible S3 ou un simple dossier de votre machine.

### L'auto-hébergement envoie-t-il quoi que ce soit à Hoard ?

Non. En mode auto-hébergé il n'y a aucun compte chez nous ni aucune télémétrie vers nous : vos sauvegardes, vos utilisateurs et vos journaux vivent sur votre propre serveur et ne touchent jamais le nôtre. C'est tout l'intérêt de ce mode, et c'est pourquoi le serveur est le même binaire open source que celui que nous faisons tourner, pas une version allégée.
`,en=`---
title: "Alternativa a Ludusavi: sincronizzazione cloud automatica dei salvataggi"
description: "Un confronto equo tra Ludusavi e Hoard. Ludusavi è un ottimo strumento open source di backup locale; Hoard aggiunge sincronizzazione cloud gestita e cronologia versionata su tutti i tuoi PC — usando gli stessi dati di posizione."
order: 5
updated: 2026-09-01
---

Se cerchi un modo per fare backup e sincronizzare i tuoi salvataggi, probabilmente hai trovato **Ludusavi** — ed è eccellente. Questa guida è un confronto onesto per aiutarti a scegliere lo strumento giusto, e spiega dove si inserisce Hoard se vuoi sincronizzazione cloud automatica tra macchine.

## Cosa fa bene Ludusavi

Ludusavi è uno strumento gratuito e open source (creato da mtkennerly) per fare backup e ripristinare i salvataggi PC su Windows, macOS e Linux. Ha una GUI pulita e una CLI, trova automaticamente i salvataggi di migliaia di giochi, conserva backup locali versionati e può inviare quei backup a un cloud tuo configurando **Rclone** (Google Drive, Dropbox e molti altri). Se vuoi pieno controllo e un setup fai-da-te, Ludusavi è una scelta fantastica — e completamente gratuita.

Hoard non vuole sostituirlo. Anzi, **Hoard usa lo stesso database comunitario di posizioni su cui si basa Ludusavi** per individuare dove ogni gioco conserva i salvataggi, quindi la qualità del rilevamento è alla pari.

## In cosa Hoard è diverso

Il punto in cui quasi tutti si bloccano con qualsiasi strumento locale è la **sincronizzazione tra dispositivi**. Con Ludusavi la fai tu: programmare un backup, configurare un remoto Rclone, poi ripristinare sull'altro PC prima di giocare. Funziona, ma è manuale.

Hoard la trasforma in **sincronizzazione cloud gestita**:

- **Accedi e via.** Niente remoti Rclone, niente script. Hoard carica il salvataggio dopo che giochi e scarica l'ultima versione prima che inizi, su ogni PC del tuo account.
- **Cronologia versionata nel cloud.** Ogni backup viene conservato, quindi puoi tornare a qualsiasi salvataggio precedente — anche dopo un guasto del disco o un'installazione pulita.
- **Consapevole dei conflitti.** Hoard confronta i timestamp e conserva una copia locale di tutto ciò che sostituisce, così una sincronizzazione non distrugge mai i progressi in silenzio.
- **Sempre open source e self-hostable.** Come Ludusavi, nessun vincolo — usa Hoard Cloud o ospita il server tu stesso.

## Testa a testa

| | Ludusavi | Hoard |
|---|---|---|
| Backup locali | Sì | Sì |
| Rilevamento dei salvataggi | Manifest comunitario | Lo stesso manifest, più le librerie Steam, i processi in esecuzione e una scansione del disco |
| Spazio cloud | Il tuo, tramite Rclone | Incluso, oppure il tuo server |
| Sincronizzazione tra PC | Manuale: backup qui, ripristino là | Automatica, dopo che smetti di giocare e prima che inizi |
| Cronologia versioni | Backup locali che poti tu | Ogni versione nel cloud, deduplicata per hash del contenuto |
| Emulatori | Sì | Sì |
| Interfacce | App desktop e CLI | App desktop, CLI e overlay in gioco |
| Prezzo | Gratuito | Piano gratis da 2 GB e 3 dispositivi, Pro oltre, nessuna quota in self-hosting |
| Licenza | MIT | AGPL-3.0 |

## Quando Ludusavi è la scelta migliore

È la parte che quasi nessuna pagina di confronto include. Ludusavi è lo strumento migliore quando:

- **Giochi su un solo PC.** La sincronizzazione cloud risolve un problema che non hai. Basta un backup locale, e Ludusavi li fa molto bene.
- **Hai già un remoto Rclone di cui ti fidi.** Se il tuo spazio è configurato e funziona, il vantaggio principale di Hoard è un passaggio che hai già pagato.
- **Vuoi usarlo dalla modalità gioco di uno Steam Deck.** Ludusavi ha un plugin Decky, quindi puoi lanciare un backup senza uscire dall'interfaccia console.
- **Vuoi una licenza permissiva.** Ludusavi è MIT, Hoard è AGPL-3.0. Se hai in mente di costruirci sopra qualcosa senza pubblicare il risultato, quella differenza pesa.
- **Non vuoi niente che giri in sottofondo.** Ospitare Hoard da soli significa tenere in piedi un piccolo server da qualche parte, anche sullo stesso PC. Ludusavi è un'app che apri quando ti serve.

## Passare da Ludusavi a Hoard

Non c'è un importatore, ed è voluto. I passaggi:

1. **Lascia i backup di Ludusavi esattamente dove sono.** Non viene migrato né cancellato nulla. Tienili come rete di sicurezza per le prime settimane.
2. **Installa Hoard e accedi**, oppure puntalo al tuo server.
3. **Lascia che faccia la scansione.** Legge lo stesso manifest, quindi l'elenco dei giochi rilevati dovrebbe esserti familiare.
4. **Non puntare Hoard alla cartella dei backup di Ludusavi.** Traccia la cartella in cui scrive il gioco. Una cartella di backup è una copia che cambia secondo un orario e non quando giochi, e sincronizzare la copia di una copia è il modo in cui si finisce per ripristinare i progressi di ieri. Hoard prova a rilevarlo da solo — \`hoard doctor\` segnala una cartella tracciata che sembra un mirror di backup — ma è più semplice non tracciarla affatto.
5. **Gioca una volta.** All'uscita, la prima versione compare nella cronologia.
6. **Ripeti sul secondo PC.** Accedi lì e le versioni sono già pronte.

## Due dettagli da sapere

**I salvataggi di Steam stanno una cartella più in profondità di quanto sembri.** Per i giochi Steam, Hoard traccia \`<AppID>/remote/\` dentro \`userdata\`, non la cartella superiore. Quella superiore contiene anche \`remotecache.vdf\` e i file di obiettivi e tempo di gioco, che legittimamente cambiano da macchina a macchina. Se sincronizzi la cartella superiore, ogni avvio sembra un conflitto anche se nessun salvataggio si è mosso. È il motivo più comune per cui un setup artigianale tra Steam Deck e desktop finisce per combattere contro sé stesso.

**Le versioni costano poco.** Gli snapshot sono archiviati per hash del contenuto, quindi un file che non cambia viene salvato una volta sola. Dieci versioni di un salvataggio da 2 GB occupano circa 2 GB, non 20 — ed è questo che rende pratico conservare tutta la cronologia invece di potarla.

## Cosa significa davvero il self-hosting

È il punto su cui quasi tutti i confronti sbagliano riguardo a Hoard, quindi vale la pena essere precisi. Ci sono due modi di usarlo, e sono davvero diversi:

- **Hoard Cloud** è l'opzione gestita: accedi e i tuoi salvataggi stanno sui nostri server, nell'UE.
- **Il self-hosting è interamente tuo.** Fai girare \`hoard-server\` sul tuo PC o sul tuo NAS, e i salvataggi vanno dalla tua macchina al tuo disco. **Nessun account con noi, nessuna telemetria verso di noi, nessuna quota e nessun relay**: non passa nulla dai nostri server, perché sul percorso non c'è niente di nostro. Non possiamo vedere un salvataggio, il nome di un gioco o un indirizzo email, per il semplice motivo che niente di tutto ciò ci arriva. Se Hoard Cloud sparisse domani, un'installazione self-hosted continuerebbe uguale.

Stesso programma, stesso rilevamento, stessa cronologia delle versioni. L'unica cosa che cambia è di chi è lo spazio di archiviazione.

## Quale scegliere?

- Scegli **Ludusavi** se vuoi uno strumento di backup gratuito e locale e non ti dispiace montare il tuo cloud con Rclone.
- Scegli **Hoard** se vuoi che backup *e* sincronizzazione tra PC funzionino da soli, con una cronologia cloud versionata, mantenendo l'opzione del self-hosting.

Molti iniziano con Ludusavi per i backup locali e passano a Hoard quando giocano agli stessi giochi su più di una macchina. Se è il tuo caso, vedi [come sincronizzare i salvataggi tra PC](/guides/sync-game-saves-across-pcs) o semplicemente [scarica Hoard](/download) e accedi. Per il quadro completo c'è un [confronto di tutti gli strumenti di sincronizzazione](/guides/game-save-sync-comparison).

<!-- faq -->

## Domande frequenti

### Posso usare Ludusavi e Hoard insieme?

Sì. Leggono le stesse posizioni e nessuno dei due tiene i file bloccati. Molti tengono Ludusavi per i backup di archivio locali e lasciano a Hoard la sincronizzazione tra macchine. L'unica regola è non puntare uno dei due alla cartella di backup dell'altro.

### Hoard importa i miei backup di Ludusavi?

No, ed è deliberato. Una cartella di backup è una copia che cambia secondo il proprio orario: tracciarla sincronizzerebbe un mirror vecchio invece del salvataggio reale. Hoard traccia la cartella in cui scrive il gioco e avvia la propria cronologia dalla sessione successiva. Tieni l'archivio di Ludusavi come rete di sicurezza.

### Hoard è gratuito?

Hoard Cloud ha un piano gratuito con 2 GB di spazio e 3 dispositivi, che copre la maggior parte delle collezioni; Pro alza entrambi. Ospitare il server per conto proprio è gratis e non ha alcuna quota. Tutto è open source sotto AGPL-3.0.

### Hoard funziona su Steam Deck?

Sì, su Steam Deck e su qualsiasi desktop Linux, oltre che su Windows e macOS. Il Deck è proprio il caso che richiede il dettaglio su \`remote/\` qui sopra, perché un Deck e un desktop scrivono file di obiettivi e tempo di gioco diversi accanto allo stesso salvataggio.

### Mi serve Rclone o un account cloud mio?

No. È la differenza pratica principale: con Hoard Cloud lo spazio è già pronto quando accedi. Se preferisci essere padrone dello spazio, fai girare il server tu stesso su un bucket compatibile con S3 o una normale cartella della tua macchina.

### Il self-hosting manda qualcosa a Hoard?

No. In modalità self-hosted non c'è alcun account con noi né telemetria verso di noi: i tuoi salvataggi, i tuoi utenti e i tuoi log stanno sul tuo server e non toccano mai il nostro. È tutto il senso di questa modalità, ed è il motivo per cui il server è lo stesso binario open source che usiamo noi e non una versione ridotta.
`,nn=`---
title: "Ludusavi の代替：セーブデータの自動クラウド同期"
description: "Ludusavi と Hoard の公平な比較。Ludusavi はローカルバックアップに優れたオープンソースツール。Hoard は同じ位置データを使いつつ、すべての PC でマネージドなクラウド同期と世代履歴を追加します。"
order: 5
updated: 2026-09-01
---

セーブデータをバックアップして同期する方法を探しているなら、おそらく **Ludusavi** にたどり着いたはずです――そして優れたツールです。このガイドは適切なツールを選べるよう正直に比較し、端末間での自動クラウド同期が欲しい場合に Hoard がどこに位置づくかを説明します。

## Ludusavi の優れている点

Ludusavi は Windows、macOS、Linux で PC のセーブをバックアップ・復元する無料のオープンソースツール（mtkennerly 作）です。すっきりした GUI と CLI を備え、数千のゲームのセーブを自動で見つけ、世代管理されたローカルバックアップを保持し、**Rclone** を設定すれば自分のクラウド（Google Drive、Dropbox など）へバックアップを送れます。完全な制御と自前のセットアップが欲しいなら、Ludusavi は素晴らしい選択肢で、しかも完全に無料です。

Hoard はそれを置き換えるためのものではありません。実際、**Hoard は Ludusavi が依拠しているのと同じコミュニティのセーブ位置データベース** を使って各ゲームのセーブ場所を特定するため、検出の品質は同等です。

## Hoard が異なる点

ローカル中心のツールで多くの人がぶつかる壁が、**端末間の同期** です。Ludusavi では自分で行います。バックアップをスケジュールし、Rclone のリモートを設定し、プレイ前にもう一方の PC で復元する。動作はしますが、手作業です。

Hoard はこれを **マネージドなクラウド同期** に変えます。

- **サインインするだけ。** Rclone のリモートもスクリプトも不要。Hoard はプレイ後にセーブをアップロードし、開始前に最新版をダウンロードします。アカウント内のすべての PC で行われます。
- **クラウド上の世代履歴。** すべてのバックアップが保持されるので、以前のどのセーブにも巻き戻せます――ディスク故障やクリーンインストールの後でも。
- **競合を認識。** Hoard はタイムスタンプを比較し、置き換えるものすべてのローカルコピーを保持するため、同期が黙って進行を壊すことはありません。
- **引き続きオープンソースでセルフホスト可能。** Ludusavi と同様にロックインはありません――Hoard Cloud を使うか、サーバーを自分でホストできます。

## 一覧で比較

| | Ludusavi | Hoard |
|---|---|---|
| ローカルバックアップ | あり | あり |
| セーブの検出 | コミュニティのマニフェスト | 同じマニフェストに加え、Steam ライブラリ、実行中プロセス、ファイルシステムの走査 |
| クラウドの保存先 | 自前、Rclone 経由 | 同梱、または自分のサーバー |
| PC 間の同期 | 手動：こちらでバックアップ、あちらで復元 | 自動：プレイ終了後と開始前 |
| 世代履歴 | 自分で整理するローカルバックアップ | すべての世代をクラウドに保持し、内容ハッシュで重複排除 |
| エミュレーター | 対応 | 対応 |
| インターフェース | デスクトップアプリと CLI | デスクトップアプリ、CLI、ゲーム内オーバーレイ |
| 価格 | 無料 | 無料枠は 2 GB・3 台、それ以上は Pro、セルフホストなら上限なし |
| ライセンス | MIT | AGPL-3.0 |

## Ludusavi のほうが向いている場合

ほとんどの比較ページが省く部分です。次の場合は Ludusavi のほうが適しています。

- **PC 1 台でしか遊ばない。** クラウド同期は存在しない問題を解決することになります。ローカルバックアップで十分で、Ludusavi はそれが得意です。
- **信頼している Rclone リモートがすでにある。** 保存先が設定済みで動いているなら、Hoard の主な利点はすでに済ませた手間の肩代わりです。
- **Steam Deck のゲームモードから使いたい。** Ludusavi には Decky プラグインがあり、コンソール画面を離れずにバックアップを実行できます。
- **緩やかなライセンスが必要。** Ludusavi は MIT、Hoard は AGPL-3.0 です。上に何かを作って結果を公開しないつもりなら、この違いは大きく効きます。
- **常駐するものを増やしたくない。** Hoard のセルフホストは、同じ PC 上であっても小さなサーバーを動かし続けることを意味します。Ludusavi は必要なときに開くアプリです。

## Ludusavi から Hoard へ移る

インポート機能はなく、それは意図的です。手順は次のとおりです。

1. **Ludusavi のバックアップはそのままの場所に残す。** 何も移行せず、何も削除しません。最初の数週間は安全網として保管してください。
2. **Hoard をインストールしてサインインする。** あるいは自分のサーバーを指定します。
3. **スキャンさせる。** 同じマニフェストを読むので、検出されるゲームの一覧は見覚えのあるものになるはずです。
4. **Hoard を Ludusavi のバックアップフォルダーに向けない。** ゲーム自身が書き込むフォルダーを追跡してください。バックアップフォルダーはプレイ時ではなくスケジュールで変わる複製であり、複製の複製を同期すると昨日の進行を復元することになります。Hoard は自力で気づこうとし、\`hoard doctor\` がバックアップの写しに見える追跡フォルダーを警告しますが、最初から追跡しないほうが簡単です。
5. **一度プレイする。** 終了すると最初の世代が履歴に現れます。
6. **2 台目の PC でも同じことを。** サインインすれば、世代はもうそこにあります。

## 知っておく価値のある 2 点

**Steam のセーブは思っているより 1 階層深い。** Steam のゲームでは、Hoard は \`userdata\` の中の \`<AppID>/remote/\` を追跡し、その上のフォルダーは追跡しません。上のフォルダーには \`remotecache.vdf\` や実績・プレイ時間のファイルもあり、これらはマシンごとに違って当然です。上を同期すると、セーブが動いていなくても起動のたびに競合に見えます。Steam Deck とデスクトップの自作構成が自分自身と喧嘩する、いちばん多い原因がこれです。

**世代は安い。** スナップショットは内容ハッシュで保存されるため、変わっていないファイルは一度だけ保存されます。2 GB のセーブの 10 世代は約 20 GB ではなく約 2 GB です。履歴を間引かずに丸ごと残せるのはこのためです。

## セルフホストの本当の意味

Hoard について多くの比較が誤解している点なので、正確に書きます。動かし方は 2 通りあり、両者は本当に別物です。

- **Hoard Cloud** はマネージドな選択肢です。サインインすると、セーブは EU にある当方のサーバーに保存されます。
- **セルフホストは完全にあなたのものです。** 自分の PC や NAS で \`hoard-server\` を動かし、セーブは自分のマシンから自分のディスクへ移ります。**当方のアカウントも、当方へのテレメトリも、容量制限も、中継もありません。** 経路上に当方のものが何一つないため、当方のサーバーを何も通りません。セーブもゲーム名もメールアドレスも見えません。そもそも届かないからです。仮に明日 Hoard Cloud が消えても、セルフホスト構成はそのまま動き続けます。

同じプログラム、同じ検出、同じ世代履歴。変わるのは保存先が誰のものかだけです。

## どちらを選ぶべきか

- 無料でローカル中心のバックアップツールが欲しく、Rclone で自分のクラウドを組むのが苦でないなら **Ludusavi** を選びましょう。
- バックアップ *と* PC 間の同期が手間なく動き、世代付きのクラウド履歴を持ちつつ、セルフホストの選択肢も残したいなら **Hoard** を選びましょう。

多くの人はローカルバックアップに Ludusavi で始め、複数のマシンで同じゲームをプレイするようになると Hoard に移行します。あなたがそうなら、[PC 間でセーブを同期する方法](/guides/sync-game-saves-across-pcs) をご覧いただくか、[Hoard をダウンロード](/download) してサインインしてください。全体像は [セーブ同期ツールの比較](/guides/game-save-sync-comparison) にまとめてあります。

<!-- faq -->

## よくある質問

### Ludusavi と Hoard を同時に使えますか？

はい。両者は同じセーブ位置を読み、どちらもファイルを掴んだままにしません。ローカルの保管用バックアップは Ludusavi に任せ、マシン間の同期は Hoard に任せている人は多くいます。唯一の注意は、一方をもう一方のバックアップフォルダーに向けないことです。

### Hoard は Ludusavi のバックアップを取り込みますか？

いいえ。これは意図的です。バックアップフォルダーは独自のスケジュールで変わる複製なので、追跡すると実際のセーブではなく古い写しを同期してしまいます。Hoard はゲームが書き込むフォルダーを追跡し、次のセッションから独自の履歴を始めます。Ludusavi の保管分は安全網として残してください。

### Hoard は無料ですか？

Hoard Cloud には 2 GB・3 台の無料枠があり、多くのセーブ環境はこれで足ります。Pro は両方を引き上げます。サーバーを自分でホストする場合は無料で、容量制限もありません。すべて AGPL-3.0 のオープンソースです。

### Steam Deck で動きますか？

はい。Steam Deck と任意の Linux デスクトップ、加えて Windows と macOS で動きます。Deck はまさに上の \`remote/\` の話が効く場面です。Deck とデスクトップは同じセーブの隣に、異なる実績・プレイ時間のファイルを書くからです。

### Rclone や自分のクラウドアカウントは必要ですか？

いいえ。そこが実用上の最大の違いです。Hoard Cloud ならサインインした時点で保存先が用意されています。保存先を自分で所有したい場合は、S3 互換のバケットか自分のマシンの普通のフォルダーを指定してサーバーを動かしてください。

### セルフホストは Hoard に何かを送信しますか？

いいえ。セルフホストでは当方のアカウントも当方へのテレメトリもありません。セーブもユーザーもログも自分のサーバーの中にとどまり、当方のサーバーには一切触れません。それがこのモードの目的であり、サーバーが機能を削った別物ではなく、当方自身が動かしているものと同じオープンソースのバイナリである理由です。
`,an=`---
title: "Alternativa ao Ludusavi: sincronização automática de saves na nuvem"
description: "Uma comparação justa entre o Ludusavi e o Hoard. O Ludusavi é uma excelente ferramenta open source de backup local; o Hoard acrescenta sincronização na nuvem gerida e histórico versionado em todos os teus PCs — usando os mesmos dados de localização."
order: 5
updated: 2026-09-01
---

Se procuras uma forma de fazer backup e sincronizar os teus saves, é provável que tenhas encontrado o **Ludusavi** — e é excelente. Este guia é uma comparação honesta para te ajudar a escolher a ferramenta certa, e explica onde o Hoard se encaixa se quiseres sincronização na nuvem automática entre máquinas.

## O que o Ludusavi faz bem

O Ludusavi é uma ferramenta gratuita e open source (criada por mtkennerly) para fazer backup e restaurar saves de PC em Windows, macOS e Linux. Tem uma GUI limpa e uma CLI, encontra automaticamente os saves de milhares de jogos, guarda backups locais versionados e pode enviar esses backups para uma nuvem tua configurando o **Rclone** (Google Drive, Dropbox e muitos outros). Se queres controlo total e uma configuração faz-tu-mesmo, o Ludusavi é uma escolha fantástica — e completamente gratuita.

O Hoard não vem substituir isso. Na verdade, **o Hoard usa a mesma base de dados comunitária de localizações em que o Ludusavi se apoia** para localizar onde cada jogo guarda os saves, por isso a qualidade da deteção está ao mesmo nível.

## Em que o Hoard é diferente

O ponto onde a maioria esbarra com qualquer ferramenta local é a **sincronização entre dispositivos**. Com o Ludusavi fá-lo tu: agendar um backup, configurar um remoto Rclone, e depois restaurar no outro PC antes de jogar. Funciona, mas é manual.

O Hoard transforma isso em **sincronização na nuvem gerida**:

- **Inicia sessão e pronto.** Sem remotos Rclone, sem scripts. O Hoard envia o teu save depois de jogares e descarrega a versão mais recente antes de começares, em cada PC da tua conta.
- **Histórico versionado na nuvem.** Cada backup é guardado, por isso podes voltar a qualquer save anterior — mesmo depois de uma falha de disco ou de uma instalação limpa.
- **Tem em conta os conflitos.** O Hoard compara os timestamps e guarda uma cópia local de tudo o que substitui, por isso uma sincronização nunca destrói progresso em silêncio.
- **Continua open source e self-hostable.** Como o Ludusavi, não há aprisionamento — usa o Hoard Cloud ou aloja o servidor tu mesmo.

## Frente a frente

| | Ludusavi | Hoard |
|---|---|---|
| Backups locais | Sim | Sim |
| Deteção de saves | Manifesto comunitário | O mesmo manifesto, mais bibliotecas Steam, processos em execução e uma varredura do disco |
| Armazenamento na nuvem | O teu, via Rclone | Incluído, ou o teu próprio servidor |
| Sincronização entre PCs | Manual: backup aqui, restauro ali | Automática, depois de jogares e antes de começares |
| Histórico de versões | Backups locais que limpas tu | Todas as versões na nuvem, deduplicadas por hash de conteúdo |
| Emuladores | Sim | Sim |
| Interfaces | App de ambiente de trabalho e CLI | App de ambiente de trabalho, CLI e overlay dentro do jogo |
| Preço | Gratuito | Plano gratuito de 2 GB e 3 dispositivos, Pro acima disso, sem qualquer quota em self-hosting |
| Licença | MIT | AGPL-3.0 |

## Quando o Ludusavi é a melhor escolha

Esta é a parte que quase nenhuma página de comparação inclui. O Ludusavi é a melhor ferramenta quando:

- **Só jogas num PC.** A sincronização na nuvem resolve um problema que não tens. Um backup local chega, e o Ludusavi faz backups locais muito bem.
- **Já tens um remoto Rclone em que confias.** Se o teu armazenamento está montado e a funcionar, a principal vantagem do Hoard é um passo de configuração que já pagaste.
- **Queres usá-lo a partir do modo de jogo de uma Steam Deck.** O Ludusavi tem um plugin Decky, por isso podes lançar um backup sem sair da interface de consola.
- **Queres uma licença permissiva.** O Ludusavi é MIT e o Hoard é AGPL-3.0. Se pensas construir algo por cima e não publicar o resultado, essa diferença pesa.
- **Não queres nada a correr em pano de fundo.** Alojar o Hoard tu mesmo implica manter um pequeno servidor de pé, mesmo que seja no próprio PC. O Ludusavi é uma aplicação que abres quando precisas.

## Passar do Ludusavi para o Hoard

Não há importador, e é de propósito. Os passos:

1. **Deixa os teus backups do Ludusavi exatamente onde estão.** Nada é migrado nem apagado. Guarda-os como rede de segurança nas primeiras semanas.
2. **Instala o Hoard e inicia sessão**, ou aponta-o ao teu próprio servidor.
3. **Deixa-o analisar.** Lê o mesmo manifesto, por isso a lista de jogos detetados deve ser-te familiar.
4. **Não apontes o Hoard para a pasta de backups do Ludusavi.** Segue a pasta onde o jogo escreve. Uma pasta de backups é uma cópia que muda por horário e não quando jogas, e sincronizar a cópia de uma cópia é como se acaba a restaurar o progresso de ontem. O Hoard tenta detetá-lo sozinho — \`hoard doctor\` assinala uma pasta seguida que parece um espelho de backups — mas é mais simples nunca a seguir.
5. **Joga uma vez.** Ao sair, a primeira versão aparece no histórico.
6. **Repete no segundo PC.** Inicias sessão e as versões já lá estão.

## Dois detalhes que vale a pena saber

**Os saves da Steam vivem uma pasta mais abaixo do que parece.** Nos jogos da Steam, o Hoard segue \`<AppID>/remote/\` dentro de \`userdata\`, não a pasta acima. A pasta acima guarda também \`remotecache.vdf\` e ficheiros de proezas e tempo de jogo, que são legitimamente diferentes em cada máquina. Se sincronizares a pasta acima, cada arranque parece um conflito mesmo sem nenhum save se ter mexido. É o motivo mais comum para uma montagem caseira entre Steam Deck e desktop acabar a lutar contra si própria.

**As versões são baratas.** Os snapshots são guardados por hash de conteúdo, por isso um ficheiro que não muda é guardado uma só vez. Dez versões de um save de 2 GB ocupam cerca de 2 GB, não 20 — e é isso que torna prático manter o histórico inteiro em vez de o ir cortando.

## O que self-hosting quer mesmo dizer

É o ponto em que quase todas as comparações se enganam sobre o Hoard, por isso convém ser exato. Há duas formas de o usar, e são genuinamente diferentes:

- **O Hoard Cloud** é a opção gerida: inicias sessão e os teus saves ficam nos nossos servidores, na UE.
- **O self-hosting é inteiramente teu.** Corres o \`hoard-server\` no teu PC ou no teu NAS, e os teus saves vão da tua máquina para o teu disco. **Não há conta connosco, nem telemetria para nós, nem quota, nem retransmissão**: não passa nada pelos nossos servidores, porque não há nada nosso no caminho. Não conseguimos ver um save, o nome de um jogo ou um endereço de email, pela simples razão de que nada disso nos chega. Se o Hoard Cloud desaparecesse amanhã, uma instalação self-hosted continuaria igual.

O mesmo programa, a mesma deteção, o mesmo histórico de versões. A única coisa que muda é de quem é o armazenamento.

## Qual deves escolher?

- Escolhe o **Ludusavi** se queres uma ferramenta de backup gratuita e local e não te importas de montar a tua própria nuvem com o Rclone.
- Escolhe o **Hoard** se queres que o backup *e* a sincronização entre PCs simplesmente funcionem, com um histórico na nuvem versionado, mantendo a opção de self-hosting.

Muita gente começa com o Ludusavi para backups locais e passa para o Hoard quando joga os mesmos jogos em mais de uma máquina. Se é o teu caso, vê [como sincronizar saves entre PCs](/guides/sync-game-saves-across-pcs) ou simplesmente [descarrega o Hoard](/download) e inicia sessão. Para o panorama completo, há uma [comparação de todas as ferramentas de sincronização](/guides/game-save-sync-comparison).

<!-- faq -->

## Perguntas frequentes

### Posso usar o Ludusavi e o Hoard ao mesmo tempo?

Sim. Leem as mesmas localizações e nenhum dos dois bloqueia os ficheiros. Muita gente mantém o Ludusavi para backups de arquivo locais e deixa o Hoard tratar da sincronização entre máquinas. A única regra é não apontar uma ferramenta para a pasta de backups da outra.

### O Hoard importa os meus backups do Ludusavi?

Não, e é deliberado. Uma pasta de backups é uma cópia que muda segundo o seu próprio horário, por isso segui-la sincronizaria um espelho desatualizado em vez do teu save real. O Hoard segue a pasta onde o jogo escreve e começa o seu próprio histórico a partir da tua sessão seguinte. Guarda o arquivo do Ludusavi como rede de segurança.

### O Hoard é gratuito?

O Hoard Cloud tem um plano gratuito com 2 GB de armazenamento e 3 dispositivos, o que cobre a maioria das coleções; o Pro sobe ambos. Alojar o servidor tu mesmo é gratuito e não tem quota nenhuma. Tudo é open source sob AGPL-3.0.

### O Hoard funciona na Steam Deck?

Sim, na Steam Deck e em qualquer ambiente de trabalho Linux, além de Windows e macOS. A Deck é exatamente o caso que precisa do detalhe do \`remote/\` acima, porque uma Deck e um desktop escrevem ficheiros de proezas e tempo de jogo diferentes ao lado do mesmo save.

### Preciso de Rclone ou de uma conta de nuvem minha?

Não. É essa a principal diferença prática: com o Hoard Cloud o armazenamento já está pronto quando inicias sessão. Se preferes ser dono do armazenamento, corre o servidor tu mesmo contra um bucket compatível com S3 ou uma pasta normal da tua máquina.

### O self-hosting envia alguma coisa para o Hoard?

Não. Em modo self-hosted não há conta connosco nem telemetria para nós: os teus saves, os teus utilizadores e os teus registos vivem no teu próprio servidor e nunca tocam no nosso. É esse o sentido do modo, e é por isso que o servidor é o mesmo binário open source que nós corremos e não uma versão reduzida.
`,on=`---
title: "Ludusavi 替代方案：游戏存档的自动云同步"
description: "对 Ludusavi 与 Hoard 的公平对比。Ludusavi 是出色的开源本地备份工具；Hoard 在使用相同位置数据的同时，为你的所有 PC 增加托管式云同步与版本历史。"
order: 5
updated: 2026-09-01
---

如果你在寻找备份和同步游戏存档的方法，那你很可能已经找到了 **Ludusavi**——它非常出色。本指南是一份诚实的对比，帮助你选对工具，并说明当你需要跨机器自动云同步时，Hoard 的定位在哪里。

## Ludusavi 的优点

Ludusavi 是一款免费的开源工具（由 mtkennerly 开发），可在 Windows、macOS 和 Linux 上备份与还原 PC 游戏存档。它有简洁的图形界面和命令行，能自动找到数千款游戏的存档，保留带版本的本地备份，并可通过配置 **Rclone** 把这些备份推送到你自己的云端（Google Drive、Dropbox 等）。如果你想要完全掌控和自己动手的方案，Ludusavi 是绝佳选择——而且完全免费。

Hoard 并非来取代它。事实上，**Hoard 使用与 Ludusavi 所依赖的相同的社区存档位置数据库**来定位每款游戏存档的位置，因此检测质量不相上下。

## Hoard 的不同之处

大多数人在任何以本地为主的工具上都会遇到的瓶颈，是**跨设备同步**。用 Ludusavi 时你得自己来：安排备份、配置 Rclone 远端，然后在玩之前在另一台 PC 上还原。这能行，但是手动的。

Hoard 把它变成**托管式云同步**：

- **登录即用。** 无需 Rclone 远端，无需脚本。Hoard 会在你玩完后上传存档，并在你开始前下载最新版本，覆盖你账号下的每台 PC。
- **云端版本历史。** 每个备份都会保留，因此你可以回退到任意较早的存档——即使在磁盘故障或全新安装之后。
- **冲突感知。** Hoard 会比较时间戳，并为它替换的一切保留本地副本，因此同步绝不会悄无声息地破坏进度。
- **依然开源且可自托管。** 与 Ludusavi 一样，没有锁定——使用 Hoard Cloud，或自己托管服务器。

## 逐项对比

| | Ludusavi | Hoard |
|---|---|---|
| 本地备份 | 有 | 有 |
| 存档检测 | 社区清单 | 同一份清单，另加 Steam 库、运行中的进程与文件系统扫描 |
| 云端存储 | 自备，通过 Rclone | 内置，或你自己的服务器 |
| 多台 PC 同步 | 手动：这边备份，那边还原 | 自动：玩完之后、开始之前 |
| 版本历史 | 需要你自己清理的本地备份 | 每个版本都留在云端，按内容哈希去重 |
| 模拟器 | 支持 | 支持 |
| 界面 | 桌面应用与命令行 | 桌面应用、命令行与游戏内浮层 |
| 价格 | 免费 | 免费额度 2 GB、3 台设备，超出走 Pro，自托管则完全没有配额 |
| 许可证 | MIT | AGPL-3.0 |

## 什么时候 Ludusavi 更合适

这是几乎所有对比页面都会略过的部分。以下情况 Ludusavi 是更好的工具：

- **你只在一台 PC 上玩。** 云同步解决的是你没有的问题。本地备份就够了，而 Ludusavi 的本地备份做得很好。
- **你已经有一个用着放心的 Rclone 远端。** 如果存储已经配好并正常运行，Hoard 的主要优势正是你已经付出过的那一步。
- **你想在 Steam Deck 的游戏模式里用。** Ludusavi 有 Decky 插件，不必离开主机界面就能触发备份。
- **你需要宽松的许可证。** Ludusavi 是 MIT，Hoard 是 AGPL-3.0。如果你打算在其之上做东西且不公开成果，这个差别很关键。
- **你不想有东西常驻运行。** 自托管 Hoard 意味着要让一个小服务器一直开着，哪怕就在同一台 PC 上。Ludusavi 是你需要时才打开的应用。

## 从 Ludusavi 迁到 Hoard

没有导入功能，这是有意为之。步骤如下：

1. **让 Ludusavi 的备份原地不动。** 不迁移也不删除任何东西。头几周把它们留作安全网。
2. **安装 Hoard 并登录**，或把它指向你自己的服务器。
3. **让它扫描。** 它读取同一份清单，因此检测出的游戏列表应该很眼熟。
4. **不要把 Hoard 指向 Ludusavi 的备份文件夹。** 请追踪游戏本身写入的那个文件夹。备份文件夹是一份按计划变化、而非随你游玩变化的副本，同步副本的副本正是最终还原到昨天进度的原因。Hoard 会尝试自行识别——\`hoard doctor\` 会对看起来像备份镜像的被追踪文件夹发出提示——但更省事的办法是从一开始就别追踪它。
5. **先玩一次。** 退出时，第一个版本会出现在历史里。
6. **在第二台 PC 上重复一遍。** 登录之后，版本已经在那里等着了。

## 两个值得知道的细节

**Steam 存档比你以为的深一层。** 对 Steam 游戏，Hoard 追踪 \`userdata\` 里的 \`<AppID>/remote/\`，而不是它上一层的文件夹。上一层还放着 \`remotecache.vdf\` 以及成就和游戏时长文件，这些在不同机器上本就应该不同。同步上一层，每次启动都像是冲突，尽管没有任何存档动过。这正是 Steam Deck 与台式机的手工方案最终自己跟自己打架的最常见原因。

**版本很便宜。** 快照按内容哈希存储，未改动的文件只存一份。一个 2 GB 存档的十个版本大约占 2 GB，而不是 20 GB——正因如此，保留完整历史才是可行的，不必不断修剪。

## 自托管到底意味着什么

这是多数对比在 Hoard 上弄错的地方，所以说得精确些。它有两种运行方式，而且两者确实不同：

- **Hoard Cloud** 是托管方案：你登录，存档保存在我们位于欧盟的服务器上。
- **自托管完全属于你。** 你在自己的 PC 或 NAS 上运行 \`hoard-server\`，存档从你的机器走到你的硬盘。**没有我们这边的账号，没有发往我们的遥测，没有配额，也没有中转**——不经过我们的任何服务器，因为这条路径上根本没有我们的东西。我们看不到任何存档、游戏名或邮箱地址，原因很简单：这些从未到达我们这里。就算 Hoard Cloud 明天消失，自托管的部署照常运行。

同一个程序，同样的检测，同样的版本历史。唯一变化的是存储归谁所有。

## 你该选哪个？

- 如果你想要一款免费、以本地为主的备份工具，并且不介意用 Rclone 搭建自己的云端，就选 **Ludusavi**。
- 如果你想让备份*和*跨 PC 同步都自动生效，拥有带版本的云端历史，同时保留自托管的选项，就选 **Hoard**。

很多人先用 Ludusavi 做本地备份，等到在不止一台机器上玩同样的游戏时再转向 Hoard。如果这就是你，请见[如何在多台 PC 之间同步存档](/guides/sync-game-saves-across-pcs)，或直接[下载 Hoard](/download) 并登录。想看完整的横向对比，可以读[所有存档同步工具的比较](/guides/game-save-sync-comparison)。

<!-- faq -->

## 常见问题

### 可以同时使用 Ludusavi 和 Hoard 吗？

可以。两者读取相同的存档位置，也都不会长期占用文件。很多人用 Ludusavi 做本地归档备份，把机器之间的同步交给 Hoard。唯一的原则是：不要让其中一个指向另一个的备份文件夹。

### Hoard 会导入我的 Ludusavi 备份吗？

不会，这是刻意的。备份文件夹是一份按自己节奏变化的副本，追踪它会同步一份过期镜像，而不是你真正的存档。Hoard 追踪游戏写入的那个文件夹，并从你的下一次游玩开始建立自己的历史。请把 Ludusavi 的归档留作安全网。

### Hoard 是免费的吗？

Hoard Cloud 提供 2 GB 存储和 3 台设备的免费额度，足够覆盖大多数存档收藏；Pro 会同时提高这两项。自己托管服务器是免费的，而且完全没有配额。全部代码以 AGPL-3.0 开源。

### Hoard 支持 Steam Deck 吗？

支持，包括 Steam Deck 和任何 Linux 桌面，以及 Windows 和 macOS。Deck 正是上面 \`remote/\` 那个细节要解决的场景：Deck 和台式机会在同一份存档旁边写入不同的成就与游戏时长文件。

### 我需要 Rclone 或自己的云账号吗？

不需要。这正是最主要的实际差别：使用 Hoard Cloud，登录时存储就已经准备好了。如果你更想自己拥有存储，可以让服务器对接兼容 S3 的存储桶，或你机器上的一个普通文件夹。

### 自托管会向 Hoard 发送任何东西吗？

不会。在自托管模式下，没有我们这边的账号，也没有发往我们的遥测：你的存档、你的用户和你的日志都留在你自己的服务器上，从不接触我们的服务器。这正是这一模式的意义，也是服务器用的是我们自己在跑的同一个开源二进制、而不是删减版的原因。
`,sn=`---
title: "OpenSave-Alternative: direkt zwischen Geräten oder über einen eigenen Server"
description: "OpenSave synchronisiert Spielstände direkt zwischen deinen PCs, ohne etwas dazwischen. Hoard synchronisiert über einen Server — unseren oder deinen — und führt eine Versionshistorie. Ein ehrlicher Blick darauf, wann welches Design gewinnt."
order: 8
updated: 2026-09-01
---

Beide Werkzeuge lösen dasselbe Problem und sind sich über die Architektur uneinig, und genau das ist das Einzige, was einen Vergleich lohnt. Diese Seite legt die zwei Ansätze nebeneinander, samt der Fälle, in denen der andere die bessere Antwort ist.

## Der eigentliche Unterschied: direkt oder über einen Server

**OpenSave** arbeitet peer-to-peer. Deine Maschinen reden direkt miteinander, dazwischen sitzt nichts. Kein Konto, kein Speicher, den man bezahlt, und optional lässt sich eine Kopie in eine Cloud spiegeln, die du ohnehin hast.

**Hoard** synchronisiert über einen Server. Dieser Server ist entweder Hoard Cloud, von uns betrieben, oder \`hoard-server\` auf deinem eigenen PC oder NAS. Dein Stand geht hoch, wenn du aufhörst, und kommt herunter, wenn eine andere Maschine danach fragt.

Alles Weitere folgt aus dieser einen Entscheidung.

## Was dir ein Server bringt

- **Die andere Maschine muss nicht laufen.** Du hörst am Desktop auf, der Laptop bleibt eine Woche zu, und beim Aufklappen wartet der neueste Stand. Peer-to-peer braucht beide Enden gleichzeitig wach — am Schreibtisch kein Problem, mit einem Handheld, das du zweimal im Monat anfasst, schon.
- **Eine Versionshistorie statt nur des letzten Zustands.** Jede Sitzung wird eine Version, zu der du zurückkannst. Das zählt an dem Tag, an dem ein Mod deine Welt frisst oder ein Stand halb geschrieben landet: eine direkte Synchronisierung kopiert die kaputte Datei getreulich auf den anderen PC.
- **Eine Kopie, die die Hardware überlebt.** Dass beide PCs in derselben Wohnung sterben, ist kein exotisches Szenario. Ein Spielstand, den es nur auf diesen zwei Maschinen gab, stirbt mit ihnen.
- **Nichts am Netzwerk zu regeln.** Kein NAT zu durchqueren, kein Port zu öffnen, keine Bedingung, dass beide im selben LAN hängen.

## Was dir peer-to-peer bringt

Fairerweise die andere Seite:

- **Nie Speicher zu bezahlen.** Es gibt kein Limit zu erreichen, weil es keinen Speicherort gibt. Hoards kostenloser Tarif sind 2 GB, darüber zahlst du oder hostest selbst.
- **Von Natur aus nichts dazwischen.** Wenn das Ziel ist, dass eine Datei nie die Platte eines Dritten berührt, ist direkte Übertragung die kürzestmögliche Antwort.
- **Nichts zu betreiben.** Kein Server, der laufen muss, nicht einmal ein eigener.

Wenn du an zwei Desktops spielst, die beide eingeschaltet sind, nie zurückrollen willst und über Speicher gar nicht nachdenken möchtest, passt dieses Design sauber, und Hoard ist mehr Maschinerie als nötig.

## Die Datenschutzfrage, präzise beantwortet

Hier gehen Vergleiche zu Hoard meist schief, deshalb genau: es gibt zwei Betriebsarten, und sie unterscheiden sich wirklich.

- **Hoard Cloud** ist die verwaltete Variante: du meldest dich an, und deine Stände liegen auf unseren Servern in der EU.
- **Selbsthosten gehört vollständig dir.** Du betreibst \`hoard-server\` auf deinem PC oder NAS, und deine Stände gehen von deiner Maschine auf deine Platte. Es gibt **kein Konto bei uns, keine Telemetrie zu uns, kein Limit und kein Relay** — nichts läuft über unsere Server, weil nichts von uns im Weg steht. Wir sehen weder Spielstand noch Spieltitel noch E-Mail-Adresse, weil davon nichts bei uns ankommt. Würde Hoard Cloud morgen abgeschaltet, liefe ein selbst gehostetes Setup unverändert weiter.

"Server" heißt also nicht "der Computer von jemand anderem", außer du willst es so. Ein selbst gehostetes Hoard hält deine Stände auf deiner eigenen Hardware, genau wie eine direkte Übertragung, und gibt dir zusätzlich Historie und den Fall der ausgeschalteten Maschine.

## Erkennung und Abdeckung

Beide Werkzeuge finden Spielstände für einen großen Katalog automatisch. Hoard liest dasselbe Community-Manifest für Speicherorte, das im Open-Source-Umfeld geteilt wird und über 20.000 Titel abdeckt, und legt Steam-Bibliotheken, laufende Prozesse und einen Dateisystem-Scan obendrauf. Bei Steam-Spielen verfolgt es \`<AppID>/remote/\` in \`userdata\` statt des Ordners darüber, denn der enthält \`remotecache.vdf\` und gerätebezogene Dateien für Erfolge und Spielzeit — synchronisiert man die, sieht jeder Start nach einem Konflikt aus. Ungewöhnliches richtest du von Hand ein.

## Was solltest du nehmen?

- **Peer-to-peer**, wenn deine Maschinen gleichzeitig laufen, Speicher gar nicht vorkommen soll und der letzte Stand alles ist, was du je gebraucht hast.
- **Hoard**, wenn du eine Historie zum Zurückrollen willst, eine Maschine eine Woche aus sein darf und eine Kopie beide PCs überleben soll — wahlweise über unsere Cloud oder deinen eigenen Server.

Es gibt einen breiteren [Vergleich aller Sync-Tools](/guides/game-save-sync-comparison) und einen [Ludusavi-Vergleich](/guides/ludusavi-alternative) für die Seite der lokalen Backups.

<!-- faq -->

## Häufige Fragen

### Braucht Hoard ein Konto?

Für Hoard Cloud ja, daran hängt die Synchronisierung. Selbst gehostet gibt es gar kein Konto bei uns: dein Server hat eigene Benutzer und ein Token je Gerät, und die verlassen deine Maschine nie.

### Funktioniert Hoard ganz ohne Cloud?

Ja. Betreibe \`hoard-server\` auf einem PC oder NAS, und deine Stände gehen von deiner Maschine auf deine Platte, ohne dass etwas über unsere Server läuft.

### Müssen beide PCs gleichzeitig online sein?

Nein, und das ist der praktische Vorteil der Synchronisierung über einen Server. Dein Stand wird hochgeladen, wenn du aufhörst, und heruntergeladen, sobald die andere Maschine das nächste Mal danach fragt.

### Führt eine Direktübertragung eine Versionshistorie?

Von sich aus nicht — eine Datei auf eine andere Maschine zu kopieren gibt dir den aktuellen Zustand auf beiden. Hoard sichert jede Sitzung als Version, und genau das macht das Zurückrollen eines beschädigten Stands möglich.

### Ist Hoard ebenfalls Open Source?

Ja, AGPL-3.0, Server inklusive. Der selbst gehostete Server ist dasselbe Binary, das wir betreiben, keine abgespeckte Edition.
`,rn=`---
title: "OpenSave alternative: peer-to-peer or a server you own"
description: "OpenSave syncs game saves directly between your PCs, with no server in the middle. Hoard syncs through a server — ours or one you host — and keeps a versioned history. An honest look at when each design wins."
order: 8
updated: 2026-09-01
---

Both tools solve the same problem and disagree about the architecture, which is the only thing worth comparing. This page lays the two designs side by side, including the cases where the other one is the better answer.

## The actual difference: peer-to-peer or a server

**OpenSave** is peer-to-peer. Your machines talk to each other directly, and nothing sits in between. There's no account and no storage to pay for, and it can optionally mirror a copy to a cloud drive you already have.

**Hoard** syncs through a server. That server is either Hoard Cloud, managed by us, or \`hoard-server\` running on your own PC or NAS. Your save goes up when you stop playing and comes down when another machine asks for it.

Everything else follows from that one choice.

## What a server buys you

- **The other machine doesn't have to be on.** You finish on the desktop, the laptop stays shut for a week, and the latest save is waiting when you open it. Peer-to-peer needs both ends awake at the same time, which is fine at a desk and awkward with a handheld you pick up twice a month.
- **A version history, not just the latest state.** Every session becomes a version you can roll back to. This is the part that matters the day a mod eats your world or a save is written half-corrupt: direct sync faithfully copies the broken file to your other PC.
- **A copy that survives the hardware.** Both your PCs dying in the same flat is not an exotic scenario. A save that only ever existed on those two machines dies with them.
- **Nothing to arrange on the network.** No NAT to traverse, no port to open, no both-devices-on-the-same-LAN caveat.

## What peer-to-peer buys you

Being fair about the other side:

- **No storage to pay for, ever.** There's no quota to hit, because there's no bucket. Hoard's free tier is 2 GB, and above that you either pay or self-host.
- **Nothing in the middle by design.** If the goal is that a file never touches a third party's disk, direct transfer is the shortest possible answer.
- **Nothing to run.** No server to keep up, not even your own.

If you play on two desktops that are both switched on, you never want to roll back, and you'd rather not think about storage at all, that design is a clean fit and Hoard is more machinery than you need.

## The privacy question, answered precisely

This is where comparisons of Hoard usually go wrong, so it's worth being exact. There are two ways to run Hoard, and they are genuinely different:

- **Hoard Cloud** is the managed option: you sign in, and your saves are stored on our servers, in the EU.
- **Self-hosting is entirely yours.** You run \`hoard-server\` on your own PC or NAS, and your saves go from your machine to your disk. There is **no account with us, no telemetry to us, no quota and no relay** — nothing passes through our servers, because there is nothing of ours in the path. We can't see a save, a game name or an email address, because none of it ever reaches us. If Hoard Cloud shut down tomorrow, a self-hosted setup would carry on unchanged.

So "server" doesn't mean "someone else's computer" unless you choose that. A self-hosted Hoard keeps your saves on hardware you own, exactly like a direct transfer does, and still gives you the history and the offline-machine case.

## Detection and coverage

Both tools find saves for a large catalogue automatically. Hoard reads the same community save-location manifest that the open-source ecosystem shares, covering 20,000+ titles, and adds Steam library scanning, running processes and a filesystem sweep on top. For Steam games it tracks \`<AppID>/remote/\` inside \`userdata\` rather than the folder above, because the parent holds \`remotecache.vdf\` and per-machine achievement and playtime files — sync those and every launch looks like a conflict. Anything unusual you can point it at by hand.

## Which one should you use?

- **Peer-to-peer** if your machines are on at the same time, you don't want storage in the picture at all, and the latest save is all you've ever needed.
- **Hoard** if you want a version history you can roll back, a machine that can be off for a week, and a copy that outlives both PCs — with the choice of our cloud or your own server.

There's a wider [comparison of every save sync tool](/guides/game-save-sync-comparison) if you want the whole field, and a [Ludusavi comparison](/guides/ludusavi-alternative) for the local-backup end of it.

<!-- faq -->

## Frequently asked questions

### Does Hoard need an account?

For Hoard Cloud, yes — that's what the sync is tied to. Self-hosted, there's no account with us at all; your server has its own users and a token per device, and they never leave your machine.

### Can Hoard work without any cloud?

Yes. Run \`hoard-server\` on a PC or a NAS and your saves go from your machine to your disk, with nothing passing through our servers.

### Do both PCs need to be online at the same time?

No, and that's the practical advantage of syncing through a server. Your save is uploaded when you stop playing and downloaded whenever the other machine next asks for it.

### Does a direct transfer keep a version history?

Not inherently — copying a file to another machine gives you the current state on both. Hoard captures every session as a version, which is what makes rolling back a corrupted save possible.

### Is Hoard open source too?

Yes, AGPL-3.0, server included. The self-hosted server is the same binary we run, not a cut-down edition.
`,tn=`---
title: "Alternativa a OpenSave: entre equipos o con un servidor tuyo"
description: "OpenSave sincroniza partidas directamente entre tus PC, sin nada en medio. Hoard sincroniza a través de un servidor —el nuestro o uno tuyo— y guarda historial de versiones. Una mirada honesta a cuándo gana cada diseño."
order: 8
updated: 2026-09-01
---

Las dos herramientas resuelven el mismo problema y discrepan en la arquitectura, que es lo único que merece compararse. Esta página pone los dos diseños uno al lado del otro, incluidos los casos en los que el otro es mejor respuesta.

## La diferencia de verdad: entre equipos o con servidor

**OpenSave** es punto a punto. Tus máquinas hablan entre ellas directamente y no hay nada en medio. No hay cuenta ni almacenamiento que pagar, y opcionalmente puede espejar una copia en una nube que ya tengas.

**Hoard** sincroniza a través de un servidor. Ese servidor es Hoard Cloud, gestionado por nosotros, o \`hoard-server\` corriendo en tu propio PC o NAS. Tu partida sube cuando dejas de jugar y baja cuando otra máquina la pide.

Todo lo demás sale de esa única decisión.

## Qué te da tener un servidor

- **La otra máquina no tiene que estar encendida.** Terminas en el sobremesa, el portátil sigue cerrado una semana, y la última partida está esperando cuando lo abres. Lo punto a punto necesita los dos extremos despiertos a la vez, que es perfecto en un escritorio e incómodo con una consola de mano que coges dos veces al mes.
- **Un historial de versiones, no sólo el último estado.** Cada sesión es una versión a la que puedes volver. Es la parte que importa el día que un mod se come tu mundo o una partida se escribe a medias: una sincronización directa copia fielmente el fichero roto al otro PC.
- **Una copia que sobrevive al hardware.** Que tus dos PC mueran en el mismo piso no es un escenario exótico. Una partida que sólo existió en esas dos máquinas se muere con ellas.
- **Nada que preparar en la red.** Ningún NAT que atravesar, ningún puerto que abrir, ninguna condición de estar los dos en la misma LAN.

## Qué te da lo punto a punto

Siendo justos con el otro lado:

- **Ningún almacenamiento que pagar, nunca.** No hay cupo que agotar porque no hay depósito. El plan gratuito de Hoard son 2 GB, y por encima pagas o te autoalojas.
- **Nada en medio por diseño.** Si el objetivo es que un fichero no toque nunca el disco de un tercero, la transferencia directa es la respuesta más corta posible.
- **Nada que mantener.** Ningún servidor en pie, ni siquiera el tuyo.

Si juegas en dos sobremesas que están los dos encendidos, nunca quieres volver atrás y prefieres no pensar en almacenamiento, ese diseño encaja limpio y Hoard es más maquinaria de la que necesitas.

## La cuestión de la privacidad, con precisión

Aquí es donde las comparativas de Hoard suelen equivocarse, así que conviene ser exacto. Hay dos formas de usar Hoard, y son genuinamente distintas:

- **Hoard Cloud** es la opción gestionada: inicias sesión y tus partidas se guardan en nuestros servidores, en la UE.
- **Autoalojarse es tuyo por completo.** Levantas \`hoard-server\` en tu PC o en tu NAS y tus partidas van de tu máquina a tu disco. **No hay cuenta con nosotros, ni telemetría hacia nosotros, ni cupo, ni relé**: no pasa nada por nuestros servidores, porque no hay nada nuestro en el camino. No podemos ver una partida, ni el nombre de un juego, ni un correo, porque nada de eso nos llega. Si Hoard Cloud cerrara mañana, un montaje autoalojado seguiría igual.

O sea que «servidor» no significa «el ordenador de otro» salvo que tú lo elijas. Un Hoard autoalojado mantiene tus partidas en hardware tuyo, exactamente igual que una transferencia directa, y encima te da el historial y el caso de la máquina apagada.

## Detección y cobertura

Las dos herramientas encuentran partidas de un catálogo grande de forma automática. Hoard lee el mismo manifiesto comunitario de ubicaciones que comparte el ecosistema open source, con más de 20.000 títulos, y le suma el barrido de bibliotecas de Steam, los procesos en ejecución y un escaneo del disco. En los juegos de Steam rastrea \`<AppID>/remote/\` dentro de \`userdata\` y no la carpeta de encima, porque la padre guarda \`remotecache.vdf\` y ficheros de logros y tiempo jugado propios de cada máquina: si sincronizas eso, cada arranque parece un conflicto. Lo raro se lo señalas a mano.

## ¿Cuál deberías usar?

- **Punto a punto** si tus máquinas están encendidas a la vez, no quieres almacenamiento en la ecuación y la última partida es todo lo que has necesitado nunca.
- **Hoard** si quieres un historial al que volver, una máquina que pueda estar apagada una semana y una copia que sobreviva a los dos PC, con la opción de usar nuestra nube o tu propio servidor.

Hay una [comparativa de todas las herramientas de sincronización](/guides/game-save-sync-comparison) si quieres el panorama completo, y una [comparativa con Ludusavi](/guides/ludusavi-alternative) para la parte de copias locales.

<!-- faq -->

## Preguntas frecuentes

### ¿Hoard necesita cuenta?

Para Hoard Cloud sí, porque es a lo que está atada la sincronización. Autoalojado no hay ninguna cuenta con nosotros: tu servidor tiene sus propios usuarios y un token por dispositivo, y no salen de tu máquina.

### ¿Puede funcionar Hoard sin ninguna nube?

Sí. Levanta \`hoard-server\` en un PC o en un NAS y tus partidas van de tu máquina a tu disco, sin que nada pase por nuestros servidores.

### ¿Tienen que estar los dos PC encendidos a la vez?

No, y ésa es la ventaja práctica de sincronizar a través de un servidor. Tu partida sube cuando dejas de jugar y baja cuando la otra máquina la pida.

### ¿Una transferencia directa guarda historial de versiones?

No de por sí: copiar un fichero a otra máquina te deja el estado actual en las dos. Hoard captura cada sesión como una versión, y eso es lo que hace posible volver atrás desde una partida corrupta.

### ¿Hoard también es open source?

Sí, AGPL-3.0, servidor incluido. El servidor autoalojado es el mismo binario que usamos nosotros, no una edición recortada.
`,un=`---
title: "Alternative à OpenSave : direct entre machines ou serveur qui vous appartient"
description: "OpenSave synchronise les parties directement entre vos PC, sans rien au milieu. Hoard passe par un serveur — le nôtre ou le vôtre — et garde un historique versionné. Un regard honnête sur les cas où chaque approche l'emporte."
order: 8
updated: 2026-09-01
---

Les deux outils résolvent le même problème et divergent sur l'architecture, et c'est bien la seule chose qui mérite comparaison. Cette page met les deux approches côte à côte, y compris les cas où l'autre est la meilleure réponse.

## La vraie différence : direct ou via un serveur

**OpenSave** est pair-à-pair. Vos machines se parlent directement, sans rien entre elles. Pas de compte, pas de stockage à payer, et la possibilité de refléter une copie vers un cloud que vous avez déjà.

**Hoard** synchronise via un serveur. Ce serveur est soit Hoard Cloud, géré par nous, soit \`hoard-server\` sur votre propre PC ou NAS. Votre sauvegarde monte quand vous arrêtez de jouer et redescend quand une autre machine la demande.

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
- **L'auto-hébergement est entièrement le vôtre.** Vous faites tourner \`hoard-server\` sur votre PC ou votre NAS, et vos sauvegardes vont de votre machine à votre disque. **Aucun compte chez nous, aucune télémétrie vers nous, aucun quota et aucun relais** : rien ne passe par nos serveurs, puisque rien de chez nous n'est sur le chemin. Nous ne voyons ni sauvegarde, ni nom de jeu, ni adresse e-mail, car rien de tout cela ne nous parvient. Si Hoard Cloud fermait demain, une installation auto-hébergée continuerait à l'identique.

Donc « serveur » ne veut pas dire « l'ordinateur de quelqu'un d'autre », sauf si vous le choisissez. Un Hoard auto-hébergé garde vos sauvegardes sur du matériel qui vous appartient, exactement comme un transfert direct, et vous donne en plus l'historique et le cas de la machine éteinte.

## Détection et couverture

Les deux outils trouvent automatiquement les sauvegardes d'un large catalogue. Hoard lit le même manifeste communautaire d'emplacements que partage l'écosystème open source, couvrant plus de 20 000 titres, et y ajoute l'analyse des bibliothèques Steam, les processus en cours et un balayage du disque. Pour les jeux Steam, il suit \`<AppID>/remote/\` dans \`userdata\` et non le dossier au-dessus, car le parent contient \`remotecache.vdf\` et des fichiers de succès et de temps de jeu propres à chaque machine : les synchroniser, et chaque lancement ressemble à un conflit. Pour les cas particuliers, vous lui désignez le dossier.

## Lequel choisir ?

- **Le pair-à-pair** si vos machines sont allumées en même temps, que le stockage ne doit pas entrer en jeu et que la dernière sauvegarde vous a toujours suffi.
- **Hoard** si vous voulez un historique où revenir, une machine qui peut rester éteinte une semaine et une copie qui survive aux deux PC — au choix via notre cloud ou votre propre serveur.

Il existe une [comparaison de tous les outils de synchro](/guides/game-save-sync-comparison) pour le paysage complet, et une [comparaison avec Ludusavi](/guides/ludusavi-alternative) pour le versant sauvegarde locale.

<!-- faq -->

## Questions fréquentes

### Hoard exige-t-il un compte ?

Pour Hoard Cloud, oui : la synchro y est rattachée. En auto-hébergé, aucun compte chez nous ; votre serveur a ses propres utilisateurs et un jeton par appareil, et ils ne quittent jamais votre machine.

### Hoard peut-il fonctionner sans aucun cloud ?

Oui. Faites tourner \`hoard-server\` sur un PC ou un NAS et vos sauvegardes vont de votre machine à votre disque, sans que rien passe par nos serveurs.

### Les deux PC doivent-ils être en ligne en même temps ?

Non, et c'est l'avantage pratique de passer par un serveur. Votre sauvegarde est envoyée quand vous arrêtez de jouer et téléchargée dès que l'autre machine la réclame.

### Un transfert direct garde-t-il un historique de versions ?

Pas en soi : copier un fichier vers une autre machine vous donne l'état actuel des deux côtés. Hoard capture chaque session comme une version, ce qui rend possible le retour en arrière après une sauvegarde corrompue.

### Hoard est-il open source lui aussi ?

Oui, AGPL-3.0, serveur compris. Le serveur auto-hébergé est le même binaire que celui que nous faisons tourner, pas une édition allégée.
`,dn=`---
title: "Alternativa a OpenSave: diretto tra macchine o con un server tuo"
description: "OpenSave sincronizza i salvataggi direttamente tra i tuoi PC, senza nulla in mezzo. Hoard sincronizza attraverso un server — il nostro o uno tuo — e tiene una cronologia versionata. Uno sguardo onesto su quando vince ciascun approccio."
order: 8
updated: 2026-09-01
---

I due strumenti risolvono lo stesso problema e non sono d'accordo sull'architettura, che è l'unica cosa che valga la pena confrontare. Questa pagina mette i due approcci uno accanto all'altro, compresi i casi in cui l'altro è la risposta migliore.

## La differenza vera: diretto o con un server

**OpenSave** è peer-to-peer. Le tue macchine si parlano direttamente e in mezzo non c'è nulla. Nessun account e nessuno spazio da pagare, e in opzione può replicare una copia su un cloud che hai già.

**Hoard** sincronizza attraverso un server. Quel server è Hoard Cloud, gestito da noi, oppure \`hoard-server\` sul tuo PC o sul tuo NAS. Il salvataggio sale quando smetti di giocare e scende quando un'altra macchina lo chiede.

Tutto il resto discende da questa singola scelta.

## Cosa ti dà un server

- **L'altra macchina non deve essere accesa.** Finisci sul fisso, il portatile resta chiuso una settimana, e all'apertura l'ultimo salvataggio è lì ad aspettare. Il peer-to-peer vuole entrambi i capi svegli nello stesso momento: ottimo alla scrivania, scomodo con una portatile che prendi in mano due volte al mese.
- **Una cronologia, non solo l'ultimo stato.** Ogni sessione diventa una versione a cui tornare. È la parte che conta il giorno in cui una mod ti mangia il mondo o un salvataggio finisce scritto a metà: una sincronizzazione diretta copia fedelmente il file rotto sull'altro PC.
- **Una copia che sopravvive all'hardware.** Che entrambi i PC muoiano nella stessa casa non è uno scenario esotico. Un salvataggio esistito solo su quelle due macchine muore con loro.
- **Niente da sistemare sulla rete.** Nessun NAT da attraversare, nessuna porta da aprire, nessun vincolo di stare sulla stessa LAN.

## Cosa ti dà il peer-to-peer

Per essere onesti con l'altra parte:

- **Nessuno spazio da pagare, mai.** Non c'è quota da esaurire perché non c'è un archivio. Il piano gratuito di Hoard è 2 GB, sopra si paga o si fa self-hosting.
- **Niente in mezzo per progetto.** Se l'obiettivo è che un file non tocchi mai il disco di terzi, il trasferimento diretto è la risposta più breve possibile.
- **Niente da mandare avanti.** Nessun server da tenere in piedi, nemmeno il tuo.

Se giochi su due fissi entrambi accesi, non vuoi mai tornare indietro e preferisci non pensare allo spazio, quell'approccio calza perfettamente e Hoard è più macchinario di quanto ti serva.

## La questione privacy, detta con precisione

È qui che i confronti su Hoard di solito sbagliano, quindi siamo esatti: ci sono due modi di usarlo e sono davvero diversi.

- **Hoard Cloud** è l'opzione gestita: accedi e i salvataggi stanno sui nostri server, nell'UE.
- **Il self-hosting è interamente tuo.** Fai girare \`hoard-server\` sul tuo PC o NAS e i salvataggi vanno dalla tua macchina al tuo disco. **Nessun account con noi, nessuna telemetria verso di noi, nessuna quota e nessun relay**: non passa nulla dai nostri server, perché sul percorso non c'è niente di nostro. Non vediamo un salvataggio, il nome di un gioco o un indirizzo email, perché niente di tutto ciò ci arriva. Se Hoard Cloud chiudesse domani, un'installazione self-hosted continuerebbe uguale.

Quindi "server" non vuol dire "il computer di qualcun altro", a meno che tu non lo scelga. Un Hoard self-hosted tiene i salvataggi su hardware tuo, esattamente come un trasferimento diretto, e in più ti dà la cronologia e il caso della macchina spenta.

## Rilevamento e copertura

Entrambi trovano automaticamente i salvataggi di un catalogo ampio. Hoard legge lo stesso manifest comunitario delle posizioni condiviso dall'ecosistema open source, oltre 20.000 titoli, e ci aggiunge le librerie Steam, i processi in esecuzione e una scansione del disco. Per i giochi Steam traccia \`<AppID>/remote/\` dentro \`userdata\` e non la cartella superiore, perché quella contiene \`remotecache.vdf\` e file di obiettivi e tempo di gioco propri di ogni macchina: sincronizzarli significa vedere un conflitto a ogni avvio. Per i casi insoliti gli indichi tu la cartella.

## Quale usare?

- **Peer-to-peer** se le tue macchine sono accese insieme, non vuoi che lo spazio entri nel discorso e l'ultimo salvataggio è tutto ciò che ti è mai servito.
- **Hoard** se vuoi una cronologia a cui tornare, una macchina che possa restare spenta una settimana e una copia che sopravviva a entrambi i PC, con la scelta tra il nostro cloud e il tuo server.

C'è un [confronto di tutti gli strumenti di sincronizzazione](/guides/game-save-sync-comparison) per il quadro completo, e un [confronto con Ludusavi](/guides/ludusavi-alternative) per il versante dei backup locali.

<!-- faq -->

## Domande frequenti

### Hoard richiede un account?

Per Hoard Cloud sì, perché la sincronizzazione è legata a quello. In self-hosting non c'è alcun account con noi: il tuo server ha i suoi utenti e un token per dispositivo, e non escono dalla tua macchina.

### Hoard può funzionare senza alcun cloud?

Sì. Fai girare \`hoard-server\` su un PC o un NAS e i salvataggi vanno dalla tua macchina al tuo disco, senza che nulla passi dai nostri server.

### Servono entrambi i PC online nello stesso momento?

No, ed è il vantaggio pratico di passare da un server. Il salvataggio viene caricato quando smetti di giocare e scaricato quando l'altra macchina lo richiede.

### Un trasferimento diretto tiene una cronologia?

Non di per sé: copiare un file su un'altra macchina ti dà lo stato attuale su entrambe. Hoard cattura ogni sessione come una versione, ed è questo a rendere possibile tornare indietro da un salvataggio corrotto.

### Anche Hoard è open source?

Sì, AGPL-3.0, server incluso. Il server self-hosted è lo stesso binario che usiamo noi, non un'edizione ridotta.
`,ln=`---
title: "OpenSave の代替：端末間の直接転送か、自分のサーバーか"
description: "OpenSave は PC 同士でセーブを直接同期し、あいだに何も置きません。Hoard はサーバー経由で同期し、世代履歴を残します。サーバーは当方のものでも、自分で立てたものでも構いません。どちらの設計がどんなときに有利かを率直に比べます。"
order: 8
updated: 2026-09-01
---

どちらのツールも同じ問題を解こうとしていて、設計思想だけが食い違っています。比べる価値があるのはそこだけです。このページでは 2 つの設計を並べ、相手のほうが良い答えになる場面も含めて説明します。

## 本当の違い：直接転送かサーバー経由か

**OpenSave** はピアツーピアです。あなたのマシン同士が直接やり取りし、あいだには何も入りません。アカウントも、支払うストレージもなく、必要なら既に持っているクラウドドライブへ複製することもできます。

**Hoard** はサーバー経由で同期します。そのサーバーは、当方が運用する Hoard Cloud か、自分の PC や NAS で動かす \`hoard-server\` のどちらかです。プレイを終えるとセーブが上がり、別のマシンが求めたときに下ります。

あとはすべて、この 1 つの選択から派生します。

## サーバーが与えてくれるもの

- **もう 1 台が起動している必要がありません。** デスクトップで遊び終え、ノート PC は 1 週間閉じたままでも、開いたときには最新のセーブが待っています。ピアツーピアは両端が同時に起きている必要があり、机の上なら問題なくても、月に 2 回しか触らない携帯機では厄介です。
- **最新の状態だけでなく、世代履歴が残ります。** 1 セッションが 1 つの世代になり、そこへ戻れます。Mod がワールドを食べた日や、セーブが途中まで書かれた日に効いてくる部分です。直接同期は、壊れたファイルを忠実にもう 1 台へコピーします。
- **ハードウェアより長生きするコピー。** 2 台の PC が同じ部屋で同時に壊れるのは、珍しい話ではありません。その 2 台にしか存在しなかったセーブは、一緒に消えます。
- **ネットワーク側の準備が不要。** NAT 越えも、開けるポートも、両方が同じ LAN にいる必要もありません。

## ピアツーピアが与えてくれるもの

相手側にも公平に。

- **ストレージ料金が一切ない。** 保管場所そのものがないので、使い切る上限もありません。Hoard の無料枠は 2 GB で、それを超えると支払うかセルフホストするかになります。
- **設計上、あいだに何も入らない。** ファイルが第三者のディスクに一度も触れないことが目的なら、直接転送はいちばん短い答えです。
- **動かし続けるものがない。** サーバーは不要で、自分のものすら要りません。

常時起動のデスクトップ 2 台で遊び、巻き戻す必要を感じたことがなく、ストレージのことを考えたくないのなら、その設計はきれいに噛み合い、Hoard は必要以上の仕掛けになります。

## プライバシーの話を、正確に

Hoard についての比較がいちばん誤りやすいのがここなので、正確に書きます。Hoard には 2 つの動かし方があり、両者は本当に別物です。

- **Hoard Cloud** はマネージドな選択肢です。サインインすると、セーブは EU にある当方のサーバーに保存されます。
- **セルフホストは完全にあなたのものです。** 自分の PC や NAS で \`hoard-server\` を動かせば、セーブは自分のマシンから自分のディスクへ移ります。**当方のアカウントも、当方へのテレメトリも、容量制限も、中継もありません。** 経路上に当方のものが何一つないため、当方のサーバーを何も通りません。セーブもゲーム名もメールアドレスも見えません。届かないからです。仮に明日 Hoard Cloud が終了しても、セルフホスト構成はそのまま動き続けます。

つまり「サーバー」は、あなたがそう選ばない限り「他人のコンピューター」を意味しません。セルフホストした Hoard は、直接転送とまったく同じように、セーブを自分のハードウェアの中にとどめたうえで、履歴と「電源が入っていないマシン」への対応を追加してくれます。

## 検出と対応範囲

どちらのツールも、広いカタログのセーブを自動で見つけます。Hoard はオープンソースの世界で共有されているのと同じ、2 万本以上を収録したコミュニティのセーブ位置マニフェストを読み、そこに Steam ライブラリの走査、実行中プロセス、ファイルシステムの掃引を重ねます。Steam のゲームでは、上のフォルダーではなく \`userdata\` 内の \`<AppID>/remote/\` を追跡します。上のフォルダーには \`remotecache.vdf\` や、実績・プレイ時間といったマシンごとのファイルがあり、それを同期すると起動のたびに競合に見えるからです。変わったものは手動で指定できます。

## どちらを使うべきか

- 2 台が同時に起動していて、ストレージを話に入れたくなく、最新のセーブだけで足りてきたのなら **ピアツーピア**。
- 巻き戻せる履歴が欲しい、1 週間電源を切っていてよいマシンがある、2 台の PC より長生きするコピーが欲しい——そのいずれかなら **Hoard**。当方のクラウドでも、自分のサーバーでも選べます。

全体像が知りたい場合は [セーブ同期ツールの比較](/guides/game-save-sync-comparison) を、ローカルバックアップ側については [Ludusavi との比較](/guides/ludusavi-alternative) をご覧ください。

<!-- faq -->

## よくある質問

### Hoard にはアカウントが必要ですか？

Hoard Cloud では必要です。同期がそこに結びついているためです。セルフホストなら当方のアカウントは一切ありません。あなたのサーバーが自分のユーザーと端末ごとのトークンを持ち、それらがマシンの外に出ることはありません。

### クラウドをまったく使わずに運用できますか？

はい。PC か NAS で \`hoard-server\` を動かせば、セーブは自分のマシンから自分のディスクへ移り、当方のサーバーを何も通りません。

### 2 台の PC を同時にオンラインにする必要はありますか？

ありません。それがサーバー経由で同期する実用上の利点です。プレイ終了時にアップロードされ、もう 1 台が次に求めたときにダウンロードされます。

### 直接転送でも世代履歴は残りますか？

そのままでは残りません。ファイルをもう 1 台へコピーすれば、両方が現在の状態になるだけです。Hoard は 1 セッションを 1 世代として取り込み、それが壊れたセーブから戻れる理由になります。

### Hoard もオープンソースですか？

はい。サーバーを含め AGPL-3.0 です。セルフホスト用のサーバーは当方が運用しているものと同じバイナリで、機能を削った版ではありません。
`,cn=`---
title: "Alternativa ao OpenSave: direto entre máquinas ou com um servidor teu"
description: "O OpenSave sincroniza saves diretamente entre os teus PCs, sem nada pelo meio. O Hoard sincroniza através de um servidor — o nosso ou um teu — e guarda histórico versionado. Um olhar honesto sobre quando cada desenho ganha."
order: 8
updated: 2026-09-01
---

As duas ferramentas resolvem o mesmo problema e discordam quanto à arquitetura, que é a única coisa que vale a pena comparar. Esta página põe os dois desenhos lado a lado, incluindo os casos em que o outro é a melhor resposta.

## A diferença a sério: direto ou com servidor

**O OpenSave** é ponto a ponto. As tuas máquinas falam diretamente umas com as outras e no meio não há nada. Sem conta e sem armazenamento a pagar, e opcionalmente pode espelhar uma cópia para uma nuvem que já tenhas.

**O Hoard** sincroniza através de um servidor. Esse servidor é o Hoard Cloud, gerido por nós, ou o \`hoard-server\` a correr no teu PC ou no teu NAS. O teu save sobe quando paras de jogar e desce quando outra máquina o pede.

Tudo o resto sai dessa única escolha.

## O que um servidor te dá

- **A outra máquina não tem de estar ligada.** Acabas no fixo, o portátil fica fechado uma semana, e o save mais recente está à espera quando o abres. O ponto a ponto precisa das duas pontas acordadas ao mesmo tempo: perfeito numa secretária, chato com uma consola portátil que pegas duas vezes por mês.
- **Um histórico de versões, não só o último estado.** Cada sessão passa a ser uma versão à qual podes voltar. É a parte que conta no dia em que uma mod te come o mundo ou um save fica escrito a meio: uma sincronização direta copia fielmente o ficheiro partido para o outro PC.
- **Uma cópia que sobrevive ao hardware.** Os dois PCs morrerem na mesma casa não é um cenário exótico. Um save que só existiu nessas duas máquinas morre com elas.
- **Nada para preparar na rede.** Sem NAT para atravessar, sem porta para abrir, sem a condição de estarem os dois na mesma LAN.

## O que o ponto a ponto te dá

Sendo justos com o outro lado:

- **Nunca há armazenamento a pagar.** Não há quota para esgotar porque não há depósito. O plano gratuito do Hoard são 2 GB; acima disso pagas ou alojas tu.
- **Nada pelo meio, por desenho.** Se o objetivo é que um ficheiro nunca toque no disco de terceiros, a transferência direta é a resposta mais curta possível.
- **Nada para manter.** Nenhum servidor de pé, nem sequer o teu.

Se jogas em dois fixos ambos ligados, nunca queres voltar atrás e preferes não pensar em armazenamento, esse desenho encaixa bem e o Hoard é mais maquinaria do que precisas.

## A questão da privacidade, com precisão

É aqui que as comparações ao Hoard costumam falhar, por isso sejamos exatos: há duas formas de o usar e são genuinamente diferentes.

- **O Hoard Cloud** é a opção gerida: inicias sessão e os saves ficam nos nossos servidores, na UE.
- **O self-hosting é inteiramente teu.** Corres o \`hoard-server\` no teu PC ou NAS e os saves vão da tua máquina para o teu disco. **Não há conta connosco, nem telemetria para nós, nem quota, nem retransmissão**: não passa nada pelos nossos servidores, porque não há nada nosso no caminho. Não vemos um save, o nome de um jogo ou um email, porque nada disso nos chega. Se o Hoard Cloud fechasse amanhã, uma instalação self-hosted continuaria igual.

Ou seja, "servidor" não significa "o computador de outra pessoa" a não ser que o escolhas. Um Hoard self-hosted mantém os saves em hardware teu, tal como uma transferência direta, e ainda te dá o histórico e o caso da máquina desligada.

## Deteção e cobertura

Ambas as ferramentas encontram automaticamente os saves de um catálogo grande. O Hoard lê o mesmo manifesto comunitário de localizações que o ecossistema open source partilha, com mais de 20.000 títulos, e junta-lhe as bibliotecas da Steam, os processos em execução e uma varredura do disco. Nos jogos da Steam segue \`<AppID>/remote/\` dentro de \`userdata\` e não a pasta acima, porque a de cima guarda \`remotecache.vdf\` e ficheiros de proezas e tempo de jogo próprios de cada máquina: sincronizá-los é ver um conflito a cada arranque. Para o que for invulgar, apontas-lhe a pasta.

## Qual deves usar?

- **Ponto a ponto** se as tuas máquinas estão ligadas ao mesmo tempo, não queres armazenamento na equação e o último save é tudo o que alguma vez precisaste.
- **O Hoard** se queres um histórico ao qual voltar, uma máquina que possa estar desligada uma semana e uma cópia que sobreviva aos dois PCs — com a escolha entre a nossa nuvem e o teu próprio servidor.

Há uma [comparação de todas as ferramentas de sincronização](/guides/game-save-sync-comparison) para o panorama completo, e uma [comparação com o Ludusavi](/guides/ludusavi-alternative) para o lado das cópias locais.

<!-- faq -->

## Perguntas frequentes

### O Hoard precisa de conta?

Para o Hoard Cloud sim, é a isso que a sincronização está ligada. Em self-hosted não há conta nenhuma connosco: o teu servidor tem os seus utilizadores e um token por dispositivo, e não saem da tua máquina.

### O Hoard funciona sem nuvem nenhuma?

Sim. Corre o \`hoard-server\` num PC ou num NAS e os teus saves vão da tua máquina para o teu disco, sem nada a passar pelos nossos servidores.

### Os dois PCs têm de estar online ao mesmo tempo?

Não, e essa é a vantagem prática de sincronizar através de um servidor. O save sobe quando paras de jogar e desce quando a outra máquina o pedir.

### Uma transferência direta guarda histórico de versões?

Por si só não: copiar um ficheiro para outra máquina dá-te o estado atual nas duas. O Hoard captura cada sessão como uma versão, e é isso que torna possível voltar atrás a partir de um save corrompido.

### O Hoard também é open source?

Sim, AGPL-3.0, servidor incluído. O servidor self-hosted é o mesmo binário que nós corremos, não uma edição reduzida.
`,mn=`---
title: "OpenSave 的替代方案：机器之间直连，还是一台属于你的服务器"
description: "OpenSave 在你的 PC 之间直接同步存档，中间不放任何东西。Hoard 通过服务器同步——我们的，或你自己的——并保留版本历史。这里坦白比较两种设计各自的胜场。"
order: 8
updated: 2026-09-01
---

两个工具解决的是同一个问题，分歧只在架构上，而这也是唯一值得比较的地方。本页把两种设计并排摆开，包括另一种才是更好答案的情形。

## 真正的差别：直连还是走服务器

**OpenSave** 是点对点的。你的机器彼此直接通信，中间不隔任何东西。没有账号，也没有要付费的存储，还可以选择把副本镜像到你已有的云盘。

**Hoard** 通过服务器同步。这台服务器要么是我们运营的 Hoard Cloud，要么是跑在你自己 PC 或 NAS 上的 \`hoard-server\`。你停止游玩时存档上传，另一台机器需要时再下载。

其余的一切，都从这一个选择衍生而来。

## 服务器带来什么

- **另一台机器不必开着。** 你在台式机上玩完，笔记本关着放一周，打开时最新存档就在等你。点对点要求两端同时醒着——在书桌前没问题，但对一个月只拿两次的掌机就很别扭。
- **版本历史，而不只是最新状态。** 每次游玩都成为一个可回退的版本。模组吞掉你的世界、或存档写到一半的那天，这一点才见真章：直连同步会忠实地把损坏的文件复制到另一台 PC。
- **一份比硬件活得久的副本。** 两台 PC 在同一间屋子里一起完蛋并不算离奇。只在这两台机器上存在过的存档，会跟着它们一起消失。
- **网络上无需张罗。** 不用穿透 NAT，不用开端口，也没有"两台必须在同一局域网"的前提。

## 点对点带来什么

对另一边也要公平：

- **永远不必为存储付费。** 没有配额可撞，因为根本没有存储桶。Hoard 的免费额度是 2 GB，超出就要付费或自托管。
- **从设计上中间就没有东西。** 如果目标是让文件永远不碰第三方的磁盘，直接传输就是最短的答案。
- **没有需要维护的东西。** 不必让任何服务器常驻，连你自己的也不用。

如果你在两台都开着的台式机上玩，从不想回退，也不愿把存储纳入考虑，那种设计干净利落，Hoard 对你而言是多余的机械。

## 隐私问题，说得精确一点

这正是关于 Hoard 的比较最常出错的地方，所以说准确些：Hoard 有两种运行方式，而且确实不同。

- **Hoard Cloud** 是托管方案：你登录，存档保存在我们位于欧盟的服务器上。
- **自托管完全属于你。** 你在自己的 PC 或 NAS 上运行 \`hoard-server\`，存档从你的机器走到你的磁盘。**没有我们这边的账号，没有发往我们的遥测，没有配额，也没有中转**——不经过我们的任何服务器，因为这条路径上根本没有我们的东西。我们看不到任何存档、游戏名或邮箱地址，因为这些从未到达我们这里。就算 Hoard Cloud 明天关停，自托管的部署照常运行。

所以除非你自己选择，"服务器"并不意味着"别人的电脑"。自托管的 Hoard 把存档留在属于你的硬件上，和直接传输一样，同时还多给你历史，以及那台可以关机的机器。

## 检测与覆盖

两个工具都能自动找到大量游戏的存档。Hoard 读取开源生态共享的同一份社区存档位置清单，覆盖两万余款游戏，并在此之上加了 Steam 库扫描、运行中进程和文件系统扫描。对 Steam 游戏，它追踪 \`userdata\` 里的 \`<AppID>/remote/\` 而不是上一层文件夹，因为上一层放着 \`remotecache.vdf\` 以及各机器各自的成就和游戏时长文件——同步它们，每次启动都会像冲突。特殊情况可以手动指定文件夹。

## 你该用哪个？

- **点对点**：如果你的机器同时开着，不希望存储进入这个话题，而最新存档也一直够用。
- **Hoard**：如果你想要能回退的历史、一台可以关一周的机器，以及一份比两台 PC 都活得久的副本——并且可以在我们的云和你自己的服务器之间选择。

想看完整的横向比较，可以读[所有存档同步工具的比较](/guides/game-save-sync-comparison)；本地备份那一侧，参见[与 Ludusavi 的比较](/guides/ludusavi-alternative)。

<!-- faq -->

## 常见问题

### Hoard 需要账号吗？

用 Hoard Cloud 需要，同步就是绑定在账号上的。自托管则完全没有我们这边的账号：你的服务器有它自己的用户和每台设备一个的令牌，它们从不离开你的机器。

### Hoard 能完全不用云吗？

能。在 PC 或 NAS 上运行 \`hoard-server\`，你的存档就从你的机器走到你的磁盘，没有任何东西经过我们的服务器。

### 两台 PC 需要同时在线吗？

不需要，这正是走服务器的实际好处。你停止游玩时存档上传，另一台机器下次索取时再下载。

### 直接传输会保留版本历史吗？

本身不会——把文件复制到另一台机器，只是让两边都停在当前状态。Hoard 把每次游玩都抓成一个版本，这才让从损坏存档中回退成为可能。

### Hoard 也是开源的吗？

是的，AGPL-3.0，服务器也包含在内。自托管服务器就是我们自己在跑的那个二进制，不是删减版。
`,pn=`---
title: "So stellst du einen alten Spielstand wieder her"
description: "Falsche Entscheidung getroffen, Datei beschädigt oder Neustart gewünscht? Springe mit Hoards Cloud-Historie zu jeder früheren Version deines Spielstands zurück — auch zu Ständen, die mit Tools wie Ludusavi gesichert wurden."
order: 3
updated: 2026-09-01
---

Eine schlechte Entscheidung im Spiel, eine beschädigte Datei oder ein verpfuschter Mod — manchmal musst du einfach zurück. Da Hoard eine vollständige Versionshistorie jedes Stands führt, dauert die Wiederherstellung eines früheren nur Sekunden.

## Eine frühere Version wiederherstellen

1. Öffne **Hoard** und gehe zum Spiel in deiner **Bibliothek**.
2. Öffne den Reiter **Historie**. Du siehst jedes Backup mit Datum und Größe.
3. Wähle die gewünschte Version und klicke auf **Wiederherstellen**.
4. Hoard schreibt diesen Snapshot zurück in den Speicherordner des Spiels. Dein aktueller Stand wird zuerst gesichert, die Wiederherstellung ist also umkehrbar.

## Auf einem neuen oder neu installierten PC wiederherstellen

1. Installiere Hoard und melde dich mit deinem Konto an.
2. Füge das Spiel zu deiner Bibliothek hinzu — Hoard findet das passende Cloud-Backup.
3. Stelle die neueste Version oder eine ältere wieder her und spiele weiter.

Da Hoard Speicherordner mit derselben Community-Datenbank wie Ludusavi findet, weiß es selbst bei einer Neuinstallation, wohin ein wiederhergestellter Stand gehört — ohne manuelle Pfadsuche.

## Wenn ein Spielstand beschädigt ist oder ein Mod ihn zerlegt hat

Ein Spiel, das beim Laden abstürzt, ein Mod, der etwas überschrieben hat, ein Autosave mitten im Schreibvorgang: die Lösung ist dieselbe. Öffne die **Historie** des Spiels, wähle die letzte Version von vor dem Problem und stelle sie wieder her. Datum und Größe reichen meist, um den Moment zu finden — ein plötzlicher Größensturz ist ein gutes Zeichen dafür, dass ein Stand abgeschnitten wurde.

Wenn du nicht sicher bist, welche die richtige ist, stelle die wahrscheinlichste wieder her und prüfe es im Spiel. Ein zweiter Versuch kostet nichts, denn die eben ersetzte Version wurde ebenfalls behalten.

## Was beim Wiederherstellen tatsächlich passiert

Drei Dinge, die man wissen sollte, denn sie machen einen Versuch gefahrlos:

1. **Dein aktueller Stand wird zuerst gesichert.** Die Wiederherstellung ist umkehrbar: das Ersetzte wird eine Version in der Historie wie jede andere.
2. **Es wird nur geladen, was fehlt.** Dateien, die mit dem richtigen Inhalt schon auf der Platte liegen, werden so verwendet — einen großen Spielstand nach einer kleinen Änderung wiederherzustellen bewegt ein paar Megabyte statt des ganzen Ordners.
3. **Dateien dieses Rechners bleiben unangetastet.** Konfiguration und Logs neben dem Spielstand werden gesichert, aber nicht über deine lokalen Kopien geschrieben: Tastenbelegung und Grafikeinstellungen überleben eine Wiederherstellung von einem anderen PC.

## Wiederherstellen ohne unsere Server

Wenn du deinen eigenen \`hoard-server\` betreibst, funktioniert das Wiederherstellen genauso, nur kommen die Versionen von deiner Maschine statt von unserer. Es gibt kein Konto bei uns, keine Telemetrie zu uns und nichts, was über unsere Server läuft. Siehe [wie du Hoard selbst hostest](/guides/self-host-hoard).

## Tipp

Wiederherstellungen sind nie zerstörerisch: Der ersetzte Stand wird zuerst als neue Version erfasst, du kannst eine Wiederherstellung also immer rückgängig machen, indem du den vorherigen Eintrag wiederherstellst. Hast du bisher nur lokale Backups geführt (etwa mit Ludusavi), ergänzt der Wechsel zu Hoard eine geräteunabhängige, versionierte Historie, aus der du selbst nach einem Festplattenausfall wiederherstellen kannst.

<!-- faq -->

## Häufige Fragen

### Überschreibt eine Wiederherstellung meinen aktuellen Fortschritt?

Erst nachdem dein aktueller Stand als neue Version gesichert wurde. Hast du die falsche gewählt, stelle den vorherigen Eintrag wieder her und du bist zurück am Ausgangspunkt.

### Wie weit reicht die Historie zurück?

So weit, wie das Versionslimit deines Tarifs erlaubt, und eine angeheftete Version wird nie weggeräumt, um Platz zu schaffen. Auf einem selbst gehosteten Server ist die einzige Grenze deine Platte.

### Kann ich auf einen PC wiederherstellen, auf dem das Spiel noch nicht installiert ist?

Installiere zuerst das Spiel, damit sein Speicherordner existiert, und stelle dann wieder her. Hoard weiß, wo jedes Spiel seine Stände erwartet, und schreibt den Snapshot an die richtige Stelle, ohne dass du den Pfad suchen musst.

### Klappt das zwischen Windows und einem Steam Deck?

Ja. Dasselbe Spiel legt seinen Stand auf beiden Geräten woanders ab — auf dem Deck im Proton-Prefix — und Hoard schreibt die wiederhergestellte Version dorthin, wo diese Maschine sie erwartet.

### Ist die Wiederherstellung auf einem selbst gehosteten Server anders?

Nein. Gleiche App, gleiche Historie, gleiche Wiederherstellung per Klick. Nur der Speicher gehört dir.
`,hn=`---
title: "How to restore an old game save"
description: "Made a wrong move, corrupted a file or want a fresh start? Roll back to any previous version of your game save with Hoard's cloud history — including saves backed up by tools like Ludusavi."
order: 3
updated: 2026-09-01
---

A bad decision in-game, a corrupted file, or a botched mod — sometimes you just need to go back. Because Hoard keeps a full version history of every save, restoring an earlier one takes seconds.

## Restore a previous version

1. Open **Hoard** and go to the game in your **Library**.
2. Open its **History** tab. You'll see every backup with its date and size.
3. Pick the version you want and choose **Restore**.
4. Hoard writes that snapshot back into the game's save folder. Your current save is backed up first, so the restore itself is reversible.

## Restore on a new or reinstalled PC

1. Install Hoard and sign in with your account.
2. Add the game to your Library — Hoard finds the matching cloud backup.
3. Restore the latest version, or any older one, and keep playing.

Because Hoard locates save folders using the same community database as Ludusavi, it knows where to put a restored save even on a fresh install — no manual path hunting.

## When a save is corrupted or a mod broke it

A game that crashes on load, a mod that rewrote something it shouldn't, an autosave that landed halfway through a write: the fix is the same. Open the game's **History**, pick the last version from before the problem started, and restore it. Dates and sizes are usually enough to spot the moment things went wrong — a sudden drop in size is a good sign that a save got truncated.

If you're not sure which version is the good one, restore the most likely candidate and check in-game. Trying again costs nothing, because the version you just replaced was kept too.

## What a restore actually does

Three things worth knowing, because they are what make a restore safe to try:

1. **Your current save is captured first.** The restore is reversible: whatever you replaced becomes a version in the history like any other.
2. **Only what's missing is downloaded.** Files already on disk with the right content are used as they are, so restoring a large save after a small change moves a few megabytes instead of the whole folder.
3. **Files that belong to this machine are left alone.** Configuration and logs sitting next to the save are backed up, but not written over your local copies — your key bindings and graphics settings survive a restore that came from another PC.

## Restoring without our servers

If you run your own \`hoard-server\`, restores work exactly the same way, except the versions come from your machine instead of ours. There is no account with us, no telemetry to us and nothing passing through our servers. See [how to self-host Hoard](/guides/self-host-hoard).

## Tip

Restores are never destructive: the save you replace is captured as a new version first, so you can always undo a restore by restoring the previous entry. If you've only ever kept local backups (for example with Ludusavi), moving to Hoard adds an off-machine, versioned history you can restore from even after a disk failure.

<!-- faq -->

## Frequently asked questions

### Will restoring overwrite my current progress?

Only after your current save has been captured as a new version. If you restore the wrong one, restore the previous entry and you're back where you started.

### How far back does the history go?

As far as the version limit on your plan allows, and a version you pin is never pruned to make room. On a self-hosted server the only limit is your disk.

### Can I restore to a PC where the game isn't installed yet?

Install the game first so its save folder exists, then restore. Hoard knows where each game expects its saves, so it writes the snapshot to the right place without you hunting for the path.

### Does restoring work between Windows and a Steam Deck?

Yes. The same game keeps its save in different places on each — on the Deck, inside the Proton prefix — and Hoard writes the restored version wherever that machine expects it.

### Is a restore any different on a self-hosted server?

No. Same app, same history, same one-click restore. Only the storage is yours.
`,vn=`---
title: "Cómo restaurar una partida guardada anterior"
description: "¿Tomaste una mala decisión, se corrompió un archivo o quieres empezar de cero? Vuelve a cualquier versión anterior de tu partida con el historial en la nube de Hoard, incluidas copias hechas con herramientas como Ludusavi."
order: 3
updated: 2026-09-01
---

Una mala decisión en el juego, un archivo corrupto o un mod que lo rompe todo: a veces solo necesitas volver atrás. Como Hoard guarda un historial completo de versiones de cada partida, restaurar una anterior lleva segundos.

## Restaurar una versión anterior

1. Abre **Hoard** y ve al juego en tu **Biblioteca**.
2. Abre su pestaña **Historial**. Verás cada copia con su fecha y tamaño.
3. Elige la versión que quieras y pulsa **Restaurar**.
4. Hoard vuelve a escribir esa instantánea en la carpeta de guardado del juego. Tu partida actual se respalda primero, así que la restauración es reversible.

## Restaurar en un PC nuevo o reinstalado

1. Instala Hoard e inicia sesión con tu cuenta.
2. Añade el juego a tu Biblioteca: Hoard encuentra la copia en la nube correspondiente.
3. Restaura la última versión, o cualquiera anterior, y sigue jugando.

Como Hoard localiza las carpetas de guardado con la misma base de datos comunitaria que Ludusavi, sabe dónde colocar una partida restaurada incluso en una instalación limpia, sin que busques rutas a mano.

## Cuando una partida se corrompe o un mod la rompe

Un juego que se cierra al cargar, un mod que reescribió lo que no debía, un autoguardado que cayó a mitad de escritura: la solución es la misma. Abre el **Historial** del juego, elige la última versión anterior al problema y restáurala. Las fechas y los tamaños suelen bastar para ver dónde se torció: una caída brusca de tamaño es buena señal de que una partida quedó truncada.

Si no tienes claro cuál es la buena, restaura la candidata más probable y compruébalo dentro del juego. Volver a intentarlo no cuesta nada, porque la versión que acabas de reemplazar también se guardó.

## Qué hace realmente una restauración

Tres cosas que conviene saber, porque son las que hacen que restaurar sea seguro:

1. **Tu partida actual se captura primero.** La restauración es reversible: lo que reemplazaste pasa a ser una versión más del historial.
2. **Sólo se descarga lo que falta.** Los ficheros que ya están en disco con el contenido correcto se aprovechan tal cual, así que restaurar una partida grande después de un cambio pequeño mueve unos megas y no la carpeta entera.
3. **Los ficheros propios de esta máquina no se tocan.** La configuración y los registros que viven junto a la partida se copian, pero no se escriben encima de los tuyos: tus controles y tus ajustes gráficos sobreviven a una restauración que venga de otro PC.

## Restaurar sin pasar por nuestros servidores

Si levantas tu propio \`hoard-server\`, las restauraciones funcionan exactamente igual, sólo que las versiones vienen de tu máquina y no de la nuestra. No hay cuenta con nosotros, ni telemetría hacia nosotros, ni nada que pase por nuestros servidores. Mira [cómo autoalojar Hoard](/guides/self-host-hoard).

## Consejo

Las restauraciones nunca son destructivas: la partida que reemplazas se guarda antes como una nueva versión, así que siempre puedes deshacer una restauración volviendo a la entrada anterior. Si hasta ahora solo guardabas copias en local (por ejemplo con Ludusavi), pasar a Hoard añade un historial versionado y fuera del equipo desde el que puedes restaurar incluso tras un fallo de disco.

<!-- faq -->

## Preguntas frecuentes

### ¿Restaurar sobrescribe mi progreso actual?

Sólo después de que tu partida actual se haya capturado como una versión nueva. Si restauras la equivocada, restaura la entrada anterior y vuelves al punto de partida.

### ¿Hasta dónde llega el historial?

Hasta donde permita el tope de versiones de tu plan, y una versión que fijes no se poda nunca para hacer sitio. En un servidor autoalojado el único límite es tu disco.

### ¿Puedo restaurar en un PC donde el juego todavía no está instalado?

Instala primero el juego para que exista su carpeta de partidas, y luego restaura. Hoard sabe dónde espera cada juego sus saves, así que escribe la instantánea en el sitio correcto sin que tengas que buscar la ruta.

### ¿Funciona restaurar entre Windows y una Steam Deck?

Sí. El mismo juego guarda en sitios distintos en cada uno — en la Deck, dentro del prefijo de Proton — y Hoard escribe la versión restaurada donde esa máquina la espera.

### ¿Cambia algo restaurar en un servidor autoalojado?

No. Misma aplicación, mismo historial, misma restauración de un clic. Lo único tuyo es el almacenamiento.
`,gn=`---
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

Si vous faites tourner votre propre \`hoard-server\`, les restaurations fonctionnent exactement pareil, sauf que les versions viennent de votre machine et non de la nôtre. Aucun compte chez nous, aucune télémétrie vers nous, rien qui passe par nos serveurs. Voir [comment auto-héberger Hoard](/guides/self-host-hoard).

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
`,fn=`---
title: "Come ripristinare un vecchio salvataggio"
description: "Scelta sbagliata, file corrotto o voglia di ricominciare? Torna a qualsiasi versione precedente del tuo salvataggio con la cronologia cloud di Hoard — inclusi salvataggi fatti con strumenti come Ludusavi."
order: 3
updated: 2026-09-01
---

Una brutta decisione nel gioco, un file corrotto o una mod che rompe tutto — a volte devi solo tornare indietro. Poiché Hoard conserva una cronologia completa delle versioni di ogni salvataggio, ripristinarne uno precedente richiede pochi secondi.

## Ripristinare una versione precedente

1. Apri **Hoard** e vai al gioco nella tua **Libreria**.
2. Apri la scheda **Cronologia**. Vedrai ogni backup con data e dimensione.
3. Scegli la versione che vuoi e premi **Ripristina**.
4. Hoard riscrive quello snapshot nella cartella di salvataggio del gioco. Il salvataggio attuale viene salvato prima, quindi il ripristino è reversibile.

## Ripristinare su un PC nuovo o reinstallato

1. Installa Hoard e accedi con il tuo account.
2. Aggiungi il gioco alla Libreria — Hoard trova il backup cloud corrispondente.
3. Ripristina l'ultima versione, o una più vecchia, e continua a giocare.

Poiché Hoard individua le cartelle di salvataggio con lo stesso database comunitario di Ludusavi, sa dove mettere un salvataggio ripristinato anche su un'installazione pulita — senza cercare percorsi a mano.

## Quando un salvataggio è corrotto o l'ha rotto una mod

Un gioco che crasha al caricamento, una mod che ha riscritto ciò che non doveva, un salvataggio automatico caduto a metà scrittura: il rimedio è lo stesso. Apri la **Cronologia** del gioco, scegli l'ultima versione precedente al problema e ripristinala. Date e dimensioni bastano di solito a individuare il momento in cui è andata storta: un calo improvviso di dimensione è un buon indizio di un salvataggio troncato.

Se non sai quale sia quella buona, ripristina la candidata più probabile e verifica nel gioco. Riprovare non costa nulla, perché anche la versione appena sostituita è stata conservata.

## Cosa fa davvero un ripristino

Tre cose da sapere, perché sono quelle che rendono sicuro provarci:

1. **Il salvataggio attuale viene catturato per primo.** Il ripristino è reversibile: ciò che hai sostituito diventa una versione della cronologia come tutte le altre.
2. **Si scarica solo ciò che manca.** I file già su disco con il contenuto giusto vengono usati così come sono, quindi ripristinare un salvataggio grande dopo una piccola modifica sposta qualche megabyte e non l'intera cartella.
3. **I file che appartengono a questa macchina restano intatti.** Configurazione e log accanto al salvataggio vengono salvati, ma non riscritti sopra le tue copie locali: i tuoi comandi e le tue impostazioni grafiche sopravvivono a un ripristino arrivato da un altro PC.

## Ripristinare senza passare dai nostri server

Se fai girare il tuo \`hoard-server\`, i ripristini funzionano esattamente allo stesso modo, solo che le versioni arrivano dalla tua macchina invece che dalla nostra. Nessun account con noi, nessuna telemetria verso di noi, niente che passi dai nostri server. Vedi [come ospitare Hoard da solo](/guides/self-host-hoard).

## Suggerimento

I ripristini non sono mai distruttivi: il salvataggio che sostituisci viene prima catturato come nuova versione, quindi puoi sempre annullare un ripristino ripristinando la voce precedente. Se finora hai tenuto solo backup locali (ad esempio con Ludusavi), passare a Hoard aggiunge una cronologia versionata fuori dalla macchina, da cui puoi ripristinare anche dopo un guasto del disco.

<!-- faq -->

## Domande frequenti

### Il ripristino sovrascrive i miei progressi attuali?

Solo dopo che il salvataggio attuale è stato catturato come nuova versione. Se ripristini quella sbagliata, ripristina la voce precedente e sei di nuovo al punto di partenza.

### Fin dove arriva la cronologia?

Fin dove lo consente il limite di versioni del tuo piano, e una versione che fissi non viene mai eliminata per fare spazio. Su un server self-hosted l'unico limite è il tuo disco.

### Posso ripristinare su un PC dove il gioco non è ancora installato?

Installa prima il gioco, così esiste la sua cartella dei salvataggi, poi ripristina. Hoard sa dove ogni gioco si aspetta i salvataggi e scrive lo snapshot nel posto giusto senza che tu debba cercare il percorso.

### Funziona tra Windows e una Steam Deck?

Sì. Lo stesso gioco tiene il salvataggio in posti diversi sui due — sulla Deck, dentro il prefisso Proton — e Hoard scrive la versione ripristinata dove quella macchina se l'aspetta.

### Il ripristino cambia su un server self-hosted?

No. Stessa app, stessa cronologia, stesso ripristino in un clic. L'unica cosa tua è lo spazio di archiviazione.
`,bn=`---
title: "古いセーブデータを復元する方法"
description: "判断を誤った、ファイルが壊れた、最初からやり直したい？ Hoard のクラウド履歴で、セーブデータの任意の過去バージョンに巻き戻せます。Ludusavi などのツールで取ったバックアップも含みます。"
order: 3
updated: 2026-09-01
---

ゲーム内での悪い決断、壊れたファイル、失敗した MOD――時にはただ巻き戻したいだけのことがあります。Hoard はすべてのセーブの完全なバージョン履歴を保持しているので、以前のものへの復元は数秒で済みます。

## 以前のバージョンを復元する

1. **Hoard** を開き、**ライブラリ** で対象のゲームに移動します。
2. その **履歴** タブを開きます。各バックアップが日付とサイズ付きで表示されます。
3. 復元したいバージョンを選び、**復元** を選択します。
4. Hoard はそのスナップショットをゲームのセーブフォルダーに書き戻します。現在のセーブが先にバックアップされるため、復元自体も元に戻せます。

## 新しい PC や再インストールした PC で復元する

1. Hoard をインストールし、自分のアカウントでサインインします。
2. ゲームをライブラリに追加します――Hoard が対応するクラウドバックアップを見つけます。
3. 最新版、または任意の古い版を復元して、プレイを続けます。

Hoard は Ludusavi と同じコミュニティデータベースでセーブフォルダーを特定するため、クリーンインストールでも復元先を把握しています。手動でパスを探す必要はありません。

## セーブが壊れたとき、Mod が壊したとき

読み込みで落ちるゲーム、書き換えてはいけないものを書き換えた Mod、書き込みの途中で保存されたオートセーブ。対処はどれも同じです。そのゲームの **履歴** を開き、問題が起きる前の最後の世代を選んで復元します。日付とサイズだけで、どこでおかしくなったかはたいてい分かります。サイズが急に落ちていれば、セーブが途中で切れた良い手がかりです。

どれが無事な世代か分からないときは、いちばんそれらしいものを復元してゲームで確かめてください。やり直しに費用はかかりません。いま置き換えた世代も残っているからです。

## 復元で実際に起きること

知っておく価値のある 3 点です。ここが、気軽に試せる理由になります。

1. **いまのセーブが先に取り込まれます。** 復元は取り消せます。置き換えたものは、履歴の中のひとつの世代になります。
2. **足りないものだけをダウンロードします。** 正しい内容ですでにディスクにあるファイルはそのまま使われるため、小さな変更のあとに大きなセーブを復元しても、動くのはフォルダー全体ではなく数メガバイトです。
3. **そのマシンに属するファイルには触れません。** セーブの隣にある設定やログはバックアップされますが、あなたのローカルの内容を上書きすることはありません。別の PC から復元しても、キー割り当てやグラフィック設定はそのまま残ります。

## 当方のサーバーを介さない復元

自分で \`hoard-server\` を動かしている場合も、復元の動きはまったく同じで、世代の出どころが当方ではなく自分のマシンになるだけです。当方のアカウントも、当方へのテレメトリも、当方のサーバーを通るものもありません。[Hoard をセルフホストする方法](/guides/self-host-hoard) を参照してください。

## ヒント

復元が破壊的になることはありません。置き換えるセーブは先に新しいバージョンとして取り込まれるので、直前のエントリを復元すればいつでも復元を取り消せます。これまでローカルバックアップ（たとえば Ludusavi）しか持っていなかった場合、Hoard に移行するとマシン外の世代履歴が加わり、ディスク故障の後でもそこから復元できます。

<!-- faq -->

## よくある質問

### 復元すると今の進行は上書きされますか？

いまのセーブが新しい世代として取り込まれたあとに限り、上書きされます。選び間違えたら、ひとつ前の項目を復元すれば元の状態に戻ります。

### 履歴はどこまで遡れますか？

プランの世代数の上限まで遡れます。ピン留めした世代は、空きを作るために削除されることはありません。セルフホストのサーバーなら、上限はディスクの容量だけです。

### ゲームがまだ入っていない PC に復元できますか？

先にゲームをインストールしてセーブ用フォルダーを作ってから復元してください。Hoard は各ゲームがセーブをどこに置くかを把握しているので、パスを探さなくても正しい場所に書き込みます。

### Windows と Steam Deck のあいだでも復元できますか？

はい。同じゲームでも保存場所は両者で異なり、Deck では Proton のプレフィックスの中にあります。Hoard は復元した世代を、そのマシンが想定する場所に書き込みます。

### セルフホストのサーバーだと復元は変わりますか？

いいえ。同じアプリ、同じ履歴、同じワンクリックの復元です。自分のものになるのは保存先だけです。
`,Sn=`---
title: "Como restaurar um save antigo"
description: "Tomaste uma má decisão, corrompeste um ficheiro ou queres recomeçar? Volta a qualquer versão anterior do teu save com o histórico na nuvem do Hoard — incluindo saves feitos com ferramentas como o Ludusavi."
order: 3
updated: 2026-09-01
---

Uma má decisão no jogo, um ficheiro corrompido ou um mod que parte tudo — às vezes só precisas de voltar atrás. Como o Hoard guarda um histórico completo de versões de cada save, restaurar um anterior leva segundos.

## Restaurar uma versão anterior

1. Abre o **Hoard** e vai ao jogo na tua **Biblioteca**.
2. Abre o separador **Histórico**. Verás cada backup com data e tamanho.
3. Escolhe a versão que queres e carrega em **Restaurar**.
4. O Hoard volta a escrever esse snapshot na pasta de save do jogo. O teu save atual é guardado primeiro, por isso a restauração é reversível.

## Restaurar num PC novo ou reinstalado

1. Instala o Hoard e inicia sessão com a tua conta.
2. Adiciona o jogo à Biblioteca — o Hoard encontra o backup na nuvem correspondente.
3. Restaura a versão mais recente, ou uma mais antiga, e continua a jogar.

Como o Hoard localiza as pastas de save com a mesma base de dados comunitária do Ludusavi, sabe onde colocar um save restaurado mesmo numa instalação limpa — sem procurares caminhos à mão.

## Quando um save fica corrompido ou uma mod o parte

Um jogo que rebenta ao carregar, uma mod que reescreveu o que não devia, um autosave que caiu a meio de uma escrita: a solução é a mesma. Abre o **Histórico** do jogo, escolhe a última versão anterior ao problema e restaura-a. As datas e os tamanhos costumam chegar para ver onde correu mal — uma queda súbita de tamanho é bom sinal de que um save ficou truncado.

Se não tens a certeza de qual é a boa, restaura a candidata mais provável e confirma dentro do jogo. Tentar de novo não custa nada, porque a versão que acabaste de substituir também ficou guardada.

## O que uma restauração faz mesmo

Três coisas que vale a pena saber, porque são as que tornam seguro experimentar:

1. **O teu save atual é capturado primeiro.** A restauração é reversível: o que substituíste passa a ser uma versão do histórico como outra qualquer.
2. **Só se descarrega o que falta.** Os ficheiros já em disco com o conteúdo certo são aproveitados tal como estão, por isso restaurar um save grande depois de uma pequena alteração move alguns megabytes e não a pasta inteira.
3. **Os ficheiros próprios desta máquina ficam intactos.** A configuração e os registos ao lado do save são copiados, mas não escritos por cima das tuas cópias locais: os teus controlos e as tuas definições gráficas sobrevivem a uma restauração vinda de outro PC.

## Restaurar sem passar pelos nossos servidores

Se corres o teu próprio \`hoard-server\`, as restaurações funcionam exatamente da mesma maneira, só que as versões vêm da tua máquina e não da nossa. Não há conta connosco, nem telemetria para nós, nem nada que passe pelos nossos servidores. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard).

## Dica

As restaurações nunca são destrutivas: o save que substituis é primeiro capturado como nova versão, por isso podes sempre desfazer uma restauração restaurando a entrada anterior. Se até agora só guardavas backups locais (por exemplo com o Ludusavi), passar para o Hoard acrescenta um histórico versionado fora da máquina, a partir do qual podes restaurar mesmo depois de uma falha de disco.

<!-- faq -->

## Perguntas frequentes

### Restaurar sobrescreve o meu progresso atual?

Só depois de o teu save atual ter sido capturado como uma nova versão. Se restaurares a errada, restaura a entrada anterior e ficas onde estavas.

### Até onde vai o histórico?

Até onde permitir o limite de versões do teu plano, e uma versão que fixes nunca é apagada para abrir espaço. Num servidor self-hosted o único limite é o teu disco.

### Posso restaurar num PC onde o jogo ainda não está instalado?

Instala primeiro o jogo para que a pasta de saves exista, e depois restaura. O Hoard sabe onde cada jogo espera os seus saves, por isso escreve o snapshot no sítio certo sem teres de procurar o caminho.

### Funciona entre Windows e uma Steam Deck?

Sim. O mesmo jogo guarda em sítios diferentes em cada um — na Deck, dentro do prefixo Proton — e o Hoard escreve a versão restaurada onde essa máquina a espera.

### Restaurar é diferente num servidor self-hosted?

Não. Mesma aplicação, mesmo histórico, mesma restauração num clique. Só o armazenamento é teu.
`,yn=`---
title: "如何还原旧的游戏存档"
description: "走错了一步、文件损坏，或者想重新开始？用 Hoard 的云端历史回退到存档的任意先前版本——包括用 Ludusavi 等工具备份的存档。"
order: 3
updated: 2026-09-01
---

游戏中的错误决定、损坏的文件，或一个搞砸的 MOD——有时你只是需要回到从前。由于 Hoard 保留每个存档的完整版本历史，还原较早的版本只需几秒。

## 还原先前版本

1. 打开 **Hoard**，在你的**库**中找到该游戏。
2. 打开它的**历史**标签。你会看到每个备份及其日期和大小。
3. 选择你想要的版本，然后选择**还原**。
4. Hoard 会把该快照写回游戏的存档文件夹。你当前的存档会先被备份，因此还原本身也可撤销。

## 在新的或重装的 PC 上还原

1. 安装 Hoard 并用你的账号登录。
2. 把游戏添加到你的库——Hoard 会找到对应的云端备份。
3. 还原最新版本，或任意较早的版本，然后继续游戏。

由于 Hoard 使用与 Ludusavi 相同的社区数据库来定位存档文件夹，即使在全新安装上，它也知道把还原的存档放到哪里——无需你手动查找路径。

## 当存档损坏，或被模组弄坏时

读档就崩溃的游戏、改写了不该改的模组、写到一半就落地的自动存档——处理方式都一样。打开该游戏的**历史**，选择出问题之前的最后一个版本，还原它。日期和大小通常就足以看出是哪一刻出了岔子：体积突然变小，往往说明存档被截断了。

如果拿不准哪个版本是好的，就先还原最可能的那个，再进游戏确认。重来一次没有代价，因为你刚刚替换掉的版本同样被保留了下来。

## 还原到底做了什么

有三点值得知道，正是它们让"先试一次"变得安全：

1. **你当前的存档会先被抓取。** 还原是可逆的：被替换掉的内容会成为历史里的一个版本，和其他版本没有区别。
2. **只下载缺少的部分。** 磁盘上内容正确的文件会被直接沿用，因此在一次小改动之后还原一个大存档，搬动的是几兆字节，而不是整个文件夹。
3. **属于这台机器的文件不会被动。** 存档旁边的配置和日志会被备份，但不会覆盖你本地的副本——从另一台 PC 还原之后，你的按键绑定和画质设置依然还在。

## 不经过我们服务器的还原

如果你运行的是自己的 \`hoard-server\`，还原的方式完全一样，只是版本来自你自己的机器而不是我们的。没有我们这边的账号，没有发往我们的遥测，也没有任何东西经过我们的服务器。参见[如何自托管 Hoard](/guides/self-host-hoard)。

## 提示

还原绝不是破坏性的：被替换的存档会先作为新版本被捕获，因此你总能通过还原上一条记录来撤销一次还原。如果你过去只保留本地备份（例如用 Ludusavi），迁移到 Hoard 会增加一份脱离本机的版本历史，即使在磁盘故障之后，你也能从中还原。

<!-- faq -->

## 常见问题

### 还原会覆盖我当前的进度吗？

只有在你当前的存档已被抓取为一个新版本之后才会。如果还原错了，把上一条记录再还原一次，就回到了原点。

### 历史能回溯多久？

取决于你所在方案的版本数上限；被你固定的版本永远不会为了腾空间而被清理。在自托管的服务器上，唯一的限制是你的磁盘。

### 可以还原到还没安装游戏的 PC 上吗？

先安装游戏，让它的存档文件夹存在，然后再还原。Hoard 知道每款游戏把存档放在哪里，会直接写到正确的位置，不必你去找路径。

### Windows 和 Steam Deck 之间能互相还原吗？

可以。同一款游戏在两边的存档位置不同——在 Deck 上位于 Proton 前缀内——Hoard 会把还原的版本写到那台机器期望的位置。

### 在自托管服务器上还原有区别吗？

没有。同样的应用、同样的历史、同样一键还原。只有存储归你所有。
`,kn=`---
title: "Hoard mit Docker selbst hosten (Self-Hosting)"
description: "Betreibe deinen eigenen Hoard-Server in Minuten mit Docker Compose. Open Source, kostenlos, auf deiner Hardware – eine voll selbst gehostete Cloud für deine Spielstände, ohne Konto und ohne Speicherlimit."
order: 0
featured: true
updated: 2026-09-03
---

Hoard ist Open Source und selbst hostbar. Statt Hoard Cloud zu nutzen, kannst du denselben \`hoard-server\` auf deiner eigenen Maschine betreiben und jedes Gerät darauf verweisen – ohne Konto und ohne Speicherlimit außer der Festplatte, die du ihm gibst. Diese Anleitung bringt einen Server in wenigen Minuten mit Docker zum Laufen.

## Warum Hoard selbst hosten

- **Volle Kontrolle.** Deine Spielstände liegen auf Hardware, die du kontrollierst, nicht in fremder Cloud.
- **Kein Limit.** Der Speicher wird nur von deiner eigenen Festplatte begrenzt.
- **Gleiche App, gleiche Funktionen.** Versionierter Verlauf und Hintergrund-Sync funktionieren genau wie mit Hoard Cloud – nur das Backend ändert sich.
- **Open Source.** Du kannst den Server lesen, prüfen und anpassen.

Das ist der entscheidende Unterschied zu Tools wie [Ludusavi](/guides/ludusavi-alternative): Ludusavi ist großartig für lokale Backups und eigene Cloud per Rclone, aber den Sync richtest du selbst ein. Hoard bietet dir einen verwalteten Sync-Server, den du einmal startest und mit dem sich jedes Gerät verbindet.

## Was Selbsthosten für deine Daten bedeutet

Das gehört klar gesagt, denn genau hier liegen die meisten Vergleiche bei Hoard falsch.

**Hoard Cloud** ist die verwaltete Variante: du meldest dich an, und deine Spielstände liegen auf unseren Servern in der EU.

**Ein selbst gehostetes Hoard gehört vollständig dir.** Deine Geräte sprechen mit deinem Server und mit sonst nichts. Es gibt **kein Konto bei uns, keine Telemetrie zu uns, kein Limit und kein Relay** — nichts läuft über unsere Server, weil nichts von uns im Weg steht. Wir können weder einen Spielstand noch einen Spieltitel noch eine E-Mail-Adresse sehen, schlicht weil davon nichts bei uns ankommt. Würde Hoard Cloud morgen abgeschaltet, liefe dein Setup unverändert weiter.

Eine Sache der Genauigkeit halber: dein Server hat sehr wohl eigene Zugänge — den Benutzer, den du unten anlegst, und ein Token je Gerät. Die gehören dir, auf deiner Maschine, in deiner Datenbank. Was es nicht gibt, ist ein Konto bei uns.

## Was du brauchst

- Eine Maschine, die durchläuft (Heimserver, NAS mit Docker oder ein kleiner VPS).
- Docker und Docker Compose installiert.
- Optional eine Domain und ein Reverse-Proxy für HTTPS (empfohlen für alles außerhalb deines LAN).

## Installation mit Docker Compose

Klone das Repo, erstelle eine Konfiguration aus dem Beispiel und starte den Stack:

\`\`\`sh
git clone https://github.com/rleeon/hoard.git && cd hoard
mkdir -p deploy/docker/config
cp deploy/config.toml.example deploy/docker/config/config.toml
$EDITOR deploy/docker/config/config.toml      # Use nano or vim or something lol

cd deploy/docker
docker compose up -d
docker compose logs -f                         # wait for "listening"
\`\`\`

Warte, bis die Logs zeigen, dass der Server lauscht. Die Daten liegen in einem benannten Docker-Volume (\`hoard-data\`) – sichere es wie jedes andere Volume. Der Container lauscht intern auf Port \`12421\`; einen anderen Host-Port setzt du mit \`HOARD_PORT=9000 docker compose up -d\`.

## Benutzer und Geräte-Token anlegen

Der Server hat keine Registrierungsseite – Benutzer legst du auf der Kommandozeile an:

\`\`\`sh
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    user create alice --admin --password 'CHANGE_ME'
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    token create alice --device 'desktop'
\`\`\`

Das Token wird nur einmal angezeigt und **kann später nicht wiederhergestellt werden**, also kopiere es jetzt.

## Die Desktop-App verbinden

Installiere die [Hoard-Desktop-App](/download) auf jedem Rechner. Wähle im Onboarding **Self-Host** und füge deine Server-URL und das eben erstellte Token ein. Ab da verhält es sich genau wie Hoard Cloud: Es erkennt deine Spiele, sichert Spielstände automatisch und führt einen versionierten Verlauf. Siehe [Spielstände zwischen PCs synchronisieren](/guides/sync-game-saves-across-pcs) für den Alltag.

## Halte deinen Server aktuell

Wie du aktualisierst, hängt davon ab, wie du installiert hast — und der falsche Befehl liefert keinen Fehler, sondern tut schlicht nichts. Es lohnt sich also zu wissen, welcher deiner ist.

**Docker Compose.** Neues Image holen und den Container neu erstellen. Beide Hälften, in dieser Reihenfolge:

\`\`\`sh
docker compose pull
docker compose up -d
\`\`\`

Hörst du nach der ersten auf, läuft der alte Container unberührt weiter: \`/v1/health\` meldet weiterhin die alte Version, und das Update sieht aus, als wäre es still gescheitert. \`git pull\` aktualisiert weder das eine noch das andere — was läuft, ist das veröffentlichte Image, nicht dein Checkout. Nagle eine Version fest (\`ghcr.io/rleeon/hoard:1.1\`) statt \`:latest\`, wenn du lieber selbst entscheidest, wann eine neue kommt.

**Unraid.** Reiter *Docker* → Hoard → *Apply update*, sobald eines angeboten wird. Nichts zu tippen.

**Bare Metal (systemd).** \`sudo hoard-server upgrade\`, danach \`sudo systemctl restart hoard-server\`. Der Befehl tauscht die Binärdatei atomar aus und startet den Dienst absichtlich nicht selbst neu, damit eine laufende Synchronisierung nicht abgeschnitten wird.

\`hoard-server upgrade\` gilt nur für die Bare-Metal-Installation. In einem Container verweigert er sich absichtlich — der Binärtausch würde das nächste \`docker compose up -d\` nicht überleben — und gibt stattdessen die beiden Befehle von oben aus; führe \`docker compose exec server hoard-server upgrade\` aus, wenn du es selbst sehen willst. Datenbankmigrationen wendet der Server beim Start an, dafür gibt es also nie einen eigenen Schritt.

## Im Produktivbetrieb

Für alles, was über dein lokales Netz hinausgeht, beende TLS an einem Reverse-Proxy (Caddy, nginx oder Traefik). Lieber Bare Metal? Das Repo liefert auch ein \`systemd\`-Installationsskript und einen Befehl \`hoard-server upgrade\`, der die Binärdatei atomar austauscht, ohne einen laufenden Sync abzubrechen.

## Selbst hosten oder Hoard Cloud?

Selbst-Hosting ist ideal, wenn du schon einen Server betreibst und volle Kontrolle ohne Limit willst. Wenn du keine Infrastruktur pflegen möchtest, bietet dir [Hoard Cloud](/pricing) denselben Sync verwaltet, mit einem kostenlosen Einstieg. So oder so bleiben App und Spielstände portabel – du kannst später wechseln.

<!-- faq -->

## Häufige Fragen

### Funkt ein selbst gehostetes Hoard nach Hause?

Nein. Die Desktop-App spricht mit der Serveradresse, die du ihr gibst. Deine Stände, deine Nutzer und deine Logs bleiben auf deiner Maschine, und nichts davon erreicht uns.

### Ist der selbst gehostete Server derselbe Code wie Hoard Cloud?

Ja, dasselbe \`hoard-server\`-Binary unter AGPL-3.0. Es gibt keine abgespeckte Community-Edition und keine Funktion, die der gehosteten Version vorbehalten wäre.

### Wo liegen die Spielstände tatsächlich?

Standardmäßig in dem Docker-Volume, das du dem Container gibst, auf deiner eigenen Platte. Wenn du bereits Objektspeicher betreibst, spricht der Server auch S3 — MinIO, Garage oder Backblaze B2 funktionieren als Ablage. So oder so sprechen deine Geräte ausschließlich mit deinem Server.

### Läuft das auf einem NAS?

Ja, auf jedem NAS mit Docker. Das Repository enthält eine Unraid-Vorlage, und das Image wechselt auf die \`PUID\`/\`PGID\`, die du angibst, damit eingebundene Ordner dem richtigen Benutzer gehören statt root.

### Brauche ich Domain und HTTPS?

Im eigenen LAN nicht. Sobald der Server von außen erreichbar ist, gehört ein Reverse Proxy davor, der TLS terminiert — Caddy, nginx oder Traefik.

### Was, wenn mein Server aus ist, wenn ich aufhöre zu spielen?

Der Snapshot entsteht lokal, es geht also nichts verloren. Er wird von selbst hochgeladen, sobald der Server wieder antwortet.

### Kann ich mit Hoard Cloud anfangen und später wechseln?

Ja, in beide Richtungen. Über die Kontoseite lässt sich alles exportieren, und die App kann ohne Neuinstallation auf einen anderen Server zeigen.
`,qn=`---
title: "How to self-host Hoard with Docker"
description: "Run your own Hoard server with Docker Compose in minutes. Open source, free, on your hardware — a fully self-hosted cloud for your game saves, no account or quota."
order: 0
featured: true
updated: 2026-09-03
---

Hoard is open source and self-hostable. Instead of using Hoard Cloud, you can run the same \`hoard-server\` on your own machine and point every device at it — no account, no storage quota beyond the disk you give it. This guide gets a server running with Docker in a few minutes.

## Why self-host Hoard

- **Full ownership.** Your game saves live on hardware you control, not someone else's cloud.
- **No quota.** Storage is limited only by your own disk.
- **Same app, same features.** Versioned history and background sync work exactly as they do with Hoard Cloud — only the backend changes.
- **Open source.** You can read, audit and modify the server.

This is the key difference from tools like [Ludusavi](/guides/ludusavi-alternative): Ludusavi is great for local backups and bring-your-own-cloud via Rclone, but you wire up the sync yourself. Hoard gives you a managed sync server you run once and every device connects to.

## What self-hosting means for your data

Worth stating plainly, because it's the thing most comparisons get wrong about Hoard.

**Hoard Cloud** is the managed option: you sign in, and your saves sit on our servers, in the EU.

**A self-hosted Hoard is entirely yours.** Your devices talk to your server and to nothing else. There is **no account with us, no telemetry to us, no quota and no relay** — nothing passes through our servers, because there is nothing of ours in the path. We can't see a save, a game name or an email address, for the simple reason that none of it ever reaches us. If Hoard Cloud shut down tomorrow, your setup would carry on unchanged.

To be exact about one thing: your server does have logins of its own — the user you create below, and a token per device. Those are yours, on your machine, in your database. What doesn't exist is an account with us.

## What you need

- A machine that stays on (a home server, NAS that runs Docker, or a small VPS).
- Docker and Docker Compose installed.
- Optionally a domain name and a reverse proxy for HTTPS (recommended for anything beyond your LAN).

## Install with Docker Compose

Clone the repo, create a config from the example, and start the stack:

\`\`\`sh
git clone https://github.com/rleeon/hoard.git && cd hoard
mkdir -p deploy/docker/config
cp deploy/config.toml.example deploy/docker/config/config.toml
$EDITOR deploy/docker/config/config.toml      # Use nano or vim or something lol

cd deploy/docker
docker compose up -d
docker compose logs -f                         # wait for "listening"
\`\`\`

Wait until the logs show that the server is listening. Data lives in a named Docker volume (\`hoard-data\`) — back it up like any other volume. The container listens on port \`12421\` internally; map a different host port with \`HOARD_PORT=9000 docker compose up -d\`.

## Create your user and a device token

The server has no signup screen — you create users from the command line:

\`\`\`sh
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    user create alice --admin --password 'CHANGE_ME'
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    token create alice --device 'desktop'
\`\`\`

The token is printed once and **cannot be retrieved later**, so copy it now.

## Connect the desktop app

Install the [Hoard desktop app](/download) on each machine. In the onboarding flow, pick **Self-Host**, then paste your server URL and the token you just created. From there it behaves exactly like Hoard Cloud: it detects your games, backs up saves automatically, and keeps versioned history. See [syncing saves across PCs](/guides/sync-game-saves-across-pcs) for the day-to-day flow.

## Keep your server up to date

How you update depends on how you installed it, and the wrong command is a no-op rather than an error — so it is worth knowing which one is yours.

**Docker Compose.** Pull the new image and recreate the container. Both halves, in order:

\`\`\`sh
docker compose pull
docker compose up -d
\`\`\`

Stop after the first and the old container keeps running untouched: \`/v1/health\` goes on reporting the old version and the update looks as if it silently failed. \`git pull\` updates neither — what runs is the published image, not your checkout. Pin a version (\`ghcr.io/rleeon/hoard:1.1\`) instead of \`:latest\` if you would rather choose when a new one lands.

**Unraid.** *Docker* tab → Hoard → *Apply update* when one is offered. Nothing to type.

**Bare metal (systemd).** \`sudo hoard-server upgrade\`, then \`sudo systemctl restart hoard-server\`. It swaps the binary atomically and deliberately does not restart the service itself, so an in-flight sync is not killed.

\`hoard-server upgrade\` is for the bare-metal install only. Inside a container it refuses on purpose — the binary swap would not survive the next \`docker compose up -d\` — and prints the two commands above instead; run \`docker compose exec server hoard-server upgrade\` if you want to see it say so. Database migrations are applied by the server when it starts, so there is never a separate step for them.

## Run it in production

For anything exposed beyond your local network, terminate TLS at a reverse proxy (Caddy, nginx or Traefik). Prefer bare metal? The repo also ships a \`systemd\` install script and a \`hoard-server upgrade\` command that swaps the binary atomically without killing an in-flight sync.

## Self-host or Hoard Cloud?

Self-hosting is ideal if you already run a server and want full control with no quota. If you'd rather not maintain infrastructure, [Hoard Cloud](/pricing) gives you the same sync managed for you, with a free tier to start. Either way the app and your saves stay portable — you can switch later.

<!-- faq -->

## Frequently asked questions

### Does a self-hosted Hoard phone home?

No. The desktop app talks to the server address you give it. Your saves, your users and your logs stay on your machine, and nothing about them reaches us.

### Is the self-hosted server the same code as Hoard Cloud?

Yes, the same \`hoard-server\` binary, under AGPL-3.0. There is no cut-down community edition and no feature held back for the hosted version.

### Where are the saves actually stored?

By default in the Docker volume you gave the container, on your own disk. If you already run object storage, the server also speaks S3, so MinIO, Garage or Backblaze B2 work as the backing store. Either way, your devices only ever talk to your server.

### Can I run it on a NAS?

Yes, on any NAS that runs Docker. The repository ships an Unraid template, and the image drops to the \`PUID\`/\`PGID\` you give it, so bind-mounted folders end up owned by the right user instead of root.

### Do I need a domain and HTTPS?

Not on your own LAN. The moment the server is reachable from outside it, put a reverse proxy in front of it and terminate TLS there — Caddy, nginx or Traefik all work.

### What if my server is down when I finish playing?

The snapshot is taken locally, so nothing is lost. It uploads on its own once the server answers again.

### Can I start on Hoard Cloud and move later?

Yes, in both directions. You can export everything from your account page, and the app can be pointed at a different server without reinstalling.
`,zn=`---
title: "Cómo autoalojar Hoard con Docker (self-hosted)"
description: "Monta tu propio servidor de Hoard con Docker Compose en minutos. Código abierto, gratis y en tu hardware: una nube totalmente self-hosted para tus partidas guardadas, sin cuenta ni límite de espacio."
order: 0
featured: true
updated: 2026-09-03
---

Hoard es de código abierto y se puede autoalojar. En lugar de usar Hoard Cloud, puedes ejecutar el mismo \`hoard-server\` en tu propia máquina y apuntar todos tus dispositivos a él: sin cuenta y sin más límite de espacio que el disco que le des. Esta guía deja un servidor funcionando con Docker en pocos minutos.

## Por qué autoalojar Hoard

- **Control total.** Tus partidas viven en hardware que tú controlas, no en la nube de otro.
- **Sin cuota.** El espacio solo lo limita tu propio disco.
- **La misma app, las mismas funciones.** El historial versionado y la sincronización en segundo plano funcionan igual que con Hoard Cloud; solo cambia el backend.
- **Código abierto.** Puedes leer, auditar y modificar el servidor.

Esta es la diferencia clave frente a herramientas como [Ludusavi](/guides/ludusavi-alternative): Ludusavi es excelente para copias locales y para usar tu propia nube vía Rclone, pero la sincronización la montas tú. Hoard te da un servidor de sincronización gestionado que arrancas una vez y al que se conectan todos los dispositivos.

## Qué significa autoalojarse para tus datos

Conviene decirlo sin rodeos, porque es lo que casi todas las comparativas se equivocan sobre Hoard.

**Hoard Cloud** es la opción gestionada: inicias sesión y tus partidas están en nuestros servidores, en la UE.

**Un Hoard autoalojado es tuyo por completo.** Tus dispositivos hablan con tu servidor y con nada más. **No hay cuenta con nosotros, ni telemetría hacia nosotros, ni cupo, ni relé**: no pasa nada por nuestros servidores, porque no hay nada nuestro en el camino. No podemos ver una partida, ni el nombre de un juego, ni un correo, por la sencilla razón de que nada de eso nos llega. Si Hoard Cloud cerrara mañana, tu montaje seguiría funcionando igual.

Y para ser exactos en una cosa: tu servidor sí tiene sus propios accesos — el usuario que crearás más abajo y un token por dispositivo. Son tuyos, en tu máquina, en tu base de datos. Lo que no existe es una cuenta con nosotros.

## Qué necesitas

- Una máquina que esté siempre encendida (un servidor casero, un NAS que ejecute Docker o un VPS pequeño).
- Docker y Docker Compose instalados.
- Opcionalmente un dominio y un proxy inverso para HTTPS (recomendado para cualquier cosa fuera de tu red local).

## Instalación con Docker Compose

Clona el repositorio, crea una configuración a partir del ejemplo y arranca el stack:

\`\`\`sh
git clone https://github.com/rleeon/hoard.git && cd hoard
mkdir -p deploy/docker/config
cp deploy/config.toml.example deploy/docker/config/config.toml
$EDITOR deploy/docker/config/config.toml      # Use nano or vim or something lol

cd deploy/docker
docker compose up -d
docker compose logs -f                         # wait for "listening"
\`\`\`

Espera a que los logs muestren que el servidor está escuchando. Los datos se guardan en un volumen de Docker (\`hoard-data\`); haz copia de seguridad como con cualquier otro volumen. El contenedor escucha internamente en el puerto \`12421\`; usa otro puerto del host con \`HOARD_PORT=9000 docker compose up -d\`.

## Crea tu usuario y un token de dispositivo

El servidor no tiene pantalla de registro: los usuarios se crean por línea de comandos:

\`\`\`sh
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    user create alice --admin --password 'CHANGE_ME'
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    token create alice --device 'desktop'
\`\`\`

El token se muestra una sola vez y **no se puede recuperar después**, así que cópialo ahora.

## Conecta la aplicación de escritorio

Instala la [app de escritorio de Hoard](/download) en cada equipo. En el asistente inicial elige **Self-Host**, y pega la URL de tu servidor y el token que acabas de crear. A partir de ahí se comporta igual que Hoard Cloud: detecta tus juegos, copia las partidas automáticamente y mantiene el historial versionado. Consulta [sincronizar partidas entre varios PC](/guides/sync-game-saves-across-pcs) para el día a día.

## Mantén tu servidor al día

Cómo se actualiza depende de cómo lo instalaste, y equivocarse de comando no da error: simplemente no hace nada. Merece la pena saber cuál es el tuyo.

**Docker Compose.** Baja la imagen nueva y recrea el contenedor. Las dos mitades, en orden:

\`\`\`sh
docker compose pull
docker compose up -d
\`\`\`

Si te quedas en la primera, el contenedor viejo sigue corriendo intacto: \`/v1/health\` sigue informando de la versión antigua y la actualización parece haber fallado en silencio. \`git pull\` no actualiza ninguna de las dos: lo que corre es la imagen publicada, no tu copia del repositorio. Fija una versión (\`ghcr.io/rleeon/hoard:1.1\`) en lugar de \`:latest\` si prefieres elegir tú cuándo llega una nueva.

**Unraid.** Pestaña *Docker* → Hoard → *Apply update* cuando aparezca. No hay nada que teclear.

**Bare metal (systemd).** \`sudo hoard-server upgrade\` y después \`sudo systemctl restart hoard-server\`. Cambia el binario de forma atómica y a propósito no reinicia el servicio por su cuenta, para no cortar una sincronización en marcha.

\`hoard-server upgrade\` es sólo para la instalación bare metal. Dentro de un contenedor se niega a propósito —el cambio de binario no sobreviviría al siguiente \`docker compose up -d\`— e imprime los dos comandos de arriba; ejecuta \`docker compose exec server hoard-server upgrade\` si quieres verlo decirlo. Las migraciones de la base de datos las aplica el servidor al arrancar, así que nunca hay un paso aparte para ellas.

## Llevarlo a producción

Para cualquier cosa expuesta fuera de tu red local, termina el TLS en un proxy inverso (Caddy, nginx o Traefik). ¿Prefieres bare metal? El repositorio también incluye un script de instalación con \`systemd\` y un comando \`hoard-server upgrade\` que cambia el binario de forma atómica sin cortar una sincronización en curso.

## ¿Self-hosted o Hoard Cloud?

Autoalojar es ideal si ya tienes un servidor y quieres control total sin límites. Si prefieres no mantener infraestructura, [Hoard Cloud](/pricing) te da la misma sincronización gestionada por nosotros, con un plan gratuito para empezar. En cualquier caso, la app y tus partidas siguen siendo portables: puedes cambiar más adelante.

<!-- faq -->

## Preguntas frecuentes

### ¿Un Hoard autoalojado llama a casa?

No. La aplicación de escritorio habla con la dirección de servidor que tú le des. Tus partidas, tus usuarios y tus registros se quedan en tu máquina, y nada de eso nos llega.

### ¿El servidor autoalojado es el mismo código que Hoard Cloud?

Sí, el mismo binario \`hoard-server\`, bajo AGPL-3.0. No hay una edición comunitaria recortada ni funciones reservadas para la versión alojada.

### ¿Dónde se guardan realmente las partidas?

Por defecto, en el volumen de Docker que le des al contenedor, en tu propio disco. Si ya tienes almacenamiento de objetos, el servidor también habla S3, así que MinIO, Garage o Backblaze B2 sirven como respaldo. En cualquier caso, tus dispositivos sólo hablan con tu servidor.

### ¿Puedo montarlo en un NAS?

Sí, en cualquier NAS que corra Docker. El repositorio incluye una plantilla de Unraid, y la imagen baja al \`PUID\`/\`PGID\` que le indiques, así que las carpetas montadas acaban siendo del usuario correcto y no de root.

### ¿Necesito dominio y HTTPS?

En tu propia red local, no. En cuanto el servidor sea accesible desde fuera, pon un proxy inverso delante y termina ahí el TLS: Caddy, nginx o Traefik valen igual.

### ¿Y si mi servidor está caído cuando termino de jugar?

La instantánea se toma en local, así que no se pierde nada. Se sube sola en cuanto el servidor vuelve a responder.

### ¿Puedo empezar en Hoard Cloud y mudarme después?

Sí, y en los dos sentidos. Puedes exportarlo todo desde la página de tu cuenta, y la aplicación se puede apuntar a otro servidor sin reinstalar nada.
`,wn=`---
title: "Comment auto-héberger Hoard avec Docker (self-hosted)"
description: "Lancez votre propre serveur Hoard en quelques minutes avec Docker Compose. Open source, gratuit, sur votre matériel : un cloud entièrement auto-hébergé pour vos sauvegardes de jeux, sans compte ni quota."
order: 0
featured: true
updated: 2026-09-03
---

Hoard est open source et auto-hébergeable. Au lieu d'utiliser Hoard Cloud, vous pouvez exécuter le même \`hoard-server\` sur votre propre machine et y connecter chaque appareil — sans compte, sans quota au-delà du disque que vous lui donnez. Ce guide met un serveur en route avec Docker en quelques minutes.

## Pourquoi auto-héberger Hoard

- **Maîtrise totale.** Vos sauvegardes vivent sur du matériel que vous contrôlez, pas sur le cloud d'un autre.
- **Aucun quota.** L'espace n'est limité que par votre propre disque.
- **Même app, mêmes fonctions.** L'historique versionné et la synchro en arrière-plan fonctionnent comme avec Hoard Cloud — seul le backend change.
- **Open source.** Vous pouvez lire, auditer et modifier le serveur.

C'est la différence clé avec des outils comme [Ludusavi](/guides/ludusavi-alternative) : Ludusavi est excellent pour les sauvegardes locales et le cloud « apportez le vôtre » via Rclone, mais c'est à vous de câbler la synchro. Hoard vous donne un serveur de synchro géré que vous lancez une fois et auquel chaque appareil se connecte.

## Ce que l'auto-hébergement veut dire pour vos données

Autant le dire franchement, car c'est là que presque toutes les comparaisons se trompent au sujet de Hoard.

**Hoard Cloud** est l'option gérée : vous vous connectez, et vos sauvegardes se trouvent sur nos serveurs, dans l'UE.

**Un Hoard auto-hébergé est entièrement le vôtre.** Vos appareils parlent à votre serveur et à rien d'autre. Il n'y a **aucun compte chez nous, aucune télémétrie vers nous, aucun quota et aucun relais** : rien ne passe par nos serveurs, puisque rien de chez nous n'est sur le chemin. Nous ne pouvons voir ni une sauvegarde, ni un nom de jeu, ni une adresse e-mail, pour la simple raison que rien de tout cela ne nous parvient. Si Hoard Cloud fermait demain, votre installation continuerait à l'identique.

Une précision, pour être exact : votre serveur a bel et bien ses propres accès — l'utilisateur que vous créez plus bas, et un jeton par appareil. Ils sont à vous, sur votre machine, dans votre base. Ce qui n'existe pas, c'est un compte chez nous.

## Ce qu'il vous faut

- Une machine qui reste allumée (serveur maison, NAS exécutant Docker ou petit VPS).
- Docker et Docker Compose installés.
- Éventuellement un nom de domaine et un reverse proxy pour le HTTPS (recommandé au-delà de votre réseau local).

## Installation avec Docker Compose

Clonez le dépôt, créez une configuration depuis l'exemple et démarrez la pile :

\`\`\`sh
git clone https://github.com/rleeon/hoard.git && cd hoard
mkdir -p deploy/docker/config
cp deploy/config.toml.example deploy/docker/config/config.toml
$EDITOR deploy/docker/config/config.toml      # Use nano or vim or something lol

cd deploy/docker
docker compose up -d
docker compose logs -f                         # wait for "listening"
\`\`\`

Attendez que les logs indiquent que le serveur écoute. Les données vivent dans un volume Docker nommé (\`hoard-data\`) — sauvegardez-le comme n'importe quel volume. Le conteneur écoute en interne sur le port \`12421\` ; choisissez un autre port hôte avec \`HOARD_PORT=9000 docker compose up -d\`.

## Créez votre utilisateur et un jeton d'appareil

Le serveur n'a pas d'écran d'inscription — vous créez les utilisateurs en ligne de commande :

\`\`\`sh
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    user create alice --admin --password 'CHANGE_ME'
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    token create alice --device 'desktop'
\`\`\`

Le jeton n'est affiché qu'une fois et **ne peut pas être récupéré ensuite**, copiez-le maintenant.

## Connectez l'application de bureau

Installez l'[app de bureau Hoard](/download) sur chaque machine. Dans l'assistant, choisissez **Self-Host**, puis collez l'URL de votre serveur et le jeton que vous venez de créer. Ensuite, le comportement est identique à Hoard Cloud : détection des jeux, sauvegarde automatique et historique versionné. Voir [synchroniser ses sauvegardes entre PC](/guides/sync-game-saves-across-pcs) pour l'usage quotidien.

## Gardez votre serveur à jour

La façon de mettre à jour dépend de la façon dont vous l'avez installé, et se tromper de commande ne produit pas d'erreur : cela ne fait tout simplement rien. Autant savoir laquelle est la vôtre.

**Docker Compose.** Récupérez la nouvelle image et recréez le conteneur. Les deux moitiés, dans cet ordre :

\`\`\`sh
docker compose pull
docker compose up -d
\`\`\`

Si vous vous arrêtez à la première, l'ancien conteneur continue de tourner intact : \`/v1/health\` annonce toujours l'ancienne version et la mise à jour a l'air d'avoir échoué en silence. \`git pull\` ne met à jour ni l'un ni l'autre — ce qui tourne, c'est l'image publiée, pas votre copie du dépôt. Épinglez une version (\`ghcr.io/rleeon/hoard:1.1\`) au lieu de \`:latest\` si vous préférez choisir quand une nouvelle arrive.

**Unraid.** Onglet *Docker* → Hoard → *Apply update* quand une mise à jour est proposée. Rien à taper.

**Bare metal (systemd).** \`sudo hoard-server upgrade\`, puis \`sudo systemctl restart hoard-server\`. La commande remplace le binaire de façon atomique et ne redémarre volontairement pas le service elle-même, pour ne pas couper une synchro en cours.

\`hoard-server upgrade\` ne concerne que l'installation bare metal. Dans un conteneur, elle refuse volontairement — le remplacement du binaire ne survivrait pas au prochain \`docker compose up -d\` — et affiche les deux commandes ci-dessus ; lancez \`docker compose exec server hoard-server upgrade\` si vous voulez le constater. Les migrations de base de données sont appliquées par le serveur au démarrage : il n'y a jamais d'étape séparée pour elles.

## En production

Pour tout ce qui dépasse votre réseau local, terminez le TLS sur un reverse proxy (Caddy, nginx ou Traefik). Plutôt bare metal ? Le dépôt fournit aussi un script d'installation \`systemd\` et une commande \`hoard-server upgrade\` qui remplace le binaire de façon atomique sans interrompre une synchro en cours.

## Auto-hébergement ou Hoard Cloud ?

L'auto-hébergement est idéal si vous avez déjà un serveur et voulez un contrôle total sans quota. Si vous préférez ne pas gérer d'infrastructure, [Hoard Cloud](/pricing) vous offre la même synchro gérée pour vous, avec une offre gratuite pour démarrer. Dans les deux cas, l'app et vos sauvegardes restent portables — vous pouvez changer plus tard.

<!-- faq -->

## Questions fréquentes

### Un Hoard auto-hébergé communique-t-il avec vous ?

Non. L'application de bureau parle à l'adresse de serveur que vous lui donnez. Vos sauvegardes, vos utilisateurs et vos journaux restent sur votre machine, et rien de tout cela ne nous parvient.

### Le serveur auto-hébergé est-il le même code que Hoard Cloud ?

Oui, le même binaire \`hoard-server\`, sous AGPL-3.0. Il n'y a pas d'édition communautaire allégée ni de fonction réservée à la version hébergée.

### Où sont réellement stockées les sauvegardes ?

Par défaut dans le volume Docker que vous donnez au conteneur, sur votre propre disque. Si vous avez déjà du stockage objet, le serveur parle aussi S3 : MinIO, Garage ou Backblaze B2 font l'affaire. Dans tous les cas, vos appareils ne parlent qu'à votre serveur.

### Puis-je le faire tourner sur un NAS ?

Oui, sur n'importe quel NAS qui exécute Docker. Le dépôt fournit un modèle Unraid, et l'image bascule sur les \`PUID\`/\`PGID\` que vous indiquez, pour que les dossiers montés appartiennent au bon utilisateur plutôt qu'à root.

### Ai-je besoin d'un domaine et de HTTPS ?

Pas sur votre réseau local. Dès que le serveur est joignable de l'extérieur, placez un reverse proxy devant et terminez-y le TLS : Caddy, nginx ou Traefik conviennent.

### Et si mon serveur est éteint quand j'arrête de jouer ?

L'instantané est pris localement, rien n'est perdu. Il s'envoie tout seul dès que le serveur répond à nouveau.

### Puis-je commencer sur Hoard Cloud et migrer plus tard ?

Oui, dans les deux sens. Vous pouvez tout exporter depuis la page de votre compte, et l'application peut pointer vers un autre serveur sans réinstallation.
`,Hn=`---
title: "Come self-hostare Hoard con Docker"
description: "Avvia il tuo server Hoard in pochi minuti con Docker Compose. Open source, gratuito, sul tuo hardware: un cloud completamente self-hosted per i salvataggi dei giochi, senza account né limiti di spazio."
order: 0
featured: true
updated: 2026-09-03
---

Hoard è open source e self-hostabile. Invece di usare Hoard Cloud, puoi eseguire lo stesso \`hoard-server\` sulla tua macchina e puntarci ogni dispositivo — senza account e senza limiti di spazio oltre al disco che gli dai. Questa guida mette in piedi un server con Docker in pochi minuti.

## Perché self-hostare Hoard

- **Controllo totale.** I tuoi salvataggi vivono su hardware che controlli tu, non sul cloud altrui.
- **Nessun limite.** Lo spazio è limitato solo dal tuo disco.
- **Stessa app, stesse funzioni.** Cronologia versionata e sync in background funzionano come con Hoard Cloud — cambia solo il backend.
- **Open source.** Puoi leggere, verificare e modificare il server.

È la differenza chiave rispetto a strumenti come [Ludusavi](/guides/ludusavi-alternative): Ludusavi è ottimo per i backup locali e per il cloud «porta il tuo» tramite Rclone, ma la sincronizzazione la configuri tu. Hoard ti dà un server di sync gestito che avvii una volta e a cui si collega ogni dispositivo.

## Cosa significa il self-hosting per i tuoi dati

Vale la pena dirlo chiaramente, perché è il punto su cui quasi tutti i confronti sbagliano riguardo a Hoard.

**Hoard Cloud** è l'opzione gestita: accedi e i tuoi salvataggi stanno sui nostri server, nell'UE.

**Un Hoard self-hosted è interamente tuo.** I tuoi dispositivi parlano con il tuo server e con nient'altro. **Nessun account con noi, nessuna telemetria verso di noi, nessuna quota e nessun relay**: non passa nulla dai nostri server, perché sul percorso non c'è niente di nostro. Non possiamo vedere un salvataggio, il nome di un gioco o un indirizzo email, per il semplice motivo che niente di tutto ciò ci arriva. Se Hoard Cloud chiudesse domani, la tua installazione andrebbe avanti identica.

Una precisazione, per essere esatti: il tuo server ha eccome i suoi accessi — l'utente che crei più sotto e un token per dispositivo. Sono tuoi, sulla tua macchina, nel tuo database. Quello che non esiste è un account con noi.

## Cosa ti serve

- Una macchina sempre accesa (un server domestico, un NAS che esegue Docker o un piccolo VPS).
- Docker e Docker Compose installati.
- Facoltativamente un dominio e un reverse proxy per l'HTTPS (consigliato per tutto ciò che esce dalla rete locale).

## Installazione con Docker Compose

Clona il repository, crea una configurazione dall'esempio e avvia lo stack:

\`\`\`sh
git clone https://github.com/rleeon/hoard.git && cd hoard
mkdir -p deploy/docker/config
cp deploy/config.toml.example deploy/docker/config/config.toml
$EDITOR deploy/docker/config/config.toml      # Use nano or vim or something lol

cd deploy/docker
docker compose up -d
docker compose logs -f                         # wait for "listening"
\`\`\`

Attendi che i log mostrino che il server è in ascolto. I dati vivono in un volume Docker (\`hoard-data\`): eseguine il backup come per qualsiasi volume. Il container ascolta internamente sulla porta \`12421\`; usa un'altra porta host con \`HOARD_PORT=9000 docker compose up -d\`.

## Crea il tuo utente e un token dispositivo

Il server non ha una schermata di registrazione: gli utenti si creano da riga di comando:

\`\`\`sh
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    user create alice --admin --password 'CHANGE_ME'
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    token create alice --device 'desktop'
\`\`\`

Il token viene mostrato una sola volta e **non può essere recuperato in seguito**, quindi copialo ora.

## Collega l'app desktop

Installa l'[app desktop di Hoard](/download) su ogni macchina. Nella procedura iniziale scegli **Self-Host**, poi incolla l'URL del server e il token appena creato. Da lì si comporta esattamente come Hoard Cloud: rileva i giochi, salva automaticamente e mantiene la cronologia versionata. Vedi [sincronizzare i salvataggi tra più PC](/guides/sync-game-saves-across-pcs) per l'uso quotidiano.

## Tieni aggiornato il tuo server

Come si aggiorna dipende da come l'hai installato, e sbagliare comando non dà errore: semplicemente non fa nulla. Vale la pena sapere qual è il tuo caso.

**Docker Compose.** Scarica la nuova immagine e ricrea il container. Entrambe le metà, in quest'ordine:

\`\`\`sh
docker compose pull
docker compose up -d
\`\`\`

Se ti fermi alla prima, il vecchio container continua a girare intatto: \`/v1/health\` riporta ancora la versione precedente e l'aggiornamento sembra fallito in silenzio. \`git pull\` non aggiorna né l'uno né l'altro: quello che gira è l'immagine pubblicata, non la tua copia del repository. Fissa una versione (\`ghcr.io/rleeon/hoard:1.1\`) al posto di \`:latest\` se preferisci scegliere tu quando ne arriva una nuova.

**Unraid.** Scheda *Docker* → Hoard → *Apply update* quando compare. Niente da digitare.

**Bare metal (systemd).** \`sudo hoard-server upgrade\`, poi \`sudo systemctl restart hoard-server\`. Sostituisce il binario in modo atomico e di proposito non riavvia il servizio da solo, per non troncare una sincronizzazione in corso.

\`hoard-server upgrade\` vale solo per l'installazione bare metal. Dentro un container si rifiuta di proposito — la sostituzione del binario non sopravvivrebbe al prossimo \`docker compose up -d\` — e stampa i due comandi qui sopra; esegui \`docker compose exec server hoard-server upgrade\` se vuoi sentirglielo dire. Le migrazioni del database le applica il server all'avvio, quindi non c'è mai un passaggio separato.

## In produzione

Per tutto ciò che è esposto oltre la rete locale, termina il TLS su un reverse proxy (Caddy, nginx o Traefik). Preferisci il bare metal? Il repository include anche uno script di installazione \`systemd\` e un comando \`hoard-server upgrade\` che sostituisce il binario in modo atomico senza interrompere una sync in corso.

## Self-host o Hoard Cloud?

Il self-hosting è ideale se hai già un server e vuoi controllo totale senza limiti. Se preferisci non gestire infrastruttura, [Hoard Cloud](/pricing) ti dà la stessa sincronizzazione gestita da noi, con un piano gratuito per iniziare. In ogni caso app e salvataggi restano portabili: puoi cambiare in seguito.

<!-- faq -->

## Domande frequenti

### Un Hoard self-hosted comunica con voi?

No. L'app desktop parla con l'indirizzo del server che le indichi tu. I tuoi salvataggi, i tuoi utenti e i tuoi log restano sulla tua macchina, e niente di tutto ciò ci arriva.

### Il server self-hosted è lo stesso codice di Hoard Cloud?

Sì, lo stesso binario \`hoard-server\`, sotto AGPL-3.0. Non c'è una community edition ridotta né funzioni tenute da parte per la versione ospitata.

### Dove finiscono davvero i salvataggi?

Per impostazione predefinita nel volume Docker che assegni al container, sul tuo disco. Se hai già uno storage a oggetti, il server parla anche S3: MinIO, Garage o Backblaze B2 vanno bene come archivio. In ogni caso i tuoi dispositivi parlano soltanto con il tuo server.

### Posso farlo girare su un NAS?

Sì, su qualsiasi NAS che esegua Docker. Il repository include un template per Unraid, e l'immagine scende ai \`PUID\`/\`PGID\` che indichi, così le cartelle montate risultano dell'utente giusto e non di root.

### Servono un dominio e HTTPS?

Sulla tua rete locale no. Non appena il server è raggiungibile dall'esterno, mettici davanti un reverse proxy e termina lì il TLS: vanno bene Caddy, nginx o Traefik.

### E se il server è spento quando smetto di giocare?

Lo snapshot viene preso in locale, quindi non si perde nulla. Sale da solo appena il server torna a rispondere.

### Posso iniziare con Hoard Cloud e spostarmi dopo?

Sì, in entrambe le direzioni. Puoi esportare tutto dalla pagina del tuo account, e l'app può essere puntata su un altro server senza reinstallare niente.
`,Cn=`---
title: "DockerでHoardをセルフホストする方法"
description: "Docker Compose を使って数分で自分専用の Hoard サーバーを構築。オープンソースで無料、自分のハードウェア上に完全セルフホストのセーブデータ用クラウドを。アカウントも容量制限も不要。"
order: 0
featured: true
updated: 2026-09-03
---

Hoard はオープンソースでセルフホスト可能です。Hoard Cloud を使う代わりに、同じ \`hoard-server\` を自分のマシンで動かし、すべての端末をそこへ接続できます。アカウントは不要で、容量制限は与えたディスク容量だけです。このガイドでは Docker を使って数分でサーバーを立ち上げます。

## なぜ Hoard をセルフホストするのか

- **完全な所有権。** セーブデータは他人のクラウドではなく、自分が管理するハードウェアに保存されます。
- **容量制限なし。** 容量は自分のディスクだけが上限です。
- **同じアプリ、同じ機能。** バージョン履歴とバックグラウンド同期は Hoard Cloud とまったく同じように動作し、変わるのはバックエンドだけです。
- **オープンソース。** サーバーを読み、監査し、改変できます。

これが [Ludusavi](/guides/ludusavi-alternative) のようなツールとの決定的な違いです。Ludusavi はローカルバックアップや Rclone 経由の「自分のクラウドを持ち込む」方式に優れていますが、同期は自分で組む必要があります。Hoard は一度立ち上げればすべての端末が接続できる、管理された同期サーバーを提供します。

## セルフホストがデータにとって何を意味するか

多くの比較が Hoard について誤解している点なので、はっきり書きます。

**Hoard Cloud** はマネージドな選択肢です。サインインすると、セーブは EU にある当方のサーバーに置かれます。

**セルフホストした Hoard は完全にあなたのものです。** あなたの端末は自分のサーバーとだけ通信し、他のどこにも接続しません。**当方のアカウントも、当方へのテレメトリも、容量制限も、中継もありません。** 経路上に当方のものが何一つないため、当方のサーバーを何も通りません。セーブもゲーム名もメールアドレスも見えません。そもそも届かないからです。仮に明日 Hoard Cloud が終了しても、あなたの構成はそのまま動き続けます。

正確を期して 1 点だけ。あなたのサーバーには確かにログインがあります。下で作成するユーザーと、端末ごとのトークンです。それらはあなたのもので、あなたのマシンの、あなたのデータベースの中にあります。存在しないのは「当方のアカウント」です。

## 必要なもの

- 常時稼働するマシン（自宅サーバー、Docker が動く NAS、または小さな VPS）。
- Docker と Docker Compose がインストール済みであること。
- 任意で、HTTPS 用のドメインとリバースプロキシ（LAN を越える用途では推奨）。

## Docker Compose でインストール

リポジトリをクローンし、サンプルから設定を作成して、スタックを起動します。

\`\`\`sh
git clone https://github.com/rleeon/hoard.git && cd hoard
mkdir -p deploy/docker/config
cp deploy/config.toml.example deploy/docker/config/config.toml
$EDITOR deploy/docker/config/config.toml      # Use nano or vim or something lol

cd deploy/docker
docker compose up -d
docker compose logs -f                         # wait for "listening"
\`\`\`

サーバーが待ち受け状態になったとログに表示されるまで待ちます。データは名前付き Docker ボリューム（\`hoard-data\`）に保存されるので、他のボリュームと同様にバックアップしてください。コンテナは内部でポート \`12421\` を待ち受けます。別のホストポートを使うには \`HOARD_PORT=9000 docker compose up -d\` とします。

## ユーザーと端末トークンを作成

サーバーにサインアップ画面はありません。ユーザーはコマンドラインで作成します。

\`\`\`sh
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    user create alice --admin --password 'CHANGE_ME'
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    token create alice --device 'desktop'
\`\`\`

トークンは一度だけ表示され、**後から取得することはできません**。今すぐコピーしてください。

## デスクトップアプリを接続

各マシンに [Hoard デスクトップアプリ](/download) をインストールします。オンボーディングで **セルフホスト** を選び、サーバーの URL と作成したトークンを貼り付けます。あとは Hoard Cloud とまったく同じで、ゲームを検出し、自動でバックアップし、バージョン履歴を保持します。日常的な使い方は [複数の PC 間でセーブを同期する](/guides/sync-game-saves-across-pcs) を参照してください。

## サーバーを最新に保つ

更新の方法はインストールの仕方によって変わります。しかも間違ったコマンドはエラーにならず、ただ何も起きないだけなので、自分がどれに当てはまるかを知っておく価値があります。

**Docker Compose.** 新しいイメージを取得し、コンテナを作り直します。次の順番で、両方とも実行してください。

\`\`\`sh
docker compose pull
docker compose up -d
\`\`\`

最初のコマンドで止めると、古いコンテナがそのまま動き続けます。\`/v1/health\` は古いバージョンを返し続け、更新が黙って失敗したように見えます。\`git pull\` はどちらも更新しません。動いているのは公開イメージであって、あなたのチェックアウトではないからです。新しいイメージが来るタイミングを自分で決めたい場合は、\`:latest\` の代わりにバージョンを固定してください（\`ghcr.io/rleeon/hoard:1.1\`）。

**Unraid.** *Docker* タブ → Hoard → 更新が出たら *Apply update*。入力するものはありません。

**ベアメタル（systemd）.** \`sudo hoard-server upgrade\` を実行し、続けて \`sudo systemctl restart hoard-server\`。バイナリをアトミックに入れ替えますが、進行中の同期を切らないよう、サービスの再起動は意図的に行いません。

\`hoard-server upgrade\` はベアメタルのインストール専用です。コンテナ内では意図的に実行を拒否し（入れ替えたバイナリは次の \`docker compose up -d\` で消えてしまうため）、代わりに上の 2 つのコマンドを表示します。実際に確かめたい場合は \`docker compose exec server hoard-server upgrade\` を実行してください。データベースのマイグレーションは起動時にサーバーが適用するので、そのための別の手順はありません。

## 本番運用

ローカルネットワークを越えて公開する場合は、リバースプロキシ（Caddy、nginx、Traefik）で TLS を終端します。ベアメタルがよい場合は、リポジトリに \`systemd\` インストールスクリプトと、進行中の同期を止めずにバイナリをアトミックに入れ替える \`hoard-server upgrade\` コマンドも含まれています。

## セルフホストと Hoard Cloud のどちら？

すでにサーバーを運用していて容量制限なしの完全な管理を望むなら、セルフホストが最適です。インフラの保守をしたくない場合は、[Hoard Cloud](/pricing) が同じ同期をこちらで管理して提供し、無料プランから始められます。どちらでもアプリとセーブデータは可搬性を保つので、後から切り替えられます。

<!-- faq -->

## よくある質問

### セルフホストした Hoard は外部に通信しますか？

いいえ。デスクトップアプリは、あなたが指定したサーバーのアドレスとだけ通信します。セーブもユーザーもログもあなたのマシンにとどまり、そのいずれも当方には届きません。

### セルフホストのサーバーは Hoard Cloud と同じコードですか？

はい。AGPL-3.0 の同じ \`hoard-server\` バイナリです。機能を削ったコミュニティ版もなければ、ホスト版だけの機能もありません。

### セーブは実際どこに保存されますか？

既定では、コンテナに与えた Docker ボリューム、つまりあなた自身のディスクです。すでにオブジェクトストレージを運用しているなら、サーバーは S3 も話せるので、MinIO、Garage、Backblaze B2 を保存先にできます。いずれの場合も、端末が通信する相手はあなたのサーバーだけです。

### NAS で動かせますか？

はい、Docker が動く NAS なら動きます。リポジトリには Unraid 用のテンプレートが同梱されており、イメージは指定した \`PUID\`/\`PGID\` に降格するので、バインドマウントしたフォルダーの所有者が root ではなく適切なユーザーになります。

### ドメインと HTTPS は必要ですか？

自宅の LAN 内だけなら不要です。サーバーが外部から到達可能になった時点で、前段にリバースプロキシを置いて TLS を終端してください。Caddy、nginx、Traefik のいずれでも構いません。

### プレイ終了時にサーバーが落ちていたら？

スナップショットはローカルで作られるので、失われるものはありません。サーバーが応答を再開すると自動でアップロードされます。

### Hoard Cloud で始めて、後から移れますか？

はい、双方向に移れます。アカウントページからすべてをエクスポートでき、アプリは再インストールなしで別のサーバーを指すように変更できます。
`,Pn=`---
title: "Como auto-hospedar o Hoard com Docker (self-hosted)"
description: "Coloque seu próprio servidor Hoard no ar em minutos com o Docker Compose. Código aberto, gratuito e no seu hardware: uma nuvem totalmente self-hosted para seus saves de jogos, sem conta nem limite de espaço."
order: 0
featured: true
updated: 2026-09-03
---

O Hoard é de código aberto e pode ser auto-hospedado. Em vez de usar o Hoard Cloud, você pode rodar o mesmo \`hoard-server\` na sua própria máquina e apontar todos os dispositivos para ele — sem conta e sem limite de espaço além do disco que você der a ele. Este guia coloca um servidor no ar com Docker em poucos minutos.

## Por que auto-hospedar o Hoard

- **Controle total.** Seus saves ficam em hardware que você controla, não na nuvem de outra pessoa.
- **Sem cota.** O espaço é limitado apenas pelo seu próprio disco.
- **Mesmo app, mesmos recursos.** Histórico versionado e sincronização em segundo plano funcionam igual ao Hoard Cloud — só muda o backend.
- **Código aberto.** Você pode ler, auditar e modificar o servidor.

Essa é a diferença principal em relação a ferramentas como o [Ludusavi](/guides/ludusavi-alternative): o Ludusavi é ótimo para backups locais e para usar sua própria nuvem via Rclone, mas a sincronização você mesmo monta. O Hoard oferece um servidor de sincronização gerenciado que você sobe uma vez e ao qual cada dispositivo se conecta.

## O que o self-hosting significa para os teus dados

Vale a pena dizê-lo sem rodeios, porque é o ponto em que quase todas as comparações se enganam sobre o Hoard.

**O Hoard Cloud** é a opção gerida: inicias sessão e os teus saves ficam nos nossos servidores, na UE.

**Um Hoard self-hosted é inteiramente teu.** Os teus dispositivos falam com o teu servidor e com mais nada. **Não há conta connosco, nem telemetria para nós, nem quota, nem retransmissão**: não passa nada pelos nossos servidores, porque não há nada nosso no caminho. Não conseguimos ver um save, o nome de um jogo ou um endereço de email, pela simples razão de que nada disso nos chega. Se o Hoard Cloud fechasse amanhã, a tua instalação continuaria igual.

E, para ser exato numa coisa: o teu servidor tem sim os seus próprios acessos — o utilizador que crias mais abaixo e um token por dispositivo. São teus, na tua máquina, na tua base de dados. O que não existe é uma conta connosco.

## O que você precisa

- Uma máquina que fique ligada (um servidor doméstico, um NAS que rode Docker ou um VPS pequeno).
- Docker e Docker Compose instalados.
- Opcionalmente um domínio e um proxy reverso para HTTPS (recomendado para qualquer coisa fora da sua rede local).

## Instalação com Docker Compose

Clone o repositório, crie uma configuração a partir do exemplo e suba o stack:

\`\`\`sh
git clone https://github.com/rleeon/hoard.git && cd hoard
mkdir -p deploy/docker/config
cp deploy/config.toml.example deploy/docker/config/config.toml
$EDITOR deploy/docker/config/config.toml      # Use nano or vim or something lol

cd deploy/docker
docker compose up -d
docker compose logs -f                         # wait for "listening"
\`\`\`

Aguarde até os logs mostrarem que o servidor está escutando. Os dados ficam em um volume nomeado do Docker (\`hoard-data\`) — faça backup como em qualquer outro volume. O contêiner escuta internamente na porta \`12421\`; use outra porta do host com \`HOARD_PORT=9000 docker compose up -d\`.

## Crie seu usuário e um token de dispositivo

O servidor não tem tela de cadastro — os usuários são criados pela linha de comando:

\`\`\`sh
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    user create alice --admin --password 'CHANGE_ME'
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    token create alice --device 'desktop'
\`\`\`

O token é exibido uma única vez e **não pode ser recuperado depois**, então copie-o agora.

## Conecte o app de desktop

Instale o [app de desktop do Hoard](/download) em cada máquina. No fluxo inicial, escolha **Self-Host** e cole a URL do seu servidor e o token recém-criado. A partir daí ele se comporta exatamente como o Hoard Cloud: detecta seus jogos, faz backup dos saves automaticamente e mantém o histórico versionado. Veja [sincronizar saves entre vários PCs](/guides/sync-game-saves-across-pcs) para o uso no dia a dia.

## Mantenha seu servidor atualizado

Como atualizar depende de como você instalou, e errar o comando não dá erro: simplesmente não faz nada. Vale saber qual é o seu caso.

**Docker Compose.** Baixe a imagem nova e recrie o contêiner. As duas metades, nesta ordem:

\`\`\`sh
docker compose pull
docker compose up -d
\`\`\`

Se parar na primeira, o contêiner antigo continua rodando intacto: \`/v1/health\` segue informando a versão antiga e a atualização parece ter falhado em silêncio. \`git pull\` não atualiza nenhum dos dois — o que roda é a imagem publicada, não o seu clone do repositório. Fixe uma versão (\`ghcr.io/rleeon/hoard:1.1\`) no lugar de \`:latest\` se preferir escolher quando uma nova chega.

**Unraid.** Aba *Docker* → Hoard → *Apply update* quando aparecer. Nada para digitar.

**Bare metal (systemd).** \`sudo hoard-server upgrade\` e depois \`sudo systemctl restart hoard-server\`. Ele troca o binário de forma atômica e de propósito não reinicia o serviço sozinho, para não cortar uma sincronização em andamento.

\`hoard-server upgrade\` é só para a instalação bare metal. Dentro de um contêiner ele se recusa de propósito — a troca de binário não sobreviveria ao próximo \`docker compose up -d\` — e imprime os dois comandos acima; rode \`docker compose exec server hoard-server upgrade\` se quiser vê-lo dizer isso. As migrações do banco de dados são aplicadas pelo servidor ao iniciar, então nunca há um passo separado para elas.

## Em produção

Para qualquer coisa exposta além da rede local, termine o TLS em um proxy reverso (Caddy, nginx ou Traefik). Prefere bare metal? O repositório também traz um script de instalação \`systemd\` e um comando \`hoard-server upgrade\` que troca o binário de forma atômica sem matar uma sincronização em andamento.

## Self-hosted ou Hoard Cloud?

Auto-hospedar é ideal se você já tem um servidor e quer controle total sem cota. Se preferir não manter infraestrutura, o [Hoard Cloud](/pricing) oferece a mesma sincronização gerenciada por nós, com um plano gratuito para começar. De qualquer forma, o app e seus saves continuam portáteis — você pode trocar depois.

<!-- faq -->

## Perguntas frequentes

### Um Hoard self-hosted comunica convosco?

Não. A aplicação de ambiente de trabalho fala com o endereço de servidor que lhe deres. Os teus saves, os teus utilizadores e os teus registos ficam na tua máquina, e nada disso nos chega.

### O servidor self-hosted é o mesmo código do Hoard Cloud?

Sim, o mesmo binário \`hoard-server\`, sob AGPL-3.0. Não há uma edição comunitária reduzida nem funcionalidades guardadas para a versão alojada.

### Onde ficam realmente guardados os saves?

Por omissão, no volume Docker que deres ao contentor, no teu próprio disco. Se já tens armazenamento de objetos, o servidor também fala S3, por isso MinIO, Garage ou Backblaze B2 servem de repositório. Em qualquer dos casos, os teus dispositivos só falam com o teu servidor.

### Posso pô-lo a correr num NAS?

Sim, em qualquer NAS que corra Docker. O repositório inclui um template de Unraid, e a imagem desce para os \`PUID\`/\`PGID\` que indicares, para que as pastas montadas fiquem do utilizador certo em vez de root.

### Preciso de domínio e HTTPS?

Na tua própria rede local, não. A partir do momento em que o servidor é acessível de fora, põe um proxy inverso à frente e termina aí o TLS: Caddy, nginx ou Traefik servem.

### E se o meu servidor estiver em baixo quando acabo de jogar?

O snapshot é tirado localmente, por isso não se perde nada. Sobe sozinho assim que o servidor voltar a responder.

### Posso começar no Hoard Cloud e mudar mais tarde?

Sim, nos dois sentidos. Podes exportar tudo a partir da página da tua conta, e a aplicação pode ser apontada a outro servidor sem reinstalar nada.
`,Dn=`---
title: "如何用 Docker 自托管 Hoard"
description: "用 Docker Compose 几分钟搭建你自己的 Hoard 服务器。开源、免费、运行在你自己的硬件上——一个完全自托管的游戏存档云，无需账号、没有容量限制。"
order: 0
featured: true
updated: 2026-09-03
---

Hoard 是开源且可自托管的。你可以不使用 Hoard Cloud，而是在自己的机器上运行同一个 \`hoard-server\`，让每台设备都连接到它——无需账号，容量只受你分配的磁盘大小限制。本指南用 Docker 在几分钟内把服务器跑起来。

## 为什么自托管 Hoard

- **完全掌控。** 你的存档保存在你自己掌控的硬件上，而不是别人的云端。
- **没有容量限制。** 空间仅受你自己的磁盘限制。
- **同一个应用，同样的功能。** 版本历史和后台同步与 Hoard Cloud 完全一致，改变的只有后端。
- **开源。** 你可以阅读、审计并修改服务器代码。

这正是它与 [Ludusavi](/guides/ludusavi-alternative) 这类工具的关键区别：Ludusavi 在本地备份和通过 Rclone「自带云」方面很出色，但同步需要你自己搭建。Hoard 则提供一个托管式的同步服务器，启动一次后每台设备都能连接。

## 自托管对你的数据意味着什么

这一点值得直说，因为多数对比在 Hoard 上正是弄错了这里。

**Hoard Cloud** 是托管方案：你登录，存档存放在我们位于欧盟的服务器上。

**自托管的 Hoard 完全属于你。** 你的设备只与你自己的服务器通信，不与任何其他地方通信。**没有我们这边的账号，没有发往我们的遥测，没有配额，也没有中转**——不经过我们的任何服务器，因为这条路径上根本没有我们的东西。我们看不到任何存档、游戏名或邮箱地址，原因很简单：这些从未到达我们这里。就算 Hoard Cloud 明天关停，你的部署照常运行。

有一点需要说准确：你的服务器确实有它自己的登录——下面你要创建的用户，以及每台设备一个令牌。它们是你的，在你的机器上、你的数据库里。不存在的是"我们这边的账号"。

## 你需要准备

- 一台保持开机的机器（家庭服务器、运行 Docker 的 NAS，或一台小型 VPS）。
- 已安装 Docker 和 Docker Compose。
- 可选：一个域名和用于 HTTPS 的反向代理（超出本地局域网的场景推荐）。

## 用 Docker Compose 安装

克隆仓库，从示例创建配置，然后启动整套服务：

\`\`\`sh
git clone https://github.com/rleeon/hoard.git && cd hoard
mkdir -p deploy/docker/config
cp deploy/config.toml.example deploy/docker/config/config.toml
$EDITOR deploy/docker/config/config.toml      # Use nano or vim or something lol

cd deploy/docker
docker compose up -d
docker compose logs -f                         # wait for "listening"
\`\`\`

等待日志显示服务器正在监听。数据保存在一个命名的 Docker 卷（\`hoard-data\`）中——像备份其他卷一样备份它。容器内部监听 \`12421\` 端口；用 \`HOARD_PORT=9000 docker compose up -d\` 可映射到其他主机端口。

## 创建用户和设备令牌

服务器没有注册页面——用户通过命令行创建：

\`\`\`sh
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    user create alice --admin --password 'CHANGE_ME'
docker compose exec server hoard-admin --config /etc/hoard/config.toml \\
    token create alice --device 'desktop'
\`\`\`

令牌只显示一次，**之后无法找回**，请立即复制。

## 连接桌面应用

在每台机器上安装 [Hoard 桌面应用](/download)。在初始引导中选择 **自托管**，然后粘贴你的服务器 URL 和刚创建的令牌。之后它的行为与 Hoard Cloud 完全相同：检测你的游戏、自动备份存档、保留版本历史。日常用法请参见[在多台 PC 之间同步存档](/guides/sync-game-saves-across-pcs)。

## 保持服务器更新

怎么更新取决于你是怎么安装的，而且用错命令不会报错，只是什么都不做 —— 所以值得先弄清楚哪一种是你的情况。

**Docker Compose.** 拉取新镜像并重建容器。两条都要执行，按顺序：

\`\`\`sh
docker compose pull
docker compose up -d
\`\`\`

只执行第一条的话，旧容器会原封不动地继续运行：\`/v1/health\` 仍然报告旧版本，看起来就像更新悄悄失败了。\`git pull\` 两者都更新不了 —— 运行的是已发布的镜像，不是你的代码副本。如果你想自己决定什么时候用上新版本，把 \`:latest\` 换成固定版本（\`ghcr.io/rleeon/hoard:1.1\`）。

**Unraid.** *Docker* 标签页 → Hoard → 出现更新时点 *Apply update*。不需要输入任何命令。

**裸机（systemd）.** 先 \`sudo hoard-server upgrade\`，再 \`sudo systemctl restart hoard-server\`。它会原子地替换二进制文件，并且故意不自己重启服务，以免中断正在进行的同步。

\`hoard-server upgrade\` 只适用于裸机安装。在容器里它会故意拒绝执行 —— 替换后的二进制文件撑不过下一次 \`docker compose up -d\` —— 并改为打印上面那两条命令；想亲眼看看的话，执行 \`docker compose exec server hoard-server upgrade\`。数据库迁移由服务器在启动时应用，所以永远不需要单独的步骤。

## 在生产环境中运行

对于任何暴露到本地网络之外的部署，请在反向代理（Caddy、nginx 或 Traefik）上终止 TLS。更喜欢裸机部署？仓库还提供了 \`systemd\` 安装脚本，以及一个 \`hoard-server upgrade\` 命令，它会原子地替换二进制文件而不会中断正在进行的同步。

## 自托管还是 Hoard Cloud？

如果你已经在运行服务器并希望完全掌控、没有容量限制，自托管是理想选择。如果你不想维护基础设施，[Hoard Cloud](/pricing) 提供由我们托管的同样同步功能，并有免费档可供起步。无论哪种方式，应用和你的存档都保持可迁移——以后可以随时切换。

<!-- faq -->

## 常见问题

### 自托管的 Hoard 会回连你们吗？

不会。桌面应用只与你给它的服务器地址通信。你的存档、你的用户和你的日志都留在你的机器上，其中没有任何内容会到达我们这里。

### 自托管服务器和 Hoard Cloud 是同一份代码吗？

是的，同一个 \`hoard-server\` 二进制，采用 AGPL-3.0。没有功能删减的社区版，也没有只留给托管版的功能。

### 存档实际保存在哪里？

默认在你分配给容器的 Docker 卷里，也就是你自己的磁盘上。如果你已经在跑对象存储，服务器同样支持 S3，MinIO、Garage 或 Backblaze B2 都可以作为后端。无论哪种方式，你的设备始终只与你的服务器通信。

### 可以跑在 NAS 上吗？

可以，任何能运行 Docker 的 NAS 都行。仓库里附带了 Unraid 模板，镜像会降权到你指定的 \`PUID\`/\`PGID\`，这样绑定挂载的文件夹归属正确的用户，而不是 root。

### 需要域名和 HTTPS 吗？

在自家局域网里不需要。一旦服务器可以从外部访问，就在前面放一个反向代理并在那里终止 TLS——Caddy、nginx 或 Traefik 都可以。

### 如果我玩完时服务器正好没开呢？

快照是在本地生成的，不会丢失任何东西。等服务器重新响应，它会自行上传。

### 可以先用 Hoard Cloud，以后再迁移吗？

可以，双向都行。你能在账号页面导出全部数据，应用也可以指向另一台服务器，无需重装。
`,Ln=`---
title: "Steam-Cloud-Alternative: sichere die Spielstände, die Steam nicht sichert"
description: "Steam Cloud deckt nur Steam-Spiele ab, deren Entwickler sie aktiviert hat, und führt keine Versionshistorie. Hoard sichert jedes Spiel, das du spielst, aus jedem Store, mit einer Historie zum Zurückrollen — in der Cloud oder auf deinem eigenen Server."
order: 7
updated: 2026-09-01
---

Steam Cloud macht die eng umrissene Aufgabe, die sie hat, wirklich gut, und die meisten stoßen erst an dem Tag an ihre Grenzen, an dem etwas verloren geht. Diese Anleitung zeigt, wo diese Grenzen liegen und was mit den Spielen zu tun ist, die dahinter liegen.

## Was Steam Cloud tatsächlich abdeckt

Steam Cloud synchronisiert den Ordner eines Spiels, wenn **der Entwickler es eingerichtet hat** — indem er angibt, welche Dateien zu synchronisieren sind, oder indem das Spiel die Steam-API aufruft. Das ist das ganze Modell, und daraus folgen drei Dinge:

- Es funktioniert nur für Spiele, die über Steam gekauft und gestartet werden.
- Ob es überhaupt funktioniert, entscheidet der Entwickler, pro Spiel und manchmal pro Plattform.
- Jedes Spiel hat sein eigenes Speicherkontingent, festgelegt von diesem Entwickler.

Wenn es funktioniert, ist es unsichtbar und hervorragend: Spiel auf dem einen PC schließen, auf dem anderen öffnen, Fortschritt ist da.

## Wo es dich im Regen stehen lässt

- **Alles, was kein Steam-Spiel ist.** GOG, Epic, itch, Battle.net, die Xbox-App, Emulatoren, alles von Hand Installierte. Steam weiß nicht, dass es existiert.
- **Steam-Spiele, bei denen es nie aktiviert wurde.** Viele Titel, gerade ältere oder kleinere, haben es schlicht nicht. Die Shop-Seite sagt es, aber niemand schaut nach, bevor er 60 Stunden investiert.
- **Es gibt kein Zurück.** Das ist der große Punkt. Steam hält den aktuellen Zustand deines Spielstands, nicht dessen Geschichte. Wird die Datei beschädigt, frisst ein Mod deine Welt, oder überschreibst du einen guten Stand mit einem schlechten, dann ist die Cloud-Kopie bereits der schlechte. Du kannst die Dateien ansehen, die Steam für ein Spiel hält, aber es gibt keine frühere Version zum Wiederherstellen.
- **Der Konfliktdialog.** Wenn Steam local und remote für uneinig hält, sollst du wählen — mit kaum mehr als zwei Zeitstempeln als Grundlage. Wählst du falsch, ist die andere Kopie weg.

## Was Hoard ergänzt

Hoard beobachtet den Ordner, in den ein Spiel wirklich schreibt, und sichert **nach jedem Spielen eine neue Version**:

- **Woher das Spiel stammt, ist egal.** Steam, GOG, Epic, itch, Emulatoren oder ein Ordner, auf den du es von Hand richtest.
- **Jede Version bleibt erhalten**, ein beschädigter Stand oder eine schlechte Entscheidung kosten also zwei Klicks statt eines Spieldurchgangs.
- **Es synchronisiert zwischen deinen Geräten**, Steam Deck und Desktop eingeschlossen.
- **Nichts wird stillschweigend zerstört.** Der ersetzte Stand wird zuerst gesichert, selbst eine falsche Wiederherstellung ist also umkehrbar.

Snapshots werden per Inhalts-Hash gespeichert, zehn Versionen eines 2 GB großen Stands kosten also etwa 2 GB, nicht 20 — und das macht die komplette Historie überhaupt praktikabel.

## Beides gleichzeitig nutzen

Sie stören sich nicht, du musst dich nicht entscheiden. Bei einem Steam-Spiel mit Cloud-Unterstützung lass Steam synchronisieren, was es ohnehin tut; Hoards Beitrag dort ist die Historie — genau das, was Steam nicht führt. Für alles andere übernimmt Hoard auch die Synchronisierung.

Ein Detail, das zählt, wenn du neben dem Desktop ein Steam Deck hast: Hoard verfolgt \`<AppID>/remote/\` innerhalb von \`userdata\`, nicht den Ordner darüber, denn der enthält \`remotecache.vdf\` und gerätebezogene Dateien für Erfolge und Spielzeit. Genau diese Unterscheidung geht bei selbstgebauter Synchronisierung meist schief, weshalb solche Setups bei jedem Start zu kollidieren scheinen.

## Wann Steam Cloud reicht

Ehrlich gesagt: wenn alle deine Spiele Steam-Spiele mit Cloud-Unterstützung sind, du an einem PC spielst und noch nie einen Spielstand zurücknehmen musstest, erledigt Steam Cloud die Aufgabe und du brauchst nichts weiter. Für Hoard sprechen die Versionshistorie, Spiele außerhalb von Steam und Geräte, die Steam Cloud nicht erreicht.

## Ganz ohne fremde Cloud

Wenn der Reiz darin liegt, von keiner Plattform abzuhängen: Hoard läuft komplett auf deiner eigenen Hardware — \`hoard-server\` auf einem PC oder NAS, und deine Stände gehen von deiner Maschine auf deine Platte. Es gibt **kein Konto bei uns, keine Telemetrie zu uns und kein Relay** — nichts läuft über unsere Server, weil nichts von uns im Weg steht. Siehe [wie du Hoard selbst hostest](/guides/self-host-hoard).

Dasselbe Programm, dieselbe Erkennung, dieselbe Versionshistorie. Es ändert sich nur, wem der Speicher gehört.

<!-- faq -->

## Häufige Fragen

### Ersetzt Hoard Steam Cloud?

Muss es nicht. Steam Cloud hält deinen aktuellen Stand für die unterstützten Spiele synchron; Hoard ergänzt die Versionshistorie und deckt die übrigen Spiele ab. Beides parallel zu nutzen ist der Normalfall.

### Kann Steam Cloud zu einem älteren Spielstand zurück?

Nein. Steam hält den aktuellen Zustand der Dateien, nicht deren Geschichte. Ist ein schlechter Stand einmal synchronisiert, steht genau der in der Cloud. Zurück geht es nur mit einem versionierenden Werkzeug.

### Warum synchronisieren nicht alle meine Steam-Spiele?

Weil der Entwickler es aktiviert, pro Spiel und manchmal pro Plattform. Die Shop-Seite führt Steam Cloud unter den Features auf, wenn es unterstützt wird — und viele Titel tun das schlicht nicht.

### Funktioniert Hoard mit Nicht-Steam-Spielen?

Ja, das ist ein Großteil des Sinns. Es findet Spielstände über eine Community-Datenbank mit über 20.000 Titeln, aus jedem Store, und für Ungewöhnliches richtest du es von Hand auf einen Ordner.

### Gibt es Konflikte, wenn beides läuft?

Nein. Hoard sichert eine Version, nachdem du aufgehört hast und der Ordner zur Ruhe kommt, und überschreibt nie, ohne das Ersetzte vorher zu sichern.

### Kann ich meine Stände aus beiden Clouds heraushalten?

Ja. Hoste den Server selbst, dann verlassen deine Spielstände nie deine eigene Hardware — ohne Konto und ohne Telemetrie an irgendwen.
`,xn=`---
title: "Steam Cloud alternative: back up the saves Steam doesn't"
description: "Steam Cloud only covers Steam games whose developer switched it on, and it keeps no version history. Hoard backs up every game you play, from any launcher, with a versioned history you can roll back — in the cloud or on your own server."
order: 7
updated: 2026-09-01
---

Steam Cloud is genuinely good at the narrow job it does, and most people only find its edges the day they lose something. This guide explains exactly where those edges are, and what to do about the games that fall outside them.

## What Steam Cloud actually covers

Steam Cloud syncs a folder for a game when **the developer set it up** — either by declaring which files to sync, or by calling the Steam API from inside the game. That's the whole model, and three things follow from it:

- It only works for games bought and launched through Steam.
- Whether it works at all is the developer's decision, per game, and sometimes per platform.
- Each game has its own storage allowance, set by that developer.

When it works, it's invisible and excellent: you close the game on one PC, open it on another, and your progress is there.

## Where it leaves you exposed

- **Everything that isn't a Steam game.** GOG, Epic, itch, Battle.net, the Xbox app, emulators, anything installed by hand. Steam doesn't know they exist.
- **Steam games where it was never switched on.** Plenty of titles, especially older or smaller ones, simply don't have it. The store page tells you, but nobody checks before starting a 60-hour run.
- **There is no going back.** This is the big one. Steam holds the current state of your save, not a history of it. Corrupt the file, let a mod eat your world, or overwrite a good save with a bad one, and the cloud copy is already the bad one. You can browse the files Steam is holding for a game, but there's no earlier version to restore.
- **The conflict dialog.** When Steam thinks the local and remote saves disagree, it asks you to choose, with little more than two timestamps to go on. Choose wrong and the other copy is gone.

## What Hoard adds

Hoard watches the folder each game actually writes to and captures a **new version every time you finish playing**:

- **It doesn't care where a game came from.** Steam, GOG, Epic, itch, emulators, a folder you pointed it at by hand.
- **Every version is kept**, so rolling back a corrupted save or a bad decision is two clicks rather than a lost run.
- **It syncs between your machines** the same way, including a Steam Deck and a desktop.
- **Nothing is destroyed silently.** The save being replaced is captured first, so even a wrong restore is reversible.

Snapshots are stored by content hash, so ten versions of a 2 GB save cost about 2 GB, not 20 — which is what makes keeping the whole history practical.

## Using both at once

They don't fight, and you don't have to pick. For a Steam game with cloud support, let Steam do the syncing it's already doing; Hoard's contribution there is the history — the thing Steam doesn't keep. For everything else, Hoard is doing the syncing too.

One detail that matters if you're on a Steam Deck as well as a desktop: Hoard tracks \`<AppID>/remote/\` inside \`userdata\`, not the folder above it, because the parent holds \`remotecache.vdf\` and per-machine achievement and playtime files. That's the distinction a hand-rolled sync usually gets wrong, and it's why those setups seem to conflict on every launch.

## When Steam Cloud is enough

Worth saying plainly: if every game you play is a Steam game with cloud support, you play on one PC, and you've never needed to undo a save, Steam Cloud already does the job and you don't need anything else. The case for adding Hoard is version history, games from outside Steam, and machines Steam Cloud doesn't reach.

## Without anyone's cloud

If the appeal is not depending on a platform at all, Hoard can be run entirely on your own hardware: \`hoard-server\` on a PC or a NAS, and your saves go from your machine to your disk. There is **no account with us, no telemetry to us and no relay** — nothing passes through our servers, because there is nothing of ours in the path. See [how to self-host Hoard](/guides/self-host-hoard).

Same program, same detection, same version history. The only thing that changes is who owns the storage.

<!-- faq -->

## Frequently asked questions

### Does Hoard replace Steam Cloud?

It doesn't have to. Steam Cloud keeps your current save in sync for the games that support it; Hoard adds a version history and covers the games it doesn't. Running both is normal.

### Can Steam Cloud roll back to an older save?

No. Steam holds the current state of the files, not a history of them. Once a bad save has synced, that's what's in the cloud. A versioned tool is the only way to go back.

### Why don't all my Steam games sync?

Because it's the developer who enables it, per game and sometimes per platform. A game's store page lists Steam Cloud among its features when it's supported — and plenty of titles simply don't.

### Does Hoard work with non-Steam games?

Yes, that's most of the point. It locates saves through a community database covering 20,000+ titles, from any launcher, and you can point it at a folder by hand for anything unusual.

### Will running both cause conflicts?

No. Hoard captures a version after you stop playing, once the folder goes quiet, and never overwrites without capturing what it replaces first.

### Can I keep my saves off both clouds?

Yes. Self-host the server and your saves never leave hardware you own, with no account and no telemetry going anywhere.
`,An=`---
title: "Alternativa a Steam Cloud: copia las partidas que Steam no guarda"
description: "Steam Cloud sólo cubre juegos de Steam cuyo desarrollador lo activó, y no guarda historial de versiones. Hoard copia todos los juegos a los que juegas, vengan de donde vengan, con un historial al que puedes volver, en la nube o en tu propio servidor."
order: 7
updated: 2026-09-01
---

Steam Cloud hace muy bien el trabajo concreto que hace, y la mayoría de la gente descubre sus límites justo el día que pierde algo. Esta guía explica dónde están esos límites y qué hacer con los juegos que se quedan fuera.

## Qué cubre realmente Steam Cloud

Steam Cloud sincroniza la carpeta de un juego cuando **el desarrollador lo configuró**, ya sea declarando qué ficheros sincronizar o llamando a la API de Steam desde dentro del juego. Ése es todo el modelo, y de ahí salen tres consecuencias:

- Sólo funciona con juegos comprados y lanzados desde Steam.
- Que funcione o no lo decide el desarrollador, juego por juego, y a veces por plataforma.
- Cada juego tiene su propio cupo de almacenamiento, fijado por ese desarrollador.

Cuando funciona es invisible y excelente: cierras el juego en un PC, lo abres en otro y tu progreso está ahí.

## Dónde te deja expuesto

- **Todo lo que no sea un juego de Steam.** GOG, Epic, itch, Battle.net, la app de Xbox, emuladores, cualquier cosa instalada a mano. Steam ni sabe que existen.
- **Juegos de Steam donde nunca se activó.** Bastantes títulos, sobre todo antiguos o pequeños, sencillamente no lo tienen. La ficha de la tienda lo dice, pero nadie lo mira antes de empezar una partida de 60 horas.
- **No hay marcha atrás.** Éste es el grande. Steam guarda el estado actual de tu partida, no su historial. Si el fichero se corrompe, si un mod se come tu mundo o si machacas una partida buena con una mala, la copia de la nube ya es la mala. Puedes ver los ficheros que Steam guarda de un juego, pero no hay una versión anterior a la que volver.
- **El diálogo de conflicto.** Cuando Steam cree que la partida local y la remota no cuadran, te pide que elijas con poco más que dos fechas delante. Si eliges mal, la otra copia desaparece.

## Qué añade Hoard

Hoard vigila la carpeta en la que escribe cada juego y captura una **versión nueva cada vez que terminas de jugar**:

- **Le da igual de dónde venga el juego.** Steam, GOG, Epic, itch, emuladores o una carpeta que le señales a mano.
- **Se conservan todas las versiones**, así que recuperarte de una partida corrupta o de una mala decisión son dos clics y no una partida perdida.
- **Sincroniza entre tus máquinas** igual, incluidas una Steam Deck y un sobremesa.
- **Nada se destruye en silencio.** La partida que se reemplaza se captura antes, así que hasta una restauración equivocada es reversible.

Las instantáneas se guardan por hash de contenido, así que diez versiones de una partida de 2 GB ocupan unos 2 GB y no 20, que es lo que hace práctico conservar el historial entero.

## Usar los dos a la vez

No se pelean, y no tienes que elegir. En un juego de Steam con soporte de nube, deja que Steam siga sincronizando lo que ya sincroniza; lo que aporta Hoard ahí es el historial, que es justo lo que Steam no guarda. Para todo lo demás, Hoard se encarga también de la sincronización.

Un detalle que importa si tienes Steam Deck además de sobremesa: Hoard rastrea \`<AppID>/remote/\` dentro de \`userdata\`, no la carpeta de encima, porque la padre guarda \`remotecache.vdf\` y ficheros de logros y tiempo jugado propios de cada máquina. Ésa es la distinción que suele fallar en una sincronización casera, y por eso esos montajes parecen entrar en conflicto en cada arranque.

## Cuándo basta con Steam Cloud

Conviene decirlo claro: si todos los juegos a los que juegas son de Steam y con soporte de nube, juegas en un solo PC y nunca has necesitado deshacer una partida, Steam Cloud ya hace el trabajo y no necesitas nada más. Lo que justifica añadir Hoard es el historial de versiones, los juegos de fuera de Steam y las máquinas a las que Steam Cloud no llega.

## Sin la nube de nadie

Si lo que te atrae es no depender de ninguna plataforma, Hoard se puede usar entero sobre tu propio hardware: \`hoard-server\` en un PC o en un NAS, y tus partidas van de tu máquina a tu disco. **No hay cuenta con nosotros, ni telemetría hacia nosotros, ni relé**: no pasa nada por nuestros servidores, porque no hay nada nuestro en el camino. Mira [cómo autoalojar Hoard](/guides/self-host-hoard).

El mismo programa, la misma detección, el mismo historial de versiones. Lo único que cambia es de quién es el almacenamiento.

<!-- faq -->

## Preguntas frecuentes

### ¿Hoard sustituye a Steam Cloud?

No tiene por qué. Steam Cloud mantiene sincronizada tu partida actual en los juegos que lo soportan; Hoard añade el historial de versiones y cubre los juegos que no. Usar los dos es lo normal.

### ¿Steam Cloud puede volver a una partida anterior?

No. Steam guarda el estado actual de los ficheros, no su historial. Una vez que una partida mala se ha sincronizado, eso es lo que hay en la nube. Para volver atrás hace falta una herramienta con versiones.

### ¿Por qué no se sincronizan todos mis juegos de Steam?

Porque quien lo activa es el desarrollador, juego por juego y a veces por plataforma. La ficha del juego en la tienda incluye Steam Cloud entre sus características cuando está soportado, y muchos títulos sencillamente no lo están.

### ¿Hoard funciona con juegos que no son de Steam?

Sí, y es buena parte del sentido que tiene. Localiza las partidas con una base de datos comunitaria que cubre más de 20.000 títulos, de cualquier tienda, y para lo raro puedes señalarle la carpeta a mano.

### ¿Usar los dos provoca conflictos?

No. Hoard captura una versión cuando dejas de jugar y la carpeta se queda quieta, y nunca sobrescribe sin capturar antes lo que reemplaza.

### ¿Puedo mantener mis partidas fuera de las dos nubes?

Sí. Autoaloja el servidor y tus partidas no salen nunca de hardware tuyo, sin cuenta y sin telemetría hacia ningún sitio.
`,jn=`---
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

Un détail qui compte si vous avez un Steam Deck en plus d'un fixe : Hoard suit \`<AppID>/remote/\` dans \`userdata\`, et non le dossier au-dessus, car le parent contient \`remotecache.vdf\` et des fichiers de succès et de temps de jeu propres à chaque machine. C'est la distinction qu'une synchro maison rate le plus souvent, et la raison pour laquelle ces montages semblent en conflit à chaque lancement.

## Quand Steam Cloud suffit

Disons-le franchement : si tous vos jeux sont des jeux Steam avec support cloud, que vous jouez sur un seul PC et que vous n'avez jamais eu besoin d'annuler une sauvegarde, Steam Cloud fait le travail et vous n'avez besoin de rien d'autre. Ce qui justifie d'ajouter Hoard, c'est l'historique de versions, les jeux hors Steam et les machines que Steam Cloud n'atteint pas.

## Sans le cloud de personne

Si l'attrait est de ne dépendre d'aucune plateforme, Hoard tourne entièrement sur votre matériel : \`hoard-server\` sur un PC ou un NAS, et vos sauvegardes vont de votre machine à votre disque. **Aucun compte chez nous, aucune télémétrie vers nous, aucun relais** : rien ne passe par nos serveurs, puisque rien de chez nous n'est sur le chemin. Voir [comment auto-héberger Hoard](/guides/self-host-hoard).

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
`,On=`---
title: "Alternativa a Steam Cloud: salva i salvataggi che Steam non copre"
description: "Steam Cloud copre solo i giochi Steam il cui sviluppatore l'ha attivato, e non tiene una cronologia. Hoard salva ogni gioco a cui giochi, da qualsiasi store, con una cronologia versionata a cui tornare — nel cloud o sul tuo server."
order: 7
updated: 2026-09-01
---

Steam Cloud fa molto bene il compito ristretto che ha, e quasi tutti ne scoprono i limiti proprio il giorno in cui perdono qualcosa. Questa guida spiega dove sono quei limiti e cosa fare con i giochi che restano fuori.

## Cosa copre davvero Steam Cloud

Steam Cloud sincronizza la cartella di un gioco quando **lo sviluppatore l'ha configurato**: dichiarando quali file sincronizzare, oppure chiamando l'API di Steam dall'interno del gioco. È tutto qui, e ne discendono tre cose:

- Funziona solo per giochi comprati e avviati tramite Steam.
- Che funzioni o no lo decide lo sviluppatore, gioco per gioco e a volte per piattaforma.
- Ogni gioco ha la sua quota di spazio, fissata da quello sviluppatore.

Quando funziona è invisibile ed eccellente: chiudi il gioco su un PC, lo apri su un altro, i progressi sono lì.

## Dove ti lascia scoperto

- **Tutto ciò che non è un gioco Steam.** GOG, Epic, itch, Battle.net, l'app Xbox, gli emulatori, qualsiasi cosa installata a mano. Steam non sa che esistono.
- **Giochi Steam dove non è mai stato attivato.** Parecchi titoli, soprattutto vecchi o piccoli, semplicemente non ce l'hanno. La pagina del negozio lo dice, ma nessuno la controlla prima di iniziare una partita da 60 ore.
- **Non si torna indietro.** Questo è il punto grosso. Steam conserva lo stato attuale del salvataggio, non la sua storia. Se il file si corrompe, se una mod ti mangia il mondo o se sovrascrivi un salvataggio buono con uno rotto, la copia nel cloud è già quella rotta. Puoi vedere i file che Steam tiene per un gioco, ma non c'è una versione precedente da ripristinare.
- **La finestra di conflitto.** Quando Steam ritiene che locale e remoto non coincidano, ti chiede di scegliere con poco più di due date davanti. Se sbagli, l'altra copia è persa.

## Cosa aggiunge Hoard

Hoard sorveglia la cartella in cui il gioco scrive davvero e cattura una **nuova versione ogni volta che smetti di giocare**:

- **Non gli importa da dove venga il gioco.** Steam, GOG, Epic, itch, emulatori o una cartella che gli indichi a mano.
- **Tutte le versioni vengono conservate**, quindi rimediare a un salvataggio corrotto o a una scelta sbagliata sono due clic e non una partita persa.
- **Sincronizza tra le tue macchine** allo stesso modo, Steam Deck e desktop inclusi.
- **Niente viene distrutto in silenzio.** Il salvataggio sostituito viene catturato prima, quindi anche un ripristino sbagliato è reversibile.

Gli snapshot sono archiviati per hash del contenuto, così dieci versioni di un salvataggio da 2 GB occupano circa 2 GB e non 20: è questo a rendere pratico conservare tutta la cronologia.

## Usarli insieme

Non litigano, e non devi scegliere. Per un gioco Steam con supporto cloud, lascia che Steam sincronizzi quello che già sincronizza; il contributo di Hoard lì è la cronologia, cioè proprio ciò che Steam non tiene. Per tutto il resto, alla sincronizzazione pensa Hoard.

Un dettaglio che conta se oltre al desktop hai una Steam Deck: Hoard traccia \`<AppID>/remote/\` dentro \`userdata\`, non la cartella superiore, perché quella contiene \`remotecache.vdf\` e file di obiettivi e tempo di gioco propri di ogni macchina. È la distinzione che una sincronizzazione artigianale sbaglia più spesso, ed è il motivo per cui quei setup sembrano andare in conflitto a ogni avvio.

## Quando Steam Cloud basta

Vale la pena dirlo chiaramente: se tutti i giochi a cui giochi sono giochi Steam con supporto cloud, giochi su un solo PC e non hai mai avuto bisogno di annullare un salvataggio, Steam Cloud fa già il suo e non ti serve altro. Ad aggiungere Hoard convincono la cronologia delle versioni, i giochi fuori da Steam e le macchine che Steam Cloud non raggiunge.

## Senza il cloud di nessuno

Se quello che ti attira è non dipendere da nessuna piattaforma, Hoard può girare interamente sul tuo hardware: \`hoard-server\` su un PC o su un NAS, e i salvataggi vanno dalla tua macchina al tuo disco. **Nessun account con noi, nessuna telemetria verso di noi e nessun relay**: non passa nulla dai nostri server, perché sul percorso non c'è niente di nostro. Vedi [come ospitare Hoard da solo](/guides/self-host-hoard).

Stesso programma, stesso rilevamento, stessa cronologia. L'unica cosa che cambia è di chi è lo spazio di archiviazione.

<!-- faq -->

## Domande frequenti

### Hoard sostituisce Steam Cloud?

Non deve per forza. Steam Cloud tiene sincronizzato il salvataggio attuale per i giochi supportati; Hoard aggiunge la cronologia e copre i giochi che non lo sono. Tenerli entrambi è normale.

### Steam Cloud può tornare a un salvataggio più vecchio?

No. Steam conserva lo stato attuale dei file, non la loro storia. Una volta che un salvataggio rotto è stato sincronizzato, è quello che sta nel cloud. Per tornare indietro serve uno strumento che versiona.

### Perché non tutti i miei giochi Steam si sincronizzano?

Perché è lo sviluppatore ad attivarlo, gioco per gioco e a volte per piattaforma. La pagina del negozio elenca Steam Cloud tra le caratteristiche quando è supportato, e molti titoli semplicemente non lo sono.

### Hoard funziona con giochi non Steam?

Sì, ed è buona parte del punto. Individua i salvataggi tramite un database comunitario che copre oltre 20.000 titoli, da qualsiasi store, e per i casi insoliti puoi indicargli la cartella a mano.

### Usarli entrambi crea conflitti?

No. Hoard cattura una versione dopo che hai smesso e la cartella si è calmata, e non sovrascrive mai senza aver prima catturato ciò che sostituisce.

### Posso tenere i salvataggi fuori da entrambi i cloud?

Sì. Ospita il server da solo: i salvataggi non lasciano mai hardware tuo, senza account e senza telemetria verso nessuno.
`,Gn=`---
title: "Steam クラウドの代替：Steam が守らないセーブを守る"
description: "Steam クラウドは、開発者が有効にした Steam のゲームしか対象にせず、世代履歴も残りません。Hoard は入手元を問わずすべてのゲームをバックアップし、巻き戻せる世代履歴を残します。クラウドでも、自分のサーバーでも。"
order: 7
updated: 2026-09-01
---

Steam クラウドは、その限られた役割を本当にうまくこなします。そして多くの人は、何かを失った日に初めてその境界を知ります。このガイドでは、その境界がどこにあるのか、そして境界の外に残るゲームをどうするのかを説明します。

## Steam クラウドが実際にカバーする範囲

Steam クラウドは、**開発者が設定した場合に限り** ゲームのフォルダーを同期します。同期するファイルを宣言するか、ゲーム内から Steam の API を呼ぶかのどちらかです。仕組みはこれだけで、そこから 3 つのことが導かれます。

- Steam で購入し、Steam から起動したゲームでしか働きません。
- そもそも働くかどうかは開発者の判断で、ゲームごと、ときにはプラットフォームごとに異なります。
- 各ゲームには、その開発者が決めた保存容量の枠があります。

うまく働いているときは、目に見えないほど快適です。1 台目でゲームを閉じ、2 台目で開けば、進行はそこにあります。

## どこが穴になるのか

- **Steam のゲームでないものすべて。** GOG、Epic、itch、Battle.net、Xbox アプリ、エミュレーター、手動で入れたもの。Steam はその存在を知りません。
- **有効化されていない Steam のゲーム。** 特に古いものや小規模なものには、そもそも搭載されていない例が多くあります。ストアページには書いてありますが、60 時間の周回を始める前に確認する人はいません。
- **戻る手段がない。** これが最大の点です。Steam が保持しているのはセーブの現在の状態であって、その履歴ではありません。ファイルが壊れても、Mod がワールドを食べても、良いセーブを悪いセーブで上書きしても、クラウドにあるのはすでに悪いほうです。ゲームごとに Steam が保持しているファイルを見ることはできますが、復元できる過去の世代はありません。
- **競合のダイアログ。** ローカルとリモートが食い違うと Steam は選択を求めますが、判断材料は 2 つの日時程度です。選び間違えれば、もう片方は消えます。

## Hoard が足すもの

Hoard はゲームが実際に書き込むフォルダーを監視し、**プレイを終えるたびに新しい世代** を取り込みます。

- **ゲームの入手元を問いません。** Steam、GOG、Epic、itch、エミュレーター、手動で指定したフォルダー。
- **すべての世代が残る** ので、壊れたセーブや判断ミスからの復帰は、周回のやり直しではなく 2 クリックです。
- **マシン間の同期も同じ仕組み** で、Steam Deck とデスクトップも含みます。
- **黙って壊れるものはありません。** 置き換えられるセーブは先に取り込まれるため、復元を間違えても元に戻せます。

スナップショットは内容ハッシュで保存されるため、2 GB のセーブの 10 世代は約 20 GB ではなく約 2 GB です。履歴を丸ごと残しておけるのはこのためです。

## 両方を同時に使う

両者はぶつかりませんし、どちらかを選ぶ必要もありません。クラウド対応の Steam ゲームでは、Steam に今までどおり同期させてください。そこで Hoard が足すのは履歴、つまり Steam が持たないものです。それ以外のすべてでは、同期も Hoard が担います。

デスクトップに加えて Steam Deck を使うなら、重要な細部がひとつあります。Hoard は \`userdata\` の中の \`<AppID>/remote/\` を追跡し、その上のフォルダーは追跡しません。上のフォルダーには \`remotecache.vdf\` や、実績・プレイ時間といったマシンごとのファイルが入っているからです。自作の同期がいちばん間違えるのがこの区別で、そうした構成が起動のたびに競合しているように見える理由でもあります。

## Steam クラウドで足りる場合

はっきり書いておきます。遊ぶゲームがすべてクラウド対応の Steam タイトルで、PC は 1 台、セーブを巻き戻したいと思ったことがないのなら、Steam クラウドで用は足りており、ほかに何も要りません。Hoard を足す理由になるのは、世代履歴、Steam の外にあるゲーム、そして Steam クラウドが届かないマシンです。

## 誰のクラウドも使わない

どのプラットフォームにも依存したくないのであれば、Hoard は自分のハードウェアだけで動かせます。PC か NAS で \`hoard-server\` を動かせば、セーブは自分のマシンから自分のディスクへ移ります。**当方のアカウントも、当方へのテレメトリも、中継もありません。** 経路上に当方のものが何一つないため、当方のサーバーを何も通りません。[Hoard をセルフホストする方法](/guides/self-host-hoard) を参照してください。

同じプログラム、同じ検出、同じ世代履歴。変わるのは保存先が誰のものかだけです。

<!-- faq -->

## よくある質問

### Hoard は Steam クラウドの置き換えですか？

置き換える必要はありません。Steam クラウドは対応ゲームの現在のセーブを同期し続け、Hoard はそこに世代履歴を足し、対応していないゲームを引き受けます。両方を使うのが普通です。

### Steam クラウドで古いセーブに戻せますか？

いいえ。Steam が保持するのはファイルの現在の状態であって、その履歴ではありません。壊れたセーブが同期されてしまえば、クラウドにあるのはそれです。戻るには世代を残すツールが必要です。

### Steam のゲームなのに同期されないのはなぜですか？

有効にするのが開発者だからです。ゲームごと、ときにはプラットフォームごとに決まります。対応している場合はストアページの機能欄に Steam クラウドが並びますが、載っていないタイトルも数多くあります。

### Steam 以外のゲームでも使えますか？

はい。むしろそこが要点のひとつです。2 万本以上を収録したコミュニティのデータベースからセーブの場所を割り出し、入手元は問いません。変わったものは手動でフォルダーを指定できます。

### 両方動かすと競合しませんか？

しません。Hoard はプレイ終了後、フォルダーが静かになってから世代を取り込み、置き換える前に必ず現物を取り込みます。

### セーブをどちらのクラウドにも置かずに済みますか？

はい。サーバーをセルフホストすれば、セーブが自分のハードウェアから出ることはありません。アカウントもなく、どこへもテレメトリを送りません。
`,En=`---
title: "Alternativa à Steam Cloud: guarda os saves que a Steam não guarda"
description: "A Steam Cloud só cobre jogos da Steam cujo programador a ativou, e não guarda histórico. O Hoard copia todos os jogos a que jogas, venham de onde vierem, com um histórico versionado a que podes voltar — na nuvem ou no teu próprio servidor."
order: 7
updated: 2026-09-01
---

A Steam Cloud faz muito bem o trabalho estreito que faz, e a maioria das pessoas só lhe descobre os limites no dia em que perde alguma coisa. Este guia explica onde estão esses limites e o que fazer com os jogos que ficam de fora.

## O que a Steam Cloud cobre mesmo

A Steam Cloud sincroniza a pasta de um jogo quando **o programador a configurou**: ou declarando que ficheiros sincronizar, ou chamando a API da Steam de dentro do jogo. É todo o modelo, e daí saem três consequências:

- Só funciona com jogos comprados e lançados pela Steam.
- Se funciona ou não é decisão do programador, jogo a jogo, e às vezes por plataforma.
- Cada jogo tem a sua própria quota de espaço, definida por esse programador.

Quando funciona é invisível e excelente: fechas o jogo num PC, abres noutro, e o progresso está lá.

## Onde te deixa exposto

- **Tudo o que não seja um jogo da Steam.** GOG, Epic, itch, Battle.net, a app da Xbox, emuladores, tudo o que instalaste à mão. A Steam nem sabe que existem.
- **Jogos da Steam onde nunca foi ativada.** Muitos títulos, sobretudo antigos ou pequenos, simplesmente não a têm. A página da loja di-lo, mas ninguém verifica antes de começar uma campanha de 60 horas.
- **Não há volta atrás.** É o ponto grande. A Steam guarda o estado atual do save, não o seu histórico. Se o ficheiro se corrompe, se uma mod te come o mundo, ou se escreves por cima de um save bom com um mau, a cópia na nuvem já é a má. Podes ver os ficheiros que a Steam guarda de um jogo, mas não há versão anterior para restaurar.
- **A janela de conflito.** Quando a Steam acha que o local e o remoto divergem, pede-te para escolher com pouco mais do que duas datas à frente. Escolhes mal e a outra cópia desapareceu.

## O que o Hoard acrescenta

O Hoard vigia a pasta onde o jogo realmente escreve e captura uma **versão nova sempre que acabas de jogar**:

- **Não lhe interessa de onde veio o jogo.** Steam, GOG, Epic, itch, emuladores ou uma pasta que lhe apontes à mão.
- **Todas as versões ficam guardadas**, por isso recuperar de um save corrompido ou de uma má decisão são dois cliques e não uma campanha perdida.
- **Sincroniza entre as tuas máquinas** da mesma forma, Steam Deck e desktop incluídos.
- **Nada é destruído em silêncio.** O save substituído é capturado primeiro, por isso até uma restauração errada é reversível.

Os snapshots são guardados por hash de conteúdo, por isso dez versões de um save de 2 GB ocupam cerca de 2 GB e não 20 — é isso que torna prático manter o histórico inteiro.

## Usar os dois ao mesmo tempo

Não se atropelam, e não tens de escolher. Num jogo da Steam com suporte de nuvem, deixa a Steam sincronizar o que já sincroniza; o que o Hoard acrescenta aí é o histórico, exatamente aquilo que a Steam não guarda. Para tudo o resto, é o Hoard que também trata da sincronização.

Um detalhe que conta se tens uma Steam Deck além do fixo: o Hoard segue \`<AppID>/remote/\` dentro de \`userdata\`, e não a pasta acima, porque a de cima guarda \`remotecache.vdf\` e ficheiros de proezas e tempo de jogo próprios de cada máquina. É a distinção que uma sincronização caseira falha com mais frequência, e a razão pela qual essas montagens parecem entrar em conflito a cada arranque.

## Quando a Steam Cloud chega

Convém dizê-lo com clareza: se todos os jogos a que jogas são da Steam e com suporte de nuvem, jogas num só PC e nunca precisaste de desfazer um save, a Steam Cloud já faz o trabalho e não precisas de mais nada. O que justifica juntar o Hoard é o histórico de versões, os jogos de fora da Steam e as máquinas onde a Steam Cloud não chega.

## Sem a nuvem de ninguém

Se o que te atrai é não depender de plataforma nenhuma, o Hoard pode correr inteiramente no teu hardware: \`hoard-server\` num PC ou num NAS, e os teus saves vão da tua máquina para o teu disco. **Não há conta connosco, nem telemetria para nós, nem retransmissão**: não passa nada pelos nossos servidores, porque não há nada nosso no caminho. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard).

O mesmo programa, a mesma deteção, o mesmo histórico. A única coisa que muda é de quem é o armazenamento.

<!-- faq -->

## Perguntas frequentes

### O Hoard substitui a Steam Cloud?

Não tem de substituir. A Steam Cloud mantém o teu save atual sincronizado nos jogos que a suportam; o Hoard acrescenta o histórico de versões e cobre os jogos que não. Ter os dois é o normal.

### A Steam Cloud consegue voltar a um save mais antigo?

Não. A Steam guarda o estado atual dos ficheiros, não o histórico deles. Assim que um save mau sincroniza, é esse que está na nuvem. Para voltar atrás é preciso uma ferramenta com versões.

### Porque é que nem todos os meus jogos da Steam sincronizam?

Porque quem a ativa é o programador, jogo a jogo e às vezes por plataforma. A página do jogo na loja indica a Steam Cloud entre as funcionalidades quando é suportada — e muitos títulos simplesmente não são.

### O Hoard funciona com jogos que não são da Steam?

Sim, e é boa parte do sentido. Localiza os saves através de uma base de dados comunitária que cobre mais de 20.000 títulos, de qualquer loja, e para o que for invulgar podes apontar-lhe a pasta à mão.

### Ter os dois a correr provoca conflitos?

Não. O Hoard captura uma versão depois de parares e de a pasta ficar quieta, e nunca escreve por cima sem capturar primeiro aquilo que substitui.

### Posso manter os meus saves fora das duas nuvens?

Sim. Aloja o servidor tu mesmo e os teus saves nunca saem de hardware teu, sem conta e sem telemetria para lado nenhum.
`,In=`---
title: "Steam 云存档的替代方案：备份 Steam 管不到的存档"
description: "Steam 云存档只覆盖开发者启用了它的 Steam 游戏，而且不保留版本历史。Hoard 会备份你玩的每一款游戏，不论来自哪个平台，并保留可回退的版本历史——在云端，或在你自己的服务器上。"
order: 7
updated: 2026-09-01
---

Steam 云存档把它那件狭窄的事做得相当好，而多数人是在丢东西的那一天才发现它的边界。本文说明这些边界在哪里，以及落在边界之外的游戏该怎么办。

## Steam 云存档实际覆盖什么

Steam 云存档只在**开发者做了配置**时才同步某款游戏的文件夹——要么声明需要同步哪些文件，要么在游戏内调用 Steam 的接口。整个模型就是这样，由此引出三件事：

- 它只对通过 Steam 购买并启动的游戏有效。
- 它到底能不能用，由开发者逐款决定，有时还分平台。
- 每款游戏有各自的存储配额，由该开发者设定。

它生效的时候是无形而出色的：在一台 PC 上关掉游戏，在另一台上打开，进度就在那里。

## 它把你晾在哪里

- **一切不是 Steam 的游戏。** GOG、Epic、itch、Battle.net、Xbox 应用、模拟器，以及你手动安装的一切。Steam 根本不知道它们存在。
- **从未启用它的 Steam 游戏。** 相当多的游戏，尤其是较老或较小的作品，压根就没有。商店页面会写明，但没人会在开一档 60 小时的存档前先去看。
- **没有回头路。** 这是最关键的一点。Steam 保存的是存档的当前状态，而不是它的历史。文件损坏、模组吞掉你的世界、或者用坏档覆盖了好档，云端那份已经是坏的了。你可以查看 Steam 为某款游戏保存的文件，但没有更早的版本可供还原。
- **冲突对话框。** 当 Steam 认为本地和云端不一致时，它让你选择，而你手上几乎只有两个时间戳。选错了，另一份就没了。

## Hoard 补上了什么

Hoard 盯着游戏真正写入的那个文件夹，并在**你每次玩完之后**抓取一个新版本：

- **它不在乎游戏从哪来。** Steam、GOG、Epic、itch、模拟器，或者你手动指给它的文件夹。
- **每个版本都会保留**，因此从损坏的存档或一次错误决定中脱身是两次点击，而不是一整轮重来。
- **同样负责在你的机器之间同步**，包括 Steam Deck 和台式机。
- **不会有东西悄悄消失。** 被替换掉的存档会先被抓取，所以连还原错了都能撤销。

快照按内容哈希存储，因此一个 2 GB 存档的十个版本大约占 2 GB，而不是 20 GB——正是这一点让保留完整历史变得现实。

## 两者同时使用

它们不会打架，你也不必二选一。对于支持云存档的 Steam 游戏，让 Steam 继续做它已经在做的同步；Hoard 在那里补上的是历史，也正是 Steam 不保留的东西。至于其他一切，同步也由 Hoard 负责。

如果你除了台式机还有 Steam Deck，有个细节很重要：Hoard 追踪的是 \`userdata\` 里的 \`<AppID>/remote/\`，而不是它上一层的文件夹，因为上一层放着 \`remotecache.vdf\` 以及各机器各自的成就和游戏时长文件。手工搭建的同步最常弄错的就是这个区别，这也是那类方案看起来每次启动都在冲突的原因。

## 什么时候 Steam 云存档就够了

不妨直说：如果你玩的每款游戏都是支持云存档的 Steam 游戏，只在一台 PC 上玩，也从没需要撤销过某个存档，那么 Steam 云存档已经把事情办了，你不需要别的。值得加上 Hoard 的理由是版本历史、Steam 之外的游戏，以及 Steam 云存档够不到的机器。

## 不用任何人的云

如果你在意的是不依赖任何平台，Hoard 可以完全跑在你自己的硬件上：在 PC 或 NAS 上运行 \`hoard-server\`，你的存档就从你的机器走到你的磁盘。**没有我们这边的账号，没有发往我们的遥测，也没有中转**——不经过我们的任何服务器，因为这条路径上根本没有我们的东西。参见[如何自托管 Hoard](/guides/self-host-hoard)。

同一个程序，同样的检测，同样的版本历史。唯一变化的是存储归谁所有。

<!-- faq -->

## 常见问题

### Hoard 是要取代 Steam 云存档吗？

不必如此。Steam 云存档为支持它的游戏同步当前存档；Hoard 补上版本历史，并覆盖那些不支持的游戏。两者同时使用很常见。

### Steam 云存档能回退到更早的存档吗？

不能。Steam 保存的是文件的当前状态，不是它们的历史。一旦坏档同步上去，云端就是那一份。要回退，只能靠会做版本管理的工具。

### 为什么我的 Steam 游戏不是每款都同步？

因为启用它的是开发者，逐款决定，有时还分平台。支持时，游戏商店页面会把 Steam 云存档列在功能里——而很多游戏根本就没有。

### Hoard 支持非 Steam 的游戏吗？

支持，这正是它的意义所在。它通过覆盖两万余款游戏的社区数据库定位存档，不限平台；遇到特别的情况，你也可以手动指定文件夹。

### 两个一起用会冲突吗？

不会。Hoard 会在你停止游玩、文件夹安静之后才抓取版本，并且在覆盖之前一定会先把被替换的内容抓取下来。

### 我能让存档不进这两朵云吗？

可以。自托管服务器，你的存档就永远不会离开属于你的硬件，没有账号，也不向任何地方发送遥测。
`,Rn=`---
title: "So synchronisierst du Spielstände über mehrere PCs"
description: "Spiele dasselbe Spiel auf Desktop und Laptop, ohne Fortschritt zu verlieren. Synchronisiere deine Spielstände automatisch über mehrere PCs mit Hoard — verwaltete Cloud-Synchronisierung, ohne Ludusavi und Rclone von Hand einzurichten."
order: 2
updated: 2026-09-01
---

Wenn du an mehr als einem Computer spielst — ein Desktop zu Hause und ein Laptop unterwegs — hält Hoard deine Stände synchron, damit du immer dort weitermachst, wo du aufgehört hast.

## So funktioniert die Synchronisierung

Hoard sichert jeden Stand in deine Cloud und lädt die neueste Version auf deinen anderen Geräten herunter. Wenn du auf einem PC fertig bist, wartet der neueste Stand auf dem nächsten.

## Synchronisierung einrichten

1. Installiere **Hoard** auf jedem PC, auf dem du spielst (Windows, macOS oder Linux).
2. Melde dich mit **demselben Konto** auf jedem Gerät an oder verbinde sie mit demselben selbst gehosteten Server.
3. Füge auf jedem PC dieselben Spiele zur **Bibliothek** hinzu. Hoard ordnet sie nach Spiel zu, sodass ein auf einem Gerät gesicherter Stand auf den anderen erscheint.
4. Lass den **Automatikmodus** an. Hoard lädt nach dem Spielen hoch und vor dem Start die neueste Version herunter.

## Wechsel von Ludusavi?

Ludusavi ist ein großartiges Open-Source-Tool, um Stände lokal zu sichern und wiederherzustellen, und es kann diese Backups in eine selbst konfigurierte Cloud mit Rclone übertragen. Aber die Synchronisierung über Geräte hinweg richtest du manuell ein: Backup planen, Remote einrichten, dann auf dem anderen PC wiederherstellen, bevor du spielst.

Hoard macht daraus verwaltete Synchronisierung. Es nutzt dieselben Community-Daten für Speicherorte wie Ludusavi, um deine Stände zu finden, lädt dann nach jeder Sitzung hoch und vor der nächsten die neueste Version herunter — auf jedem PC deines Kontos, mit versionierter Historie in der Cloud. Keine Rclone-Remotes, keine Skripte. Und wie Ludusavi ist Hoard Open Source und selbst hostbar. Siehe den vollständigen [Ludusavi-Alternative-Vergleich](/guides/ludusavi-alternative).

## Konflikte vermeiden

Hoard ist konfliktbewusst: Es vergleicht Änderungszeiten und behält eine lokale Kopie jedes ersetzten Stands, sodass eine Synchronisierung nie stillschweigend Fortschritt zerstört. Läuft ein Spiel noch oder wurde ein Stand in den letzten Minuten berührt, wartet Hoard.

## Steam Deck und Desktop

Das häufigste Zwei-Geräte-Setup ist auch das, was von Hand gebaut am öftesten kaputtgeht, und fast immer aus demselben Grund.

Unter Windows liegt der Spielstand vielleicht in \`Dokumente\\My Games\\…\` oder in Steams \`userdata\`. Auf einem Steam Deck läuft dasselbe Windows-Spiel über Proton, sein Stand liegt also in einem Kompatibilitäts-Prefix: \`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\`. Zwei sehr verschiedene Pfade, ein Spiel, ein Spielfortschritt. Hoard liest die Proton-Prefixes ebenso wie die nativen Orte und ordnet Gefundenes dem Spiel zu, sodass Deck-Stand und Desktop-Stand zwei Versionen einer Historie werden statt zweier zusammenhangloser Ordner.

Das Detail, an dem alles hängt: Bei Steam-Spielen verfolgt Hoard \`<AppID>/remote/\` innerhalb von \`userdata\`, **nicht** den Ordner darüber. Der übergeordnete Ordner enthält auch \`remotecache.vdf\` sowie gerätebezogene Dateien für Erfolge und Spielzeit, die sich zwischen Deck und Desktop unterscheiden sollen. Synchronisierst du den übergeordneten Ordner, sieht jeder Start nach einem Konflikt aus, obwohl sich kein Stand bewegt hat. Genau dieser eine Fehler lässt die meisten selbstgebauten Deck-PC-Setups defekt wirken.

## Spiele, die Steam Cloud nicht abdeckt

Würden alle deine Spiele Steam Cloud unterstützen, bräuchtest du nichts davon. In der Praxis:

- **Spiele von überall außer Steam.** GOG, Epic, itch, Battle.net, die Xbox-App und alles von Hand Installierte.
- **Steam-Spiele, bei denen die Entwickler es nie aktiviert haben**, oder nur für eine Plattform.
- **Emulatoren.** RetroArch, Dolphin, PCSX2, RPCS3 und der Rest speichern, wo sie wollen, und Steam weiß nichts davon.
- **Spiele, die außerhalb des von Steam beobachteten Ordners schreiben**, und das sind mehr, als man denkt.

Hoard ist egal, wer ein Spiel veröffentlicht hat oder woher es kommt. Es verfolgt den Ordner, der sich beim Spielen ändert.

## Wenn zwei PCs denselben Stand ändern

Du spielst am Laptop, ohne den Desktop zu Ende synchronisieren zu lassen, und hast das klassische Problem: zwei Stände, beide neuer als die letzte gemeinsame Version.

Hoard überschreibt nie blind. Es vergleicht Änderungszeiten, behält eine lokale Kopie von allem, was es ersetzt, und wartet, solange ein Spiel läuft oder der Stand in den letzten Minuten angefasst wurde — eine Datei, die gerade geschrieben wird, will man nicht halb hochladen. Alle früheren Versionen bleiben in der Cloud-Historie, die falsche Wahl kostet dich also zwei Klicks statt eines Wochenendes.

Die ehrliche Grenze: **Hoard führt zwei auseinandergelaufene Stände nicht zusammen.** Das kann kein Werkzeug — eine Speicherdatei ist undurchsichtig, und es gibt keinen richtigen Weg, zwei verschiedene Spielnachmittage zu vermischen. Was du stattdessen bekommst: jede Version, auf jedem Gerät, und die Wahl.

## Synchronisieren ohne unsere Server

Das gehört ausdrücklich gesagt, weil die meisten Vergleiche genau hier danebenliegen. Es gibt zwei Betriebsarten:

- **Hoard Cloud** ist die verwaltete Variante: du meldest dich an, und deine Stände liegen auf unseren Servern in der EU.
- **Selbsthosten gehört vollständig dir.** Du betreibst \`hoard-server\` auf deinem eigenen PC oder NAS, und deine Geräte synchronisieren darüber. Es gibt **kein Konto bei uns, keine Telemetrie zu uns, kein Limit und kein Relay** — nichts läuft über unsere Server, weil nichts von uns im Weg steht. Siehe [wie du Hoard selbst hostest](/guides/self-host-hoard).

Dasselbe Programm, dieselbe Erkennung, dieselbe Versionshistorie. Es ändert sich nur, wem der Speicher gehört.

## Tipp

Gib jedem Gerät einen Moment, um die Synchronisierung abzuschließen, bevor du ein Spiel startest — das Dashboard zeigt den Live-Status, damit du weißt, dass der neueste Stand bereit ist.

<!-- faq -->

## Häufige Fragen

### Wie viele PCs kann ich synchronisieren?

Drei im kostenlosen Tarif, unbegrenzt mit Pro und unbegrenzt beim Selbsthosten — dein Server, deine Regeln.

### Müssen beide Geräte gleichzeitig online sein?

Nein. Dein Stand geht nach dem Spielen zum Server und kommt herunter, wenn das andere Gerät danach fragt. Der zweite PC kann also eine Woche ausgeschaltet sein und bekommt beim Einschalten trotzdem die neueste Version.

### Was, wenn ich offline spiele?

Kein Problem. Der Snapshot entsteht lokal, wenn du aufhörst zu spielen, und wird von selbst hochgeladen, sobald die Maschine wieder Verbindung hat.

### Werden auch Mods und Einstellungen synchronisiert?

Spielstände ja. Dateien, die zu einem bestimmten Rechner gehören — Konfiguration, Logs und Ähnliches — werden hochgeladen, damit sie im Backup sind, aber nicht über die Kopie eines anderen PCs geschrieben: eine Grafikeinstellung, die zu deinem Desktop passt, ist selten die, die dein Laptop will.

### Sendet Selbsthosten irgendetwas an Hoard?

Nein. Im selbst gehosteten Betrieb gibt es kein Konto bei uns und keine Telemetrie zu uns: deine Stände, deine Nutzer und deine Logs liegen auf deinem eigenen Server und berühren unseren nie.
`,Tn=`---
title: "How to sync game saves across multiple PCs"
description: "Play the same game on your desktop and laptop without losing progress. Sync your game saves across PCs automatically with Hoard — managed cloud sync without wiring up Ludusavi and Rclone by hand."
order: 2
updated: 2026-09-01
---

If you play on more than one computer — a desktop at home and a laptop on the go — Hoard keeps your saves in sync so you always pick up where you left off.

## How sync works

Hoard backs up each save to your cloud and pulls the latest version down on your other machines. When you finish playing on one PC, the newest save is waiting on the next one.

## Set up sync

1. Install **Hoard** on every PC you play on (Windows, macOS or Linux).
2. Sign in with the **same account** on each machine, or connect them to the same self-hosted server.
3. Add the same games to your **Library** on each PC. Hoard matches them by game, so a save backed up on one shows up on the others.
4. Keep **automatic mode** on. Hoard uploads after you play and downloads the latest before you start.

## Coming from Ludusavi?

Ludusavi is a great open-source tool for backing up and restoring saves locally, and it can push those backups to a cloud you configure yourself with Rclone. But syncing across devices is something you wire up manually: schedule the backup, set up the remote, then restore on the other PC before you play.

Hoard turns that into managed sync. It uses the same community save-location data as Ludusavi to find your saves, then uploads after each session and downloads the latest before the next one — across every PC on your account, with versioned history in the cloud. No Rclone remotes, no scripts. And like Ludusavi, Hoard is open source and can be self-hosted. See the full [Ludusavi alternative comparison](/guides/ludusavi-alternative).

## Avoiding conflicts

Hoard is conflict-aware: it compares modification times and keeps a local copy of any replaced save, so a sync never silently destroys progress. If a game is still running or a save was touched in the last few minutes, Hoard waits.

## Steam Deck and desktop

The most common two-machine setup is also the one that breaks most often when it's wired by hand, and nearly always for the same reason.

On Windows, a game's save might sit in \`Documents\\My Games\\…\` or inside Steam's \`userdata\`. On a Steam Deck, that same Windows game runs through Proton, so its save lives inside a compatibility prefix: \`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\`. Two very different paths, one game, one run of progress. Hoard reads the Proton prefixes as well as the native locations and matches what it finds by game, so the Deck save and the desktop save become two versions of one history instead of two unrelated folders.

The detail that decides whether any of this works: for Steam games Hoard tracks \`<AppID>/remote/\` inside \`userdata\`, **not** the folder above it. The parent also holds \`remotecache.vdf\` and per-machine achievement and playtime files, which are supposed to differ between your Deck and your desktop. Sync the parent and every launch looks like a conflict even though no save actually moved. That single mistake is what makes most hand-rolled Deck ↔ PC setups feel broken.

## Games Steam Cloud doesn't cover

If every game you played supported Steam Cloud, you wouldn't need any of this. In practice:

- **Games from anywhere but Steam.** GOG, Epic, itch, Battle.net, the Xbox app, and anything you installed by hand.
- **Steam games where the developer never turned it on**, or turned it on for one platform only.
- **Emulators.** RetroArch, Dolphin, PCSX2, RPCS3 and the rest save where they like, and Steam knows nothing about it.
- **Games that write outside the folder Steam watches**, which is more of them than you'd expect.

Hoard doesn't care who published a game or where it came from. It tracks the folder that changes when you play.

## When two PCs edit the same save

Play on the laptop without letting the desktop finish syncing and you get the classic problem: two saves, both newer than the last common version.

Hoard never overwrites blind. It compares modification times, keeps a local copy of whatever it replaces, and holds off while a game is running or the save was touched in the last few minutes — a save file being written is not a save you want to upload halfway. Every earlier version stays in the cloud history, so picking the wrong one costs you two clicks, not a weekend.

The honest limit: **Hoard does not merge two divergent saves.** No tool can — a save file is opaque, and there is no correct way to blend two different afternoons of play. What you get instead is every version, on every machine, and the ability to choose.

## Syncing without our servers

Worth being explicit, because it's the part most comparisons get wrong. There are two ways to run this:

- **Hoard Cloud** is the managed option: you sign in, and your saves are stored on our servers, in the EU.
- **Self-hosting is entirely yours.** You run \`hoard-server\` on your own PC or NAS and your machines sync through it. There is **no account with us, no telemetry to us, no quota and no relay** — nothing passes through our servers, because there is nothing of ours in the path. See [how to self-host Hoard](/guides/self-host-hoard).

Same program, same detection, same version history. The only thing that changes is who owns the storage.

## Tip

Give each machine a moment to finish syncing before you launch a game — the dashboard shows live status, so you know the latest save is in place.

<!-- faq -->

## Frequently asked questions

### How many PCs can I sync?

Three on the free tier, unlimited on Pro, and unlimited when you self-host — your server, your rules.

### Do both machines have to be online at the same time?

No. Your save goes up to the server when you finish playing and comes down when the other machine asks for it, so the second PC can be switched off for a week and still get the latest version when it wakes up.

### What if I play offline?

Fine. The snapshot is taken locally when you stop playing, and it uploads on its own once the machine has a connection again.

### Does it sync my mods and settings too?

Saves, yes. Files that belong to one machine — configuration, logs, and similar — are uploaded so they're in the backup, but are not written back over another PC's copy, because a graphics setting that suits your desktop is rarely the one your laptop wants.

### Does self-hosting send anything to Hoard?

No. In self-hosted mode there is no account with us and no telemetry to us: your saves, your users and your logs live on your own server and never touch ours.
`,_n=`---
title: "Cómo sincronizar partidas guardadas entre varios PC"
description: "Juega al mismo juego en tu sobremesa y tu portátil sin perder progreso. Sincroniza tus partidas entre PC automáticamente con Hoard: sincronización en la nube gestionada, sin montar Ludusavi y Rclone a mano."
order: 2
updated: 2026-09-01
---

Si juegas en más de un ordenador —un sobremesa en casa y un portátil de viaje— Hoard mantiene tus partidas sincronizadas para que siempre retomes donde lo dejaste.

## Cómo funciona la sincronización

Hoard sube cada partida a tu nube y descarga la última versión en tus otros equipos. Cuando terminas de jugar en un PC, la partida más reciente te espera en el siguiente.

## Configura la sincronización

1. Instala **Hoard** en cada PC en el que juegues (Windows, macOS o Linux).
2. Inicia sesión con la **misma cuenta** en cada equipo, o conéctalos al mismo servidor autoalojado.
3. Añade los mismos juegos a tu **Biblioteca** en cada PC. Hoard los empareja por juego, así que una partida guardada en uno aparece en los demás.
4. Mantén el **modo automático** activado. Hoard sube cuando terminas de jugar y descarga la última versión antes de empezar.

## ¿Vienes de Ludusavi?

Ludusavi es una gran herramienta open source para hacer copias y restaurar partidas en local, y puede subir esas copias a una nube que configures tú mismo con Rclone. Pero sincronizar entre dispositivos es algo que montas a mano: programas la copia, configuras el remoto y luego restauras en el otro PC antes de jugar.

Hoard convierte eso en sincronización gestionada. Usa los mismos datos comunitarios de ubicación de partidas que Ludusavi para encontrar tus saves, y luego sube tras cada sesión y descarga la última versión antes de la siguiente, en todos los PC de tu cuenta y con historial versionado en la nube. Sin remotos de Rclone, sin scripts. Y, como Ludusavi, Hoard es open source y se puede autoalojar. Mira la [comparativa completa con Ludusavi](/guides/ludusavi-alternative).

## Evitar conflictos

Hoard tiene en cuenta los conflictos: compara las fechas de modificación y guarda una copia local de cualquier partida que reemplaza, así que una sincronización nunca destruye progreso en silencio. Si un juego sigue abierto o la partida se tocó hace pocos minutos, Hoard espera.

## Steam Deck y sobremesa

El montaje de dos máquinas más habitual es también el que más se rompe cuando se monta a mano, y casi siempre por el mismo motivo.

En Windows, la partida de un juego puede estar en \`Documentos\\My Games\\…\` o dentro del \`userdata\` de Steam. En una Steam Deck, ese mismo juego de Windows corre bajo Proton, así que su partida vive dentro de un prefijo de compatibilidad: \`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\`. Dos rutas muy distintas, un solo juego, un solo progreso. Hoard lee los prefijos de Proton además de las ubicaciones nativas y empareja lo que encuentra por juego, así que la partida de la Deck y la del sobremesa pasan a ser dos versiones de un mismo historial en vez de dos carpetas sin relación.

El detalle que decide si esto funciona: en los juegos de Steam, Hoard rastrea \`<AppID>/remote/\` dentro de \`userdata\`, **no** la carpeta de encima. La carpeta padre guarda además \`remotecache.vdf\` y ficheros de logros y de tiempo jugado propios de cada máquina, que deben ser distintos entre tu Deck y tu sobremesa. Si sincronizas la padre, cada arranque parece un conflicto aunque no se haya movido ninguna partida. Ese único error es lo que hace que la mayoría de los montajes caseros entre Deck y PC parezcan estropeados.

## Juegos que Steam Cloud no cubre

Si todos los juegos a los que juegas soportaran Steam Cloud, no necesitarías nada de esto. En la práctica:

- **Juegos de cualquier sitio que no sea Steam.** GOG, Epic, itch, Battle.net, la app de Xbox y todo lo que hayas instalado a mano.
- **Juegos de Steam en los que el desarrollador nunca lo activó**, o lo activó sólo para una plataforma.
- **Emuladores.** RetroArch, Dolphin, PCSX2, RPCS3 y compañía guardan donde les parece, y Steam no sabe nada de eso.
- **Juegos que escriben fuera de la carpeta que vigila Steam**, que son más de los que imaginas.

A Hoard le da igual quién publicara el juego o de dónde venga: rastrea la carpeta que cambia cuando juegas.

## Cuando dos PC tocan la misma partida

Juegas en el portátil sin dejar que el sobremesa termine de sincronizar y tienes el problema clásico: dos partidas, las dos más nuevas que la última versión común.

Hoard nunca sobrescribe a ciegas. Compara fechas de modificación, guarda una copia local de lo que reemplaza, y espera mientras haya un juego abierto o la partida se haya tocado en los últimos minutos: un fichero que se está escribiendo no es un fichero que quieras subir a medias. Todas las versiones anteriores siguen en el historial de la nube, así que equivocarte de versión cuesta dos clics y no un fin de semana.

El límite honesto: **Hoard no fusiona dos partidas divergentes.** Ninguna herramienta puede — un fichero de partida es opaco, y no existe una forma correcta de mezclar dos tardes distintas de juego. Lo que te da a cambio es todas las versiones, en todas las máquinas, y la posibilidad de elegir.

## Sincronizar sin pasar por nuestros servidores

Conviene decirlo explícitamente, porque es la parte que casi todas las comparativas se equivocan. Hay dos formas de usar esto:

- **Hoard Cloud** es la opción gestionada: inicias sesión y tus partidas se guardan en nuestros servidores, en la UE.
- **Autoalojarse es tuyo por completo.** Levantas \`hoard-server\` en tu PC o en tu NAS y tus máquinas sincronizan a través de él. **No hay cuenta con nosotros, ni telemetría hacia nosotros, ni cupo, ni relé**: no pasa nada por nuestros servidores, porque no hay nada nuestro en el camino. Mira [cómo autoalojar Hoard](/guides/self-host-hoard).

El mismo programa, la misma detección, el mismo historial de versiones. Lo único que cambia es de quién es el almacenamiento.

## Consejo

Deja que cada equipo termine de sincronizar antes de abrir un juego: el panel muestra el estado en vivo, así sabes que la última partida ya está en su sitio.

<!-- faq -->

## Preguntas frecuentes

### ¿Cuántos PC puedo sincronizar?

Tres en el plan gratuito, ilimitados en Pro, e ilimitados si te autoalojas: tu servidor, tus reglas.

### ¿Tienen que estar las dos máquinas encendidas a la vez?

No. Tu partida sube al servidor cuando terminas de jugar y baja cuando la otra máquina la pide, así que el segundo PC puede estar apagado una semana y aun así recibir la última versión al encenderse.

### ¿Y si juego sin conexión?

Sin problema. La instantánea se toma en local al dejar de jugar, y se sube sola en cuanto la máquina vuelve a tener conexión.

### ¿Sincroniza también mods y ajustes?

Las partidas, sí. Los ficheros que son de una máquina concreta — configuración, registros y similares — se suben para que estén en la copia, pero no se escriben encima de la copia de otro PC, porque un ajuste gráfico que le va bien a tu sobremesa rara vez es el que quiere tu portátil.

### ¿Autoalojarse envía algo a Hoard?

No. En modo autoalojado no hay cuenta con nosotros ni telemetría hacia nosotros: tus partidas, tus usuarios y tus registros viven en tu propio servidor y nunca tocan el nuestro.
`,Wn=`---
title: "Comment synchroniser vos parties entre plusieurs PC"
description: "Jouez au même jeu sur votre fixe et votre portable sans perdre votre progression. Synchronisez vos parties entre PC automatiquement avec Hoard — une synchro cloud gérée, sans configurer Ludusavi et Rclone à la main."
order: 2
updated: 2026-09-01
---

Si vous jouez sur plus d'un ordinateur — un fixe à la maison et un portable en déplacement — Hoard garde vos sauvegardes synchronisées pour que vous repreniez toujours là où vous en étiez.

## Comment fonctionne la synchronisation

Hoard sauvegarde chaque partie vers votre cloud et récupère la dernière version sur vos autres machines. Quand vous finissez de jouer sur un PC, la sauvegarde la plus récente vous attend sur le suivant.

## Configurer la synchronisation

1. Installez **Hoard** sur chaque PC où vous jouez (Windows, macOS ou Linux).
2. Connectez-vous avec le **même compte** sur chaque machine, ou reliez-les au même serveur auto-hébergé.
3. Ajoutez les mêmes jeux à votre **Bibliothèque** sur chaque PC. Hoard les associe par jeu, donc une sauvegarde faite sur l'un apparaît sur les autres.
4. Gardez le **mode automatique** activé. Hoard envoie après que vous jouez et télécharge la dernière version avant que vous commenciez.

## Vous venez de Ludusavi ?

Ludusavi est un excellent outil open source pour sauvegarder et restaurer des parties en local, et il peut envoyer ces sauvegardes vers un cloud que vous configurez vous-même avec Rclone. Mais la synchro entre appareils, vous la montez à la main : planifier la sauvegarde, configurer le distant, puis restaurer sur l'autre PC avant de jouer.

Hoard transforme cela en synchro gérée. Il utilise les mêmes données communautaires d'emplacements que Ludusavi pour trouver vos sauvegardes, puis envoie après chaque session et télécharge la dernière version avant la suivante — sur chaque PC de votre compte, avec un historique versionné dans le cloud. Pas de distants Rclone, pas de scripts. Et comme Ludusavi, Hoard est open source et peut être auto-hébergé. Voir la [comparaison complète avec Ludusavi](/guides/ludusavi-alternative).

## Éviter les conflits

Hoard gère les conflits : il compare les dates de modification et conserve une copie locale de toute sauvegarde remplacée, donc une synchro ne détruit jamais la progression en silence. Si un jeu tourne encore ou qu'une sauvegarde a été modifiée il y a quelques minutes, Hoard attend.

## Steam Deck et PC de bureau

Le montage à deux machines le plus courant est aussi celui qui casse le plus souvent quand on le fait à la main, et presque toujours pour la même raison.

Sous Windows, la sauvegarde d'un jeu peut se trouver dans \`Documents\\My Games\\…\` ou dans le \`userdata\` de Steam. Sur un Steam Deck, ce même jeu Windows tourne via Proton : sa sauvegarde vit donc dans un préfixe de compatibilité, \`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\`. Deux chemins très différents, un seul jeu, une seule progression. Hoard lit les préfixes Proton comme les emplacements natifs et rapproche ce qu'il trouve par jeu : la sauvegarde du Deck et celle du bureau deviennent deux versions d'un même historique au lieu de deux dossiers sans rapport.

Le détail dont tout dépend : pour les jeux Steam, Hoard suit \`<AppID>/remote/\` dans \`userdata\`, et **non** le dossier au-dessus. Le dossier parent contient aussi \`remotecache.vdf\` ainsi que des fichiers de succès et de temps de jeu propres à chaque machine, qui doivent différer entre votre Deck et votre bureau. Synchronisez le parent et chaque lancement ressemble à un conflit alors qu'aucune sauvegarde n'a bougé. Cette seule erreur suffit à faire paraître cassés la plupart des montages maison Deck ↔ PC.

## Les jeux que Steam Cloud ne couvre pas

Si tous vos jeux géraient Steam Cloud, vous n'auriez besoin de rien de tout cela. En pratique :

- **Les jeux venus d'ailleurs que Steam.** GOG, Epic, itch, Battle.net, l'application Xbox, et tout ce que vous avez installé à la main.
- **Les jeux Steam où le développeur ne l'a jamais activé**, ou seulement pour une plateforme.
- **Les émulateurs.** RetroArch, Dolphin, PCSX2, RPCS3 et les autres écrivent où bon leur semble, et Steam n'en sait rien.
- **Les jeux qui écrivent hors du dossier surveillé par Steam**, et il y en a plus qu'on ne croit.

Hoard se moque de qui a publié un jeu et d'où il vient : il suit le dossier qui change quand vous jouez.

## Quand deux PC modifient la même sauvegarde

Vous jouez sur le portable sans laisser le fixe finir sa synchro, et voilà le problème classique : deux sauvegardes, toutes deux plus récentes que la dernière version commune.

Hoard n'écrase jamais à l'aveugle. Il compare les dates de modification, conserve une copie locale de ce qu'il remplace, et attend tant qu'un jeu tourne ou que la sauvegarde a été touchée dans les dernières minutes : un fichier en cours d'écriture n'est pas un fichier qu'on veut envoyer à moitié. Toutes les versions antérieures restent dans l'historique cloud : se tromper de version coûte deux clics, pas un week-end.

La limite honnête : **Hoard ne fusionne pas deux sauvegardes divergentes.** Aucun outil ne le peut — un fichier de sauvegarde est opaque, et il n'existe aucune façon correcte de mélanger deux après-midi de jeu différents. Ce que vous obtenez à la place, c'est toutes les versions, sur toutes les machines, et le choix.

## Synchroniser sans passer par nos serveurs

Autant le dire franchement, car c'est le point sur lequel presque toutes les comparaisons se trompent. Il y a deux façons de l'utiliser :

- **Hoard Cloud** est l'option gérée : vous vous connectez, et vos sauvegardes sont stockées sur nos serveurs, dans l'UE.
- **L'auto-hébergement est entièrement le vôtre.** Vous faites tourner \`hoard-server\` sur votre PC ou votre NAS et vos machines se synchronisent à travers lui. Il n'y a **aucun compte chez nous, aucune télémétrie vers nous, aucun quota et aucun relais** : rien ne passe par nos serveurs, puisque rien de chez nous n'est sur le chemin. Voir [comment auto-héberger Hoard](/guides/self-host-hoard).

Le même programme, la même détection, le même historique de versions. La seule chose qui change, c'est à qui appartient le stockage.

## Astuce

Laissez chaque machine finir de synchroniser avant de lancer un jeu — le tableau de bord affiche l'état en direct, vous savez donc que la dernière sauvegarde est en place.

<!-- faq -->

## Questions fréquentes

### Combien de PC puis-je synchroniser ?

Trois avec l'offre gratuite, un nombre illimité avec Pro, et illimité en auto-hébergement : votre serveur, vos règles.

### Les deux machines doivent-elles être allumées en même temps ?

Non. Votre sauvegarde monte vers le serveur quand vous finissez de jouer et redescend quand l'autre machine la demande : le second PC peut rester éteint une semaine et recevoir quand même la dernière version à l'allumage.

### Et si je joue hors ligne ?

Aucun souci. L'instantané est pris localement quand vous arrêtez de jouer, et il part tout seul dès que la machine retrouve une connexion.

### Est-ce que ça synchronise aussi mes mods et réglages ?

Les sauvegardes, oui. Les fichiers propres à une machine — configuration, journaux et compagnie — sont envoyés pour figurer dans la sauvegarde, mais ne sont pas réécrits par-dessus la copie d'un autre PC : un réglage graphique qui convient à votre fixe est rarement celui que veut votre portable.

### L'auto-hébergement envoie-t-il quoi que ce soit à Hoard ?

Non. En mode auto-hébergé il n'y a aucun compte chez nous ni aucune télémétrie vers nous : vos sauvegardes, vos utilisateurs et vos journaux vivent sur votre propre serveur et ne touchent jamais le nôtre.
`,Nn=`---
title: "Come sincronizzare i salvataggi tra più PC"
description: "Gioca allo stesso gioco su fisso e portatile senza perdere progressi. Sincronizza i tuoi salvataggi tra PC automaticamente con Hoard — sincronizzazione cloud gestita, senza configurare Ludusavi e Rclone a mano."
order: 2
updated: 2026-09-01
---

Se giochi su più di un computer — un fisso a casa e un portatile in giro — Hoard mantiene i salvataggi sincronizzati così riprendi sempre da dove avevi lasciato.

## Come funziona la sincronizzazione

Hoard fa il backup di ogni salvataggio sul tuo cloud e scarica l'ultima versione sulle altre macchine. Quando finisci di giocare su un PC, il salvataggio più recente ti aspetta sul successivo.

## Imposta la sincronizzazione

1. Installa **Hoard** su ogni PC su cui giochi (Windows, macOS o Linux).
2. Accedi con lo **stesso account** su ogni macchina, o collegale allo stesso server self-hosted.
3. Aggiungi gli stessi giochi alla **Libreria** su ogni PC. Hoard li abbina per gioco, così un salvataggio fatto su uno appare sugli altri.
4. Tieni attiva la **modalità automatica**. Hoard carica dopo che giochi e scarica l'ultima versione prima che inizi.

## Arrivi da Ludusavi?

Ludusavi è un ottimo strumento open source per fare backup e ripristinare salvataggi in locale, e può inviare quei backup a un cloud che configuri tu stesso con Rclone. Ma la sincronizzazione tra dispositivi la imposti a mano: programmare il backup, configurare il remoto, poi ripristinare sull'altro PC prima di giocare.

Hoard trasforma tutto questo in sincronizzazione gestita. Usa gli stessi dati comunitari di posizione di Ludusavi per trovare i tuoi salvataggi, poi carica dopo ogni sessione e scarica l'ultima versione prima della successiva — su ogni PC del tuo account, con cronologia versionata nel cloud. Niente remoti Rclone, niente script. E come Ludusavi, Hoard è open source e può essere self-hosted. Vedi il [confronto completo con Ludusavi](/guides/ludusavi-alternative).

## Evitare i conflitti

Hoard è consapevole dei conflitti: confronta le date di modifica e conserva una copia locale di ogni salvataggio sostituito, così una sincronizzazione non distrugge mai i progressi in silenzio. Se un gioco è ancora aperto o un salvataggio è stato toccato negli ultimi minuti, Hoard aspetta.

## Steam Deck e desktop

Il setup a due macchine più comune è anche quello che si rompe più spesso quando lo si monta a mano, e quasi sempre per lo stesso motivo.

Su Windows il salvataggio di un gioco può stare in \`Documenti\\My Games\\…\` oppure dentro \`userdata\` di Steam. Su una Steam Deck lo stesso gioco Windows gira con Proton, quindi il salvataggio vive dentro un prefisso di compatibilità: \`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\`. Due percorsi molto diversi, un gioco solo, un solo progresso. Hoard legge i prefissi Proton oltre alle posizioni native e abbina quello che trova per gioco, così il salvataggio della Deck e quello del desktop diventano due versioni della stessa cronologia invece di due cartelle scollegate.

Il dettaglio da cui dipende tutto: per i giochi Steam, Hoard traccia \`<AppID>/remote/\` dentro \`userdata\`, **non** la cartella superiore. Quella superiore contiene anche \`remotecache.vdf\` e i file di obiettivi e tempo di gioco propri di ogni macchina, che tra Deck e desktop devono essere diversi. Se sincronizzi la cartella superiore, ogni avvio sembra un conflitto anche se nessun salvataggio si è mosso. È quell'unico errore a far sembrare rotti quasi tutti i setup artigianali tra Deck e PC.

## I giochi che Steam Cloud non copre

Se tutti i giochi a cui giochi supportassero Steam Cloud non ti servirebbe niente di tutto questo. Nella pratica:

- **Giochi che non vengono da Steam.** GOG, Epic, itch, Battle.net, l'app Xbox e tutto ciò che hai installato a mano.
- **Giochi Steam in cui lo sviluppatore non l'ha mai attivato**, o l'ha attivato per una sola piattaforma.
- **Emulatori.** RetroArch, Dolphin, PCSX2, RPCS3 e gli altri salvano dove preferiscono, e Steam non ne sa nulla.
- **Giochi che scrivono fuori dalla cartella sorvegliata da Steam**, e sono più di quanti immagini.

A Hoard non importa chi abbia pubblicato un gioco né da dove arrivi: traccia la cartella che cambia quando giochi.

## Quando due PC toccano lo stesso salvataggio

Giochi sul portatile senza lasciare che il fisso finisca di sincronizzare ed ecco il problema classico: due salvataggi, entrambi più recenti dell'ultima versione comune.

Hoard non sovrascrive mai alla cieca. Confronta le date di modifica, conserva una copia locale di ciò che sostituisce e aspetta finché un gioco è aperto o il salvataggio è stato toccato negli ultimi minuti: un file in scrittura non è un file da caricare a metà. Tutte le versioni precedenti restano nella cronologia cloud, quindi sbagliare versione costa due clic e non un fine settimana.

Il limite onesto: **Hoard non fonde due salvataggi divergenti.** Nessuno strumento può farlo — un file di salvataggio è opaco e non esiste un modo corretto di mescolare due pomeriggi di gioco diversi. Quello che ottieni invece è ogni versione, su ogni macchina, e la possibilità di scegliere.

## Sincronizzare senza passare dai nostri server

Vale la pena dirlo chiaramente, perché è il punto su cui quasi tutti i confronti sbagliano. Ci sono due modi di usarlo:

- **Hoard Cloud** è l'opzione gestita: accedi e i salvataggi stanno sui nostri server, nell'UE.
- **Il self-hosting è interamente tuo.** Fai girare \`hoard-server\` sul tuo PC o sul tuo NAS e le tue macchine si sincronizzano attraverso quello. **Nessun account con noi, nessuna telemetria verso di noi, nessuna quota e nessun relay**: non passa nulla dai nostri server, perché sul percorso non c'è niente di nostro. Vedi [come ospitare Hoard da solo](/guides/self-host-hoard).

Stesso programma, stesso rilevamento, stessa cronologia delle versioni. L'unica cosa che cambia è di chi è lo spazio di archiviazione.

## Suggerimento

Lascia che ogni macchina finisca di sincronizzare prima di avviare un gioco — la dashboard mostra lo stato in tempo reale, così sai che l'ultimo salvataggio è al suo posto.

<!-- faq -->

## Domande frequenti

### Quanti PC posso sincronizzare?

Tre nel piano gratuito, illimitati con Pro e illimitati in self-hosting: il tuo server, le tue regole.

### Le due macchine devono essere accese nello stesso momento?

No. Il salvataggio sale al server quando smetti di giocare e scende quando l'altra macchina lo chiede: il secondo PC può restare spento una settimana e ricevere comunque l'ultima versione all'accensione.

### E se gioco offline?

Nessun problema. Lo snapshot viene preso in locale quando smetti di giocare e parte da solo appena la macchina torna online.

### Sincronizza anche mod e impostazioni?

I salvataggi sì. I file che appartengono a una macchina specifica — configurazione, log e simili — vengono caricati per essere nel backup, ma non riscritti sopra la copia di un altro PC: un'impostazione grafica che va bene al fisso è raramente quella che vuole il portatile.

### Il self-hosting manda qualcosa a Hoard?

No. In modalità self-hosted non c'è alcun account con noi né telemetria verso di noi: i tuoi salvataggi, i tuoi utenti e i tuoi log stanno sul tuo server e non toccano mai il nostro.
`,Bn=`---
title: "複数の PC 間でセーブデータを同期する方法"
description: "デスクトップとノート PC で同じゲームを進行を失わずにプレイ。Hoard でセーブデータを PC 間で自動同期。Ludusavi と Rclone を手動で設定することなく、マネージドなクラウド同期を実現します。"
order: 2
updated: 2026-09-01
---

複数のコンピューター（自宅のデスクトップと外出先のノート PC など）でプレイするなら、Hoard がセーブデータを同期し続けるので、いつでも続きから再開できます。

## 同期の仕組み

Hoard は各セーブをクラウドにバックアップし、ほかのマシンに最新バージョンをダウンロードします。ある PC でプレイを終えると、最新のセーブが次の PC で待っています。

## 同期を設定する

1. プレイするすべての PC に **Hoard をインストール** します（Windows、macOS、Linux）。
2. 各マシンで **同じアカウント** でサインインするか、同じセルフホストサーバーに接続します。
3. 各 PC の **ライブラリ** に同じゲームを追加します。Hoard はゲーム単位で対応付けるので、一方でバックアップしたセーブが他方にも表示されます。
4. **自動モード** をオンのままにします。Hoard はプレイ後にアップロードし、開始前に最新版をダウンロードします。

## Ludusavi から移行しますか？

Ludusavi はローカルでセーブをバックアップ・復元する優れたオープンソースツールで、Rclone で自分で設定したクラウドへバックアップを送ることもできます。ただし端末間の同期は自分で組む必要があります。バックアップをスケジュールし、リモートを設定し、プレイ前にもう一方の PC で復元する、という流れです。

Hoard はこれをマネージドな同期に変えます。Ludusavi と同じコミュニティのセーブ位置データを使ってセーブを見つけ、各セッション後にアップロードし、次の前に最新版をダウンロードします。アカウント内のすべての PC で、クラウド上に世代履歴を保ちながら行われます。Rclone のリモートもスクリプトも不要です。そして Ludusavi と同様に、Hoard もオープンソースでセルフホスト可能です。詳しくは [Ludusavi 代替の比較](/guides/ludusavi-alternative) をご覧ください。

## 競合を避ける

Hoard は競合を認識します。更新時刻を比較し、置き換えるセーブのローカルコピーを保持するため、同期が黙って進行を壊すことはありません。ゲームがまだ起動中だったり、直近数分でセーブが変更されていたりする場合、Hoard は待機します。

## Steam Deck とデスクトップ

2 台構成として最も多い組み合わせは、手作業で組んだときに最も壊れやすい組み合わせでもあり、原因はほぼ毎回同じです。

Windows では、セーブは \`ドキュメント\\My Games\\…\` か Steam の \`userdata\` にあります。Steam Deck では同じ Windows 版ゲームが Proton 経由で動くため、セーブは互換プレフィックスの中、\`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\` に置かれます。まるで違うパス、同じゲーム、ひとつづきの進行です。Hoard はネイティブの場所に加えて Proton のプレフィックスも読み、見つけたものをゲーム単位で結びつけます。こうして Deck のセーブとデスクトップのセーブは、無関係な 2 つのフォルダーではなく、ひとつの履歴の 2 つの世代になります。

すべてを左右する細部があります。Steam のゲームでは、Hoard は \`userdata\` の中の \`<AppID>/remote/\` を追跡し、その **上のフォルダーは追跡しません**。上のフォルダーには \`remotecache.vdf\` や、実績・プレイ時間といったマシンごとに異なって当然のファイルが入っています。上を同期すると、セーブが動いていなくても起動のたびに競合に見えます。自作の Deck と PC の構成が壊れているように感じられる原因は、たいていこの一点です。

## Steam クラウドが面倒を見ないゲーム

遊ぶゲームがすべて Steam クラウドに対応していれば、こうした仕組みは要りません。現実には:

- **Steam 以外から来たゲーム。** GOG、Epic、itch、Battle.net、Xbox アプリ、そして手動で入れたもの全部。
- **開発者が有効にしなかった Steam のゲーム。** あるいは片方のプラットフォームでしか有効にしていないもの。
- **エミュレーター。** RetroArch、Dolphin、PCSX2、RPCS3 などは好きな場所に保存し、Steam はそれを知りません。
- **Steam が見ているフォルダーの外に書き込むゲーム。** 思っているより多くあります。

Hoard は誰が出したゲームかも、どこから来たかも問いません。プレイすると変化するフォルダーを追跡するだけです。

## 2 台の PC が同じセーブを触ったとき

デスクトップの同期が終わらないうちにノート PC で遊ぶと、古典的な問題が起きます。最後の共通世代より新しいセーブが 2 つある状態です。

Hoard は決して無言で上書きしません。更新時刻を比べ、置き換えるものはローカルに控えを残し、ゲームが動作中か、セーブが直前の数分に触られていれば待ちます。書き込み途中のファイルは、半端な状態でアップロードしたくないからです。以前の世代はすべてクラウドの履歴に残るので、選び間違えても週末ではなく 2 クリックで済みます。

正直な限界を書いておきます。**Hoard は分岐した 2 つのセーブを統合しません。** どのツールにもできません。セーブファイルは中身が読めず、異なる 2 回のプレイを正しく混ぜる方法は存在しないからです。代わりに手に入るのは、すべての世代がすべてのマシンにあり、選べるという状態です。

## 当方のサーバーを介さない同期

多くの比較が誤解している点なので、はっきり書きます。動かし方は 2 通りあります。

- **Hoard Cloud** はマネージドな選択肢です。サインインすると、セーブは EU にある当方のサーバーに保存されます。
- **セルフホストは完全にあなたのものです。** 自分の PC や NAS で \`hoard-server\` を動かし、各マシンはそれを介して同期します。**当方のアカウントも、当方へのテレメトリも、容量制限も、中継もありません。** 経路上に当方のものが何一つないため、当方のサーバーを何も通りません。[Hoard をセルフホストする方法](/guides/self-host-hoard) を参照してください。

同じプログラム、同じ検出、同じ世代履歴。変わるのは保存先が誰のものかだけです。

## ヒント

ゲームを起動する前に、各マシンの同期が完了するまで少し待ちましょう。ダッシュボードがリアルタイムの状態を表示するので、最新のセーブが揃っているか分かります。

<!-- faq -->

## よくある質問

### 何台の PC を同期できますか？

無料枠は 3 台、Pro は無制限、セルフホストでも無制限です。自分のサーバーなら台数は自分で決められます。

### 2 台を同時に起動しておく必要はありますか？

いいえ。セーブはプレイ終了時にサーバーへ上がり、もう一方のマシンが求めたときに下ります。2 台目は 1 週間電源を切っていても、次に起動したときに最新の世代を受け取れます。

### オフラインで遊んだ場合は？

問題ありません。スナップショットはプレイ終了時にローカルで作られ、回線が戻ったときに自動でアップロードされます。

### Mod や設定も同期されますか？

セーブは同期されます。特定のマシンに属するファイル、つまり設定やログなどはバックアップに含めるためアップロードされますが、他の PC の同じファイルを上書きすることはありません。デスクトップに合うグラフィック設定が、ノート PC にも合うとは限らないからです。

### セルフホストは Hoard に何かを送信しますか？

いいえ。セルフホストでは当方のアカウントも当方へのテレメトリもありません。セーブもユーザーもログも自分のサーバーの中にとどまり、当方のサーバーには一切触れません。
`,Mn=`---
title: "Como sincronizar saves entre vários PCs"
description: "Joga o mesmo jogo no fixo e no portátil sem perder progresso. Sincroniza os teus saves entre PCs automaticamente com o Hoard — sincronização na nuvem gerida, sem configurar o Ludusavi e o Rclone à mão."
order: 2
updated: 2026-09-01
---

Se jogas em mais de um computador — um fixo em casa e um portátil em viagem — o Hoard mantém os teus saves sincronizados para que retomes sempre onde paraste.

## Como funciona a sincronização

O Hoard faz backup de cada save para a tua nuvem e descarrega a versão mais recente nas tuas outras máquinas. Quando acabas de jogar num PC, o save mais recente espera-te no seguinte.

## Configurar a sincronização

1. Instala o **Hoard** em cada PC onde jogas (Windows, macOS ou Linux).
2. Inicia sessão com a **mesma conta** em cada máquina, ou liga-as ao mesmo servidor self-hosted.
3. Adiciona os mesmos jogos à **Biblioteca** em cada PC. O Hoard associa-os por jogo, por isso um save feito num aparece nos outros.
4. Mantém o **modo automático** ligado. O Hoard envia depois de jogares e descarrega a versão mais recente antes de começares.

## Vens do Ludusavi?

O Ludusavi é uma excelente ferramenta open source para fazer backup e restaurar saves localmente, e pode enviar esses backups para uma nuvem que configuras tu mesmo com o Rclone. Mas a sincronização entre dispositivos montas tu à mão: agendar o backup, configurar o remoto, e depois restaurar no outro PC antes de jogar.

O Hoard transforma isso em sincronização gerida. Usa os mesmos dados comunitários de localização do Ludusavi para encontrar os teus saves, depois envia após cada sessão e descarrega a versão mais recente antes da seguinte — em cada PC da tua conta, com histórico versionado na nuvem. Sem remotos de Rclone, sem scripts. E como o Ludusavi, o Hoard é open source e pode ser self-hosted. Vê a [comparação completa com o Ludusavi](/guides/ludusavi-alternative).

## Evitar conflitos

O Hoard tem em conta os conflitos: compara as datas de modificação e guarda uma cópia local de qualquer save substituído, por isso uma sincronização nunca destrói progresso em silêncio. Se um jogo ainda estiver aberto ou um save foi tocado nos últimos minutos, o Hoard espera.

## Steam Deck e desktop

A montagem de duas máquinas mais comum é também a que mais se estraga quando é feita à mão, e quase sempre pelo mesmo motivo.

No Windows, o save de um jogo pode estar em \`Documentos\\My Games\\…\` ou dentro do \`userdata\` da Steam. Numa Steam Deck, esse mesmo jogo de Windows corre com Proton, por isso o save vive dentro de um prefixo de compatibilidade: \`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\`. Dois caminhos muito diferentes, um só jogo, um só progresso. O Hoard lê os prefixos Proton além das localizações nativas e associa o que encontra por jogo, por isso o save da Deck e o do desktop passam a ser duas versões do mesmo histórico em vez de duas pastas sem relação.

O detalhe de que tudo depende: nos jogos da Steam, o Hoard segue \`<AppID>/remote/\` dentro de \`userdata\`, e **não** a pasta acima. A pasta acima guarda também \`remotecache.vdf\` e ficheiros de proezas e tempo de jogo próprios de cada máquina, que devem ser diferentes entre a tua Deck e o teu desktop. Se sincronizares a de cima, cada arranque parece um conflito mesmo sem nenhum save se ter mexido. É esse único erro que faz parecerem avariadas quase todas as montagens caseiras entre Deck e PC.

## Jogos que a Steam Cloud não cobre

Se todos os jogos que jogas suportassem Steam Cloud, não precisarias de nada disto. Na prática:

- **Jogos vindos de qualquer sítio que não a Steam.** GOG, Epic, itch, Battle.net, a app da Xbox e tudo o que instalaste à mão.
- **Jogos da Steam em que o programador nunca a ativou**, ou ativou só para uma plataforma.
- **Emuladores.** RetroArch, Dolphin, PCSX2, RPCS3 e companhia guardam onde lhes apetece, e a Steam não sabe nada disso.
- **Jogos que escrevem fora da pasta vigiada pela Steam**, e são mais do que se imagina.

Ao Hoard tanto lhe faz quem publicou o jogo ou de onde veio: segue a pasta que muda quando jogas.

## Quando dois PCs mexem no mesmo save

Jogas no portátil sem deixar o fixo acabar de sincronizar e tens o problema clássico: dois saves, ambos mais recentes do que a última versão comum.

O Hoard nunca escreve por cima às cegas. Compara datas de modificação, guarda uma cópia local do que substitui, e espera enquanto houver um jogo aberto ou o save tiver sido tocado nos últimos minutos: um ficheiro a ser escrito não é um ficheiro que queiras enviar a meio. Todas as versões anteriores ficam no histórico da nuvem, por isso enganares-te na versão custa dois cliques e não um fim de semana.

O limite honesto: **o Hoard não funde dois saves divergentes.** Nenhuma ferramenta o consegue — um ficheiro de save é opaco, e não há forma correta de misturar duas tardes de jogo diferentes. O que tens em troca é todas as versões, em todas as máquinas, e a possibilidade de escolher.

## Sincronizar sem passar pelos nossos servidores

Vale a pena dizê-lo de forma explícita, porque é o ponto em que quase todas as comparações se enganam. Há duas formas de o usar:

- **O Hoard Cloud** é a opção gerida: inicias sessão e os teus saves ficam nos nossos servidores, na UE.
- **O self-hosting é inteiramente teu.** Corres o \`hoard-server\` no teu PC ou no teu NAS e as tuas máquinas sincronizam através dele. **Não há conta connosco, nem telemetria para nós, nem quota, nem retransmissão**: não passa nada pelos nossos servidores, porque não há nada nosso no caminho. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard).

O mesmo programa, a mesma deteção, o mesmo histórico de versões. A única coisa que muda é de quem é o armazenamento.

## Dica

Dá a cada máquina um momento para terminar a sincronização antes de abrires um jogo — o painel mostra o estado em tempo real, por isso sabes que o save mais recente já está no sítio.

<!-- faq -->

## Perguntas frequentes

### Quantos PCs posso sincronizar?

Três no plano gratuito, ilimitados no Pro e ilimitados em self-hosting: o teu servidor, as tuas regras.

### As duas máquinas têm de estar ligadas ao mesmo tempo?

Não. O teu save sobe para o servidor quando acabas de jogar e desce quando a outra máquina o pede, por isso o segundo PC pode estar desligado uma semana e mesmo assim receber a versão mais recente ao ligar.

### E se jogar sem ligação?

Sem problema. O snapshot é tirado localmente quando páras de jogar, e sobe sozinho assim que a máquina volta a ter ligação.

### Também sincroniza mods e definições?

Os saves, sim. Os ficheiros que pertencem a uma máquina em concreto — configuração, registos e afins — são enviados para ficarem no backup, mas não são escritos por cima da cópia de outro PC: uma definição gráfica que serve ao teu fixo raramente é a que o teu portátil quer.

### O self-hosting envia alguma coisa para o Hoard?

Não. Em modo self-hosted não há conta connosco nem telemetria para nós: os teus saves, os teus utilizadores e os teus registos vivem no teu próprio servidor e nunca tocam no nosso.
`,Un=`---
title: "如何在多台 PC 之间同步游戏存档"
description: "在台式机和笔记本上玩同一款游戏而不丢失进度。用 Hoard 在多台 PC 之间自动同步存档——托管式云同步，无需手动配置 Ludusavi 和 Rclone。"
order: 2
updated: 2026-09-01
---

如果你在不止一台电脑上玩游戏——家里的台式机和外出用的笔记本——Hoard 会让你的存档保持同步，让你总能从上次离开的地方继续。

## 同步的原理

Hoard 会把每个存档备份到你的云端，并在你的其他机器上拉取最新版本。当你在一台 PC 上玩完，最新的存档就已在下一台等着你。

## 设置同步

1. 在你玩游戏的每台 PC 上**安装 Hoard**（Windows、macOS 或 Linux）。
2. 在每台机器上用**同一账号**登录，或把它们连接到同一台自托管服务器。
3. 在每台 PC 的**库**中添加相同的游戏。Hoard 按游戏进行匹配，因此在一台上备份的存档会出现在其他机器上。
4. 保持**自动模式**开启。Hoard 会在你玩完后上传，并在你开始前下载最新版本。

## 从 Ludusavi 迁移？

Ludusavi 是一款出色的开源工具，可在本地备份和还原存档，并能通过你自己用 Rclone 配置的云端推送这些备份。但跨设备同步需要你自己搭建：安排备份、配置远端，然后在玩之前在另一台 PC 上还原。

Hoard 把这一切变成托管式同步。它使用与 Ludusavi 相同的社区存档位置数据来找到你的存档，然后在每次会话后上传、在下一次之前下载最新版本——覆盖你账号下的每台 PC，并在云端保留版本历史。无需 Rclone 远端，无需脚本。而且与 Ludusavi 一样，Hoard 同样开源且可自托管。请见完整的 [Ludusavi 替代方案对比](/guides/ludusavi-alternative)。

## 避免冲突

Hoard 具备冲突感知：它会比较修改时间，并为任何被替换的存档保留一份本地副本，因此同步绝不会悄无声息地破坏进度。如果某款游戏仍在运行，或某个存档在最近几分钟内被改动过，Hoard 会等待。

## Steam Deck 与台式机

最常见的双机组合，也正是手工搭建时最容易出问题的组合，而且原因几乎每次都一样。

在 Windows 上，存档可能在 \`文档\\My Games\\…\`，也可能在 Steam 的 \`userdata\` 里。在 Steam Deck 上，同一款 Windows 游戏通过 Proton 运行，存档因此位于兼容层前缀内：\`steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…\`。两条完全不同的路径，同一款游戏，同一份进度。Hoard 除了原生位置之外也会读取 Proton 前缀，并按游戏把找到的内容对应起来，于是 Deck 的存档和台式机的存档成为同一段历史的两个版本，而不是两个毫不相干的文件夹。

决定成败的细节：对 Steam 游戏，Hoard 追踪 \`userdata\` 里的 \`<AppID>/remote/\`，而**不是**它上一层的文件夹。上一层还放着 \`remotecache.vdf\` 以及成就和游戏时长这类本就属于各台机器的文件，它们在 Deck 和台式机之间理应不同。同步上一层，每次启动都像冲突，尽管没有任何存档动过。绝大多数手工搭建的 Deck 与 PC 方案让人觉得"坏掉了"，就坏在这一点上。

## Steam 云存档管不到的游戏

如果你玩的每款游戏都支持 Steam 云存档，这一切都不需要。但实际上：

- **不是来自 Steam 的游戏。** GOG、Epic、itch、Battle.net、Xbox 应用，以及你手动安装的一切。
- **开发者从未开启云存档的 Steam 游戏**，或者只为某一个平台开启。
- **模拟器。** RetroArch、Dolphin、PCSX2、RPCS3 等等想存哪儿就存哪儿，Steam 对此一无所知。
- **写在 Steam 监视范围之外的游戏**，而且比你想的要多。

Hoard 不在乎游戏由谁发行、从哪儿来：它追踪的是你游玩时会变化的那个文件夹。

## 当两台 PC 改动同一份存档

没等台式机同步完就在笔记本上玩，就会遇到经典问题：两份存档，都比上一次共同的版本更新。

Hoard 从不盲目覆盖。它比较修改时间，为被替换的内容保留本地副本，并在游戏仍在运行、或存档在最近几分钟内被改动过时先等一等——正在写入的文件，不是你想传到一半的文件。所有更早的版本都留在云端历史里，因此选错版本的代价是两次点击，而不是一个周末。

坦白说出限制：**Hoard 不会合并两份已经分叉的存档。** 任何工具都做不到——存档文件是不透明的，把两个不同下午的游玩正确地揉在一起并不存在。你得到的是每一个版本、每一台机器上都有，以及自己选择的余地。

## 不经过我们服务器的同步

值得说明白，因为这正是多数对比弄错的地方。它有两种运行方式：

- **Hoard Cloud** 是托管方案：你登录，存档保存在我们位于欧盟的服务器上。
- **自托管完全属于你。** 你在自己的 PC 或 NAS 上运行 \`hoard-server\`，各台机器通过它同步。**没有我们这边的账号，没有发往我们的遥测，没有配额，也没有中转**——不经过我们的任何服务器，因为这条路径上根本没有我们的东西。参见[如何自托管 Hoard](/guides/self-host-hoard)。

同一个程序，同样的检测，同样的版本历史。唯一变化的是存储归谁所有。

## 提示

在启动游戏前，给每台机器一点时间完成同步——仪表盘会显示实时状态，让你知道最新存档已经就位。

<!-- faq -->

## 常见问题

### 我可以同步多少台 PC？

免费额度三台，Pro 不限台数，自托管同样不限——你的服务器，你说了算。

### 两台机器需要同时开机吗？

不需要。你玩完之后存档会上传到服务器，另一台机器需要时再取下来，所以第二台 PC 关机一周，开机后照样能拿到最新版本。

### 离线玩怎么办？

没问题。快照是在你停止游玩时于本地生成的，等机器重新联网后会自行上传。

### 它也会同步模组和设置吗？

存档会。属于某一台机器的文件——配置、日志之类——会上传以便进入备份，但不会覆盖另一台 PC 上的同名文件：适合你台式机的画质设置，通常并不是笔记本想要的。

### 自托管会向 Hoard 发送任何东西吗？

不会。在自托管模式下，没有我们这边的账号，也没有发往我们的遥测：你的存档、你的用户和你的日志都留在你自己的服务器上，从不接触我们的服务器。
`,Vn=`---
title: "Syncthing für Spielstände: was klappt und was bricht"
description: "Syncthing ist ein hervorragender universeller Datei-Sync, aber Spielstände brechen drei seiner Annahmen. Was schiefgeht, wie man es umgeht, und wann ein Werkzeug besser ist, das weiß, was ein Spielstand ist."
order: 9
updated: 2026-09-01
---

Syncthing ist die Antwort, zu der viele zuerst greifen, und das aus gutem Grund: kostenlos, quelloffen, peer-to-peer, und es funktioniert. Doch Spielstände brechen drei Annahmen, auf denen ein universeller Datei-Sync aufbaut, und die Fehler sind leise. Diese Anleitung handelt davon, was wirklich schiefgeht, und wann sich ein Werkzeug lohnt, das weiß, was ein Spielstand ist.

## Warum man dort landet

Es ist wirklich gute Software. Kein Konto, kein Abo, deine Dateien liegen nie auf der Platte einer Firma, und es synchronisiert alles: Dokumente, Fotos, einen Ordner mit Spielständen. Wenn du es ohnehin betreibst, kostet dich ein zusätzlicher Ordner dreißig Sekunden. Das ist ein echtes Argument, und für manche Setups das richtige.

## Die drei Dinge, die brechen

**Es synchronisiert, während das Spiel läuft.** Syncthing reagiert darauf, dass sich eine Datei ändert — für ein Dokument genau richtig. Ein Spiel schreibt seinen Stand mitten in der Sitzung, manchmal in mehreren Durchgängen, und eine Datei, die mitten im Schreiben erwischt wird, verbreitet sich halbfertig. Die andere Maschine hat dann einen Stand, den das Spiel womöglich nicht lädt.

**Konflikte werden zu Dateien statt zu Entscheidungen.** Ändern beide Maschinen denselben Stand, tut Syncthing das Sichere und behält beide, indem es einen in \`etwas.sync-conflict-20260901-143022-ABCDEFG.sav\` umbenennt. Verloren geht nichts — aber das Spiel weiß nicht, was diese Datei ist, und du vergleichst Zeitstempel im Dateimanager, um zu entscheiden, welchen Spielnachmittag du behältst. Ein paar Mal, und der Ordner füllt sich mit Konfliktdateien, die niemand zu löschen wagt.

**Versionierung ist pro Datei, nicht pro Sitzung.** Syncthing kann alte Kopien in \`.stversions\` aufheben, besser als nichts. Aber ein Spielstand besteht oft aus mehreren Dateien, die nur zusammen Sinn ergeben, und Wiederherstellen heißt, für jede den richtigen Zeitstempel von Hand zu finden. Ein "setz dieses Spiel auf Dienstag zurück" gibt es nicht.

Und ein vierter Punkt, speziell für Steam: richtest du es auf \`userdata/<UserID>/<AppID>/\` statt auf den \`remote/\`-Ordner darin, synchronisierst du auch \`remotecache.vdf\` sowie Dateien für Erfolge und Spielzeit, die sich zwischen Maschinen unterscheiden **sollen**. Dann sieht jeder Start nach einem Konflikt aus, obwohl sich kein Stand bewegt hat. Das ist der häufigste Grund, warum ein selbstgebautes Setup zwischen Steam Deck und Desktop kaputt wirkt.

## Was du am Ende selbst baust

Nichts davon ist unlösbar. Man behilft sich mit Ignore-Mustern je Spiel, einer Versionierungsrichtlinie und der Gewohnheit, das Spiel zu schließen und zu warten, bevor man den anderen PC anfasst. Das funktioniert, und es ist Pflege, die dir für immer gehört: ein neues Spiel heißt neue Pfade, und der Tag, an dem du das Warten vergisst, ist der Tag, an dem du es merkst.

## Was ein spielstandbewusstes Werkzeug stattdessen tut

Hoard sichert **nachdem du aufgehört hast**, sobald der Ordner zur Ruhe kommt, ein Snapshot ist also nie eine halb geschriebene Datei. Jede Sicherung ist eine Version des ganzen Spielstands, nicht einzelner Dateien, das Wiederherstellen ist ein Klick und setzt alles gemeinsam zurück. Es weiß, welcher Ordner zu welchem Spiel gehört — es liest dasselbe Community-Manifest für Speicherorte, das im Open-Source-Umfeld geteilt wird, mit über 20.000 Titeln — es gibt also keine Pfade zu pflegen, und es verfolgt \`<AppID>/remote/\` statt des Ordners darüber.

## Wann Syncthing die bessere Antwort ist

Fairerweise:

- **Du betreibst es ohnehin**, ein Ordner mehr ist gratis.
- **Du willst peer-to-peer ganz ohne Server**, nicht einmal einen eigenen.
- **Du synchronisierst weit mehr als Spielstände** und hättest lieber ein Werkzeug für alles.
- **Du rollst nie zurück.** Wenn der letzte Stand immer gereicht hat, ist eine Versionshistorie Maschinerie, die du nicht nutzt.

## Beides nutzen

Sie vertragen sich, und das ist ein vernünftiges Setup: der universelle Sync übernimmt Dokumente und den Rest, ein spielstandbewusstes Werkzeug die Speicherordner. Die einzige Regel: richte nicht beide auf denselben Ordner — zwei Programme, die dieselben Dateien schreiben, erzeugen genau die Konflikte, die du vermeiden wolltest.

## Auch ohne unsere Server

Wenn ein Teil des Reizes ist, dass nichts die Platte einer Firma berührt: Hoard geht genauso. \`hoard-server\` auf deinem eigenen PC oder NAS, und deine Stände gehen von deiner Maschine auf deine Platte. Es gibt **kein Konto bei uns, keine Telemetrie zu uns und kein Relay** — nichts läuft über unsere Server, weil nichts von uns im Weg steht. Siehe [wie du Hoard selbst hostest](/guides/self-host-hoard).

Dasselbe Binary, dieselbe Erkennung, dieselbe Historie. Es ändert sich nur, wem der Speicher gehört. Es gibt außerdem einen vollständigen [Vergleich aller Sync-Tools](/guides/game-save-sync-comparison).

<!-- faq -->

## Häufige Fragen

### Kann Syncthing Spielstände überhaupt synchronisieren?

Ja, und in einfachen Fällen tut es das gut. Schwierig wird es bei Spielen, die während des Spielens schreiben, bei Spielständen aus mehreren Dateien, und überall dort, wo beide Maschinen zwischen zwei Synchronisierungen bearbeitet werden.

### Was sind die .sync-conflict-Dateien in meinem Speicherordner?

Das ist der Sync, der nach einem Konflikt beide Fassungen behält, statt eine zu wählen. Verloren geht nichts, aber das Spiel kann sie nicht lesen, und die Entscheidung ist jedes Mal Handarbeit.

### Warum kollidiert mein Steam-Spielstand bei jedem Start?

Fast immer, weil der synchronisierte Ordner der über \`remote/\` ist. Er enthält \`remotecache.vdf\` sowie Dateien für Erfolge und Spielzeit, die sich zu Recht je Rechner unterscheiden — die beiden Enden werden sich also nie einig.

### Muss ich das Spiel vor dem Synchronisieren schließen?

Mit einem universellen Sync ja, das ist die Gewohnheit, die halb geschriebene Stände verhindert. Ein spielstandbewusstes Werkzeug wartet von selbst, bis der Ordner ruhig ist.

### Kann ich beide zusammen nutzen?

Ja. Richte sie nur nicht auf denselben Ordner, sonst streiten sie sich um dieselben Dateien.
`,Fn=`---
title: "Syncthing for game saves: what works and what breaks"
description: "Syncthing is an excellent general-purpose file syncer, but game saves break three of its assumptions. What goes wrong, how people work around it, and when a save-aware tool is the better answer."
order: 9
updated: 2026-09-01
---

Syncthing is the answer a lot of people reach for first, and for good reason: it's free, open source, peer-to-peer, and it works. But game saves break three of the assumptions a general-purpose file syncer is built on, and the failures are quiet ones. This guide is about what actually goes wrong, and when it's worth using something that knows what a save is.

## Why people reach for it

It's genuinely good software. No account, no subscription, your files never sit on a company's disk, and it syncs anything: documents, photos, a folder of saves. If you already run it for other things, pointing it at a save folder costs you thirty seconds. That's a real argument, and for some setups it's the right one.

## The three things that break

**It syncs while the game is running.** Syncthing reacts to a file changing, because that's the correct behaviour for a document. A game writes its save in the middle of a session, sometimes in several passes, and a file caught mid-write is a file that propagates half-finished. The other machine now holds a save the game may refuse to load.

**Conflicts become files, not decisions.** When both machines change the same save, Syncthing does the safe thing and keeps both, renaming one to \`something.sync-conflict-20260901-143022-ABCDEFG.sav\`. Nothing is lost — but the game doesn't know what that file is, and you're left comparing timestamps in a file manager to work out which afternoon of play to keep. Do this a few times and the folder fills with conflict files nobody dares delete.

**Versioning is per file, not per session.** Syncthing can keep old copies in \`.stversions\`, and that's better than nothing. But a save is often several files that only make sense together, and restoring means finding the right timestamp for each one by hand. There's no "put this game back the way it was on Tuesday".

And a fourth, specific to Steam: point it at \`userdata/<UserID>/<AppID>/\` instead of the \`remote/\` folder inside, and you're also syncing \`remotecache.vdf\` plus achievement and playtime files that are *supposed* to differ between machines. Every launch then looks like a conflict even though no save actually moved. This is the single most common reason a hand-rolled Steam Deck and desktop setup feels broken.

## What you end up building

None of the above is unfixable. People handle it with ignore patterns per game, a versioning policy, and the habit of closing the game and waiting before touching the other PC. That works, and it's a maintenance job you own forever: a new game means new paths, and the day you forget to wait is the day you find out.

## What a save-aware tool does instead

Hoard captures **after you stop playing**, once the folder goes quiet, so a snapshot is never a half-written file. Each capture is a version of the whole save, not of individual files, so restoring is one click and puts everything back together. It knows which folder belongs to which game — reading the same community save-location manifest the open-source ecosystem shares, covering 20,000+ titles — so there are no paths to maintain, and it tracks \`<AppID>/remote/\` rather than the folder above it.

## When Syncthing is the better answer

Being fair about it:

- **You already run it**, and adding a folder is free.
- **You want peer-to-peer with no server at all**, not even your own.
- **You're syncing much more than saves** and would rather have one tool for everything.
- **You never roll back.** If the latest save is all you've ever needed, a version history is machinery you won't use.

## Using both

They coexist without a fight, and it's a reasonable setup: let the general syncer handle your documents and whatever else, and let a save-aware tool handle the save folders. The only rule is not to point both at the same folder — two tools writing the same files is how you manufacture the conflicts you were trying to avoid.

## Without our servers either

If part of the appeal is that nothing touches a company's disk, Hoard can be run the same way: \`hoard-server\` on your own PC or NAS, and your saves go from your machine to your disk. There is **no account with us, no telemetry to us and no relay** — nothing passes through our servers, because there is nothing of ours in the path. See [how to self-host Hoard](/guides/self-host-hoard).

Same binary, same detection, same history. The only thing that changes is who owns the storage. There's also a full [comparison of every save sync tool](/guides/game-save-sync-comparison).

<!-- faq -->

## Frequently asked questions

### Can Syncthing sync game saves at all?

Yes, and for simple cases it does it fine. The trouble starts with games that write while you play, saves made of several files, and any setup where both machines get edited between syncs.

### What are the .sync-conflict files in my save folder?

That's the syncer keeping both versions after a conflict instead of choosing one. Nothing is lost, but the game can't read them, and deciding which to keep is manual work every time.

### Why does my Steam save conflict on every launch?

Almost always because the synced folder is the one above \`remote/\`. It contains \`remotecache.vdf\` and achievement and playtime files that legitimately differ per machine, so the two ends never agree.

### Do I need to close the game before syncing?

With a general-purpose syncer, yes — that's the habit that prevents half-written saves. A save-aware tool waits for the folder to go quiet on its own.

### Can I keep using both together?

Yes. Just don't point both at the same folder, or the two of them will fight over the same files.
`,Kn=`---
title: "Syncthing para partidas guardadas: qué funciona y qué se rompe"
description: "Syncthing es un sincronizador de ficheros excelente, pero las partidas guardadas rompen tres de sus supuestos. Qué falla, cómo lo apaña la gente, y cuándo conviene una herramienta que sepa lo que es un save."
order: 9
updated: 2026-09-01
---

Syncthing es la respuesta a la que mucha gente llega primero, y con razón: es gratis, open source, punto a punto, y funciona. Pero las partidas guardadas rompen tres de los supuestos sobre los que se construye un sincronizador de ficheros genérico, y los fallos son silenciosos. Esta guía va de qué se rompe de verdad, y de cuándo merece la pena usar algo que sepa lo que es una partida.

## Por qué la gente acaba ahí

Es software genuinamente bueno. Sin cuenta, sin suscripción, tus ficheros no se quedan en el disco de ninguna empresa, y sincroniza cualquier cosa: documentos, fotos, una carpeta de partidas. Si ya lo tienes montado para otras cosas, apuntarlo a una carpeta de saves te cuesta treinta segundos. Ése es un argumento real, y en algunos montajes es el correcto.

## Las tres cosas que se rompen

**Sincroniza con el juego abierto.** Syncthing reacciona a que un fichero cambie, porque eso es lo correcto para un documento. Un juego escribe su partida en mitad de la sesión, a veces en varias pasadas, y un fichero pillado a medio escribir es un fichero que se propaga incompleto. La otra máquina se queda con una partida que el juego puede negarse a cargar.

**Los conflictos se convierten en ficheros, no en decisiones.** Cuando las dos máquinas cambian la misma partida, Syncthing hace lo seguro y conserva las dos, renombrando una a \`algo.sync-conflict-20260901-143022-ABCDEFG.sav\`. No se pierde nada, pero el juego no sabe qué es ese fichero y tú acabas comparando fechas en un explorador para decidir qué tarde de juego te quedas. Repítelo unas cuantas veces y la carpeta se llena de ficheros de conflicto que nadie se atreve a borrar.

**El versionado es por fichero, no por sesión.** Syncthing puede guardar copias viejas en \`.stversions\`, y eso es mejor que nada. Pero una partida suele ser varios ficheros que sólo tienen sentido juntos, y restaurar significa buscar a mano la fecha correcta de cada uno. No existe un «deja este juego como estaba el martes».

Y una cuarta, específica de Steam: si lo apuntas a \`userdata/<UserID>/<AppID>/\` en vez de a la carpeta \`remote/\` de dentro, también estás sincronizando \`remotecache.vdf\` y ficheros de logros y tiempo jugado que **deben** ser distintos entre máquinas. Entonces cada arranque parece un conflicto aunque no se haya movido ninguna partida. Es el motivo más común de que un montaje casero entre Steam Deck y sobremesa parezca estropeado.

## Lo que acabas construyendo

Nada de lo anterior es irresoluble. La gente lo apaña con patrones de exclusión por juego, una política de versionado, y la costumbre de cerrar el juego y esperar antes de tocar el otro PC. Funciona, y es un mantenimiento que te llevas de por vida: un juego nuevo son rutas nuevas, y el día que se te olvide esperar es el día que te enteras.

## Qué hace en su lugar una herramienta que entiende de partidas

Hoard captura **cuando dejas de jugar**, una vez que la carpeta se queda quieta, así que una instantánea nunca es un fichero a medio escribir. Cada captura es una versión de la partida entera, no de ficheros sueltos, así que restaurar es un clic y lo devuelve todo junto. Sabe qué carpeta es de qué juego — leyendo el mismo manifiesto comunitario de ubicaciones que comparte el ecosistema open source, con más de 20.000 títulos — así que no hay rutas que mantener, y rastrea \`<AppID>/remote/\` y no la carpeta de encima.

## Cuándo Syncthing es la mejor respuesta

Siendo justos:

- **Ya lo tienes corriendo**, y añadir una carpeta te sale gratis.
- **Quieres punto a punto sin servidor ninguno**, ni siquiera el tuyo.
- **Sincronizas mucho más que partidas** y prefieres una sola herramienta para todo.
- **Nunca vuelves atrás.** Si la última partida es todo lo que has necesitado, un historial de versiones es maquinaria que no vas a usar.

## Usar los dos

Conviven sin pelearse, y es un montaje razonable: que el sincronizador genérico se ocupe de tus documentos y de lo que sea, y que de las carpetas de partidas se ocupe una herramienta que las entienda. La única regla es no apuntar los dos a la misma carpeta: dos programas escribiendo los mismos ficheros es la forma de fabricar justo los conflictos que querías evitar.

## Sin nuestros servidores tampoco

Si parte del atractivo es que nada toque el disco de una empresa, Hoard se puede usar igual: \`hoard-server\` en tu propio PC o NAS, y tus partidas van de tu máquina a tu disco. **No hay cuenta con nosotros, ni telemetría hacia nosotros, ni relé**: no pasa nada por nuestros servidores, porque no hay nada nuestro en el camino. Mira [cómo autoalojar Hoard](/guides/self-host-hoard).

El mismo binario, la misma detección, el mismo historial. Lo único que cambia es de quién es el almacenamiento. También hay una [comparativa de todas las herramientas de sincronización](/guides/game-save-sync-comparison).

<!-- faq -->

## Preguntas frecuentes

### ¿Syncthing sirve para sincronizar partidas?

Sí, y en casos sencillos lo hace bien. El problema empieza con juegos que escriben mientras juegas, partidas hechas de varios ficheros, y cualquier montaje donde las dos máquinas se editen entre sincronizaciones.

### ¿Qué son los ficheros .sync-conflict de mi carpeta de partidas?

Es el sincronizador conservando las dos versiones tras un conflicto en vez de elegir una. No se pierde nada, pero el juego no puede leerlos, y decidir cuál te quedas es trabajo manual cada vez.

### ¿Por qué mi partida de Steam da conflicto en cada arranque?

Casi siempre porque la carpeta sincronizada es la que está por encima de \`remote/\`. Contiene \`remotecache.vdf\` y ficheros de logros y tiempo jugado que son legítimamente distintos en cada máquina, así que los dos extremos nunca coinciden.

### ¿Tengo que cerrar el juego antes de sincronizar?

Con un sincronizador genérico, sí: ésa es la costumbre que evita las partidas a medio escribir. Una herramienta que entiende de saves espera sola a que la carpeta se quede quieta.

### ¿Puedo seguir usando los dos a la vez?

Sí. Sólo que no apuntes los dos a la misma carpeta, o se pelearán por los mismos ficheros.
`,Qn=`---
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

**Les conflits deviennent des fichiers, pas des décisions.** Quand les deux machines modifient la même sauvegarde, Syncthing fait le choix sûr et garde les deux, en renommant l'une en \`truc.sync-conflict-20260901-143022-ABCDEFG.sav\`. Rien n'est perdu, mais le jeu ignore ce qu'est ce fichier, et vous voilà à comparer des horodatages dans un explorateur pour décider quel après-midi de jeu garder. Répétez quelques fois et le dossier se remplit de fichiers de conflit que personne n'ose supprimer.

**Le versionnage est par fichier, pas par session.** Syncthing peut garder d'anciennes copies dans \`.stversions\`, ce qui vaut mieux que rien. Mais une sauvegarde est souvent plusieurs fichiers qui n'ont de sens qu'ensemble, et restaurer signifie retrouver à la main le bon horodatage pour chacun. Il n'y a pas de « remets ce jeu comme il était mardi ».

Et un quatrième, propre à Steam : pointez-le sur \`userdata/<UserID>/<AppID>/\` au lieu du dossier \`remote/\` à l'intérieur, et vous synchronisez aussi \`remotecache.vdf\` ainsi que des fichiers de succès et de temps de jeu qui **doivent** différer d'une machine à l'autre. Chaque lancement ressemble alors à un conflit alors qu'aucune sauvegarde n'a bougé. C'est la raison la plus fréquente pour laquelle un montage maison entre Steam Deck et PC de bureau paraît cassé.

## Ce que vous finissez par construire

Rien de tout cela n'est insoluble. On s'en sort avec des motifs d'exclusion par jeu, une politique de versionnage, et l'habitude de fermer le jeu et d'attendre avant de toucher l'autre PC. Ça marche, et c'est un entretien qui vous appartient pour toujours : un nouveau jeu, ce sont de nouveaux chemins, et le jour où vous oubliez d'attendre est le jour où vous l'apprenez.

## Ce que fait à la place un outil qui connaît les sauvegardes

Hoard capture **après que vous avez arrêté de jouer**, une fois le dossier calmé : un instantané n'est donc jamais un fichier à moitié écrit. Chaque capture est une version de la sauvegarde entière, pas de fichiers isolés, donc restaurer se fait en un clic et remet tout ensemble. Il sait quel dossier appartient à quel jeu — il lit le même manifeste communautaire d'emplacements que partage l'écosystème open source, couvrant plus de 20 000 titres — donc aucun chemin à maintenir, et il suit \`<AppID>/remote/\` plutôt que le dossier au-dessus.

## Quand Syncthing est la meilleure réponse

Pour être juste :

- **Vous l'utilisez déjà**, et ajouter un dossier est gratuit.
- **Vous voulez du pair-à-pair sans aucun serveur**, pas même le vôtre.
- **Vous synchronisez bien plus que des sauvegardes** et préférez un seul outil pour tout.
- **Vous ne revenez jamais en arrière.** Si la dernière sauvegarde vous a toujours suffi, un historique de versions est une mécanique que vous n'utiliserez pas.

## Utiliser les deux

Ils cohabitent sans se battre, et c'est un montage raisonnable : le synchroniseur généraliste s'occupe de vos documents et du reste, un outil qui connaît les sauvegardes s'occupe des dossiers de sauvegarde. La seule règle : ne pointez pas les deux sur le même dossier — deux programmes qui écrivent les mêmes fichiers, c'est fabriquer exactement les conflits que vous vouliez éviter.

## Sans nos serveurs non plus

Si une partie de l'attrait est que rien ne touche le disque d'une entreprise, Hoard se prête au même usage : \`hoard-server\` sur votre PC ou votre NAS, et vos sauvegardes vont de votre machine à votre disque. **Aucun compte chez nous, aucune télémétrie vers nous, aucun relais** : rien ne passe par nos serveurs, puisque rien de chez nous n'est sur le chemin. Voir [comment auto-héberger Hoard](/guides/self-host-hoard).

Le même binaire, la même détection, le même historique. La seule chose qui change, c'est à qui appartient le stockage. Il existe aussi une [comparaison complète des outils de synchro](/guides/game-save-sync-comparison).

<!-- faq -->

## Questions fréquentes

### Syncthing peut-il synchroniser des sauvegardes de jeux ?

Oui, et pour les cas simples il le fait très bien. Les ennuis commencent avec les jeux qui écrivent pendant que vous jouez, les sauvegardes faites de plusieurs fichiers, et tout montage où les deux machines sont modifiées entre deux synchros.

### Que sont les fichiers .sync-conflict dans mon dossier de sauvegardes ?

C'est le synchroniseur qui garde les deux versions après un conflit au lieu d'en choisir une. Rien n'est perdu, mais le jeu ne sait pas les lire, et décider laquelle garder est un travail manuel à chaque fois.

### Pourquoi ma sauvegarde Steam entre-t-elle en conflit à chaque lancement ?

Presque toujours parce que le dossier synchronisé est celui au-dessus de \`remote/\`. Il contient \`remotecache.vdf\` et des fichiers de succès et de temps de jeu qui diffèrent légitimement selon la machine : les deux bouts ne seront jamais d'accord.

### Dois-je fermer le jeu avant de synchroniser ?

Avec un synchroniseur généraliste, oui : c'est l'habitude qui évite les sauvegardes à moitié écrites. Un outil qui connaît les sauvegardes attend tout seul que le dossier se calme.

### Puis-je continuer à utiliser les deux ?

Oui. Ne les pointez simplement pas sur le même dossier, sinon ils se disputeront les mêmes fichiers.
`,$n=`---
title: "Syncthing per i salvataggi: cosa funziona e cosa si rompe"
description: "Syncthing è un ottimo strumento di sincronizzazione generico, ma i salvataggi ne infrangono tre presupposti. Cosa va storto, come ci si arrangia, e quando conviene uno strumento che sa cos'è un salvataggio."
order: 9
updated: 2026-09-01
---

Syncthing è la risposta a cui molti arrivano per primi, e per buone ragioni: è gratuito, open source, peer-to-peer e funziona. Ma i salvataggi infrangono tre presupposti su cui si regge un sincronizzatore generico, e i guasti sono silenziosi. Questa guida parla di cosa va storto davvero, e di quando vale la pena usare qualcosa che sappia cos'è un salvataggio.

## Perché ci si finisce

È software genuinamente buono. Nessun account, nessun abbonamento, i tuoi file non stanno mai sul disco di un'azienda, e sincronizza qualsiasi cosa: documenti, foto, una cartella di salvataggi. Se già lo usi per altro, puntarlo a una cartella di salvataggi ti costa trenta secondi. È un argomento vero, e per certi setup è quello giusto.

## Le tre cose che si rompono

**Sincronizza mentre il gioco è aperto.** Syncthing reagisce al cambiamento di un file, che è il comportamento corretto per un documento. Un gioco scrive il salvataggio a metà sessione, a volte in più passaggi, e un file colto durante la scrittura è un file che si propaga a metà. L'altra macchina si ritrova un salvataggio che il gioco può rifiutarsi di caricare.

**I conflitti diventano file, non decisioni.** Quando entrambe le macchine cambiano lo stesso salvataggio, Syncthing fa la cosa sicura e li tiene entrambi, rinominandone uno in \`qualcosa.sync-conflict-20260901-143022-ABCDEFG.sav\`. Non si perde nulla, ma il gioco non sa cosa sia quel file, e tu finisci a confrontare date in un gestore file per decidere quale pomeriggio di gioco tenere. Ripetilo qualche volta e la cartella si riempie di file di conflitto che nessuno osa cancellare.

**Il versionamento è per file, non per sessione.** Syncthing può conservare copie vecchie in \`.stversions\`, ed è meglio di niente. Ma un salvataggio è spesso fatto di più file che hanno senso solo insieme, e ripristinare significa trovare a mano la data giusta per ciascuno. Non esiste un "rimetti questo gioco com'era martedì".

E un quarto punto, specifico di Steam: se lo punti a \`userdata/<UserID>/<AppID>/\` invece che alla cartella \`remote/\` al suo interno, stai sincronizzando anche \`remotecache.vdf\` e i file di obiettivi e tempo di gioco che **devono** essere diversi tra le macchine. A quel punto ogni avvio sembra un conflitto anche se nessun salvataggio si è mosso. È il motivo più comune per cui un setup artigianale tra Steam Deck e desktop sembra rotto.

## Cosa finisci per costruire

Niente di tutto ciò è irrisolvibile. Ci si arrangia con pattern di esclusione per gioco, una politica di versionamento e l'abitudine di chiudere il gioco e aspettare prima di toccare l'altro PC. Funziona, ed è manutenzione che ti porti dietro per sempre: un gioco nuovo sono percorsi nuovi, e il giorno in cui dimentichi di aspettare è il giorno in cui lo scopri.

## Cosa fa invece uno strumento che conosce i salvataggi

Hoard cattura **dopo che hai smesso di giocare**, quando la cartella si è calmata, quindi uno snapshot non è mai un file scritto a metà. Ogni cattura è una versione dell'intero salvataggio, non dei singoli file, quindi ripristinare è un clic e rimette tutto insieme. Sa quale cartella appartiene a quale gioco — legge lo stesso manifest comunitario delle posizioni condiviso dall'ecosistema open source, oltre 20.000 titoli — quindi non ci sono percorsi da mantenere, e traccia \`<AppID>/remote/\` invece della cartella superiore.

## Quando Syncthing è la risposta migliore

Per essere onesti:

- **Lo hai già in funzione**, e aggiungere una cartella è gratis.
- **Vuoi peer-to-peer senza alcun server**, nemmeno il tuo.
- **Sincronizzi molto più dei salvataggi** e preferisci un solo strumento per tutto.
- **Non torni mai indietro.** Se l'ultimo salvataggio è tutto ciò che ti è servito, una cronologia è macchinario che non userai.

## Usarli entrambi

Convivono senza litigare, ed è un setup ragionevole: il sincronizzatore generico si occupa dei documenti e del resto, uno strumento che conosce i salvataggi si occupa delle cartelle di salvataggio. L'unica regola è non puntarli entrambi alla stessa cartella: due programmi che scrivono gli stessi file sono il modo di fabbricare proprio i conflitti che volevi evitare.

## Nemmeno dai nostri server

Se parte dell'attrattiva è che nulla tocchi il disco di un'azienda, Hoard si può usare allo stesso modo: \`hoard-server\` sul tuo PC o NAS, e i salvataggi vanno dalla tua macchina al tuo disco. **Nessun account con noi, nessuna telemetria verso di noi e nessun relay**: non passa nulla dai nostri server, perché sul percorso non c'è niente di nostro. Vedi [come ospitare Hoard da solo](/guides/self-host-hoard).

Stesso binario, stesso rilevamento, stessa cronologia. L'unica cosa che cambia è di chi è lo spazio di archiviazione. C'è anche un [confronto completo di tutti gli strumenti di sincronizzazione](/guides/game-save-sync-comparison).

<!-- faq -->

## Domande frequenti

### Syncthing può sincronizzare i salvataggi?

Sì, e nei casi semplici lo fa bene. I problemi iniziano con i giochi che scrivono mentre giochi, con i salvataggi fatti di più file e con qualsiasi situazione in cui entrambe le macchine vengano modificate tra una sincronizzazione e l'altra.

### Cosa sono i file .sync-conflict nella mia cartella dei salvataggi?

È il sincronizzatore che dopo un conflitto tiene entrambe le versioni invece di sceglierne una. Non si perde nulla, ma il gioco non sa leggerli, e decidere quale tenere è lavoro manuale ogni volta.

### Perché il mio salvataggio Steam va in conflitto a ogni avvio?

Quasi sempre perché la cartella sincronizzata è quella sopra \`remote/\`. Contiene \`remotecache.vdf\` e file di obiettivi e tempo di gioco che sono legittimamente diversi su ogni macchina, quindi i due capi non andranno mai d'accordo.

### Devo chiudere il gioco prima di sincronizzare?

Con un sincronizzatore generico sì: è l'abitudine che evita i salvataggi scritti a metà. Uno strumento che conosce i salvataggi aspetta da solo che la cartella si calmi.

### Posso continuare a usarli insieme?

Sì. Solo, non puntarli entrambi alla stessa cartella, o si contenderanno gli stessi file.
`,Xn=`---
title: "Syncthing でセーブデータを同期する：うまくいく点と壊れる点"
description: "Syncthing は汎用のファイル同期として非常に優秀ですが、ゲームのセーブはその前提を 3 つ壊します。何が起きるのか、どう回避されているのか、そしてセーブを理解したツールが必要になるのはいつかを説明します。"
order: 9
updated: 2026-09-01
---

Syncthing は多くの人が最初にたどり着く答えで、それには十分な理由があります。無料、オープンソース、ピアツーピアで、ちゃんと動きます。しかしゲームのセーブは、汎用のファイル同期が前提にしていることを 3 つ壊します。しかも壊れ方が静かです。このガイドでは、実際に何が起きるのか、そしてセーブを理解したツールを使う価値が出るのはいつかを扱います。

## なぜそこにたどり着くのか

本当に良いソフトウェアだからです。アカウントも購読もなく、ファイルが企業のディスクに置かれることもなく、何でも同期できます。書類でも、写真でも、セーブのフォルダーでも。すでに別の用途で動かしているなら、セーブのフォルダーを足すのは 30 秒の作業です。これは本物の利点で、構成によってはそれが正解です。

## 壊れる 3 つのこと

**ゲームが動いている最中に同期します。** Syncthing はファイルが変わったことに反応します。書類にとってはそれが正しい振る舞いです。ゲームはセッションの途中で、ときには複数回に分けてセーブを書きます。書き込み途中で捕まえられたファイルは、半端なまま伝播します。もう 1 台には、ゲームが読み込みを拒むかもしれないセーブが残ります。

**競合が「判断」ではなく「ファイル」になります。** 両方のマシンが同じセーブを変更すると、Syncthing は安全側に倒して両方を残し、片方を \`something.sync-conflict-20260901-143022-ABCDEFG.sav\` のような名前に変えます。失われるものはありませんが、ゲームはそのファイルが何なのかを知りません。結局、ファイルマネージャーで日時を見比べて、どちらのプレイを残すか決めることになります。何度か繰り返せば、フォルダーは誰も消す勇気のない競合ファイルで埋まります。

**世代管理はファイル単位で、セッション単位ではありません。** Syncthing は古いコピーを \`.stversions\` に残せます。何もないよりはずっと良い。ただ、セーブはしばしば複数のファイルがそろって初めて意味を持ちます。復元するには、それぞれについて正しい日時を手で探す必要があります。「このゲームを火曜の状態に戻す」に当たるものはありません。

そして 4 つ目、Steam に特有の点です。中の \`remote/\` ではなく \`userdata/<UserID>/<AppID>/\` を指定すると、\`remotecache.vdf\` や、マシンごとに **違って当然の** 実績・プレイ時間のファイルまで同期されます。こうなると、セーブが動いていなくても起動のたびに競合に見えます。自作の Steam Deck とデスクトップの構成が壊れているように感じられる、いちばん多い原因がこれです。

## 結局あなたが組み立てるもの

以上はどれも解決不能ではありません。ゲームごとの除外パターン、世代管理の方針、そして「ゲームを閉じて少し待ってから、もう 1 台に触る」という習慣で、みんな回しています。それで動きますし、その保守はずっとあなたのものです。ゲームが増えればパスが増え、待つのを忘れた日にそれを知ることになります。

## セーブを理解したツールは代わりに何をするか

Hoard は **プレイを終えたあと**、フォルダーが静かになってから取り込みます。だからスナップショットが書き込み途中のファイルになることはありません。取り込みの単位は個々のファイルではなくセーブ全体の 1 世代なので、復元はワンクリックで、まとめて元に戻ります。どのフォルダーがどのゲームのものかも把握しています。オープンソースの世界で共有されている、2 万本以上を収録した同じコミュニティのセーブ位置マニフェストを読むためです。保守するパスはなく、上のフォルダーではなく \`<AppID>/remote/\` を追跡します。

## Syncthing のほうが良い場合

公平に書いておきます。

- **すでに動かしている。** フォルダーを 1 つ足すのは無料です。
- **サーバーをまったく置きたくない。** 自分のものすら含めて、ピアツーピアで済ませたい。
- **セーブ以外もたくさん同期している。** 何でも 1 つのツールで済ませたい。
- **巻き戻したことがない。** 最新のセーブで足りてきたのなら、世代履歴は使わない仕掛けです。

## 両方を使う

両者は衝突せず、これは理にかなった構成です。汎用の同期には書類やその他を任せ、セーブのフォルダーはセーブを理解したツールに任せる。唯一の注意は、両方を同じフォルダーに向けないこと。同じファイルを 2 つのプログラムが書けば、避けたかった競合を自分で作り出すことになります。

## 当方のサーバーも通さずに

「企業のディスクに何も触れさせない」ことが魅力の一部なら、Hoard も同じように使えます。自分の PC や NAS で \`hoard-server\` を動かせば、セーブは自分のマシンから自分のディスクへ移ります。**当方のアカウントも、当方へのテレメトリも、中継もありません。** 経路上に当方のものが何一つないため、当方のサーバーを何も通りません。[Hoard をセルフホストする方法](/guides/self-host-hoard) を参照してください。

同じバイナリ、同じ検出、同じ履歴。変わるのは保存先が誰のものかだけです。[セーブ同期ツールの比較](/guides/game-save-sync-comparison) もあります。

<!-- faq -->

## よくある質問

### Syncthing でセーブデータは同期できますか？

できますし、単純なケースなら問題なく動きます。困り始めるのは、プレイ中に書き込むゲーム、複数ファイルで構成されるセーブ、そして同期のあいだに両方のマシンが編集される構成です。

### セーブのフォルダーにある .sync-conflict ファイルは何ですか？

競合が起きたときに、どちらかを選ばず両方を残した結果です。失われるものはありませんが、ゲームはそれを読めず、どちらを残すかの判断は毎回手作業になります。

### Steam のセーブが起動のたびに競合するのはなぜですか？

ほぼ確実に、同期しているフォルダーが \`remote/\` の 1 つ上だからです。そこには \`remotecache.vdf\` や、マシンごとに違って当然の実績・プレイ時間のファイルが入っているため、両端が一致することはありません。

### 同期の前にゲームを閉じる必要はありますか？

汎用の同期なら必要です。それが書き込み途中のセーブを防ぐ習慣になります。セーブを理解したツールは、フォルダーが静かになるまで自分で待ちます。

### 両方を併用し続けられますか？

はい。ただし両方を同じフォルダーに向けないでください。同じファイルを取り合うことになります。
`,Zn=`---
title: "Syncthing para saves de jogos: o que funciona e o que parte"
description: "O Syncthing é um excelente sincronizador de ficheiros genérico, mas os saves de jogos partem três dos seus pressupostos. O que corre mal, como as pessoas contornam, e quando compensa uma ferramenta que sabe o que é um save."
order: 9
updated: 2026-09-01
---

O Syncthing é a resposta a que muita gente chega primeiro, e com razão: é gratuito, open source, ponto a ponto, e funciona. Mas os saves de jogos partem três dos pressupostos em que assenta um sincronizador genérico, e as falhas são silenciosas. Este guia é sobre o que corre mal a sério, e sobre quando vale a pena usar algo que saiba o que é um save.

## Porque se acaba aí

É software genuinamente bom. Sem conta, sem subscrição, os teus ficheiros nunca ficam no disco de uma empresa, e sincroniza qualquer coisa: documentos, fotos, uma pasta de saves. Se já o tens a correr para outras coisas, apontá-lo a uma pasta de saves custa-te trinta segundos. É um argumento real, e para certas montagens é o correto.

## As três coisas que partem

**Sincroniza com o jogo aberto.** O Syncthing reage à alteração de um ficheiro, que é o comportamento certo para um documento. Um jogo escreve o save a meio da sessão, às vezes em várias passagens, e um ficheiro apanhado a meio da escrita propaga-se incompleto. A outra máquina fica com um save que o jogo pode recusar carregar.

**Os conflitos tornam-se ficheiros, não decisões.** Quando ambas as máquinas mudam o mesmo save, o Syncthing faz o seguro e guarda os dois, renomeando um para \`algo.sync-conflict-20260901-143022-ABCDEFG.sav\`. Não se perde nada, mas o jogo não sabe o que é esse ficheiro, e ficas a comparar datas num explorador para decidir que tarde de jogo manténs. Repete umas quantas vezes e a pasta enche-se de ficheiros de conflito que ninguém se atreve a apagar.

**O versionamento é por ficheiro, não por sessão.** O Syncthing pode guardar cópias antigas em \`.stversions\`, e é melhor do que nada. Mas um save é muitas vezes vários ficheiros que só fazem sentido juntos, e restaurar significa encontrar à mão a data certa de cada um. Não existe um "põe este jogo como estava na terça".

E um quarto ponto, específico da Steam: se o apontares a \`userdata/<UserID>/<AppID>/\` em vez da pasta \`remote/\` lá dentro, também estás a sincronizar \`remotecache.vdf\` e ficheiros de proezas e tempo de jogo que **devem** ser diferentes entre máquinas. A partir daí cada arranque parece um conflito mesmo sem nenhum save se ter mexido. É o motivo mais comum para uma montagem caseira entre Steam Deck e desktop parecer avariada.

## O que acabas por construir

Nada disto é insolúvel. As pessoas safam-se com padrões de exclusão por jogo, uma política de versionamento, e o hábito de fechar o jogo e esperar antes de tocar no outro PC. Funciona, e é manutenção que passa a ser tua para sempre: um jogo novo são caminhos novos, e o dia em que te esqueces de esperar é o dia em que dás por isso.

## O que faz em vez disso uma ferramenta que percebe de saves

O Hoard captura **quando paras de jogar**, assim que a pasta fica quieta, por isso um snapshot nunca é um ficheiro escrito a meio. Cada captura é uma versão do save inteiro, e não de ficheiros soltos, por isso restaurar é um clique e devolve tudo junto. Sabe que pasta é de que jogo — lê o mesmo manifesto comunitário de localizações que o ecossistema open source partilha, com mais de 20.000 títulos — por isso não há caminhos para manter, e segue \`<AppID>/remote/\` em vez da pasta acima.

## Quando o Syncthing é a melhor resposta

Sendo justos:

- **Já o tens a correr**, e acrescentar uma pasta sai grátis.
- **Queres ponto a ponto sem servidor nenhum**, nem sequer o teu.
- **Sincronizas muito mais do que saves** e preferes uma só ferramenta para tudo.
- **Nunca voltas atrás.** Se o último save é tudo o que alguma vez precisaste, um histórico de versões é maquinaria que não vais usar.

## Usar os dois

Convivem sem se atropelar, e é uma montagem razoável: o sincronizador genérico trata dos teus documentos e do resto, e das pastas de saves trata uma ferramenta que as perceba. A única regra é não apontar os dois à mesma pasta — dois programas a escrever os mesmos ficheiros é a forma de fabricar precisamente os conflitos que querias evitar.

## Sem os nossos servidores também

Se parte do apelo é que nada toque no disco de uma empresa, o Hoard pode ser usado da mesma forma: \`hoard-server\` no teu PC ou NAS, e os teus saves vão da tua máquina para o teu disco. **Não há conta connosco, nem telemetria para nós, nem retransmissão**: não passa nada pelos nossos servidores, porque não há nada nosso no caminho. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard).

O mesmo binário, a mesma deteção, o mesmo histórico. A única coisa que muda é de quem é o armazenamento. Há também uma [comparação completa de todas as ferramentas de sincronização](/guides/game-save-sync-comparison).

<!-- faq -->

## Perguntas frequentes

### O Syncthing consegue sincronizar saves de jogos?

Consegue, e em casos simples fá-lo bem. Os problemas começam com jogos que escrevem enquanto jogas, saves feitos de vários ficheiros, e qualquer montagem em que as duas máquinas sejam editadas entre sincronizações.

### O que são os ficheiros .sync-conflict na minha pasta de saves?

É o sincronizador a guardar as duas versões depois de um conflito, em vez de escolher uma. Não se perde nada, mas o jogo não os consegue ler, e decidir qual ficar é trabalho manual de cada vez.

### Porque é que o meu save da Steam dá conflito a cada arranque?

Quase sempre porque a pasta sincronizada é a que está acima de \`remote/\`. Contém \`remotecache.vdf\` e ficheiros de proezas e tempo de jogo que são legitimamente diferentes em cada máquina, por isso as duas pontas nunca coincidem.

### Tenho de fechar o jogo antes de sincronizar?

Com um sincronizador genérico, sim: é esse o hábito que evita saves escritos a meio. Uma ferramenta que percebe de saves espera sozinha que a pasta fique quieta.

### Posso continuar a usar os dois?

Sim. Só não apontes os dois à mesma pasta, ou vão andar à luta pelos mesmos ficheiros.
`,Yn=`---
title: "用 Syncthing 同步游戏存档：哪些可行，哪些会坏"
description: "Syncthing 是出色的通用文件同步工具，但游戏存档打破了它的三个前提。会出什么问题、大家怎么绕过，以及什么时候该换一个懂存档的工具。"
order: 9
updated: 2026-09-01
---

Syncthing 是很多人最先想到的答案，理由也很充分：免费、开源、点对点，而且确实好用。但游戏存档打破了通用同步工具赖以成立的三个前提，而且失败得很安静。本文讲的是实际会出什么问题，以及什么时候值得换一个懂存档是什么的工具。

## 为什么大家会走到这一步

它确实是好软件。没有账号，没有订阅，你的文件从不停留在某家公司的磁盘上，而且什么都能同步：文档、照片、一个存档文件夹。如果你本来就在用它做别的事，再加一个文件夹只花三十秒。这是实打实的理由，对某些配置来说也是正确的选择。

## 会坏掉的三件事

**它会在游戏运行时同步。** Syncthing 对"文件发生变化"作出反应，对文档而言这完全正确。但游戏是在游玩过程中写存档的，有时还分几次写，一个在写入途中被抓到的文件，会以残缺的状态传过去。另一台机器于是拿到一个游戏可能拒绝读取的存档。

**冲突变成了文件，而不是决定。** 当两台机器都改了同一个存档，Syncthing 会做安全的事——两个都留下，把其中一个重命名为 \`something.sync-conflict-20260901-143022-ABCDEFG.sav\`。什么都没丢，但游戏不知道那个文件是什么，于是你只能在文件管理器里比对时间戳，决定保留哪一个下午的游玩。重复几次，文件夹里就堆满了没人敢删的冲突文件。

**版本是按文件算的，不是按一次游玩算的。** Syncthing 可以把旧副本留在 \`.stversions\` 里，这比没有强。但一个存档往往由多个只有凑在一起才有意义的文件组成，恢复就意味着为每一个手动找出正确的时间点。并不存在"把这个游戏恢复到周二的样子"。

还有第四点，是 Steam 独有的：如果你指的是 \`userdata/<UserID>/<AppID>/\` 而不是里面的 \`remote/\`，那你连 \`remotecache.vdf\` 以及那些**本就应该**因机器而异的成就和游戏时长文件也一起同步了。于是每次启动都像冲突，尽管没有任何存档动过。这正是手工搭建的 Steam Deck 与台式机方案让人觉得"坏掉了"的最常见原因。

## 你最后会自己搭出什么

上面这些都不是无解的。大家用逐个游戏的排除规则、一套版本策略，以及"先关游戏、等一会儿再碰另一台 PC"的习惯来应付。这行得通，而这份维护从此归你所有：多一款游戏就是多几条路径，而你忘记等待的那一天，就是你发现问题的那一天。

## 一个懂存档的工具会怎么做

Hoard 在**你停止游玩之后**、文件夹安静下来时才抓取，所以快照永远不会是写到一半的文件。每次抓取都是整个存档的一个版本，而不是单个文件的版本，因此还原只需一次点击，并且是整体复原。它知道哪个文件夹属于哪款游戏——读取开源生态共享的同一份社区存档位置清单，覆盖两万余款游戏——所以没有路径需要你维护，而且它追踪的是 \`<AppID>/remote/\`，不是它上一层。

## 什么时候 Syncthing 才是更好的答案

公平地说：

- **你本来就在跑它**，再加一个文件夹是白得的。
- **你想要完全没有服务器的点对点**，连自己的也不要。
- **你同步的远不止存档**，宁愿一个工具管所有事。
- **你从不回退。** 如果最新存档一直就够用，版本历史就是你用不上的机械。

## 两个一起用

它们可以共存，而且这是个合理的配置：通用同步负责文档和其他一切，懂存档的工具负责存档文件夹。唯一的原则是别把两者指向同一个文件夹——两个程序写同一批文件，正是在亲手制造你想避免的冲突。

## 也不经过我们的服务器

如果吸引你的一部分正是"不让任何东西碰到公司的磁盘"，Hoard 也可以这样用：在自己的 PC 或 NAS 上运行 \`hoard-server\`，存档就从你的机器走到你的磁盘。**没有我们这边的账号，没有发往我们的遥测，也没有中转**——不经过我们的任何服务器，因为这条路径上根本没有我们的东西。参见[如何自托管 Hoard](/guides/self-host-hoard)。

同一个二进制、同样的检测、同样的历史。唯一变化的是存储归谁所有。也可以看[所有存档同步工具的完整比较](/guides/game-save-sync-comparison)。

<!-- faq -->

## 常见问题

### Syncthing 到底能不能同步游戏存档？

能，简单场景下也做得不错。麻烦出在会在游玩过程中写入的游戏、由多个文件组成的存档，以及两台机器在两次同步之间都被改动过的情况。

### 我存档文件夹里的 .sync-conflict 文件是什么？

那是同步工具在冲突后保留了两个版本，而不是替你选一个。什么都没丢，但游戏读不了它们，而且每次都得靠你手动决定留哪个。

### 为什么我的 Steam 存档每次启动都冲突？

几乎总是因为被同步的是 \`remote/\` 的上一层文件夹。它包含 \`remotecache.vdf\` 以及本就应该因机器而异的成就和游戏时长文件，所以两端永远达不成一致。

### 同步前必须先关掉游戏吗？

用通用同步工具的话，是的——正是这个习惯避免了写到一半的存档。懂存档的工具会自己等到文件夹安静下来。

### 我可以两个继续一起用吗？

可以。只是别把两者指向同一个文件夹，否则它们会为同一批文件打架。
`;function $(){return{async:!1,breaks:!1,extensions:null,gfm:!0,hooks:null,pedantic:!1,renderer:null,silent:!1,tokenizer:null,walkTokens:null}}var D=$();function ce(a){D=a}var C={exec:()=>null};function L(a){let e=[];return n=>{let s=Math.max(0,Math.min(3,n-1)),o=e[s];return o||(o=a(s),e[s]=o),o}}function h(a,e=""){let n=typeof a=="string"?a:a.source,s={replace:(o,r)=>{let t=typeof r=="string"?r:r.source;return t=t.replace(S.caret,"$1"),n=n.replace(o,t),s},getRegex:()=>new RegExp(n,e)};return s}var Jn=((a="")=>{try{return!!new RegExp("(?<=1)(?<!1)"+a)}catch{return!1}})(),S={codeRemoveIndent:/^(?: {1,4}| {0,3}\t)/gm,outputLinkReplace:/\\([\[\]])/g,indentCodeCompensation:/^(\s+)(?:```)/,beginningSpace:/^\s+/,endingHash:/#$/,startingSpaceChar:/^ /,endingSpaceChar:/ $/,nonSpaceChar:/[^ ]/,newLineCharGlobal:/\n/g,tabCharGlobal:/\t/g,multipleSpaceGlobal:/\s+/g,blankLine:/^[ \t]*$/,doubleBlankLine:/\n[ \t]*\n[ \t]*$/,blockquoteStart:/^ {0,3}>/,blockquoteSetextReplace:/\n {0,3}((?:=+|-+) *)(?=\n|$)/g,blockquoteSetextReplace2:/^ {0,3}>[ \t]?/gm,listReplaceNesting:/^ {1,4}(?=( {4})*[^ ])/g,listIsTask:/^\[[ xX]\] +\S/,listReplaceTask:/^\[[ xX]\] +/,listTaskCheckbox:/\[[ xX]\]/,anyLine:/\n.*\n/,hrefBrackets:/^<(.*)>$/,tableDelimiter:/[:|]/,tableAlignChars:/^\||\| *$/g,tableRowBlankLine:/\n[ \t]*$/,tableAlignRight:/^ *-+: *$/,tableAlignCenter:/^ *:-+: *$/,tableAlignLeft:/^ *:-+ *$/,startATag:/^<a /i,endATag:/^<\/a>/i,startPreScriptTag:/^<(pre|code|kbd|script)(\s|>)/i,endPreScriptTag:/^<\/(pre|code|kbd|script)(\s|>)/i,startAngleBracket:/^</,endAngleBracket:/>$/,pedanticHrefTitle:/^([^'"]*[^\s])\s+(['"])(.*)\2/,unicodeAlphaNumeric:/[\p{L}\p{N}]/u,escapeTest:/[&<>"']/,escapeReplace:/[&<>"']/g,escapeTestNoEncode:/[<>"']|&(?!(#\d{1,7}|#[Xx][a-fA-F0-9]{1,6}|\w+);)/,escapeReplaceNoEncode:/[<>"']|&(?!(#\d{1,7}|#[Xx][a-fA-F0-9]{1,6}|\w+);)/g,caret:/(^|[^\[])\^/g,percentDecode:/%25/g,findPipe:/\|/g,splitPipe:/ \|/,slashPipe:/\\\|/g,carriageReturn:/\r\n|\r/g,spaceLine:/^ +$/gm,notSpaceStart:/^\S*/,endingNewline:/\n$/,listItemRegex:a=>new RegExp(`^( {0,3}${a})((?:[	 ][^\\n]*)?(?:\\n|$))`),nextBulletRegex:L(a=>new RegExp(`^ {0,${a}}(?:[*+-]|\\d{1,9}[.)])((?:[ 	][^\\n]*)?(?:\\n|$))`)),hrRegex:L(a=>new RegExp(`^ {0,${a}}((?:- *){3,}|(?:_ *){3,}|(?:\\* *){3,})(?:\\n+|$)`)),fencesBeginRegex:L(a=>new RegExp(`^ {0,${a}}(?:\`\`\`|~~~)`)),headingBeginRegex:L(a=>new RegExp(`^ {0,${a}}#`)),htmlBeginRegex:L(a=>new RegExp(`^ {0,${a}}<(?:[a-z].*>|!--)`,"i")),blockquoteBeginRegex:L(a=>new RegExp(`^ {0,${a}}>`))},ea=/^(?:[ \t]*(?:\n|$))+/,na=/^((?: {4}| {0,3}\t)[^\n]+(?:\n(?:[ \t]*(?:\n|$))*)?)+/,aa=/^ {0,3}(`{3,}(?=[^`\n]*(?:\n|$))|~{3,})([^\n]*)(?:\n|$)(?:|([\s\S]*?)(?:\n|$))(?: {0,3}\1[~`]* *(?=\n|$)|$)/,E=/^ {0,3}((?:-[\t ]*){3,}|(?:_[ \t]*){3,}|(?:\*[ \t]*){3,})(?:\n+|$)/,oa=/^ {0,3}(#{1,6})(?=\s|$)(.*)(?:\n+|$)/,X=/ {0,3}(?:[*+-]|\d{1,9}[.)])/,me=/^(?!bull |blockCode|fences|blockquote|heading|html|table)((?:.|\n(?!\s*?\n|bull |blockCode|fences|blockquote|heading|html|table))+?)\n {0,3}(=+|-+) *(?:\n+|$)/,pe=h(me).replace(/bull/g,X).replace(/blockCode/g,/(?: {4}| {0,3}\t)/).replace(/fences/g,/ {0,3}(?:`{3,}|~{3,})/).replace(/blockquote/g,/ {0,3}>/).replace(/heading/g,/ {0,3}#{1,6}/).replace(/html/g,/ {0,3}<[^\n>]+>\n/).replace(/\|table/g,"").getRegex(),sa=h(me).replace(/bull/g,X).replace(/blockCode/g,/(?: {4}| {0,3}\t)/).replace(/fences/g,/ {0,3}(?:`{3,}|~{3,})/).replace(/blockquote/g,/ {0,3}>/).replace(/heading/g,/ {0,3}#{1,6}/).replace(/html/g,/ {0,3}<[^\n>]+>\n/).replace(/table/g,/ {0,3}\|?(?:[:\- ]*\|)+[\:\- ]*\n/).getRegex(),Z=/^([^\n]+(?:\n(?!hr|heading|lheading|blockquote|fences|list|html|table| +\n)[^\n]+)*)/,ia=/^[^\n]+/,Y=/(?!\s*\])(?:\\[\s\S]|[^\[\]\\])+/,ra=h(/^ {0,3}\[(label)\]: *(?:\n[ \t]*)?([^<\s][^\s]*|<.*?>)(?:(?: +(?:\n[ \t]*)?| *\n[ \t]*)(title))? *(?:\n+|$)/).replace("label",Y).replace("title",/(?:"(?:\\"?|[^"\\])*"|'[^'\n]*(?:\n[^'\n]+)*\n?'|\([^()]*\))/).getRegex(),ta=h(/^(bull)([ \t][^\n]*?)?(?:\n|$)/).replace(/bull/g,X).getRegex(),B="address|article|aside|base|basefont|blockquote|body|caption|center|col|colgroup|dd|details|dialog|dir|div|dl|dt|fieldset|figcaption|figure|footer|form|frame|frameset|h[1-6]|head|header|hr|html|iframe|legend|li|link|main|menu|menuitem|meta|nav|noframes|ol|optgroup|option|p|param|search|section|summary|table|tbody|td|tfoot|th|thead|title|tr|track|ul",J=/<!--(?:-?>|[\s\S]*?(?:-->|$))/,ua=h("^ {0,3}(?:<(script|pre|style|textarea)[\\s>][\\s\\S]*?(?:</\\1>[^\\n]*\\n+|$)|comment[^\\n]*(\\n+|$)|<\\?[\\s\\S]*?(?:\\?>\\n*|$)|<![A-Z][\\s\\S]*?(?:>\\n*|$)|<!\\[CDATA\\[[\\s\\S]*?(?:\\]\\]>\\n*|$)|</?(tag)(?: +|\\n|/?>)[\\s\\S]*?(?:(?:\\n[ 	]*)+\\n|$)|<(?!script|pre|style|textarea)([a-z][\\w-]*)(?:attribute)*? */?>(?=[ \\t]*(?:\\n|$))[\\s\\S]*?(?:(?:\\n[ 	]*)+\\n|$)|</(?!script|pre|style|textarea)[a-z][\\w-]*\\s*>(?=[ \\t]*(?:\\n|$))[\\s\\S]*?(?:(?:\\n[ 	]*)+\\n|$))","i").replace("comment",J).replace("tag",B).replace("attribute",/ +[a-zA-Z:_][\w.:-]*(?: *= *"[^"\n]*"| *= *'[^'\n]*'| *= *[^\s"'=<>`]+)?/).getRegex(),he=h(Z).replace("hr",E).replace("heading"," {0,3}#{1,6}(?:\\s|$)").replace("|lheading","").replace("|table","").replace("blockquote"," {0,3}>").replace("fences"," {0,3}(?:`{3,}(?=[^`\\n]*\\n)|~{3,})[^\\n]*\\n").replace("list"," {0,3}(?:[*+-]|1[.)])[ \\t]+[^ \\t\\n]").replace("html","</?(?:tag)(?: +|\\n|/?>)|<(?:script|pre|style|textarea|!--)").replace("tag",B).getRegex(),da=h(/^( {0,3}> ?(paragraph|[^\n]*)(?:\n|$))+/).replace("paragraph",he).getRegex(),ee={blockquote:da,code:na,def:ra,fences:aa,heading:oa,hr:E,html:ua,lheading:pe,list:ta,newline:ea,paragraph:he,table:C,text:ia},se=h("^ *([^\\n ].*)\\n {0,3}((?:\\| *)?:?-+:? *(?:\\| *:?-+:? *)*(?:\\| *)?)(?:\\n((?:(?! *\\n|hr|heading|blockquote|code|fences|list|html).*(?:\\n|$))*)\\n*|$)").replace("hr",E).replace("heading"," {0,3}#{1,6}(?:\\s|$)").replace("blockquote"," {0,3}>").replace("code","(?: {4}| {0,3}	)[^\\n]").replace("fences"," {0,3}(?:`{3,}(?=[^`\\n]*\\n)|~{3,})[^\\n]*\\n").replace("list"," {0,3}(?:[*+-]|1[.)])[ \\t]").replace("html","</?(?:tag)(?: +|\\n|/?>)|<(?:script|pre|style|textarea|!--)").replace("tag",B).getRegex(),la={...ee,lheading:sa,table:se,paragraph:h(Z).replace("hr",E).replace("heading"," {0,3}#{1,6}(?:\\s|$)").replace("|lheading","").replace("table",se).replace("blockquote"," {0,3}>").replace("fences"," {0,3}(?:`{3,}(?=[^`\\n]*\\n)|~{3,})[^\\n]*\\n").replace("list"," {0,3}(?:[*+-]|1[.)])[ \\t]+[^ \\t\\n]").replace("html","</?(?:tag)(?: +|\\n|/?>)|<(?:script|pre|style|textarea|!--)").replace("tag",B).getRegex()},ca={...ee,html:h(`^ *(?:comment *(?:\\n|\\s*$)|<(tag)[\\s\\S]+?</\\1> *(?:\\n{2,}|\\s*$)|<tag(?:"[^"]*"|'[^']*'|\\s[^'"/>\\s]*)*?/?> *(?:\\n{2,}|\\s*$))`).replace("comment",J).replace(/tag/g,"(?!(?:a|em|strong|small|s|cite|q|dfn|abbr|data|time|code|var|samp|kbd|sub|sup|i|b|u|mark|ruby|rt|rp|bdi|bdo|span|br|wbr|ins|del|img)\\b)\\w+(?!:|[^\\w\\s@]*@)\\b").getRegex(),def:/^ *\[([^\]]+)\]: *<?([^\s>]+)>?(?: +(["(][^\n]+[")]))? *(?:\n+|$)/,heading:/^(#{1,6})(.*)(?:\n+|$)/,fences:C,lheading:/^(.+?)\n {0,3}(=+|-+) *(?:\n+|$)/,paragraph:h(Z).replace("hr",E).replace("heading",` *#{1,6} *[^
]`).replace("lheading",pe).replace("|table","").replace("blockquote"," {0,3}>").replace("|fences","").replace("|list","").replace("|html","").replace("|tag","").getRegex()},ma=/^\\([!"#$%&'()*+,\-./:;<=>?@\[\]\\^_`{|}~])/,pa=/^(`+)([^`]|[^`][\s\S]*?[^`])\1(?!`)/,ve=/^( {2,}|\\)\n(?!\s*$)/,ha=/^(`+|[^`])(?:(?= {2,}\n)|[\s\S]*?(?:(?=[\\<!\[`*_]|\b_|$)|[^ ](?= {2,}\n)))/,A=/[\p{P}\p{S}]/u,M=/[\s\p{P}\p{S}]/u,ne=/[^\s\p{P}\p{S}]/u,va=h(/^((?![*_])punctSpace)/,"u").replace(/punctSpace/g,M).getRegex(),ge=/(?!~)[\p{P}\p{S}]/u,ga=/(?!~)[\s\p{P}\p{S}]/u,fa=/(?:[^\s\p{P}\p{S}]|~)/u,ba=h(/link|precode-code|html/,"g").replace("link",/\[(?:[^\[\]`]|(?<a>`+)[^`]+\k<a>(?!`))*?\]\((?:\\[\s\S]|[^\\\(\)]|\((?:\\[\s\S]|[^\\\(\)])*\))*\)/).replace("precode-",Jn?"(?<!`)()":"(^^|[^`])").replace("code",/(?<b>`+)[^`]+\k<b>(?!`)/).replace("html",/<(?! )[^<>]*?>/).getRegex(),fe=/^(?:\*+(?:((?!\*)punct)|([^\s*]))?)|^_+(?:((?!_)punct)|([^\s_]))?/,Sa=h(fe,"u").replace(/punct/g,A).getRegex(),ya=h(fe,"u").replace(/punct/g,ge).getRegex(),be="^[^_*]*?__[^_*]*?\\*[^_*]*?(?=__)|[^*]+(?=[^*])|(?!\\*)punct(\\*+)(?=[\\s]|$)|notPunctSpace(\\*+)(?!\\*)(?=punctSpace|$)|(?!\\*)punctSpace(\\*+)(?=notPunctSpace)|[\\s](\\*+)(?!\\*)(?=punct)|(?!\\*)punct(\\*+)(?!\\*)(?=punct)|notPunctSpace(\\*+)(?=notPunctSpace)",ka=h(be,"gu").replace(/notPunctSpace/g,ne).replace(/punctSpace/g,M).replace(/punct/g,A).getRegex(),qa=h(be,"gu").replace(/notPunctSpace/g,fa).replace(/punctSpace/g,ga).replace(/punct/g,ge).getRegex(),za=h("^[^_*]*?\\*\\*[^_*]*?_[^_*]*?(?=\\*\\*)|[^_]+(?=[^_])|(?!_)punct(_+)(?=[\\s]|$)|notPunctSpace(_+)(?!_)(?=punctSpace|$)|(?!_)punctSpace(_+)(?=notPunctSpace)|[\\s](_+)(?!_)(?=punct)|(?!_)punct(_+)(?!_)(?=punct)","gu").replace(/notPunctSpace/g,ne).replace(/punctSpace/g,M).replace(/punct/g,A).getRegex(),wa=h(/^~~?(?:((?!~)punct)|[^\s~])/,"u").replace(/punct/g,A).getRegex(),Ha="^[^~]+(?=[^~])|(?!~)punct(~~?)(?=[\\s]|$)|notPunctSpace(~~?)(?!~)(?=punctSpace|$)|(?!~)punctSpace(~~?)(?=notPunctSpace)|[\\s](~~?)(?!~)(?=punct)|(?!~)punct(~~?)(?!~)(?=punct)|notPunctSpace(~~?)(?=notPunctSpace)",Ca=h(Ha,"gu").replace(/notPunctSpace/g,ne).replace(/punctSpace/g,M).replace(/punct/g,A).getRegex(),Pa=h(/\\(punct)/,"gu").replace(/punct/g,A).getRegex(),Da=h(/^<(scheme:[^\s\x00-\x1f<>]*|email)>/).replace("scheme",/[a-zA-Z][a-zA-Z0-9+.-]{1,31}/).replace("email",/[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+(@)[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)+(?![-_])/).getRegex(),La=h(J).replace("(?:-->|$)","-->").getRegex(),xa=h("^comment|^</[a-zA-Z][\\w:-]*\\s*>|^<[a-zA-Z][\\w-]*(?:attribute)*?\\s*/?>|^<\\?[\\s\\S]*?\\?>|^<![a-zA-Z]+\\s[\\s\\S]*?>|^<!\\[CDATA\\[[\\s\\S]*?\\]\\]>").replace("comment",La).replace("attribute",/\s+[a-zA-Z:_][\w.:-]*(?:\s*=\s*"[^"]*"|\s*=\s*'[^']*'|\s*=\s*[^\s"'=<>`]+)?/).getRegex(),_=/(?:\[(?:\\[\s\S]|[^\[\]\\])*\]|\\[\s\S]|`+(?!`)[^`]*?`+(?!`)|``+(?=\])|[^\[\]\\`])*?/,Aa=h(/^!?\[(label)\]\(\s*(href)(?:(?:[ \t]+(?:\n[ \t]*)?|\n[ \t]*)(title))?\s*\)/).replace("label",_).replace("href",/<(?:\\.|[^\n<>\\])+>|[^ \t\n\x00-\x1f]*/).replace("title",/"(?:\\"?|[^"\\])*"|'(?:\\'?|[^'\\])*'|\((?:\\\)?|[^)\\])*\)/).getRegex(),Se=h(/^!?\[(label)\]\[(ref)\]/).replace("label",_).replace("ref",Y).getRegex(),ye=h(/^!?\[(ref)\](?:\[\])?/).replace("ref",Y).getRegex(),ja=h("reflink|nolink(?!\\()","g").replace("reflink",Se).replace("nolink",ye).getRegex(),ie=/[hH][tT][tT][pP][sS]?|[fF][tT][pP]/,ae={_backpedal:C,anyPunctuation:Pa,autolink:Da,blockSkip:ba,br:ve,code:pa,del:C,delLDelim:C,delRDelim:C,emStrongLDelim:Sa,emStrongRDelimAst:ka,emStrongRDelimUnd:za,escape:ma,link:Aa,nolink:ye,punctuation:va,reflink:Se,reflinkSearch:ja,tag:xa,text:ha,url:C},Oa={...ae,link:h(/^!?\[(label)\]\((.*?)\)/).replace("label",_).getRegex(),reflink:h(/^!?\[(label)\]\s*\[([^\]]*)\]/).replace("label",_).getRegex()},F={...ae,emStrongRDelimAst:qa,emStrongLDelim:ya,delLDelim:wa,delRDelim:Ca,url:h(/^((?:protocol):\/\/|www\.)(?:[a-zA-Z0-9\-]+\.?)+[^\s<]*|^email/).replace("protocol",ie).replace("email",/[A-Za-z0-9._+-]+(@)[a-zA-Z0-9-_]+(?:\.[a-zA-Z0-9-_]*[a-zA-Z0-9])+(?![-_])/).getRegex(),_backpedal:/(?:[^?!.,:;*_'"~()&]+|\([^)]*\)|&(?![a-zA-Z0-9]+;$)|[?!.,:;*_'"~)]+(?!$))+/,del:/^(~~?)(?=[^\s~])((?:\\[\s\S]|[^\\])*?(?:\\[\s\S]|[^\s~\\]))\1(?=[^~]|$)/,text:h(/^([`~]+|[^`~])(?:(?= {2,}\n)|(?=[a-zA-Z0-9.!#$%&'*+\/=?_`{\|}~-]+@)|[\s\S]*?(?:(?=[\\<!\[`*~_]|\b_|protocol:\/\/|www\.|$)|[^ ](?= {2,}\n)|[^a-zA-Z0-9.!#$%&'*+\/=?_`{\|}~-](?=[a-zA-Z0-9.!#$%&'*+\/=?_`{\|}~-]+@)))/).replace("protocol",ie).getRegex()},Ga={...F,br:h(ve).replace("{2,}","*").getRegex(),text:h(F.text).replace("\\b_","\\b_| {2,}\\n").replace(/\{2,\}/g,"*").getRegex()},R={normal:ee,gfm:la,pedantic:ca},O={normal:ae,gfm:F,breaks:Ga,pedantic:Oa},Ea={"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"},re=a=>Ea[a];function z(a,e){if(e){if(S.escapeTest.test(a))return a.replace(S.escapeReplace,re)}else if(S.escapeTestNoEncode.test(a))return a.replace(S.escapeReplaceNoEncode,re);return a}function te(a){try{a=encodeURI(a).replace(S.percentDecode,"%")}catch{return null}return a}function ue(a,e){var r;let n=a.replace(S.findPipe,(t,u,i)=>{let l=!1,d=u;for(;--d>=0&&i[d]==="\\";)l=!l;return l?"|":" |"}),s=n.split(S.splitPipe),o=0;if(s[0].trim()||s.shift(),s.length>0&&!((r=s.at(-1))!=null&&r.trim())&&s.pop(),e)if(s.length>e)s.splice(e);else for(;s.length<e;)s.push("");for(;o<s.length;o++)s[o]=s[o].trim().replace(S.slashPipe,"|");return s}function w(a,e,n){let s=a.length;if(s===0)return"";let o=0;for(;o<s&&a.charAt(s-o-1)===e;)o++;return a.slice(0,s-o)}function de(a){let e=a.split(`
`),n=e.length-1;for(;n>=0&&S.blankLine.test(e[n]);)n--;return e.length-n<=2?a:e.slice(0,n+1).join(`
`)}function Ia(a,e){if(a.indexOf(e[1])===-1)return-1;let n=0;for(let s=0;s<a.length;s++)if(a[s]==="\\")s++;else if(a[s]===e[0])n++;else if(a[s]===e[1]&&(n--,n<0))return s;return n>0?-2:-1}function Ra(a,e=0){let n=e,s="";for(let o of a)if(o==="	"){let r=4-n%4;s+=" ".repeat(r),n+=r}else s+=o,n++;return s}function le(a,e,n,s,o){let r=e.href,t=e.title||null,u=a[1].replace(o.other.outputLinkReplace,"$1");s.state.inLink=!0;let i={type:a[0].charAt(0)==="!"?"image":"link",raw:n,href:r,title:t,text:u,tokens:s.inlineTokens(u)};return s.state.inLink=!1,i}function Ta(a,e,n){let s=a.match(n.other.indentCodeCompensation);if(s===null)return e;let o=s[1];return e.split(`
`).map(r=>{let t=r.match(n.other.beginningSpace);if(t===null)return r;let[u]=t;return u.length>=o.length?r.slice(o.length):r}).join(`
`)}var W=class{constructor(a){g(this,"options");g(this,"rules");g(this,"lexer");this.options=a||D}space(a){let e=this.rules.block.newline.exec(a);if(e&&e[0].length>0)return{type:"space",raw:e[0]}}code(a){let e=this.rules.block.code.exec(a);if(e){let n=this.options.pedantic?e[0]:de(e[0]),s=n.replace(this.rules.other.codeRemoveIndent,"");return{type:"code",raw:n,codeBlockStyle:"indented",text:s}}}fences(a){let e=this.rules.block.fences.exec(a);if(e){let n=e[0],s=Ta(n,e[3]||"",this.rules);return{type:"code",raw:n,lang:e[2]?e[2].trim().replace(this.rules.inline.anyPunctuation,"$1"):e[2],text:s}}}heading(a){let e=this.rules.block.heading.exec(a);if(e){let n=e[2].trim();if(this.rules.other.endingHash.test(n)){let s=w(n,"#");(this.options.pedantic||!s||this.rules.other.endingSpaceChar.test(s))&&(n=s.trim())}return{type:"heading",raw:w(e[0],`
`),depth:e[1].length,text:n,tokens:this.lexer.inline(n)}}}hr(a){let e=this.rules.block.hr.exec(a);if(e)return{type:"hr",raw:w(e[0],`
`)}}blockquote(a){let e=this.rules.block.blockquote.exec(a);if(e){let n=w(e[0],`
`).split(`
`),s="",o="",r=[];for(;n.length>0;){let t=!1,u=[],i;for(i=0;i<n.length;i++)if(this.rules.other.blockquoteStart.test(n[i]))u.push(n[i]),t=!0;else if(!t)u.push(n[i]);else break;n=n.slice(i);let l=u.join(`
`),d=l.replace(this.rules.other.blockquoteSetextReplace,`
    $1`).replace(this.rules.other.blockquoteSetextReplace2,"");s=s?`${s}
${l}`:l,o=o?`${o}
${d}`:d;let m=this.lexer.state.top;if(this.lexer.state.top=!0,this.lexer.blockTokens(d,r,!0),this.lexer.state.top=m,n.length===0)break;let p=r.at(-1);if((p==null?void 0:p.type)==="code")break;if((p==null?void 0:p.type)==="blockquote"){let b=p,c=b.raw+`
`+n.join(`
`),y=this.blockquote(c);r[r.length-1]=y,s=s.substring(0,s.length-b.raw.length)+y.raw,o=o.substring(0,o.length-b.text.length)+y.text;break}else if((p==null?void 0:p.type)==="list"){let b=p,c=b.raw+`
`+n.join(`
`),y=this.list(c);r[r.length-1]=y,s=s.substring(0,s.length-p.raw.length)+y.raw,o=o.substring(0,o.length-b.raw.length)+y.raw,n=c.substring(r.at(-1).raw.length).split(`
`);continue}}return{type:"blockquote",raw:s,tokens:r,text:o}}}list(a){let e=this.rules.block.list.exec(a);if(e){let n=e[1].trim(),s=n.length>1,o={type:"list",raw:"",ordered:s,start:s?+n.slice(0,-1):"",loose:!1,items:[]};n=s?`\\d{1,9}\\${n.slice(-1)}`:`\\${n}`,this.options.pedantic&&(n=s?n:"[*+-]");let r=this.rules.other.listItemRegex(n),t=!1;for(;a;){let i=!1,l="",d="";if(!(e=r.exec(a))||this.rules.block.hr.test(a))break;l=e[0],a=a.substring(l.length);let m=Ra(e[2].split(`
`,1)[0],e[1].length),p=a.split(`
`,1)[0],b=!m.trim(),c=0;if(this.options.pedantic?(c=2,d=m.trimStart()):b?c=e[1].length+1:(c=m.search(this.rules.other.nonSpaceChar),c=c>4?1:c,d=m.slice(c),c+=e[1].length),b&&this.rules.other.blankLine.test(p)&&(l+=p+`
`,a=a.substring(p.length+1),i=!0),!i){let y=this.rules.other.nextBulletRegex(c),f=this.rules.other.hrRegex(c),I=this.rules.other.fencesBeginRegex(c),H=this.rules.other.headingBeginRegex(c),U=this.rules.other.htmlBeginRegex(c),ke=this.rules.other.blockquoteBeginRegex(c);for(;a;){let V=a.split(`
`,1)[0],j;if(p=V,this.options.pedantic?(p=p.replace(this.rules.other.listReplaceNesting,"  "),j=p):j=p.replace(this.rules.other.tabCharGlobal,"    "),I.test(p)||H.test(p)||U.test(p)||ke.test(p)||y.test(p)||f.test(p))break;if(j.search(this.rules.other.nonSpaceChar)>=c||!p.trim())d+=`
`+j.slice(c);else{if(b||m.replace(this.rules.other.tabCharGlobal,"    ").search(this.rules.other.nonSpaceChar)>=4||I.test(m)||H.test(m)||f.test(m))break;d+=`
`+p}b=!p.trim(),l+=V+`
`,a=a.substring(V.length+1),m=j.slice(c)}}o.loose||(t?o.loose=!0:this.rules.other.doubleBlankLine.test(l)&&(t=!0)),o.items.push({type:"list_item",raw:l,task:!!this.options.gfm&&this.rules.other.listIsTask.test(d),loose:!1,text:d,tokens:[]}),o.raw+=l}let u=o.items.at(-1);if(u)u.raw=u.raw.trimEnd(),u.text=u.text.trimEnd();else return;o.raw=o.raw.trimEnd();for(let i of o.items){this.lexer.state.top=!1,i.tokens=this.lexer.blockTokens(i.text,[]);let l=i.tokens[0];if(i.task&&((l==null?void 0:l.type)==="text"||(l==null?void 0:l.type)==="paragraph")){i.text=i.text.replace(this.rules.other.listReplaceTask,""),l.raw=l.raw.replace(this.rules.other.listReplaceTask,""),l.text=l.text.replace(this.rules.other.listReplaceTask,"");for(let m=this.lexer.inlineQueue.length-1;m>=0;m--)if(this.rules.other.listIsTask.test(this.lexer.inlineQueue[m].src)){this.lexer.inlineQueue[m].src=this.lexer.inlineQueue[m].src.replace(this.rules.other.listReplaceTask,"");break}let d=this.rules.other.listTaskCheckbox.exec(i.raw);if(d){let m={type:"checkbox",raw:d[0]+" ",checked:d[0]!=="[ ]"};i.checked=m.checked,o.loose?i.tokens[0]&&["paragraph","text"].includes(i.tokens[0].type)&&"tokens"in i.tokens[0]&&i.tokens[0].tokens?(i.tokens[0].raw=m.raw+i.tokens[0].raw,i.tokens[0].text=m.raw+i.tokens[0].text,i.tokens[0].tokens.unshift(m)):i.tokens.unshift({type:"paragraph",raw:m.raw,text:m.raw,tokens:[m]}):i.tokens.unshift(m)}}else i.task&&(i.task=!1);if(!o.loose){let d=i.tokens.filter(p=>p.type==="space"),m=d.length>0&&d.some(p=>this.rules.other.anyLine.test(p.raw));o.loose=m}}if(o.loose)for(let i of o.items){i.loose=!0;for(let l of i.tokens)l.type==="text"&&(l.type="paragraph")}return o}}html(a){let e=this.rules.block.html.exec(a);if(e){let n=de(e[0]);return{type:"html",block:!0,raw:n,pre:e[1]==="pre"||e[1]==="script"||e[1]==="style",text:n}}}def(a){let e=this.rules.block.def.exec(a);if(e){let n=e[1].toLowerCase().replace(this.rules.other.multipleSpaceGlobal," "),s=e[2]?e[2].replace(this.rules.other.hrefBrackets,"$1").replace(this.rules.inline.anyPunctuation,"$1"):"",o=e[3]?e[3].substring(1,e[3].length-1).replace(this.rules.inline.anyPunctuation,"$1"):e[3];return{type:"def",tag:n,raw:w(e[0],`
`),href:s,title:o}}}table(a){var t;let e=this.rules.block.table.exec(a);if(!e||!this.rules.other.tableDelimiter.test(e[2]))return;let n=ue(e[1]),s=e[2].replace(this.rules.other.tableAlignChars,"").split("|"),o=(t=e[3])!=null&&t.trim()?e[3].replace(this.rules.other.tableRowBlankLine,"").split(`
`):[],r={type:"table",raw:w(e[0],`
`),header:[],align:[],rows:[]};if(n.length===s.length){for(let u of s)this.rules.other.tableAlignRight.test(u)?r.align.push("right"):this.rules.other.tableAlignCenter.test(u)?r.align.push("center"):this.rules.other.tableAlignLeft.test(u)?r.align.push("left"):r.align.push(null);for(let u=0;u<n.length;u++)r.header.push({text:n[u],tokens:this.lexer.inline(n[u]),header:!0,align:r.align[u]});for(let u of o)r.rows.push(ue(u,r.header.length).map((i,l)=>({text:i,tokens:this.lexer.inline(i),header:!1,align:r.align[l]})));return r}}lheading(a){let e=this.rules.block.lheading.exec(a);if(e){let n=e[1].trim();return{type:"heading",raw:w(e[0],`
`),depth:e[2].charAt(0)==="="?1:2,text:n,tokens:this.lexer.inline(n)}}}paragraph(a){let e=this.rules.block.paragraph.exec(a);if(e){let n=e[1].charAt(e[1].length-1)===`
`?e[1].slice(0,-1):e[1];return{type:"paragraph",raw:e[0],text:n,tokens:this.lexer.inline(n)}}}text(a){let e=this.rules.block.text.exec(a);if(e)return{type:"text",raw:e[0],text:e[0],tokens:this.lexer.inline(e[0])}}escape(a){let e=this.rules.inline.escape.exec(a);if(e)return{type:"escape",raw:e[0],text:e[1]}}tag(a){let e=this.rules.inline.tag.exec(a);if(e)return!this.lexer.state.inLink&&this.rules.other.startATag.test(e[0])?this.lexer.state.inLink=!0:this.lexer.state.inLink&&this.rules.other.endATag.test(e[0])&&(this.lexer.state.inLink=!1),!this.lexer.state.inRawBlock&&this.rules.other.startPreScriptTag.test(e[0])?this.lexer.state.inRawBlock=!0:this.lexer.state.inRawBlock&&this.rules.other.endPreScriptTag.test(e[0])&&(this.lexer.state.inRawBlock=!1),{type:"html",raw:e[0],inLink:this.lexer.state.inLink,inRawBlock:this.lexer.state.inRawBlock,block:!1,text:e[0]}}link(a){let e=this.rules.inline.link.exec(a);if(e){let n=e[2].trim();if(!this.options.pedantic&&this.rules.other.startAngleBracket.test(n)){if(!this.rules.other.endAngleBracket.test(n))return;let r=w(n.slice(0,-1),"\\");if((n.length-r.length)%2===0)return}else{let r=Ia(e[2],"()");if(r===-2)return;if(r>-1){let t=(e[0].indexOf("!")===0?5:4)+e[1].length+r;e[2]=e[2].substring(0,r),e[0]=e[0].substring(0,t).trim(),e[3]=""}}let s=e[2],o="";if(this.options.pedantic){let r=this.rules.other.pedanticHrefTitle.exec(s);r&&(s=r[1],o=r[3])}else o=e[3]?e[3].slice(1,-1):"";return s=s.trim(),this.rules.other.startAngleBracket.test(s)&&(this.options.pedantic&&!this.rules.other.endAngleBracket.test(n)?s=s.slice(1):s=s.slice(1,-1)),le(e,{href:s&&s.replace(this.rules.inline.anyPunctuation,"$1"),title:o&&o.replace(this.rules.inline.anyPunctuation,"$1")},e[0],this.lexer,this.rules)}}reflink(a,e){let n;if((n=this.rules.inline.reflink.exec(a))||(n=this.rules.inline.nolink.exec(a))){let s=(n[2]||n[1]).replace(this.rules.other.multipleSpaceGlobal," "),o=e[s.toLowerCase()];if(!o){let r=n[0].charAt(0);return{type:"text",raw:r,text:r}}return le(n,o,n[0],this.lexer,this.rules)}}emStrong(a,e,n=""){let s=this.rules.inline.emStrongLDelim.exec(a);if(!(!s||!s[1]&&!s[2]&&!s[3]&&!s[4]||s[4]&&n.match(this.rules.other.unicodeAlphaNumeric))&&(!(s[1]||s[3])||!n||this.rules.inline.punctuation.exec(n))){let o=[...s[0]].length-1,r,t,u=o,i=0,l=s[0][0]==="*"?this.rules.inline.emStrongRDelimAst:this.rules.inline.emStrongRDelimUnd;for(l.lastIndex=0,e=e.slice(-1*a.length+o);(s=l.exec(e))!==null;){if(r=s[1]||s[2]||s[3]||s[4]||s[5]||s[6],!r)continue;if(t=[...r].length,s[3]||s[4]){u+=t;continue}else if((s[5]||s[6])&&o%3&&!((o+t)%3)){i+=t;continue}if(u-=t,u>0)continue;t=Math.min(t,t+u+i);let d=[...s[0]][0].length,m=a.slice(0,o+s.index+d+t);if(Math.min(o,t)%2){let b=m.slice(1,-1);return{type:"em",raw:m,text:b,tokens:this.lexer.inlineTokens(b)}}let p=m.slice(2,-2);return{type:"strong",raw:m,text:p,tokens:this.lexer.inlineTokens(p)}}}}codespan(a){let e=this.rules.inline.code.exec(a);if(e){let n=e[2].replace(this.rules.other.newLineCharGlobal," "),s=this.rules.other.nonSpaceChar.test(n),o=this.rules.other.startingSpaceChar.test(n)&&this.rules.other.endingSpaceChar.test(n);return s&&o&&(n=n.substring(1,n.length-1)),{type:"codespan",raw:e[0],text:n}}}br(a){let e=this.rules.inline.br.exec(a);if(e)return{type:"br",raw:e[0]}}del(a,e,n=""){let s=this.rules.inline.delLDelim.exec(a);if(s&&(!s[1]||!n||this.rules.inline.punctuation.exec(n))){let o=[...s[0]].length-1,r,t,u=o,i=this.rules.inline.delRDelim;for(i.lastIndex=0,e=e.slice(-1*a.length+o);(s=i.exec(e))!==null;){if(r=s[1]||s[2]||s[3]||s[4]||s[5]||s[6],!r||(t=[...r].length,t!==o))continue;if(s[3]||s[4]){u+=t;continue}if(u-=t,u>0)continue;t=Math.min(t,t+u);let l=[...s[0]][0].length,d=a.slice(0,o+s.index+l+t),m=d.slice(o,-o);return{type:"del",raw:d,text:m,tokens:this.lexer.inlineTokens(m)}}}}autolink(a){let e=this.rules.inline.autolink.exec(a);if(e){let n,s;return e[2]==="@"?(n=e[1],s="mailto:"+n):(n=e[1],s=n),{type:"link",raw:e[0],text:n,href:s,tokens:[{type:"text",raw:n,text:n}]}}}url(a){var n;let e;if(e=this.rules.inline.url.exec(a)){let s,o;if(e[2]==="@")s=e[0],o="mailto:"+s;else{let r;do r=e[0],e[0]=((n=this.rules.inline._backpedal.exec(e[0]))==null?void 0:n[0])??"";while(r!==e[0]);s=e[0],e[1]==="www."?o="http://"+e[0]:o=e[0]}return{type:"link",raw:e[0],text:s,href:o,tokens:[{type:"text",raw:s,text:s}]}}}inlineText(a){let e=this.rules.inline.text.exec(a);if(e){let n=this.lexer.state.inRawBlock;return{type:"text",raw:e[0],text:e[0],escaped:n}}}},k=class K{constructor(e){g(this,"tokens");g(this,"options");g(this,"state");g(this,"inlineQueue");g(this,"tokenizer");this.tokens=[],this.tokens.links=Object.create(null),this.options=e||D,this.options.tokenizer=this.options.tokenizer||new W,this.tokenizer=this.options.tokenizer,this.tokenizer.options=this.options,this.tokenizer.lexer=this,this.inlineQueue=[],this.state={inLink:!1,inRawBlock:!1,top:!0};let n={other:S,block:R.normal,inline:O.normal};this.options.pedantic?(n.block=R.pedantic,n.inline=O.pedantic):this.options.gfm&&(n.block=R.gfm,this.options.breaks?n.inline=O.breaks:n.inline=O.gfm),this.tokenizer.rules=n}static get rules(){return{block:R,inline:O}}static lex(e,n){return new K(n).lex(e)}static lexInline(e,n){return new K(n).inlineTokens(e)}lex(e){e=e.replace(S.carriageReturn,`
`),this.blockTokens(e,this.tokens);for(let n=0;n<this.inlineQueue.length;n++){let s=this.inlineQueue[n];this.inlineTokens(s.src,s.tokens)}return this.inlineQueue=[],this.tokens}blockTokens(e,n=[],s=!1){var r,t,u;this.tokenizer.lexer=this,this.options.pedantic&&(e=e.replace(S.tabCharGlobal,"    ").replace(S.spaceLine,""));let o=1/0;for(;e;){if(e.length<o)o=e.length;else{this.infiniteLoopError(e.charCodeAt(0));break}let i;if((t=(r=this.options.extensions)==null?void 0:r.block)!=null&&t.some(d=>(i=d.call({lexer:this},e,n))?(e=e.substring(i.raw.length),n.push(i),!0):!1))continue;if(i=this.tokenizer.space(e)){e=e.substring(i.raw.length);let d=n.at(-1);i.raw.length===1&&d!==void 0?d.raw+=`
`:n.push(i);continue}if(i=this.tokenizer.code(e)){e=e.substring(i.raw.length);let d=n.at(-1);(d==null?void 0:d.type)==="paragraph"||(d==null?void 0:d.type)==="text"?(d.raw+=(d.raw.endsWith(`
`)?"":`
`)+i.raw,d.text+=`
`+i.text,this.inlineQueue.at(-1).src=d.text):n.push(i);continue}if(i=this.tokenizer.fences(e)){e=e.substring(i.raw.length),n.push(i);continue}if(i=this.tokenizer.heading(e)){e=e.substring(i.raw.length),n.push(i);continue}if(i=this.tokenizer.hr(e)){e=e.substring(i.raw.length),n.push(i);continue}if(i=this.tokenizer.blockquote(e)){e=e.substring(i.raw.length),n.push(i);continue}if(i=this.tokenizer.list(e)){e=e.substring(i.raw.length),n.push(i);continue}if(i=this.tokenizer.html(e)){e=e.substring(i.raw.length),n.push(i);continue}if(i=this.tokenizer.def(e)){e=e.substring(i.raw.length);let d=n.at(-1);(d==null?void 0:d.type)==="paragraph"||(d==null?void 0:d.type)==="text"?(d.raw+=(d.raw.endsWith(`
`)?"":`
`)+i.raw,d.text+=`
`+i.raw,this.inlineQueue.at(-1).src=d.text):this.tokens.links[i.tag]||(this.tokens.links[i.tag]={href:i.href,title:i.title},n.push(i));continue}if(i=this.tokenizer.table(e)){e=e.substring(i.raw.length),n.push(i);continue}if(i=this.tokenizer.lheading(e)){e=e.substring(i.raw.length),n.push(i);continue}let l=e;if((u=this.options.extensions)!=null&&u.startBlock){let d=1/0,m=e.slice(1),p;this.options.extensions.startBlock.forEach(b=>{p=b.call({lexer:this},m),typeof p=="number"&&p>=0&&(d=Math.min(d,p))}),d<1/0&&d>=0&&(l=e.substring(0,d+1))}if(this.state.top&&(i=this.tokenizer.paragraph(l))){let d=n.at(-1);s&&(d==null?void 0:d.type)==="paragraph"?(d.raw+=(d.raw.endsWith(`
`)?"":`
`)+i.raw,d.text+=`
`+i.text,this.inlineQueue.pop(),this.inlineQueue.at(-1).src=d.text):n.push(i),s=l.length!==e.length,e=e.substring(i.raw.length);continue}if(i=this.tokenizer.text(e)){e=e.substring(i.raw.length);let d=n.at(-1);(d==null?void 0:d.type)==="text"?(d.raw+=(d.raw.endsWith(`
`)?"":`
`)+i.raw,d.text+=`
`+i.text,this.inlineQueue.pop(),this.inlineQueue.at(-1).src=d.text):n.push(i);continue}if(e){this.infiniteLoopError(e.charCodeAt(0));break}}return this.state.top=!0,n}inline(e,n=[]){return this.inlineQueue.push({src:e,tokens:n}),n}inlineTokens(e,n=[]){var l,d,m,p,b;this.tokenizer.lexer=this;let s=e,o=null;if(this.tokens.links){let c=Object.keys(this.tokens.links);if(c.length>0)for(;(o=this.tokenizer.rules.inline.reflinkSearch.exec(s))!==null;)c.includes(o[0].slice(o[0].lastIndexOf("[")+1,-1))&&(s=s.slice(0,o.index)+"["+"a".repeat(o[0].length-2)+"]"+s.slice(this.tokenizer.rules.inline.reflinkSearch.lastIndex))}for(;(o=this.tokenizer.rules.inline.anyPunctuation.exec(s))!==null;)s=s.slice(0,o.index)+"++"+s.slice(this.tokenizer.rules.inline.anyPunctuation.lastIndex);let r;for(;(o=this.tokenizer.rules.inline.blockSkip.exec(s))!==null;)r=o[2]?o[2].length:0,s=s.slice(0,o.index+r)+"["+"a".repeat(o[0].length-r-2)+"]"+s.slice(this.tokenizer.rules.inline.blockSkip.lastIndex);s=((d=(l=this.options.hooks)==null?void 0:l.emStrongMask)==null?void 0:d.call({lexer:this},s))??s;let t=!1,u="",i=1/0;for(;e;){if(e.length<i)i=e.length;else{this.infiniteLoopError(e.charCodeAt(0));break}t||(u=""),t=!1;let c;if((p=(m=this.options.extensions)==null?void 0:m.inline)!=null&&p.some(f=>(c=f.call({lexer:this},e,n))?(e=e.substring(c.raw.length),n.push(c),!0):!1))continue;if(c=this.tokenizer.escape(e)){e=e.substring(c.raw.length),n.push(c);continue}if(c=this.tokenizer.tag(e)){e=e.substring(c.raw.length),n.push(c);continue}if(c=this.tokenizer.link(e)){e=e.substring(c.raw.length),n.push(c);continue}if(c=this.tokenizer.reflink(e,this.tokens.links)){e=e.substring(c.raw.length);let f=n.at(-1);c.type==="text"&&(f==null?void 0:f.type)==="text"?(f.raw+=c.raw,f.text+=c.text):n.push(c);continue}if(c=this.tokenizer.emStrong(e,s,u)){e=e.substring(c.raw.length),n.push(c);continue}if(c=this.tokenizer.codespan(e)){e=e.substring(c.raw.length),n.push(c);continue}if(c=this.tokenizer.br(e)){e=e.substring(c.raw.length),n.push(c);continue}if(c=this.tokenizer.del(e,s,u)){e=e.substring(c.raw.length),n.push(c);continue}if(c=this.tokenizer.autolink(e)){e=e.substring(c.raw.length),n.push(c);continue}if(!this.state.inLink&&(c=this.tokenizer.url(e))){e=e.substring(c.raw.length),n.push(c);continue}let y=e;if((b=this.options.extensions)!=null&&b.startInline){let f=1/0,I=e.slice(1),H;this.options.extensions.startInline.forEach(U=>{H=U.call({lexer:this},I),typeof H=="number"&&H>=0&&(f=Math.min(f,H))}),f<1/0&&f>=0&&(y=e.substring(0,f+1))}if(c=this.tokenizer.inlineText(y)){e=e.substring(c.raw.length),c.raw.slice(-1)!=="_"&&(u=c.raw.slice(-1)),t=!0;let f=n.at(-1);(f==null?void 0:f.type)==="text"?(f.raw+=c.raw,f.text+=c.text):n.push(c);continue}if(e){this.infiniteLoopError(e.charCodeAt(0));break}}return n}infiniteLoopError(e){let n="Infinite loop on byte: "+e;if(this.options.silent)console.error(n);else throw new Error(n)}},N=class{constructor(a){g(this,"options");g(this,"parser");this.options=a||D}space(a){return""}code({text:a,lang:e,escaped:n}){var r;let s=(r=(e||"").match(S.notSpaceStart))==null?void 0:r[0],o=a.replace(S.endingNewline,"")+`
`;return s?'<pre><code class="language-'+z(s)+'">'+(n?o:z(o,!0))+`</code></pre>
`:"<pre><code>"+(n?o:z(o,!0))+`</code></pre>
`}blockquote({tokens:a}){return`<blockquote>
${this.parser.parse(a)}</blockquote>
`}html({text:a}){return a}def(a){return""}heading({tokens:a,depth:e}){return`<h${e}>${this.parser.parseInline(a)}</h${e}>
`}hr(a){return`<hr>
`}list(a){let e=a.ordered,n=a.start,s="";for(let t=0;t<a.items.length;t++){let u=a.items[t];s+=this.listitem(u)}let o=e?"ol":"ul",r=e&&n!==1?' start="'+n+'"':"";return"<"+o+r+`>
`+s+"</"+o+`>
`}listitem(a){return`<li>${this.parser.parse(a.tokens)}</li>
`}checkbox({checked:a}){return"<input "+(a?'checked="" ':"")+'disabled="" type="checkbox"> '}paragraph({tokens:a}){return`<p>${this.parser.parseInline(a)}</p>
`}table(a){let e="",n="";for(let o=0;o<a.header.length;o++)n+=this.tablecell(a.header[o]);e+=this.tablerow({text:n});let s="";for(let o=0;o<a.rows.length;o++){let r=a.rows[o];n="";for(let t=0;t<r.length;t++)n+=this.tablecell(r[t]);s+=this.tablerow({text:n})}return s&&(s=`<tbody>${s}</tbody>`),`<table>
<thead>
`+e+`</thead>
`+s+`</table>
`}tablerow({text:a}){return`<tr>
${a}</tr>
`}tablecell(a){let e=this.parser.parseInline(a.tokens),n=a.header?"th":"td";return(a.align?`<${n} align="${a.align}">`:`<${n}>`)+e+`</${n}>
`}strong({tokens:a}){return`<strong>${this.parser.parseInline(a)}</strong>`}em({tokens:a}){return`<em>${this.parser.parseInline(a)}</em>`}codespan({text:a}){return`<code>${z(a,!0)}</code>`}br(a){return"<br>"}del({tokens:a}){return`<del>${this.parser.parseInline(a)}</del>`}link({href:a,title:e,tokens:n}){let s=this.parser.parseInline(n),o=te(a);if(o===null)return s;a=o;let r='<a href="'+a+'"';return e&&(r+=' title="'+z(e)+'"'),r+=">"+s+"</a>",r}image({href:a,title:e,text:n,tokens:s}){s&&(n=this.parser.parseInline(s,this.parser.textRenderer));let o=te(a);if(o===null)return z(n);a=o;let r=`<img src="${a}" alt="${z(n)}"`;return e&&(r+=` title="${z(e)}"`),r+=">",r}text(a){return"tokens"in a&&a.tokens?this.parser.parseInline(a.tokens):"escaped"in a&&a.escaped?a.text:z(a.text)}},oe=class{strong({text:a}){return a}em({text:a}){return a}codespan({text:a}){return a}del({text:a}){return a}html({text:a}){return a}text({text:a}){return a}link({text:a}){return""+a}image({text:a}){return""+a}br(){return""}checkbox({raw:a}){return a}},q=class Q{constructor(e){g(this,"options");g(this,"renderer");g(this,"textRenderer");this.options=e||D,this.options.renderer=this.options.renderer||new N,this.renderer=this.options.renderer,this.renderer.options=this.options,this.renderer.parser=this,this.textRenderer=new oe}static parse(e,n){return new Q(n).parse(e)}static parseInline(e,n){return new Q(n).parseInline(e)}parse(e){var s,o;this.renderer.parser=this;let n="";for(let r=0;r<e.length;r++){let t=e[r];if((o=(s=this.options.extensions)==null?void 0:s.renderers)!=null&&o[t.type]){let i=t,l=this.options.extensions.renderers[i.type].call({parser:this},i);if(l!==!1||!["space","hr","heading","code","table","blockquote","list","html","def","paragraph","text"].includes(i.type)){n+=l||"";continue}}let u=t;switch(u.type){case"space":{n+=this.renderer.space(u);break}case"hr":{n+=this.renderer.hr(u);break}case"heading":{n+=this.renderer.heading(u);break}case"code":{n+=this.renderer.code(u);break}case"table":{n+=this.renderer.table(u);break}case"blockquote":{n+=this.renderer.blockquote(u);break}case"list":{n+=this.renderer.list(u);break}case"checkbox":{n+=this.renderer.checkbox(u);break}case"html":{n+=this.renderer.html(u);break}case"def":{n+=this.renderer.def(u);break}case"paragraph":{n+=this.renderer.paragraph(u);break}case"text":{n+=this.renderer.text(u);break}default:{let i='Token with "'+u.type+'" type was not found.';if(this.options.silent)return console.error(i),"";throw new Error(i)}}}return n}parseInline(e,n=this.renderer){var o,r;this.renderer.parser=this;let s="";for(let t=0;t<e.length;t++){let u=e[t];if((r=(o=this.options.extensions)==null?void 0:o.renderers)!=null&&r[u.type]){let l=this.options.extensions.renderers[u.type].call({parser:this},u);if(l!==!1||!["escape","html","link","image","strong","em","codespan","br","del","text"].includes(u.type)){s+=l||"";continue}}let i=u;switch(i.type){case"escape":{s+=n.text(i);break}case"html":{s+=n.html(i);break}case"link":{s+=n.link(i);break}case"image":{s+=n.image(i);break}case"checkbox":{s+=n.checkbox(i);break}case"strong":{s+=n.strong(i);break}case"em":{s+=n.em(i);break}case"codespan":{s+=n.codespan(i);break}case"br":{s+=n.br(i);break}case"del":{s+=n.del(i);break}case"text":{s+=n.text(i);break}default:{let l='Token with "'+i.type+'" type was not found.';if(this.options.silent)return console.error(l),"";throw new Error(l)}}}return s}},T,G=(T=class{constructor(a){g(this,"options");g(this,"block");this.options=a||D}preprocess(a){return a}postprocess(a){return a}processAllTokens(a){return a}emStrongMask(a){return a}provideLexer(a=this.block){return a?k.lex:k.lexInline}provideParser(a=this.block){return a?q.parse:q.parseInline}},g(T,"passThroughHooks",new Set(["preprocess","postprocess","processAllTokens","emStrongMask"])),g(T,"passThroughHooksRespectAsync",new Set(["preprocess","postprocess","processAllTokens"])),T),_a=class{constructor(...a){g(this,"defaults",$());g(this,"options",this.setOptions);g(this,"parse",this.parseMarkdown(!0));g(this,"parseInline",this.parseMarkdown(!1));g(this,"Parser",q);g(this,"Renderer",N);g(this,"TextRenderer",oe);g(this,"Lexer",k);g(this,"Tokenizer",W);g(this,"Hooks",G);this.use(...a)}walkTokens(a,e){var s,o;let n=[];for(let r of a)switch(n=n.concat(e.call(this,r)),r.type){case"table":{let t=r;for(let u of t.header)n=n.concat(this.walkTokens(u.tokens,e));for(let u of t.rows)for(let i of u)n=n.concat(this.walkTokens(i.tokens,e));break}case"list":{let t=r;n=n.concat(this.walkTokens(t.items,e));break}default:{let t=r;(o=(s=this.defaults.extensions)==null?void 0:s.childTokens)!=null&&o[t.type]?this.defaults.extensions.childTokens[t.type].forEach(u=>{let i=t[u].flat(1/0);n=n.concat(this.walkTokens(i,e))}):t.tokens&&(n=n.concat(this.walkTokens(t.tokens,e)))}}return n}use(...a){let e=this.defaults.extensions||{renderers:{},childTokens:{}};return a.forEach(n=>{let s={...n};if(s.async=this.defaults.async||s.async||!1,n.extensions&&(n.extensions.forEach(o=>{if(!o.name)throw new Error("extension name required");if("renderer"in o){let r=e.renderers[o.name];r?e.renderers[o.name]=function(...t){let u=o.renderer.apply(this,t);return u===!1&&(u=r.apply(this,t)),u}:e.renderers[o.name]=o.renderer}if("tokenizer"in o){if(!o.level||o.level!=="block"&&o.level!=="inline")throw new Error("extension level must be 'block' or 'inline'");let r=e[o.level];r?r.unshift(o.tokenizer):e[o.level]=[o.tokenizer],o.start&&(o.level==="block"?e.startBlock?e.startBlock.push(o.start):e.startBlock=[o.start]:o.level==="inline"&&(e.startInline?e.startInline.push(o.start):e.startInline=[o.start]))}"childTokens"in o&&o.childTokens&&(e.childTokens[o.name]=o.childTokens)}),s.extensions=e),n.renderer){let o=this.defaults.renderer||new N(this.defaults);for(let r in n.renderer){if(!(r in o))throw new Error(`renderer '${r}' does not exist`);if(["options","parser"].includes(r))continue;let t=r,u=n.renderer[t],i=o[t];o[t]=(...l)=>{let d=u.apply(o,l);return d===!1&&(d=i.apply(o,l)),d||""}}s.renderer=o}if(n.tokenizer){let o=this.defaults.tokenizer||new W(this.defaults);for(let r in n.tokenizer){if(!(r in o))throw new Error(`tokenizer '${r}' does not exist`);if(["options","rules","lexer"].includes(r))continue;let t=r,u=n.tokenizer[t],i=o[t];o[t]=(...l)=>{let d=u.apply(o,l);return d===!1&&(d=i.apply(o,l)),d}}s.tokenizer=o}if(n.hooks){let o=this.defaults.hooks||new G;for(let r in n.hooks){if(!(r in o))throw new Error(`hook '${r}' does not exist`);if(["options","block"].includes(r))continue;let t=r,u=n.hooks[t],i=o[t];G.passThroughHooks.has(r)?o[t]=l=>{if(this.defaults.async&&G.passThroughHooksRespectAsync.has(r))return(async()=>{let m=await u.call(o,l);return i.call(o,m)})();let d=u.call(o,l);return i.call(o,d)}:o[t]=(...l)=>{if(this.defaults.async)return(async()=>{let m=await u.apply(o,l);return m===!1&&(m=await i.apply(o,l)),m})();let d=u.apply(o,l);return d===!1&&(d=i.apply(o,l)),d}}s.hooks=o}if(n.walkTokens){let o=this.defaults.walkTokens,r=n.walkTokens;s.walkTokens=function(t){let u=[];return u.push(r.call(this,t)),o&&(u=u.concat(o.call(this,t))),u}}this.defaults={...this.defaults,...s}}),this}setOptions(a){return this.defaults={...this.defaults,...a},this}lexer(a,e){return k.lex(a,e??this.defaults)}parser(a,e){return q.parse(a,e??this.defaults)}parseMarkdown(a){return(e,n)=>{let s={...n},o={...this.defaults,...s},r=this.onError(!!o.silent,!!o.async);if(this.defaults.async===!0&&s.async===!1)return r(new Error("marked(): The async option was set to true by an extension. Remove async: false from the parse options object to return a Promise."));if(typeof e>"u"||e===null)return r(new Error("marked(): input parameter is undefined or null"));if(typeof e!="string")return r(new Error("marked(): input parameter is of type "+Object.prototype.toString.call(e)+", string expected"));if(o.hooks&&(o.hooks.options=o,o.hooks.block=a),o.async)return(async()=>{let t=o.hooks?await o.hooks.preprocess(e):e,u=await(o.hooks?await o.hooks.provideLexer(a):a?k.lex:k.lexInline)(t,o),i=o.hooks?await o.hooks.processAllTokens(u):u;o.walkTokens&&await Promise.all(this.walkTokens(i,o.walkTokens));let l=await(o.hooks?await o.hooks.provideParser(a):a?q.parse:q.parseInline)(i,o);return o.hooks?await o.hooks.postprocess(l):l})().catch(r);try{o.hooks&&(e=o.hooks.preprocess(e));let t=(o.hooks?o.hooks.provideLexer(a):a?k.lex:k.lexInline)(e,o);o.hooks&&(t=o.hooks.processAllTokens(t)),o.walkTokens&&this.walkTokens(t,o.walkTokens);let u=(o.hooks?o.hooks.provideParser(a):a?q.parse:q.parseInline)(t,o);return o.hooks&&(u=o.hooks.postprocess(u)),u}catch(t){return r(t)}}}onError(a,e){return n=>{if(n.message+=`
Please report this to https://github.com/markedjs/marked.`,a){let s="<p>An error occurred:</p><pre>"+z(n.message+"",!0)+"</pre>";return e?Promise.resolve(s):s}if(e)return Promise.reject(n);throw n}}},P=new _a;function v(a,e){return P.parse(a,e)}v.options=v.setOptions=function(a){return P.setOptions(a),v.defaults=P.defaults,ce(v.defaults),v};v.getDefaults=$;v.defaults=D;v.use=function(...a){return P.use(...a),v.defaults=P.defaults,ce(v.defaults),v};v.walkTokens=function(a,e){return P.walkTokens(a,e)};v.parseInline=P.parseInline;v.Parser=q;v.parser=q.parse;v.Renderer=N;v.TextRenderer=oe;v.Lexer=k;v.lexer=k.lex;v.Tokenizer=W;v.Hooks=G;v.parse=v;v.options;v.setOptions;v.use;v.walkTokens;v.parseInline;q.parse;k.lex;v.setOptions({gfm:!0,breaks:!1});const Wa=/^(?:https?:|mailto:|tel:|#|\/|\.\/|\.\.\/)/i;v.use({walkTokens(a){if(a.type==="html")a.text="";else if(a.type==="link"||a.type==="image"){const e=a;(!e.href||!Wa.test(e.href.trim()))&&(e.href="#")}}});const Na=Object.assign({"./content/back-up-emulator-saves/de.md":Ce,"./content/back-up-emulator-saves/en.md":Pe,"./content/back-up-emulator-saves/es.md":De,"./content/back-up-emulator-saves/fr.md":Le,"./content/back-up-emulator-saves/it.md":xe,"./content/back-up-emulator-saves/ja.md":Ae,"./content/back-up-emulator-saves/pt.md":je,"./content/back-up-emulator-saves/zh.md":Oe,"./content/back-up-game-saves/de.md":Ge,"./content/back-up-game-saves/en.md":Ee,"./content/back-up-game-saves/es.md":Ie,"./content/back-up-game-saves/fr.md":Re,"./content/back-up-game-saves/it.md":Te,"./content/back-up-game-saves/ja.md":_e,"./content/back-up-game-saves/pt.md":We,"./content/back-up-game-saves/zh.md":Ne,"./content/game-save-sync-comparison/de.md":Be,"./content/game-save-sync-comparison/en.md":Me,"./content/game-save-sync-comparison/es.md":Ue,"./content/game-save-sync-comparison/fr.md":Ve,"./content/game-save-sync-comparison/it.md":Fe,"./content/game-save-sync-comparison/ja.md":Ke,"./content/game-save-sync-comparison/pt.md":Qe,"./content/game-save-sync-comparison/zh.md":$e,"./content/ludusavi-alternative/de.md":Xe,"./content/ludusavi-alternative/en.md":Ze,"./content/ludusavi-alternative/es.md":Ye,"./content/ludusavi-alternative/fr.md":Je,"./content/ludusavi-alternative/it.md":en,"./content/ludusavi-alternative/ja.md":nn,"./content/ludusavi-alternative/pt.md":an,"./content/ludusavi-alternative/zh.md":on,"./content/opensave-alternative/de.md":sn,"./content/opensave-alternative/en.md":rn,"./content/opensave-alternative/es.md":tn,"./content/opensave-alternative/fr.md":un,"./content/opensave-alternative/it.md":dn,"./content/opensave-alternative/ja.md":ln,"./content/opensave-alternative/pt.md":cn,"./content/opensave-alternative/zh.md":mn,"./content/restore-a-game-save/de.md":pn,"./content/restore-a-game-save/en.md":hn,"./content/restore-a-game-save/es.md":vn,"./content/restore-a-game-save/fr.md":gn,"./content/restore-a-game-save/it.md":fn,"./content/restore-a-game-save/ja.md":bn,"./content/restore-a-game-save/pt.md":Sn,"./content/restore-a-game-save/zh.md":yn,"./content/self-host-hoard/de.md":kn,"./content/self-host-hoard/en.md":qn,"./content/self-host-hoard/es.md":zn,"./content/self-host-hoard/fr.md":wn,"./content/self-host-hoard/it.md":Hn,"./content/self-host-hoard/ja.md":Cn,"./content/self-host-hoard/pt.md":Pn,"./content/self-host-hoard/zh.md":Dn,"./content/steam-cloud-alternative/de.md":Ln,"./content/steam-cloud-alternative/en.md":xn,"./content/steam-cloud-alternative/es.md":An,"./content/steam-cloud-alternative/fr.md":jn,"./content/steam-cloud-alternative/it.md":On,"./content/steam-cloud-alternative/ja.md":Gn,"./content/steam-cloud-alternative/pt.md":En,"./content/steam-cloud-alternative/zh.md":In,"./content/sync-game-saves-across-pcs/de.md":Rn,"./content/sync-game-saves-across-pcs/en.md":Tn,"./content/sync-game-saves-across-pcs/es.md":_n,"./content/sync-game-saves-across-pcs/fr.md":Wn,"./content/sync-game-saves-across-pcs/it.md":Nn,"./content/sync-game-saves-across-pcs/ja.md":Bn,"./content/sync-game-saves-across-pcs/pt.md":Mn,"./content/sync-game-saves-across-pcs/zh.md":Un,"./content/syncthing-game-saves/de.md":Vn,"./content/syncthing-game-saves/en.md":Fn,"./content/syncthing-game-saves/es.md":Kn,"./content/syncthing-game-saves/fr.md":Qn,"./content/syncthing-game-saves/it.md":$n,"./content/syncthing-game-saves/ja.md":Xn,"./content/syncthing-game-saves/pt.md":Zn,"./content/syncthing-game-saves/zh.md":Yn});function Ba(a){const e=a.match(/^---\r?\n([\s\S]*?)\r?\n---\r?\n?([\s\S]*)$/);if(!e)return{meta:{},body:a};const n={};for(const s of e[1].split(/\r?\n/)){const o=s.indexOf(":");if(o===-1)continue;const r=s.slice(0,o).trim();let t=s.slice(o+1).trim();(t.startsWith('"')&&t.endsWith('"')||t.startsWith("'")&&t.endsWith("'"))&&(t=t.slice(1,-1)),n[r]=t}return{meta:n,body:e[2]}}const Ma=a=>a.replace(/\[([^\]]+)\]\([^)]*\)/g,"$1").replace(/[*_`]/g,"").trim();function Ua(a){const e=a.indexOf("<!-- faq -->");return e===-1?[]:a.slice(e).split(/^###[ \t]+/m).slice(1).map(n=>{const s=n.indexOf(`
`),o=Ma(s===-1?n:n.slice(0,s)),t=(s===-1?"":n.slice(s+1)).split(/^##[ \t]/m)[0].trim();return{question:o,answer:t?v.parse(t):""}}).filter(n=>n.question&&n.answer)}const x={};for(const[a,e]of Object.entries(Na)){const n=a.match(/\/content\/([^/]+)\/([^/]+)\.md$/);if(!n)continue;const[,s,o]=n;if(!we.includes(o))continue;const{meta:r,body:t}=Ba(e);(x[s]??(x[s]={}))[o]={slug:s,title:r.title??s,description:r.description??"",order:Number(r.order??999),featured:r.featured==="true",updated:r.updated??"",html:v.parse(t.trim()),faq:Ua(t)}}function Va(a,e){const n=x[a];return n?n[e]??n[He]??null:null}function Qa(a){return Object.keys(x).map(e=>Va(e,a)).filter(e=>e!==null).sort((e,n)=>e.order-n.order||e.title.localeCompare(n.title))}function $a(){return Object.keys(x)}export{$a as a,Va as g,Qa as l};
