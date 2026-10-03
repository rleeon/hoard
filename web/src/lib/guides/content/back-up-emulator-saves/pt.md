---
title: "Como fazer backup e sincronizar saves de emuladores (RetroArch, Dolphin, PCSX2)"
description: "Backup e sincronização de saves de emuladores entre PC e Steam Deck: RetroArch, Dolphin, PCSX2, DuckStation e mais, com histórico e onde cada um grava."
order: 6
updated: 2026-10-01
---

Os saves de emulador perdem-se com facilidade: ficheiros de save e save states vivem em pastas espalhadas, e uma reinstalação ou um PC novo podem apagar anos de progresso. O Hoard faz backup deles automaticamente e mantém-nos sincronizados entre as tuas máquinas, Steam Deck incluída.

## Emuladores com que o Hoard funciona

O Hoard trata os ficheiros de save habituais dos emuladores (`.srm`, `.sav`, memory cards, pastas de save por jogo) e os save states. Sabe de origem onde estes emuladores guardam:

- **Sony:** PCSX2 (PS2), DuckStation (PS1), PPSSPP (PSP), RPCS3 (PS3), shadPS4 (PS4), Vita3K (PS Vita)
- **Nintendo:** Dolphin (GameCube / Wii), Cemu (Wii U), Ryujinx, yuzu, Eden, Suyu, Citron e Sudachi (Switch), Citra / Azahar (3DS), melonDS (DS), mGBA (GBA), Project64 (N64)
- **Outros:** RetroArch (multissistema), xemu (Xbox), Flycast (Dreamcast)

Como o Hoard encontra as pastas de save com a mesma base de dados comunitária que o Ludusavi usa, muitos caminhos são detetados automaticamente. Para qualquer caminho personalizado, podes apontar o Hoard para uma pasta à mão.

## Configurar backups de saves de emulador

1. **Instala o Hoard** para Windows, macOS ou Linux e inicia sessão.
2. Abre a **Biblioteca** e adiciona o teu emulador, ou adiciona manualmente a sua pasta de saves/estados se mudaste a localização predefinida.
3. Mantém o **modo automático** ligado. O Hoard faz backup depois de cada sessão e guarda um histórico versionado.
4. Instala o Hoard nos teus outros PCs com a mesma conta para sincronizar esses saves em todo o lado — vê [como sincronizar saves entre PCs](/guides/sync-game-saves-across-pcs).

## Ludusavi para emuladores?

O Ludusavi também consegue fazer backup local de saves de emulador, e é uma ótima opção gratuita para isso. Se além disso quiseres que esses saves se sincronizem automaticamente entre máquinas e tenham um histórico de versões na nuvem sem configurar o Rclone, é aí que o Hoard ajuda — lê a [comparação completa entre Ludusavi e Hoard](/guides/ludusavi-alternative).

## Saves na nuvem para cada emulador

Nenhum dos emuladores independentes abaixo sincroniza saves entre máquinas por conta própria: os saves são ficheiros normais no teu disco. Isso é uma boa notícia, porque qualquer ferramenta que vigie a pasta certa consegue levá-los. Eis onde cada um os guarda. «Steam Deck» refere-se à versão Flatpak instalada a partir da loja Discover.

### Saves na nuvem do PCSX2 (PS2)

O PCSX2 escreve as memory cards (ficheiros `.ps2`) em `memcards/`:

- Windows: `Documentos\PCSX2\memcards`
- Linux: `~/.config/PCSX2/memcards`
- Steam Deck: `~/.var/app/net.pcsx2.PCSX2/config/PCSX2/memcards`

Uma memory card guarda os saves de todos os jogos que jogaste nela, por isso viaja como uma peça única: restaurar uma versão anterior recua a card inteira, não um jogo só.

### Saves na nuvem do Dolphin (GameCube e Wii)

Os saves de GameCube vivem em `GC/` (imagens de memory card ou uma pasta por card) e os de Wii na NAND emulada, em `Wii/`:

- Windows: `Documentos\Dolphin Emulator\GC` e `\Wii`
- Linux: `~/.local/share/dolphin-emu/GC` e `/Wii`
- Steam Deck: `~/.var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu/`

