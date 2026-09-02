/**
 * The Hoard-Wrapped card's phrases.
 *
 * The card finishes with a phrase picked at random from the range's most-played
 * game: if the most-played one is a Fallout, any of them, out comes "war never
 * changes"; if we do not recognise the game, it draws from the generic bag. The user
 * can always write their own over it (it stays local) or roll the dice again.
 *
 * Every phrase comes in the app's eight languages. They live here and not in
 * `i18n/locales/*.json` for the same reason as the rest of the Pro layer's text:
 * they are copy for one specific surface and have no business inflating the public
 * translation files.
 *
 * The matching is by *slug* and on token boundaries: the slug is wrapped in dashes
 * (`-elden-ring-`) and the patterns are tried against that shape, so `rust` matches
 * the game Rust but neither `rustler` nor `trust-issues`.
 */

/** Una frase en los ocho idiomas de la app. */
export type Quote = {
  en: string;
  es: string;
  de: string;
  fr: string;
  it: string;
  ja: string;
  pt: string;
  zh: string;
};

type QuoteEntry = {
  /** Id estable, se usa para la semilla del dado y para depurar. */
  id: string;
  /** Se prueba contra `-<slug>-`. */
  test: RegExp;
  quotes: Quote[];
};

/** Phrases per game. Adding a new one is adding an entry here. */
const BY_GAME: QuoteEntry[] = [
  {
    id: "fallout",
    test: /-fallout-/,
    quotes: [
      {
        es: "Supongo… que no cambia nunca.",
        en: "War… war never changes.",
        de: "Krieg… Krieg bleibt immer gleich.",
        fr: "La guerre… la guerre ne change jamais.",
        it: "La guerra… la guerra non cambia mai.",
        ja: "戦争は……変わらないものだ。",
        pt: "A guerra… a guerra nunca muda.",
        zh: "战争……战争从未改变。",
      },
      {
        es: "El refugio era más seguro. Y más aburrido.",
        en: "The Vault was safer. And duller.",
        de: "Im Vault war's sicherer. Und langweiliger.",
        fr: "L'abri était plus sûr. Et plus ennuyeux.",
        it: "Il Vault era più sicuro. E più noioso.",
        ja: "ヴォルトは安全だった。退屈だったけどね。",
        pt: "O Vault era mais seguro. E mais chato.",
        zh: "避难所更安全，也更无聊。",
      },
    ],
  },
  {
    id: "rust",
    test: /-rust-/,
    quotes: [
      {
        es: "Te dormiste vestido y despertaste desnudo.",
        en: "You went to sleep dressed and woke up naked.",
        de: "Angezogen eingeschlafen, nackt aufgewacht.",
        fr: "Endormi habillé, réveillé tout nu.",
        it: "Ti sei addormentato vestito e svegliato nudo.",
        ja: "服を着て寝たのに、裸で目が覚めた。",
        pt: "Dormiu vestido e acordou pelado.",
        zh: "穿着衣服睡下，光着身子醒来。",
      },
      {
        es: "Piedra, madera y traición.",
        en: "Stone, wood, and betrayal.",
        de: "Stein, Holz und Verrat.",
        fr: "Pierre, bois et trahison.",
        it: "Pietra, legno e tradimento.",
        ja: "石と木と、裏切り。",
        pt: "Pedra, madeira e traição.",
        zh: "石头、木头，还有背叛。",
      },
    ],
  },
  {
    id: "minecraft",
    test: /-minecraft-/,
    quotes: [
      {
        es: "Solo un bloque más y me acuesto.",
        en: "Just one more block, then bed.",
        de: "Nur noch ein Block, dann ins Bett.",
        fr: "Encore un bloc, et au lit.",
        it: "Ancora un blocco e poi a letto.",
        ja: "あと1ブロックだけ掘って寝る。",
        pt: "Só mais um bloco e vou dormir.",
        zh: "再挖一个方块就睡。",
      },
      {
        es: "La cama está ahí. Justo ahí. Desde hace tres horas.",
        en: "The bed is right there. Has been for three hours.",
        de: "Das Bett steht da. Seit drei Stunden.",
        fr: "Le lit est là. Depuis trois heures.",
        it: "Il letto è lì. Da tre ore.",
        ja: "ベッドはそこにある。3時間前から。",
        pt: "A cama está ali. Há três horas.",
        zh: "床就在那儿，已经三个小时了。",
      },
    ],
  },
  {
    id: "skyrim",
    test: /-skyrim-|-elder-scrolls-/,
    quotes: [
      {
        es: "Ibas para aventurero, hasta lo de la rodilla.",
        en: "You were an adventurer, until the knee thing.",
        de: "Du warst Abenteurer – bis zu der Sache mit dem Knie.",
        fr: "Tu étais aventurier, jusqu'à cette histoire de genou.",
        it: "Eri un avventuriero, poi la storia del ginocchio.",
        ja: "昔は冒険者だった。膝に矢を受けるまでは。",
        pt: "Você era aventureiro, até aquilo do joelho.",
        zh: "你曾是个冒险者，直到膝盖中了一箭。",
      },
      {
        es: "Cinco misiones abiertas y estás vendiendo queso.",
        en: "Five quests open and you're selling cheese.",
        de: "Fünf offene Quests, und du verkaufst Käse.",
        fr: "Cinq quêtes en cours, et tu vends du fromage.",
        it: "Cinque missioni aperte e vendi formaggio.",
        ja: "クエストを5つ放置して、チーズを売っている。",
        pt: "Cinco missões abertas e você vendendo queijo.",
        zh: "五个任务没做，你在卖奶酪。",
      },
    ],
  },
  {
    id: "elden-ring",
    test: /-elden-ring-/,
    quotes: [
      {
        es: "Has muerto. Y aun así, volviste.",
        en: "You died. And still, you went back.",
        de: "Du bist gestorben. Und trotzdem weitergegangen.",
        fr: "Tu es mort. Et tu y es retourné.",
        it: "Sei morto. E ci sei tornato lo stesso.",
        ja: "死んだ。それでも、また向かった。",
        pt: "Você morreu. E mesmo assim voltou.",
        zh: "你死了。然后又回去了。",
      },
      {
        es: "“Un jefe más”, dijiste. Hace seis horas.",
        en: "“One more boss,” you said. Six hours ago.",
        de: "„Nur noch ein Boss“, sagtest du. Vor sechs Stunden.",
        fr: "« Encore un boss », disais-tu. Il y a six heures.",
        it: "“Ancora un boss”, hai detto. Sei ore fa.",
        ja: "「あと1体だけ」と言ってから6時間。",
        pt: "“Só mais um chefe”, você disse. Há seis horas.",
        zh: "你说“再打一个boss”，那是六小时前。",
      },
    ],
  },
  {
    id: "souls",
    test: /-dark-souls-|-demons?-souls-|-bloodborne-|-sekiro-/,
    quotes: [
      {
        es: "Hoguera, muerte, hoguera. Es un ciclo.",
        en: "Bonfire, death, bonfire. It's a cycle.",
        de: "Leuchtfeuer, Tod, Leuchtfeuer. Ein Kreislauf.",
        fr: "Feu de camp, mort, feu de camp. Un cycle.",
        it: "Falò, morte, falò. È un ciclo.",
        ja: "篝火、死、篝火。ただの輪廻だ。",
        pt: "Fogueira, morte, fogueira. É um ciclo.",
        zh: "篝火，死亡，篝火。不过是轮回。",
      },
      {
        es: "Dejaste tus almas ahí abajo. Ve a por ellas.",
        en: "You left your souls down there. Go get them.",
        de: "Deine Seelen liegen da unten. Hol sie dir.",
        fr: "Tes âmes sont restées en bas. Va les chercher.",
        it: "Hai lasciato le anime laggiù. Vai a riprenderle.",
        ja: "ソウルは下に置いてきた。取りに行こう。",
        pt: "Você deixou suas almas lá embaixo. Vá buscá-las.",
        zh: "魂还留在下面，去拿回来。",
      },
    ],
  },
  {
    id: "stardew",
    test: /-stardew-/,
    quotes: [
      {
        es: "El abuelo estaría orgulloso. Creo.",
        en: "Grandpa would be proud. Probably.",
        de: "Opa wäre stolz. Wahrscheinlich.",
        fr: "Grand-père serait fier. Sans doute.",
        it: "Il nonno sarebbe fiero. Credo.",
        ja: "じいちゃんも喜んでる。たぶん。",
        pt: "O vovô ficaria orgulhoso. Acho.",
        zh: "爷爷会为你骄傲的。大概吧。",
      },
      {
        es: "Te levantas a las seis para regar. En vacaciones.",
        en: "Up at six to water crops. On your day off.",
        de: "Um sechs auf zum Gießen. An deinem freien Tag.",
        fr: "Debout à six heures pour arroser. Pendant tes vacances.",
        it: "Sveglia alle sei per innaffiare. In vacanza.",
        ja: "休みの日も6時起きで水やり。",
        pt: "Acorda às seis para regar. De folga.",
        zh: "假期也六点起来浇水。",
      },
    ],
  },
  {
    id: "factorio",
    test: /-factorio-/,
    quotes: [
      {
        es: "La fábrica debe crecer.",
        en: "The factory must grow.",
        de: "Die Fabrik muss wachsen.",
        fr: "L'usine doit grandir.",
        it: "La fabbrica deve crescere.",
        ja: "工場は拡張し続けねばならない。",
        pt: "A fábrica deve crescer.",
        zh: "工厂必须扩张。",
      },
      {
        es: "“Solo arreglo esta cinta”, y amaneció.",
        en: "“Just fixing this belt,” and the sun came up.",
        de: "„Nur schnell dieses Band“, und es wurde hell.",
        fr: "« Je répare juste ce convoyeur », et le jour s'est levé.",
        it: "“Sistemo solo questo nastro”, ed è arrivata l'alba.",
        ja: "「このベルトを直すだけ」で朝になった。",
        pt: "“Só vou ajeitar essa esteira”, e amanheceu.",
        zh: "“就修一下这条传送带”，然后天亮了。",
      },
    ],
  },
  {
    id: "terraria",
    test: /-terraria-/,
    quotes: [
      {
        es: "Cavar, cavar, cavar. Hasta el infierno.",
        en: "Dig, dig, dig. All the way to hell.",
        de: "Graben, graben, graben. Bis zur Hölle.",
        fr: "Creuser, creuser, creuser. Jusqu'en enfer.",
        it: "Scava, scava, scava. Fino all'inferno.",
        ja: "掘って掘って掘って、地獄まで。",
        pt: "Cavar, cavar, cavar. Até o inferno.",
        zh: "挖啊挖，一直挖到地狱。",
      },
      {
        es: "Ese muro sospechoso escondía algo. Siempre lo hace.",
        en: "That suspicious wall was hiding something. It always is.",
        de: "Hinter der verdächtigen Wand war was. Immer.",
        fr: "Ce mur suspect cachait un truc. Comme toujours.",
        it: "Quel muro sospetto nascondeva qualcosa. Come sempre.",
        ja: "怪しい壁の奥には何かある。いつもそうだ。",
        pt: "Aquela parede suspeita escondia algo. Sempre esconde.",
        zh: "那面可疑的墙后面有东西。总是有。",
      },
    ],
  },
  {
    id: "cyberpunk",
    test: /-cyberpunk-/,
    quotes: [
      {
        es: "Despertaste en Night City y ya llegabas tarde.",
        en: "You woke up in Night City already running late.",
        de: "In Night City aufgewacht – und schon zu spät.",
        fr: "Réveillé à Night City, déjà en retard.",
        it: "Ti sei svegliato a Night City già in ritardo.",
        ja: "ナイトシティで目覚めた時点で、もう遅刻。",
        pt: "Acordou em Night City já atrasado.",
        zh: "在夜之城醒来，已经迟到了。",
      },
      {
        es: "La ciudad te quería. Tú querías el coche.",
        en: "The city wanted you. You wanted the car.",
        de: "Die Stadt wollte dich. Du wolltest das Auto.",
        fr: "La ville te voulait. Toi, tu voulais la bagnole.",
        it: "La città voleva te. Tu volevi l'auto.",
        ja: "街はお前を求めた。お前は車を求めた。",
        pt: "A cidade queria você. Você queria o carro.",
        zh: "这座城市想要你，你只想要那辆车。",
      },
    ],
  },
  {
    id: "witcher",
    test: /-witcher-/,
    quotes: [
      {
        es: "Un trago, una de Gwent y a matar monstruos.",
        en: "A drink, a hand of Gwent, then monsters.",
        de: "Ein Schluck, eine Runde Gwint, dann Monster.",
        fr: "Un verre, une partie de Gwent, puis les monstres.",
        it: "Un sorso, una mano a Gwent, poi i mostri.",
        ja: "一杯やって、グウェントを一局、それから怪物退治。",
        pt: "Um gole, uma de Gwent, e aí monstros.",
        zh: "先喝一杯，来局昆特牌，再去猎魔。",
      },
      {
        es: "Los monstruos pagan mal, pero pagan.",
        en: "Monsters pay badly, but they pay.",
        de: "Monster zahlen schlecht, aber sie zahlen.",
        fr: "Les monstres paient mal, mais ils paient.",
        it: "I mostri pagano poco, ma pagano.",
        ja: "怪物退治は安い。だが金にはなる。",
        pt: "Monstro paga mal, mas paga.",
        zh: "猎魔赚得少，但总归有钱拿。",
      },
    ],
  },
  {
    id: "gta",
    test: /-grand-theft-auto-|-gta-/,
    quotes: [
      {
        es: "Cinco estrellas y ninguna era una reseña.",
        en: "Five stars, and none of them a review.",
        de: "Fünf Sterne – keiner davon eine Bewertung.",
        fr: "Cinq étoiles, et aucune n'était un avis.",
        it: "Cinque stelle, e nessuna era una recensione.",
        ja: "星5つ。どれもレビューじゃない。",
        pt: "Cinco estrelas, e nenhuma era avaliação.",
        zh: "五颗星，没有一颗是好评。",
      },
      {
        es: "Ibas a hacer la misión. Ibas.",
        en: "You were going to do the mission. You were.",
        de: "Du wolltest die Mission machen. Wolltest.",
        fr: "Tu allais faire la mission. Tu allais.",
        it: "Dovevi fare la missione. Dovevi.",
        ja: "ミッションをやるつもりだった。つもりは。",
        pt: "Você ia fazer a missão. Ia.",
        zh: "你本来是要去做任务的。本来。",
      },
    ],
  },
  {
    id: "red-dead",
    test: /-red-dead-/,
    quotes: [
      {
        es: "Tengo un plan, Dutch.",
        en: "I have a plan, Dutch.",
        de: "Ich habe einen Plan, Dutch.",
        fr: "J'ai un plan, Dutch.",
        it: "Ho un piano, Dutch.",
        ja: "計画があるんだ、ダッチ。",
        pt: "Eu tenho um plano, Dutch.",
        zh: "我有个计划，达奇。",
      },
      {
        es: "El caballo era el verdadero protagonista.",
        en: "The horse was the real protagonist.",
        de: "Das Pferd war der eigentliche Held.",
        fr: "Le vrai héros, c'était le cheval.",
        it: "Il vero protagonista era il cavallo.",
        ja: "本当の主人公は馬だった。",
        pt: "O verdadeiro protagonista era o cavalo.",
        zh: "真正的主角是那匹马。",
      },
    ],
  },
  {
    id: "hollow-knight",
    test: /-hollow-knight-|-silksong-/,
    quotes: [
      {
        es: "Otro banco. Un poquito más lejos.",
        en: "One more bench. A little further out.",
        de: "Noch eine Bank. Ein Stück weiter.",
        fr: "Encore un banc. Un peu plus loin.",
        it: "Un'altra panchina. Un po' più in là.",
        ja: "ベンチをもう一つ、少し先へ。",
        pt: "Mais um banco. Um pouco mais longe.",
        zh: "再多一张长椅，再往前一点。",
      },
      {
        es: "El mapa se compra después de perderse.",
        en: "You buy the map after getting lost.",
        de: "Die Karte kauft man, nachdem man sich verlaufen hat.",
        fr: "On achète la carte après s'être perdu.",
        it: "La mappa si compra dopo essersi persi.",
        ja: "地図は迷ってから買うものだ。",
        pt: "O mapa se compra depois de se perder.",
        zh: "地图总是在迷路之后才买。",
      },
    ],
  },
  {
    id: "hades",
    test: /-hades-/,
    quotes: [
      {
        es: "La muerte es solo otro pasillo.",
        en: "Death is just another hallway.",
        de: "Der Tod ist nur ein weiterer Flur.",
        fr: "La mort n'est qu'un couloir de plus.",
        it: "La morte è solo un altro corridoio.",
        ja: "死もまた、ただの通路にすぎない。",
        pt: "A morte é só mais um corredor.",
        zh: "死亡不过是另一条走廊。",
      },
      {
        es: "Padre, no es nada personal. Bueno, sí.",
        en: "Father, it's not personal. Fine, it is.",
        de: "Vater, das ist nichts Persönliches. Gut, doch.",
        fr: "Père, ce n'est pas personnel. Bon, si.",
        it: "Padre, non è personale. Va bene, lo è.",
        ja: "父さん、私情はない。……いや、ある。",
        pt: "Pai, não é pessoal. Tá, é.",
        zh: "父亲，这无关私怨。好吧，有关。",
      },
    ],
  },
  {
    id: "no-mans-sky",
    test: /-no-mans?-sky-/,
    quotes: [
      {
        es: "Otro planeta, la misma piedra.",
        en: "Another planet, the same rock.",
        de: "Ein neuer Planet, derselbe Stein.",
        fr: "Une autre planète, le même caillou.",
        it: "Un altro pianeta, la stessa roccia.",
        ja: "別の惑星、同じ石。",
        pt: "Outro planeta, a mesma pedra.",
        zh: "换了个星球，还是那块石头。",
      },
      {
        es: "Ibas a explorar la galaxia y montaste un almacén.",
        en: "You set out to explore the galaxy and built a warehouse.",
        de: "Du wolltest die Galaxie erkunden – und baust ein Lager.",
        fr: "Tu partais explorer la galaxie, tu as monté un entrepôt.",
        it: "Volevi esplorare la galassia e hai aperto un magazzino.",
        ja: "銀河を探検するはずが、倉庫を建てていた。",
        pt: "Ia explorar a galáxia e montou um depósito.",
        zh: "本想探索银河，结果开了个仓库。",
      },
    ],
  },
  {
    id: "subnautica",
    test: /-subnautica-/,
    quotes: [
      {
        es: "El agua estaba tranquila hasta que algo gritó.",
        en: "The water was calm until something screamed.",
        de: "Das Wasser war ruhig – bis etwas schrie.",
        fr: "L'eau était calme, jusqu'à ce que ça hurle.",
        it: "L'acqua era calma, poi qualcosa ha urlato.",
        ja: "海は静かだった。何かが叫ぶまでは。",
        pt: "A água estava calma até algo gritar.",
        zh: "海水很平静，直到有什么东西尖叫。",
      },
      {
        es: "Aquí abajo no hay nada. Nada. ¿Eso qué era?",
        en: "There's nothing down here. Nothing. What was that?",
        de: "Hier unten ist nichts. Nichts. Was war das?",
        fr: "Il n'y a rien ici. Rien. C'était quoi, ça ?",
        it: "Quaggiù non c'è niente. Niente. Cos'era?",
        ja: "ここには何もいない。何も。……今のは？",
        pt: "Aqui embaixo não tem nada. Nada. O que foi isso?",
        zh: "下面什么都没有。什么都没有。刚才那是什么？",
      },
    ],
  },
  {
    id: "baldurs-gate",
    test: /-baldurs?-gate-/,
    quotes: [
      {
        es: "Un descanso largo lo arregla casi todo.",
        en: "A long rest fixes almost everything.",
        de: "Eine lange Rast richtet fast alles.",
        fr: "Un repos long répare presque tout.",
        it: "Un riposo lungo sistema quasi tutto.",
        ja: "大休憩でだいたい何とかなる。",
        pt: "Um descanso longo resolve quase tudo.",
        zh: "长休几乎能解决一切。",
      },
      {
        es: "Sacaste un 1 y la historia mejoró.",
        en: "You rolled a 1 and the story got better.",
        de: "Du hast eine 1 gewürfelt – die Geschichte wurde besser.",
        fr: "Tu as fait un 1, et l'histoire s'est améliorée.",
        it: "Hai tirato 1 e la storia è migliorata.",
        ja: "1を出した。物語は面白くなった。",
        pt: "Você tirou 1 e a história ficou melhor.",
        zh: "你掷出了1，故事反而更精彩了。",
      },
    ],
  },
  {
    id: "civilization",
    test: /-civilization-|-sid-meiers-civ/,
    quotes: [
      {
        es: "Un turno más.",
        en: "Just one more turn.",
        de: "Nur noch eine Runde.",
        fr: "Encore un tour.",
        it: "Ancora un turno.",
        ja: "あと1ターンだけ。",
        pt: "Só mais um turno.",
        zh: "再来一回合就好。",
      },
      {
        es: "Empezaste pacífico. Preguntemos a tus vecinos.",
        en: "You started peaceful. Let's ask your neighbours.",
        de: "Du hast friedlich angefangen. Fragen wir die Nachbarn.",
        fr: "Tu commençais pacifique. Demandons à tes voisins.",
        it: "Eri partito pacifico. Chiediamo ai vicini.",
        ja: "平和主義で始めたはず。隣国に聞いてみよう。",
        pt: "Você começou pacífico. Vamos perguntar aos vizinhos.",
        zh: "你本来走的是和平路线。问问邻居吧。",
      },
    ],
  },
  {
    id: "portal",
    test: /-portal-/,
    quotes: [
      {
        es: "La tarta sigue siendo mentira.",
        en: "The cake is still a lie.",
        de: "Der Kuchen ist immer noch eine Lüge.",
        fr: "Le gâteau est toujours un mensonge.",
        it: "La torta è ancora una bugia.",
        ja: "ケーキはやっぱり嘘だった。",
        pt: "O bolo ainda é mentira.",
        zh: "蛋糕依然是个谎言。",
      },
      {
        es: "Ahora piensas con portales hasta en la cocina.",
        en: "Now you think with portals, even in the kitchen.",
        de: "Jetzt denkst du in Portalen. Sogar in der Küche.",
        fr: "Maintenant tu penses en portails. Même en cuisine.",
        it: "Ora pensi con i portali. Anche in cucina.",
        ja: "もう台所でもポータルで考えている。",
        pt: "Agora você pensa com portais até na cozinha.",
        zh: "现在你连在厨房都用传送门思考。",
      },
    ],
  },
  {
    id: "doom",
    test: /-doom-/,
    quotes: [
      {
        es: "Desgarra y destroza, hasta acabar.",
        en: "Rip and tear, until it is done.",
        de: "Reiß und zerfetz, bis es vollbracht ist.",
        fr: "Déchire et massacre, jusqu'au bout.",
        it: "Squarcia e distruggi, fino alla fine.",
        ja: "引き裂け、切り裂け、終わるまで。",
        pt: "Rasgue e destrua, até acabar.",
        zh: "撕裂，粉碎，直到结束。",
      },
      {
        es: "Los demonios preguntaron por ti. Con miedo.",
        en: "The demons asked about you. Nervously.",
        de: "Die Dämonen haben nach dir gefragt. Nervös.",
        fr: "Les démons ont demandé après toi. Nerveusement.",
        it: "I demoni hanno chiesto di te. Con timore.",
        ja: "デーモンたちがお前の噂をしていた。怯えながら。",
        pt: "Os demônios perguntaram por você. Com medo.",
        zh: "恶魔们打听过你，语气很紧张。",
      },
    ],
  },
  {
    id: "valheim",
    test: /-valheim-/,
    quotes: [
      {
        es: "El bosque tiene ojos y tú tienes un hacha.",
        en: "The forest has eyes and you have an axe.",
        de: "Der Wald hat Augen, du hast eine Axt.",
        fr: "La forêt a des yeux, toi tu as une hache.",
        it: "La foresta ha occhi e tu hai un'ascia.",
        ja: "森には目がある。こっちには斧がある。",
        pt: "A floresta tem olhos e você tem um machado.",
        zh: "森林有眼睛，而你有一把斧头。",
      },
      {
        es: "Otro barco hundido, otro cargamento perdido.",
        en: "Another ship sunk, another cargo lost.",
        de: "Noch ein Schiff versenkt, noch eine Ladung weg.",
        fr: "Encore un bateau coulé, encore une cargaison perdue.",
        it: "Un'altra nave affondata, un altro carico perso.",
        ja: "また船が沈んで、また積荷が消えた。",
        pt: "Mais um barco afundado, mais uma carga perdida.",
        zh: "又沉了一条船，又丢了一船货。",
      },
    ],
  },
];

