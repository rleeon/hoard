import data from './games.json';
import type { Locale } from '$lib/i18n/locales';

/**
 * Generated "<game> save location" guides: one Markdown template per locale,
 * filled from `games.json`. That file is built offline from the community save
 * manifest (paths from PCGamingWiki) and Steam's popularity data, and only
 * keeps games with a real Windows save path, a Steam AppID and local saves
 * worth finding; online-first games are left out on purpose, because a page
 * per game only works while every page answers its query properly.
 *
 * The Steam Deck paths are precomputed the same way the agent expands a
 * Windows template inside a Proton prefix (`pathexpand.rs`).
 */

type Store = 'steam' | 'epic' | 'gog';

export type Game = {
  n: string;
  s: string;
  a: number;
  c: Partial<Record<Store, boolean>>;
  w: string[];
  d: string[];
  l: string[];
  m: string[];
  k: string[];
};

export const GAMES = data as Game[];
export const GAMES_UPDATED = '2026-11-05';
export const GAME_RELATED = [
  'where-does-steam-store-saves',
  'where-are-pc-game-saves-stored',
  'sync-saves-steam-deck-pc'
];

const SUFFIX = '-save-location';
const bySlug = new Map(GAMES.map((g) => [g.s + SUFFIX, g]));

export const gameSlug = (g: Game) => g.s + SUFFIX;
export const gameSlugs = () => [...bySlug.keys()];
export const gameBySlug = (slug: string) => bySlug.get(slug) ?? null;

