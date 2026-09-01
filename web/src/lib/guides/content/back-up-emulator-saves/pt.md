---
title: "Como fazer backup e sincronizar saves de emuladores (RetroArch, Dolphin, PCSX2)"
description: "Faz backup e sincroniza os ficheiros de save e os save states dos teus emuladores entre PCs — RetroArch, Dolphin, PCSX2, DuckStation e mais — automaticamente com o Hoard."
order: 6
updated: 2026-09-01
---

Os saves de emulador perdem-se com facilidade: ficheiros de save e save states vivem em pastas espalhadas, e uma reinstalação ou um PC novo podem apagar anos de progresso. O Hoard faz-lhes backup automaticamente e mantém-nos sincronizados entre máquinas.

## Emuladores com que o Hoard funciona

O Hoard trata os ficheiros de save padrão de emulador (`.srm`, `.sav`, memory cards) e os save states dos emuladores populares, incluindo:

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

- **RetroArch** — `saves/` e `states/` dentro da pasta de configuração: `%APPDATA%\RetroArch` no Windows, `~/.config/retroarch` no Linux.
- **Dolphin** — memory cards em `GC/`, saves de Wii na NAND emulada, dentro de `Documentos\Dolphin Emulator` ou `~/.local/share/dolphin-emu`.
- **PCSX2** — `memcards/`, em `Documentos\PCSX2` ou `~/.config/PCSX2`.
- **DuckStation** — `memcards/` e `savestates/` na sua própria pasta de dados.
- **PPSSPP** — `PSP/SAVEDATA` para os saves e `PSP/PPSSPP_STATE` para os estados.
- **RPCS3** — `dev_hdd0/home/00000001/savedata`.
- **Cemu** — `mlc01/usr/save`.
- **mGBA e a maioria dos cores autónomos** — um `.sav` ao lado da ROM, salvo se lhes disseste outra coisa.

Uma **instalação portable** — o normal em consolas portáteis e pens USB — guarda tudo isso ao lado do executável. Se é o teu caso, aponta o Hoard para essa pasta e ele segue-a como qualquer outro save.

## Save e save state não são a mesma coisa

Vale a pena separá-los, porque viajam de maneira diferente:

- Um **save** (`.srm`, um memory card, uma pasta `SAVEDATA`) é o guardado próprio do jogo, escrito pela consola emulada. Passa de máquina para máquina e entre versões de emulador sem se queixar.
- Um **save state** é um despejo da memória do emulador. Está preso àquela build, e muitas vezes ao core exato, por isso um estado escrito por uma versão pode recusar-se a carregar noutra.

O Hoard copia os dois. Só não estranhes que um estado de uma máquina atualizada não abra numa que ficou para trás: mantém os emuladores na mesma versão e apoia-te nos saves normais para o que te importa.

## Um emulador, muitos jogos

Um emulador é um único processo a alojar dezenas de títulos, e é isso que torna os saves de emulador incómodos para uma ferramenta que pensa em "o jogo que está a correr". O Hoard mantém os títulos separados em vez de tratar o emulador inteiro como um bloco só, por isso cada jogo tem o seu histórico e não um monte comum que muda sempre que abres seja o que for.

## Saves de emulador sem passar pelos nossos servidores

Tudo isto funciona igual contra o teu próprio servidor: corre o `hoard-server`, aponta a aplicação para lá, e os teus saves vão da tua máquina para o teu disco. Sem conta connosco, sem telemetria para nós, nada pelos nossos servidores. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard).

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