### Saves na nuvem do DuckStation (PS1)

O DuckStation guarda as memory cards em `memcards/` e, por predefinição, cria uma card separada para cada jogo, o que encaixa muito bem na sincronização:

- Windows: `Documentos\DuckStation\memcards` (as versões recentes usam `%LOCALAPPDATA%\DuckStation\memcards`)
- Linux: `~/.local/share/duckstation/memcards`
- Steam Deck: `~/.var/app/org.duckstation.DuckStation/`, em `data/` ou `config/`

### Sincronizar saves do RetroArch

O RetroArch separa `saves/` (os saves dos jogos) de `states/` (os save states). O Hoard acompanha a pasta de saves; adiciona `states/` como entrada própria se jogas com estados:

- Windows: `%APPDATA%\RetroArch`, ou junto a `retroarch.exe` numa instalação portátil
- Linux: `~/.config/retroarch`
- Steam Deck: `~/.var/app/org.libretro.RetroArch/config/retroarch`, ou `~/Emulation/saves/retroarch` se o configuraste com o EmuDeck

O RetroArch tem ainda um Cloud Sync integrado que fala com um servidor WebDAV fornecido por ti. É uma escolha razoável se só usas o RetroArch e já tens WebDAV. O Hoard não precisa de WebDAV, guarda um histórico de versões para recuperar e cobre também os emuladores independentes.

### PPSSPP (PSP)

Os saves vão para `PSP/SAVEDATA` e os estados para `PSP/PPSSPP_STATE`:

- Windows: `Documentos\PPSSPP\PSP\SAVEDATA`, ou `memstick\PSP\SAVEDATA` junto ao executável numa instalação portátil
- Linux: `~/.config/ppsspp/PSP/SAVEDATA`
- Steam Deck: `~/.var/app/org.ppsspp.PPSSPP/config/ppsspp/PSP/SAVEDATA`

### RPCS3 (PS3)

Os saves vivem em `dev_hdd0/home/00000001/savedata`, dentro da pasta do RPCS3 no Windows e em `~/.config/rpcs3/` no Linux e na Steam Deck.

### Emuladores de Switch: Ryujinx, yuzu, Eden, Suyu, Citron, Sudachi

O Ryujinx guarda os saves em `bis/user/save` (em `%APPDATA%\Ryujinx` ou `~/.config/Ryujinx`). A família yuzu usa `nand/user/save` dentro da sua própria pasta em `%APPDATA%` ou `~/.local/share`.

Há aqui uma armadilha. A árvore ao estilo yuzu é `save/<conta>/<perfil>/<id-do-jogo>/`, e o ID do perfil é gerado na primeira vez que o emulador arranca, por isso é diferente em cada instalação. Se sincronizares a pasta `save/` inteira entre duas máquinas, cada uma fica com o perfil da outra ao lado do seu, e nenhum jogo vê o progresso do outro. O Hoard desce antes até à pasta de cada jogo, para que o mesmo título corresponda entre máquinas seja qual for o nome do perfil.

### Citra e Azahar (3DS)

Os saves estão bem fundo em `sdmc/Nintendo 3DS/<id0>/<id1>/title/…`, e `id0`/`id1` vêm das chaves da consola emulada, por isso também mudam em cada instalação. O Hoard trata-os como a árvore da Switch: uma entrada por jogo, emparelhada entre máquinas.

### O resto

- **Cemu (Wii U):** `mlc01/usr/save`, em `%APPDATA%\Cemu` ou `~/.local/share/Cemu`.
- **shadPS4 (PS4):** `savedata`, em `%APPDATA%\shadPS4` ou `~/.local/share/shadPS4`.
- **Vita3K (PS Vita):** `ux0/user/00/savedata` dentro da sua pasta de dados.
- **mGBA, melonDS e a maioria dos emuladores da era dos cartuchos:** um `.sav` junto à ROM, salvo se lhes disseste outra coisa. Adiciona à mão os saves da pasta das ROMs.

## Saves de emulador na Steam Deck