const code = (p: string) => '`' + p + '`';
const list = (ps: string[]) => ps.map((p) => `- ${code(p)}`).join('\n');
// Names go into Markdown prose; a stray `*` or `_` in a title would turn into
// emphasis, and `<`/`>` into raw HTML that the renderer then blanks.
const esc = (s: string) => s.replace(/([\\`*_[\]<>])/g, '\\$1');

type Ctx = { n: string; g: Game; cloud: Store[]; prefix: boolean; linux: string | null };

type Template = {
  title: (n: string) => string;
  desc: (n: string) => string;
  lead: (c: Ctx) => string;
  where: (n: string) => string;
  win: string;
  deck: string;
  linux: string;
  mac: string;
  user: string;
  steamDir: string;
  source: (c: Ctx) => string;
  cfgTitle: string;
  cfg: (n: string, k: string) => string;
  hoard: (n: string) => string;
  faqTitle: string;
  qCloud: (n: string) => string;
  aCloud: (c: Ctx, stores: string) => string;
  qDeck: (n: string) => string;
  aDeck: (c: Ctx) => string;
  fixed: string;
  stores: Record<Store, string>;
  join: (xs: string[]) => string;
};

const joinWith = (and: string, sep = ', ') => (xs: string[]) =>
  xs.length < 2 ? xs.join('') : `${xs.slice(0, -1).join(sep)}${and}${xs.at(-1)}`;

// French elides "de" before a vowel: "d'Elden Ring", "de Hades".
const frDe = (n: string) => (/^[aeiouyàâéèêîôû]/i.test(n) ? `d'${n}` : `de ${n}`);

const T: Record<Locale, Template> = {
  en: {
    title: (n) => `${n} save location (PC & Steam Deck)`,
    desc: (n) =>
      `Where ${n} keeps its saves on PC and Steam Deck, whether it has cloud saves, and how to back them up and sync them.`,
    lead: (c) =>
      `${c.n} keeps its saves in ${code(c.g.w[0])} on Windows.${c.g.c.steam ? ' It supports Steam Cloud, which keeps the latest save on your Steam account.' : ''} [Hoard](/download) finds that folder on its own, backs it up every time the game saves and keeps it in sync between your PCs and a Steam Deck.`,
    where: (n) => `## Where ${n} keeps its saves`,
    win: '**Windows**',
    deck: '**Steam Deck and Linux (Proton)**',
    linux: '**Native Linux version**',
    mac: '**macOS**',
    user: "`<user-id>` is a folder named after your account's numeric ID; there is one for each account that has played the game on that machine.",
    steamDir: 'Paths under `C:\\Program Files (x86)\\Steam` assume Steam is in its default folder.',
    source: (c) =>
      `These locations come from PCGamingWiki, through the community save database Hoard uses for detection.${c.prefix ? ` The Steam Deck paths are the Windows paths inside the game's Proton prefix, \`compatdata/${c.g.a}\`, which appears after the game's first launch.` : ''}`,
    cfgTitle: '## Settings files',
    cfg: (n, k) => `${n} keeps settings such as graphics and controls apart from the saves, in ${code(k)}.`,
    hoard: (n) => `## Keep it backed up and in sync with Hoard

[Hoard](/download) is free and open source. It backs up your ${n} saves every time the game writes one and again when you quit, keeps a history you can go back to, and syncs the folder between your PCs and a Steam Deck. Only the files that changed travel, compressed, and every file is checked against its SHA-256 hash when it comes back.

1. Install Hoard and sign in to Hoard One, recommended for everyone, or point it at [your own server](/guides/self-host-hoard).
2. Open the **Library**. ${n} is detected from your Steam library and the community save database.
3. Play. When you quit, the first version appears in the history.`,
    faqTitle: '## Frequently asked questions',
    qCloud: (n) => `### Does ${n} have cloud saves?`,
    aCloud: (c, s) =>
      c.cloud.length
        ? `Yes, according to the save database: ${s}. A store's cloud keeps only the latest save for that account, though, with no versions to go back to. Hoard keeps every version on top, on Hoard One or your own server.`
        : c.g.c.steam === false
          ? 'Not on Steam, according to the save database. Without a cloud, the only copy of the save is on that PC unless you make another; Hoard keeps one for you, with every version.'
          : `The save database doesn't say for ${c.n}, so check its store page. Either way, Hoard keeps its own copy with every version, on Hoard One or your own server.`,
    qDeck: (n) => `### Where are ${n} saves on Steam Deck?`,
    aDeck: (c) =>
      `In ${code(c.g.d[0])}.${c.prefix ? " That folder sits inside the game's Proton prefix, so it only exists once the game has run at least once." : ''}${c.linux ? ` If you play the native Linux version instead, look in ${code(c.linux)}.` : ''}`,
    fixed: `### Is Hoard free?

Yes. The app, the command line and the server are free and open source (AGPL-3.0), and running your own server costs nothing but the machine. Hoard One, the managed server we recommend for everyone, is free too with 2 GB and three devices; Pro adds room for €1.99 a month.

### Does self-hosting send anything to Hoard?

No. On your own server there's no account with us and no telemetry to us: your saves, your users and your logs stay on your machine and never touch ours.`,
    stores: { steam: 'Steam Cloud', epic: 'the Epic Games Store', gog: 'GOG Galaxy' },
    join: joinWith(' and ')
  },
  es: {
    title: (n) => `Dónde están las partidas de ${n} (PC y Steam Deck)`,
    desc: (n) =>
      `Dónde guarda ${n} sus partidas en PC y Steam Deck, si tiene partidas en la nube y cómo copiarlas y sincronizarlas solas.`,
    lead: (c) =>
      `${c.n} guarda sus partidas en ${code(c.g.w[0])} en Windows.${c.g.c.steam ? ' Es compatible con Steam Cloud, que mantiene la última partida en tu cuenta de Steam.' : ''} [Hoard](/download) encuentra esa carpeta solo, la copia cada vez que el juego guarda y la mantiene sincronizada entre tus PC y una Steam Deck.`,
    where: (n) => `## Dónde guarda ${n} las partidas`,
    win: '**Windows**',
    deck: '**Steam Deck y Linux (Proton)**',
    linux: '**Versión nativa de Linux**',
    mac: '**macOS**',
    user: '`<user-id>` es una carpeta con el número de tu cuenta; hay una por cada cuenta que ha jugado en esa máquina.',
    steamDir: 'Las rutas bajo `C:\\Program Files (x86)\\Steam` suponen que Steam está en su carpeta por defecto.',
    source: (c) =>
      `Estas ubicaciones salen de PCGamingWiki, a través de la base de datos comunitaria de partidas que Hoard usa para detectarlas.${c.prefix ? ` Las rutas de la Steam Deck son las de Windows dentro del prefijo de Proton del juego, \`compatdata/${c.g.a}\`, que aparece tras abrir el juego por primera vez.` : ''}`,
    cfgTitle: '## Ficheros de ajustes',
    cfg: (n, k) => `${n} guarda los ajustes, como gráficos y controles, aparte de las partidas, en ${code(k)}.`,
    hoard: (n) => `## Mantenlo copiado y sincronizado con Hoard

[Hoard](/download) es gratis y de código abierto. Copia las partidas de ${n} cada vez que el juego guarda y otra vez al salir, guarda un historial al que puedes volver y sincroniza la carpeta entre tus PC y una Steam Deck. Solo viajan los ficheros que han cambiado, comprimidos, y cada fichero se comprueba con su hash SHA-256 al volver.

1. Instala Hoard e inicia sesión en Hoard One, recomendado para todos, o apúntalo a [tu propio servidor](/guides/self-host-hoard).
2. Abre la **Biblioteca**. ${n} se detecta desde tu biblioteca de Steam y la base de datos comunitaria de partidas.
3. Juega. Al salir, la primera versión aparece en el historial.`,
    faqTitle: '## Preguntas frecuentes',
    qCloud: (n) => `### ¿${n} tiene partidas en la nube?`,
    aCloud: (c, s) =>
      c.cloud.length
        ? `Sí, según la base de datos de partidas: ${s}. Pero la nube de una tienda solo guarda la última partida de esa cuenta, sin versiones a las que volver. Hoard guarda además todas las versiones, en Hoard One o en tu propio servidor.`
        : c.g.c.steam === false
          ? 'No en Steam, según la base de datos de partidas. Sin nube, la única copia de la partida está en ese PC a menos que hagas otra; Hoard la hace por ti, con todas las versiones.'
          : `La base de datos de partidas no lo indica para ${c.n}, así que mira su página en la tienda. En cualquier caso, Hoard guarda su propia copia con todas las versiones, en Hoard One o en tu propio servidor.`,
    qDeck: (n) => `### ¿Dónde están las partidas de ${n} en la Steam Deck?`,
    aDeck: (c) =>
      `En ${code(c.g.d[0])}.${c.prefix ? ' Esa carpeta está dentro del prefijo de Proton del juego, así que solo existe después de abrirlo al menos una vez.' : ''}${c.linux ? ` Si juegas a la versión nativa de Linux, mira en ${code(c.linux)}.` : ''}`,
    fixed: `### ¿Hoard es gratis?

Sí. La aplicación, la línea de comandos y el servidor son gratis y de código abierto (AGPL-3.0), y montar tu propio servidor no cuesta más que la máquina. Hoard One, el servidor gestionado que recomendamos a todos, también es gratis con 2 GB y tres dispositivos; Pro añade espacio por 1,99 € al mes.

### ¿El self-hosting envía algo a Hoard?

No. En tu propio servidor no hay cuenta con nosotros ni telemetría hacia nosotros: tus partidas, tus usuarios y tus logs se quedan en tu máquina y nunca tocan las nuestras.`,
    stores: { steam: 'Steam Cloud', epic: 'la Epic Games Store', gog: 'GOG Galaxy' },
    join: joinWith(' y ')
  },
  de: {
    title: (n) => `${n}: Speicherort der Spielstände (PC & Steam Deck)`,
    desc: (n) =>
      `Wo ${n} seine Spielstände auf PC und Steam Deck ablegt, ob es Cloud-Spielstände gibt und wie du sie automatisch sicherst.`,
    lead: (c) =>
      `${c.n} speichert unter Windows in ${code(c.g.w[0])}.${c.g.c.steam ? ' Es unterstützt Steam Cloud, das den neuesten Spielstand in deinem Steam-Konto hält.' : ''} [Hoard](/download) findet diesen Ordner von selbst, sichert ihn bei jedem Speichern und hält ihn zwischen deinen PCs und einem Steam Deck synchron.`,
    where: (n) => `## Wo ${n} die Spielstände ablegt`,
    win: '**Windows**',
    deck: '**Steam Deck und Linux (Proton)**',
    linux: '**Native Linux-Version**',
    mac: '**macOS**',
    user: '`<user-id>` ist ein Ordner mit der Nummer deines Kontos; es gibt einen pro Konto, das auf diesem Rechner gespielt hat.',
    steamDir: 'Pfade unter `C:\\Program Files (x86)\\Steam` gehen davon aus, dass Steam im Standardordner liegt.',
    source: (c) =>
      `Diese Speicherorte stammen von PCGamingWiki, über die Community-Datenbank für Spielstände, mit der Hoard sie erkennt.${c.prefix ? ` Die Steam-Deck-Pfade sind die Windows-Pfade im Proton-Präfix des Spiels, \`compatdata/${c.g.a}\`, das nach dem ersten Start entsteht.` : ''}`,
    cfgTitle: '## Einstellungsdateien',
    cfg: (n, k) => `${n} legt Einstellungen wie Grafik und Steuerung getrennt von den Spielständen ab, in ${code(k)}.`,
    hoard: (n) => `## Automatisch sichern und synchronisieren mit Hoard

[Hoard](/download) ist kostenlos und Open Source. Es sichert deine Spielstände von ${n} jedes Mal, wenn das Spiel speichert, und noch einmal beim Beenden, führt einen Verlauf, zu dem du zurückkehren kannst, und synchronisiert den Ordner zwischen deinen PCs und einem Steam Deck. Nur geänderte Dateien werden übertragen, komprimiert, und jede Datei wird bei der Rückkehr gegen ihren SHA-256-Hash geprüft.

1. Installiere Hoard und melde dich bei Hoard One an, empfohlen für alle, oder richte es auf [deinen eigenen Server](/guides/self-host-hoard).
2. Öffne die **Bibliothek**. ${n} wird über deine Steam-Bibliothek und die Community-Datenbank für Spielstände erkannt.
3. Spiele. Beim Beenden erscheint die erste Version im Verlauf.`,
    faqTitle: '## Häufige Fragen',
    qCloud: (n) => `### Hat ${n} Cloud-Spielstände?`,
    aCloud: (c, s) =>
      c.cloud.length
        ? `Ja, laut der Spielstand-Datenbank: ${s}. Die Cloud eines Stores hält aber nur den neuesten Spielstand dieses Kontos, ohne Versionen zum Zurückgehen. Hoard behält zusätzlich jede Version, auf Hoard One oder deinem eigenen Server.`
        : c.g.c.steam === false
          ? 'Nicht auf Steam, laut der Spielstand-Datenbank. Ohne Cloud liegt die einzige Kopie des Spielstands auf diesem PC, solange du keine weitere machst; Hoard macht sie für dich, mit jeder Version.'
          : `Die Spielstand-Datenbank sagt es für ${c.n} nicht, also sieh auf der Store-Seite nach. So oder so behält Hoard eine eigene Kopie mit jeder Version, auf Hoard One oder deinem eigenen Server.`,
    qDeck: (n) => `### Wo liegen die Spielstände von ${n} auf dem Steam Deck?`,
    aDeck: (c) =>
      `In ${code(c.g.d[0])}.${c.prefix ? ' Dieser Ordner liegt im Proton-Präfix des Spiels und existiert daher erst, wenn das Spiel mindestens einmal gelaufen ist.' : ''}${c.linux ? ` Spielst du stattdessen die native Linux-Version, sieh in ${code(c.linux)} nach.` : ''}`,
    fixed: `### Ist Hoard kostenlos?

Ja. Die App, die Kommandozeile und der Server sind kostenlos und Open Source (AGPL-3.0), und ein eigener Server kostet dich nur die Hardware. Hoard One, der verwaltete Server, den wir allen empfehlen, ist ebenfalls kostenlos mit 2 GB und drei Geräten; Pro bringt mehr Platz für 1,99 € im Monat.

### Sendet Self-Hosting irgendetwas an Hoard?

Nein. Auf deinem eigenen Server gibt es kein Konto bei uns und keine Telemetrie an uns: Deine Spielstände, deine Nutzer und deine Logs bleiben auf deiner Maschine und berühren unsere nie.`,
    stores: { steam: 'Steam Cloud', epic: 'der Epic Games Store', gog: 'GOG Galaxy' },
    join: joinWith(' und ')
  },
  fr: {
    title: (n) => `Emplacement des sauvegardes ${frDe(n)} (PC et Steam Deck)`,
    desc: (n) =>
      `Où ${n} range ses sauvegardes sur PC et Steam Deck, s'il a le cloud, et comment les sauvegarder et les synchroniser.`,
    lead: (c) =>
      `${c.n} range ses sauvegardes dans ${code(c.g.w[0])} sous Windows.${c.g.c.steam ? ' Il prend en charge Steam Cloud, qui garde la dernière sauvegarde sur votre compte Steam.' : ''} [Hoard](/download) trouve ce dossier tout seul, le sauvegarde chaque fois que le jeu enregistre et le garde synchronisé entre vos PC et un Steam Deck.`,
    where: (n) => `## Où ${n} range ses sauvegardes`,
    win: '**Windows**',
    deck: '**Steam Deck et Linux (Proton)**',
    linux: '**Version Linux native**',
    mac: '**macOS**',
    user: "`<user-id>` est un dossier au numéro de votre compte ; il y en a un par compte ayant joué sur cette machine.",
    steamDir: 'Les chemins sous `C:\\Program Files (x86)\\Steam` supposent que Steam est dans son dossier par défaut.',
    source: (c) =>
      `Ces emplacements viennent de PCGamingWiki, via la base communautaire des sauvegardes que Hoard utilise pour la détection.${c.prefix ? ` Les chemins du Steam Deck sont les chemins Windows à l'intérieur du préfixe Proton du jeu, \`compatdata/${c.g.a}\`, qui apparaît après le premier lancement.` : ''}`,
    cfgTitle: '## Fichiers de réglages',
    cfg: (n, k) => `${n} range les réglages, comme les graphismes et les commandes, à part des sauvegardes, dans ${code(k)}.`,
    hoard: (n) => `## Sauvegarde et synchro automatiques avec Hoard

[Hoard](/download) est gratuit et open source. Il sauvegarde vos parties ${frDe(n)} chaque fois que le jeu en écrit une, puis encore quand vous quittez, garde un historique où revenir et synchronise le dossier entre vos PC et une Steam Deck. Seuls les fichiers modifiés voyagent, compressés, et chaque fichier est vérifié par son empreinte SHA-256 à son retour.

1. Installez Hoard et connectez-vous à Hoard One, recommandé pour tout le monde, ou pointez-le vers [votre propre serveur](/guides/self-host-hoard).
2. Ouvrez la **Bibliothèque**. ${n} est détecté à partir de votre bibliothèque Steam et de la base de données communautaire des sauvegardes.
3. Jouez. En quittant, la première version apparaît dans l'historique.`,
    faqTitle: '## Questions fréquentes',
    qCloud: (n) => `### ${n} a-t-il des sauvegardes cloud ?`,
    aCloud: (c, s) =>
      c.cloud.length
        ? `Oui, d'après la base des sauvegardes : ${s}. Mais le cloud d'une boutique ne garde que la dernière sauvegarde de ce compte, sans versions où revenir. Hoard garde en plus toutes les versions, sur Hoard One ou votre propre serveur.`
        : c.g.c.steam === false
          ? "Pas sur Steam, d'après la base des sauvegardes. Sans cloud, la seule copie de la sauvegarde est sur ce PC, à moins d'en faire une autre ; Hoard la fait pour vous, avec toutes les versions."
          : `La base des sauvegardes ne le précise pas pour ${c.n} : vérifiez sa page en boutique. Dans tous les cas, Hoard garde sa propre copie avec toutes les versions, sur Hoard One ou votre propre serveur.`,
    qDeck: (n) => `### Où sont les sauvegardes ${frDe(n)} sur Steam Deck ?`,
    aDeck: (c) =>
      `Dans ${code(c.g.d[0])}.${c.prefix ? " Ce dossier se trouve dans le préfixe Proton du jeu : il n'existe donc qu'après au moins un lancement." : ''}${c.linux ? ` Si vous jouez à la version Linux native, regardez dans ${code(c.linux)}.` : ''}`,
    fixed: `### Hoard est-il gratuit ?

Oui. L'application, la ligne de commande et le serveur sont gratuits et open source (AGPL-3.0), et faire tourner votre propre serveur ne coûte que la machine. Hoard One, le serveur géré que nous recommandons à tous, est gratuit lui aussi avec 2 Go et trois appareils ; Pro ajoute de la place pour 1,99 € par mois.

### L'auto-hébergement envoie-t-il quelque chose à Hoard ?

Non. Sur votre propre serveur, il n'y a ni compte chez nous ni télémétrie vers nous : vos sauvegardes, vos utilisateurs et vos journaux restent sur votre machine et ne touchent jamais les nôtres.`,
    stores: { steam: 'Steam Cloud', epic: "l'Epic Games Store", gog: 'GOG Galaxy' },
    join: joinWith(' et ')
  },
  it: {
    title: (n) => `Dove sono i salvataggi di ${n} (PC e Steam Deck)`,
    desc: (n) =>
      `Dove ${n} tiene i salvataggi su PC e Steam Deck, se ha i salvataggi nel cloud e come farne il backup e sincronizzarli.`,
    lead: (c) =>
      `${c.n} tiene i salvataggi in ${code(c.g.w[0])} su Windows.${c.g.c.steam ? " Supporta Steam Cloud, che conserva l'ultimo salvataggio nel tuo account Steam." : ''} [Hoard](/download) trova quella cartella da solo, ne fa il backup ogni volta che il gioco salva e la tiene sincronizzata tra i tuoi PC e uno Steam Deck.`,
    where: (n) => `## Dove ${n} tiene i salvataggi`,
    win: '**Windows**',
    deck: '**Steam Deck e Linux (Proton)**',
    linux: '**Versione nativa per Linux**',
    mac: '**macOS**',
    user: "`<user-id>` è una cartella con il numero del tuo account; ce n'è una per ogni account che ha giocato su quella macchina.",
    steamDir: 'I percorsi sotto `C:\\Program Files (x86)\\Steam` presumono che Steam sia nella sua cartella predefinita.',
    source: (c) =>
      `Queste posizioni vengono da PCGamingWiki, tramite il database comunitario dei salvataggi che Hoard usa per il rilevamento.${c.prefix ? ` I percorsi dello Steam Deck sono quelli di Windows dentro il prefisso Proton del gioco, \`compatdata/${c.g.a}\`, che compare dopo il primo avvio.` : ''}`,
    cfgTitle: '## File di impostazioni',
    cfg: (n, k) => `${n} tiene le impostazioni, come grafica e comandi, separate dai salvataggi, in ${code(k)}.`,
    hoard: (n) => `## Backup e sync automatici con Hoard

[Hoard](/download) è gratuito e open source. Fa il backup dei salvataggi di ${n} ogni volta che il gioco ne scrive uno e di nuovo quando esci, tiene una cronologia a cui puoi tornare e sincronizza la cartella tra i tuoi PC e una Steam Deck. Viaggiano solo i file cambiati, compressi, e ogni file viene verificato con il suo hash SHA-256 quando torna.

1. Installa Hoard e accedi a Hoard One, consigliato per tutti, oppure puntalo verso [il tuo server](/guides/self-host-hoard).
2. Apri la **Libreria**. ${n} viene rilevato dalla tua libreria Steam e dal database comunitario dei salvataggi.
3. Gioca. Quando esci, la prima versione compare nella cronologia.`,
    faqTitle: '## Domande frequenti',
    qCloud: (n) => `### ${n} ha i salvataggi nel cloud?`,
    aCloud: (c, s) =>
      c.cloud.length
        ? `Sì, secondo il database dei salvataggi: ${s}. Il cloud di un negozio però conserva solo l'ultimo salvataggio di quell'account, senza versioni a cui tornare. Hoard in più conserva ogni versione, su Hoard One o sul tuo server.`
        : c.g.c.steam === false
          ? "Non su Steam, secondo il database dei salvataggi. Senza cloud, l'unica copia del salvataggio è su quel PC finché non ne fai un'altra; Hoard la fa per te, con ogni versione."
          : `Il database dei salvataggi non lo indica per ${c.n}, quindi controlla la sua pagina sul negozio. In ogni caso, Hoard tiene una sua copia con ogni versione, su Hoard One o sul tuo server.`,
    qDeck: (n) => `### Dove sono i salvataggi di ${n} su Steam Deck?`,
    aDeck: (c) =>
      `In ${code(c.g.d[0])}.${c.prefix ? ' Quella cartella sta dentro il prefisso Proton del gioco, quindi esiste solo dopo che il gioco è stato avviato almeno una volta.' : ''}${c.linux ? ` Se giochi alla versione nativa per Linux, guarda in ${code(c.linux)}.` : ''}`,
    fixed: `### Hoard è gratuito?

Sì. L'app, la riga di comando e il server sono gratuiti e open source (AGPL-3.0), e far girare il tuo server non costa altro che la macchina. Hoard One, il server gestito che consigliamo a tutti, è gratuito anche lui con 2 GB e tre dispositivi; Pro aggiunge spazio per 1,99 € al mese.

### Il self-hosting invia qualcosa a Hoard?

No. Sul tuo server non c'è nessun account da noi e nessuna telemetria verso di noi: i tuoi salvataggi, i tuoi utenti e i tuoi log restano sulla tua macchina e non toccano mai i nostri.`,
    stores: { steam: 'Steam Cloud', epic: "l'Epic Games Store", gog: 'GOG Galaxy' },
    join: joinWith(' e ')
  },
  ja: {
    title: (n) => `${n} のセーブデータの場所（PC・Steam Deck）`,
    desc: (n) =>
      `${n} が PC と Steam Deck でセーブを置く場所、クラウドセーブの有無、自動でバックアップ・同期する方法。`,
    lead: (c) =>
      `${c.n} は Windows では ${code(c.g.w[0])} にセーブを保存します。${c.g.c.steam ? 'Steam クラウドに対応しており、最新のセーブが Steam アカウントに保管されます。' : ''}[Hoard](/download) はこのフォルダーを自動で見つけ、ゲームがセーブするたびにバックアップし、PC と Steam Deck の間で同期を保ちます。`,
    where: (n) => `## ${n} のセーブの保存場所`,
    win: '**Windows**',
    deck: '**Steam Deck と Linux（Proton）**',
    linux: '**ネイティブ Linux 版**',
    mac: '**macOS**',
    user: '`<user-id>` はアカウントの数字 ID が名前になったフォルダーで、そのマシンで遊んだアカウントごとに 1 つあります。',
    steamDir: '`C:\\Program Files (x86)\\Steam` から始まるパスは、Steam が既定のフォルダーにある場合です。',
    source: (c) =>
      `これらの場所は PCGamingWiki の情報で、Hoard が検出に使うコミュニティのセーブデータベースから取得しています。${c.prefix ? `Steam Deck のパスは、ゲームの Proton プレフィックス \`compatdata/${c.g.a}\` の中にある Windows のパスで、初回起動後に作られます。` : ''}`,
    cfgTitle: '## 設定ファイル',
    cfg: (n, k) => `${n} はグラフィックや操作などの設定をセーブとは別に ${code(k)} に保存します。`,
    hoard: (n) => `## Hoard で自動バックアップと同期

[Hoard](/download) は無料のオープンソースです。ゲームがセーブを書くたびに、そして終了時にもう一度 ${n} のセーブをバックアップし、いつでも戻れる履歴を残し、フォルダーを PC と Steam Deck の間で同期します。転送されるのは変更されたファイルだけで、圧縮して送られ、戻ってきたファイルはすべて SHA-256 ハッシュで照合されます。

1. Hoard をインストールして、すべての人におすすめの Hoard One にサインインするか、[自分のサーバー](/guides/self-host-hoard) を指定します。
2. **ライブラリ** を開きます。${n} は Steam ライブラリとコミュニティのセーブデータベースから検出されます。
3. プレイします。終了すると、最初のバージョンが履歴に表示されます。`,
    faqTitle: '## よくある質問',
    qCloud: (n) => `### ${n} はクラウドセーブに対応していますか？`,
    aCloud: (c, s) =>
      c.cloud.length
        ? `はい。セーブデータベースによれば ${s} に対応しています。ただしストアのクラウドが残すのはそのアカウントの最新のセーブだけで、戻れる世代はありません。Hoard はそれに加えてすべての世代を、Hoard One か自分のサーバーに残します。`
        : c.g.c.steam === false
          ? 'セーブデータベースによれば、Steam では対応していません。クラウドがなければ、別にコピーを取らない限りセーブはその PC にしかありません。Hoard がすべての世代とともにコピーを取ります。'
          : `セーブデータベースには ${c.n} についての記載がないので、ストアページで確認してください。いずれにしても、Hoard はすべての世代付きで独自のコピーを、Hoard One か自分のサーバーに残します。`,
    qDeck: (n) => `### Steam Deck での ${n} のセーブはどこですか？`,
    aDeck: (c) =>
      `${code(c.g.d[0])} です。${c.prefix ? 'このフォルダーはゲームの Proton プレフィックスの中にあるため、ゲームを一度起動するまでは存在しません。' : ''}${c.linux ? `ネイティブ Linux 版で遊ぶ場合は ${code(c.linux)} を見てください。` : ''}`,
    fixed: `### Hoard は無料ですか？

はい。アプリ、コマンドライン、サーバーは無料のオープンソース（AGPL-3.0）で、自分のサーバーを動かすのにかかるのはマシン代だけです。すべての人におすすめの管理サーバーである Hoard One も 2 GB・3 台まで無料で、Pro は月 1.99 ユーロで容量を増やします。

### セルフホストすると Hoard に何か送られますか？

いいえ。自分のサーバーでは当方のアカウントも当方へのテレメトリーもありません。セーブ、ユーザー、ログはあなたのマシンに留まり、当方のマシンに触れることはありません。`,
    stores: { steam: 'Steam クラウド', epic: 'Epic Games Store', gog: 'GOG Galaxy' },
    join: joinWith('、', '、')
  },
  pt: {
    title: (n) => `Onde ficam os saves de ${n} (PC e Steam Deck)`,
    desc: (n) =>
      `Onde o ${n} guarda os saves no PC e na Steam Deck, se tem saves na nuvem e como fazer backup e sincronizá-los sozinhos.`,
    lead: (c) =>
      `O ${c.n} guarda os saves em ${code(c.g.w[0])} no Windows.${c.g.c.steam ? ' É compatível com o Steam Cloud, que guarda o save mais recente na tua conta Steam.' : ''} O [Hoard](/download) encontra essa pasta sozinho, faz backup dela sempre que o jogo grava e mantém-na sincronizada entre os teus PCs e uma Steam Deck.`,
    where: (n) => `## Onde o ${n} guarda os saves`,
    win: '**Windows**',
    deck: '**Steam Deck e Linux (Proton)**',
    linux: '**Versão nativa para Linux**',
    mac: '**macOS**',
    user: '`<user-id>` é uma pasta com o número da tua conta; há uma por cada conta que jogou nessa máquina.',
    steamDir: 'Os caminhos em `C:\\Program Files (x86)\\Steam` assumem que o Steam está na pasta predefinida.',
    source: (c) =>
      `Estas localizações vêm da PCGamingWiki, através da base de dados comunitária de saves que o Hoard usa para os detetar.${c.prefix ? ` Os caminhos da Steam Deck são os caminhos do Windows dentro do prefixo do Proton do jogo, \`compatdata/${c.g.a}\`, que aparece depois do primeiro arranque.` : ''}`,
    cfgTitle: '## Ficheiros de definições',
    cfg: (n, k) => `O ${n} guarda as definições, como gráficos e controlos, à parte dos saves, em ${code(k)}.`,
    hoard: (n) => `## Backup e sincronização automáticos com o Hoard