/** The generic bag: it comes out when the game is not in the list (or there is no game). */
const GENERIC: Quote[] = [
  {
    es: "Las horas no se pierden si se guardan.",
    en: "Hours aren't lost if they're saved.",
    de: "Stunden gehen nicht verloren, wenn man sie speichert.",
    fr: "Les heures ne se perdent pas si on les sauvegarde.",
    it: "Le ore non si perdono, se le salvi.",
    ja: "時間はセーブすれば失われない。",
    pt: "As horas não se perdem se forem salvas.",
    zh: "只要存档，时间就不会白费。",
  },
  {
    es: "Tu partida está a salvo. Tu sueño, no.",
    en: "Your save is safe. Your sleep isn't.",
    de: "Dein Spielstand ist sicher. Dein Schlaf nicht.",
    fr: "Ta sauvegarde est en sécurité. Ton sommeil, non.",
    it: "Il tuo salvataggio è al sicuro. Il tuo sonno no.",
    ja: "セーブデータは無事。睡眠は無事じゃない。",
    pt: "Seu save está seguro. Seu sono, não.",
    zh: "你的存档很安全，你的睡眠不是。",
  },
  {
    es: "Nadie recuerda las noches que dormiste.",
    en: "Nobody remembers the nights you slept.",
    de: "Niemand erinnert sich an die Nächte, in denen du geschlafen hast.",
    fr: "Personne ne se souvient des nuits où tu as dormi.",
    it: "Nessuno ricorda le notti in cui hai dormito.",
    ja: "よく寝た夜のことは、誰も覚えていない。",
    pt: "Ninguém lembra das noites em que você dormiu.",
    zh: "没人记得你睡着的那些夜晚。",
  },
  {
    es: "Guardaste antes del jefe. Buena decisión.",
    en: "You saved before the boss. Good call.",
    de: "Du hast vor dem Boss gespeichert. Kluge Entscheidung.",
    fr: "Tu as sauvegardé avant le boss. Bien joué.",
    it: "Hai salvato prima del boss. Ottima idea.",
    ja: "ボス前にセーブした。賢明だ。",
    pt: "Você salvou antes do chefe. Boa escolha.",
    zh: "你在boss前存了档。明智。",
  },
  {
    es: "Otro año, otra vida paralela.",
    en: "Another year, another parallel life.",
    de: "Noch ein Jahr, noch ein Parallelleben.",
    fr: "Une année de plus, une vie parallèle de plus.",
    it: "Un altro anno, un'altra vita parallela.",
    ja: "また一年、もう一つの人生。",
    pt: "Mais um ano, mais uma vida paralela.",
    zh: "又一年，又一段平行人生。",
  },
  {
    es: "El progreso pesa poco. Menos mal.",
    en: "Progress weighs almost nothing. Luckily.",
    de: "Fortschritt wiegt fast nichts. Zum Glück.",
    fr: "Le progrès ne pèse presque rien. Heureusement.",
    it: "I progressi pesano poco. Per fortuna.",
    ja: "進行状況は軽い。ありがたい。",
    pt: "O progresso pesa pouco. Ainda bem.",
    zh: "进度不占多少空间。万幸。",
  },
];