Na Steam Deck os emuladores costumam vir em Flatpak, por isso as pastas ficam em `~/.var/app/<id>/` em vez dos habituais `~/.config` ou `~/.local/share`. O EmuDeck junta tudo em `~/Emulation/saves/`, uma pasta por emulador. Seja como for, adicionas a pasta uma vez e o Hoard vigia-a.

O que conta numa portátil: o motor do Hoard corre como um serviço em segundo plano, por isso faz o backup quando sais de um jogo no modo Jogo, sem nenhuma janela aberta. Pegas na Deck depois de uma sessão no PC e o save já lá está.

## Save e save state não são a mesma coisa

Vale a pena separá-los, porque comportam-se de forma diferente quando viajam:

- Um **save** (`.srm`, uma memory card, uma pasta `SAVEDATA`) é o save do próprio jogo, escrito pela consola emulada. Passa entre máquinas e versões do emulador sem problemas.
- Um **save state** é uma cópia da memória do emulador. Está preso à build do emulador, muitas vezes ao core exato, por isso um estado criado numa versão pode recusar-se a carregar noutra.

O Hoard faz backup dos dois. Só não te surpreendas se um estado de uma máquina atualizada não abrir numa que ficou para trás: mantém os emuladores na mesma versão e confia nos saves para o que importa.

## Um emulador, muitos jogos

Um emulador é um único processo que aloja dezenas de títulos, e é isso que torna os seus saves incómodos para uma ferramenta que pensa em «o jogo que está a correr». O Hoard mantém os títulos separados em vez de tratar o emulador como um bloco único, por isso cada jogo tem o seu histórico em vez de um monte comum que muda sempre que abres alguma coisa. Se um save se estragar, podes [voltar a uma versão anterior](/guides/restore-a-game-save).

## Saves de emulador sem passar pelos nossos servidores

Tudo isto funciona da mesma forma com o teu próprio servidor: arranca o `hoard-server`, aponta a aplicação para lá, e os teus saves vão da tua máquina para o teu disco. Sem conta connosco, sem telemetria para nós, nada passa pelos nossos servidores. Vê [como fazer self-host do Hoard](/guides/self-host-hoard).

## Dica

Os save states dependem de uma versão concreta do emulador. Mantém os emuladores atualizados de forma coerente entre PCs para que um estado sincronizado carregue bem em todo o lado.

<!-- faq -->

## Perguntas frequentes

### O Hoard também faz backup das minhas ROMs?

Não. Acompanha pastas de saves, não ficheiros de jogo. As ROMs são grandes, não mudam e já as tens: não há nada para versionar.

### O PCSX2, o Dolphin ou o DuckStation têm saves na nuvem integrados?

Não. Escrevem os saves em pastas locais e deixam a sincronização contigo. Aponta uma ferramenta de sincronização às pastas acima e os saves seguem-te entre máquinas.

### O RetroArch tem sincronização na nuvem?

Sim, um Cloud Sync integrado que precisa de um servidor WebDAV que geres ou alugas. O Hoard é a alternativa se preferires não configurar WebDAV, quiseres um histórico de versões para recuperar ou também jogares com emuladores independentes.

### Funciona numa Steam Deck no modo Jogo?

Sim. O motor corre como um serviço em segundo plano, por isso os saves são copiados quando sais de um jogo, sem janela aberta. As pastas de Flatpak e do EmuDeck funcionam como qualquer outra.

### O meu emulador é portátil. Funciona?

Sim. Adiciona à mão a pasta junto ao executável e o Hoard acompanha-a como qualquer outra localização de saves. É a configuração habitual nas consolas portáteis.

### Posso sincronizar save states entre dois PCs?

Podes, e o Hoard fá-lo. Se um estado carrega depende de os emuladores estarem na mesma versão nas duas máquinas, uma limitação do emulador e não da sincronização. Os saves não têm esse problema.

### Funciona com um emulador que não está na lista?

Quase de certeza. A deteção cobre os habituais automaticamente, e qualquer outro adicionas apontando o Hoard para a sua pasta de saves.

### O self-host muda alguma coisa para os emuladores?

Não. A mesma deteção, as mesmas versões, a mesma sincronização. Só o armazenamento é teu.