O [Hoard](/download) é gratuito e de código aberto. Faz backup dos teus saves de ${n} sempre que o jogo grava um e outra vez quando sais, guarda um histórico para onde podes voltar e sincroniza a pasta entre os teus PCs e uma Steam Deck. Só viajam os ficheiros que mudaram, comprimidos, e cada ficheiro é verificado com o seu hash SHA-256 quando regressa.

1. Instala o Hoard e inicia sessão no Hoard One, recomendado para todos, ou aponta-o para [o teu próprio servidor](/guides/self-host-hoard).
2. Abre a **Biblioteca**. O ${n} é detetado a partir da tua biblioteca Steam e da base de dados comunitária de saves.
3. Joga. Ao sair, a primeira versão aparece no histórico.`,
    faqTitle: '## Perguntas frequentes',
    qCloud: (n) => `### O ${n} tem saves na nuvem?`,
    aCloud: (c, s) =>
      c.cloud.length
        ? `Sim, segundo a base de dados de saves: ${s}. Mas a nuvem de uma loja só guarda o save mais recente dessa conta, sem versões para onde voltar. O Hoard guarda ainda todas as versões, no Hoard One ou no teu próprio servidor.`
        : c.g.c.steam === false
          ? 'Não no Steam, segundo a base de dados de saves. Sem nuvem, a única cópia do save está nesse PC, a não ser que faças outra; o Hoard faz isso por ti, com todas as versões.'
          : `A base de dados de saves não o indica para o ${c.n}, por isso confirma na página da loja. Em qualquer caso, o Hoard guarda a sua própria cópia com todas as versões, no Hoard One ou no teu próprio servidor.`,
    qDeck: (n) => `### Onde ficam os saves de ${n} na Steam Deck?`,
    aDeck: (c) =>
      `Em ${code(c.g.d[0])}.${c.prefix ? ' Essa pasta fica dentro do prefixo do Proton do jogo, por isso só existe depois de o jogo arrancar pelo menos uma vez.' : ''}${c.linux ? ` Se jogas a versão nativa para Linux, vê em ${code(c.linux)}.` : ''}`,
    fixed: `### O Hoard é gratuito?

Sim. A aplicação, a linha de comandos e o servidor são gratuitos e de código aberto (AGPL-3.0), e correr o teu próprio servidor não custa mais do que a máquina. O Hoard One, o servidor gerido que recomendamos a todos, também é gratuito com 2 GB e três dispositivos; o Pro acrescenta espaço por 1,99 € por mês.

### O self-hosting envia alguma coisa ao Hoard?

Não. No teu próprio servidor não há conta connosco nem telemetria para nós: os teus saves, os teus utilizadores e os teus logs ficam na tua máquina e nunca tocam nos nossos.`,
    stores: { steam: 'o Steam Cloud', epic: 'a Epic Games Store', gog: 'o GOG Galaxy' },
    join: joinWith(' e ')
  },
  zh: {
    title: (n) => `${n} 存档位置（PC 与 Steam Deck）`,
    desc: (n) => `${n} 在 PC 和 Steam Deck 上把存档放在哪里、是否支持云存档，以及如何自动备份和同步。`,
    lead: (c) =>
      `在 Windows 上，${c.n} 把存档放在 ${code(c.g.w[0])}。${c.g.c.steam ? '它支持 Steam 云，会把最新的存档保存在你的 Steam 账号里。' : ''}[Hoard](/download) 会自己找到这个文件夹，在游戏每次存档时备份，并在你的 PC 和 Steam Deck 之间保持同步。`,
    where: (n) => `## ${n} 把存档放在哪里`,
    win: '**Windows**',
    deck: '**Steam Deck 与 Linux（Proton）**',
    linux: '**Linux 原生版本**',
    mac: '**macOS**',
    user: '`<user-id>` 是以你的账号数字 ID 命名的文件夹；在这台机器上玩过的每个账号各有一个。',
    steamDir: '以 `C:\\Program Files (x86)\\Steam` 开头的路径假定 Steam 装在默认文件夹。',
    source: (c) =>
      `这些位置来自 PCGamingWiki，经由 Hoard 用于检测的社区存档数据库获得。${c.prefix ? `Steam Deck 路径是游戏 Proton 前缀 \`compatdata/${c.g.a}\` 里的 Windows 路径，首次启动游戏后才会出现。` : ''}`,
    cfgTitle: '## 设置文件',
    cfg: (n, k) => `${n} 把画面、操作等设置与存档分开保存，位置是 ${code(k)}。`,
    hoard: (n) => `## 用 Hoard 自动备份和同步