/** How many games the catalogue recognises. The UI uses it to boast. */
export const KNOWN_GAMES = BY_GAME.length;

/** Normalises a slug to `-token-token-` so it can be matched on boundaries. */
function bounded(slug: string): string {
  return `-${slug.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "")}-`;
}

/** The entry a slug maps to, or `null` when it is not in the catalogue. */
export function matchGame(slug: string | null | undefined): string | null {
  if (!slug) return null;
  const key = bounded(slug);
  return BY_GAME.find((g) => g.test.test(key))?.id ?? null;
}

/**
 * The bag of phrases for a slug. When we recognise the game, **only** its own come
 * out: the point is that somebody who plays a Fallout reads the line about war, not
 * a filler phrase. The generic ones are for when we do not know what they play.
 */
export function quotesFor(slug: string | null | undefined): Quote[] {
  const id = matchGame(slug);
  const entry = BY_GAME.find((g) => g.id === id);
  return entry ? [...entry.quotes] : [...GENERIC];
}

/**
 * Picks a phrase deterministically: the same seed and the same game always give the
 * same phrase, so the card does not flicker between renders and the dice button is
 * no more than "bump the seed".
 */
export function pickQuote(slug: string | null | undefined, seed: number): Quote {
  const pool = quotesFor(slug);
  return pool[Math.abs(Math.trunc(seed)) % pool.length];
}