[Hoard](/download) 免费且开源。每次游戏写入存档时它都会备份你的 ${n} 存档，退出时再备份一次，保留可以随时回退的历史记录，并在你的 PC 和 Steam Deck 之间同步该文件夹。只有改动过的文件会被传输，而且经过压缩；每个文件取回时都会用 SHA-256 哈希校验。

1. 安装 Hoard 并登录 Hoard One（推荐所有人使用），或者把它指向[你自己的服务器](/guides/self-host-hoard)。
2. 打开**游戏库**。${n} 会通过你的 Steam 库和社区存档数据库被识别出来。
3. 开始游戏。退出后，第一个版本就会出现在历史记录里。`,
    faqTitle: '## 常见问题',
    qCloud: (n) => `### ${n} 支持云存档吗？`,
    aCloud: (c, s) =>
      c.cloud.length
        ? `支持，据存档数据库记载：${s}。不过商店的云只保留该账号最新的存档，没有可以回退的版本。Hoard 会在此之上保留所有版本，存放在 Hoard One 或你自己的服务器上。`
        : c.g.c.steam === false
          ? '据存档数据库记载，在 Steam 上不支持。没有云的话，除非你另做一份，存档就只在那台 PC 上；Hoard 会替你做这份副本，并保留所有版本。'
          : `存档数据库没有记录 ${c.n} 的这项信息，请查看它的商店页面。无论如何，Hoard 都会保留一份带所有版本的副本，存放在 Hoard One 或你自己的服务器上。`,
    qDeck: (n) => `### ${n} 在 Steam Deck 上的存档在哪里？`,
    aDeck: (c) =>
      `在 ${code(c.g.d[0])}。${c.prefix ? '这个文件夹位于游戏的 Proton 前缀里，所以要等游戏至少运行过一次才会出现。' : ''}${c.linux ? `如果你玩的是 Linux 原生版本，请看 ${code(c.linux)}。` : ''}`,
    fixed: `### Hoard 免费吗？

免费。应用、命令行和服务器都免费且开源（AGPL-3.0），运行你自己的服务器除了机器本身不花一分钱。我们推荐所有人使用的托管服务器 Hoard One 同样免费，提供 2 GB 和三台设备；Pro 每月 1.99 欧元，增加空间。

### 自托管会向 Hoard 发送任何东西吗？

不会。在你自己的服务器上，没有我们的账号，也没有发往我们的遥测：你的存档、你的用户和你的日志都留在你的机器上，从不接触我们的机器。`,
    stores: { steam: 'Steam 云', epic: 'Epic Games Store', gog: 'GOG Galaxy' },
    join: joinWith(' 和 ', '、')
  }
};

/** The page for one game in one locale, as Markdown plus its metadata. */
export function gameMarkdown(g: Game, locale: Locale) {
  const t = T[locale] ?? T.en;
  const n = esc(g.n);
  const cloud = (['steam', 'epic', 'gog'] as Store[]).filter((s) => g.c[s]);
  const prefix = g.d.some((p) => p.includes('/compatdata/'));
  const linux = g.l.find((p) => p !== g.d[0]) ?? null;
  const c: Ctx = { n, g, cloud, prefix, linux };
  const all = [...g.w, ...g.d, ...g.l, ...g.m];
  const body = [
    t.lead(c),
    t.where(n),
    `${t.win}\n\n${list(g.w)}`,
    g.d.length ? `${t.deck}\n\n${list(g.d)}` : '',
    g.l.length ? `${t.linux}\n\n${list(g.l)}` : '',
    g.m.length ? `${t.mac}\n\n${list(g.m)}` : '',
    all.some((p) => p.includes('<user-id>')) ? t.user : '',
    all.some((p) => p.includes('Program Files (x86)\\Steam')) ? t.steamDir : '',
    t.source(c),
    g.k.length ? `${t.cfgTitle}\n\n${t.cfg(n, g.k[0])}` : '',
    t.hoard(n),
    '<!-- faq -->',
    t.faqTitle,
    `${t.qCloud(n)}\n\n${t.aCloud(c, t.join(cloud.map((s) => t.stores[s])))}`,
    g.d.length ? `${t.qDeck(n)}\n\n${t.aDeck(c)}` : '',
    t.fixed
  ]
    .filter(Boolean)
    .join('\n\n');
  return { title: t.title(g.n), description: t.desc(g.n), body };
}
