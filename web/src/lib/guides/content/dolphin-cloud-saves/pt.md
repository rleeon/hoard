---
title: "Saves na nuvem para o Dolphin: sincroniza GameCube e Wii entre PC e Steam Deck"
description: "O Dolphin não tem saves na nuvem. Sincroniza GameCube e Wii entre PC e Steam Deck automaticamente, com histórico: caminhos, cartões e armadilhas."
order: 17
updated: 2026-10-09
---

O Dolphin não sincroniza saves entre máquinas: os teus memory cards de GameCube e a tua Wii emulada vivem numa pasta de um só PC. O Hoard sincroniza-os automaticamente. Quando fechas o Dolphin, faz backup dos teus saves de GameCube e Wii, descarrega-os nos teus outros PCs e na tua Steam Deck, e guarda todas as versões para que possas sempre voltar atrás.

## Onde o Dolphin guarda os teus saves

Tudo vive na pasta de utilizador do Dolphin. A forma mais rápida de a encontrar é **File → Open User Folder** dentro do Dolphin. Lá dentro:

- `GC` tem os memory cards de GameCube.
- `Wii` é a memória interna da Wii emulada, com os saves.
- `StateSaves` tem os estados.

Onde fica essa pasta:

- **Windows:** `Documents\Dolphin Emulator`. As instalações mais recentes podem usar `%APPDATA%\Dolphin Emulator`, e uma instalação portátil tem uma pasta `User` ao lado de `Dolphin.exe`.
- **Linux:** `~/.local/share/dolphin-emu`.
- **Steam Deck** (o Flatpak da Discover): `~/.var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu`.
- **Mac:** `~/Library/Application Support/Dolphin`.

O Hoard encontra sozinho as pastas de `Documents`, Linux e Steam Deck. Para `%APPDATA%`, uma instalação portátil ou um Mac, aponta uma vez para as pastas `GC` e `Wii`.

## GameCube: ficheiros de cartão ou pastas GCI

Em **Options → Configuration → GameCube**, cada ranhura de memory card pode ser uma de duas coisas:

- **Um ficheiro de cartão**, uma imagem em bruto como `MemoryCardA.USA.raw` com os saves de todos os jogos. Cada gravação reescreve o ficheiro inteiro.
- **Uma pasta GCI**, onde cada save é o seu próprio ficheiro `.gci`, numa pasta como `GC/USA/Card A`. Só é novo o save que mudou, por isso as versões ficam leves e fáceis de ler.

Para sincronizar, as pastas GCI encaixam melhor. Seja como for, **usa a mesma definição em todas as máquinas**: um ficheiro de cartão num PC e uma pasta GCI no outro faz com que cada um veja um cartão vazio. Se precisares de passar saves de um tipo para o outro, o **Tools → Memory Card Manager** do Dolphin importa e exporta ficheiros `.gci`.

Os cartões também são separados **por região** (USA, EUR, JAP). Uma cópia PAL e uma NTSC do mesmo jogo não veem os saves uma da outra, por isso usa a mesma imagem de disco em todo o lado.

## Wii: a memória da consola emulada

Os saves da Wii vivem dentro da pasta `Wii`, em `Wii/title/00010000/<ID do jogo>/data` para os jogos em disco. Essa pasta é toda a memória da consola emulada: saves, Miis, definições do sistema e os canais que tenhas instalado. O Hoard faz backup dela como um só elemento, por isso restaurar uma versão repõe a memória da consola tal como estava nesse momento. Antes de confirmares, o Hoard mostra o que vai mudar, e os teus ficheiros atuais são guardados primeiro.

Se só quiseres passar um save da Wii à mão, o Dolphin consegue exportá-lo: clica com o botão direito no jogo da lista e escolhe **Export Wii Save**.

## Como funciona a sincronização no dia a dia

Jogas no desktop e fechas o Dolphin. O Hoard espera que o Dolphin tenha saído e que as pastas fiquem paradas, e envia a versão nova. Mais tarde pegas na Steam Deck; assim que fica online, o Hoard descarrega os saves mais recentes. Fecha o Dolphin na Deck e acontece o mesmo no sentido contrário. Nenhuma das máquinas tem de estar ligada ao mesmo tempo que a outra.

## Armadilhas que convém conhecer

- **Fecha o Dolphin, não só o jogo.** O Hoard faz backup quando o emulador sai. Numa Deck, suspender não conta como fechar.
- **Os estados são frágeis.** Os estados do Dolphin partem-se muitas vezes entre versões do Dolphin. O Hoard sincroniza os saves a sério; se também quiseres `StateSaves`, acrescenta-a como elemento próprio e mantém o Dolphin na mesma versão em todo o lado.
- **Caminhos personalizados.** Se mudaste a raiz da NAND da Wii ou o caminho das pastas GCI em **Options → Configuration → Paths**, aponta o Hoard para essas pastas.

## Configurar

1. Instala o Hoard em cada máquina e inicia sessão com a mesma conta.
2. Na **Biblioteca**, acrescenta o Dolphin a partir da lista de emuladores.
3. Usa a mesma definição de cartão e a mesma região em todas as máquinas.
4. Joga, fecha o Dolphin e continua na outra máquina.

Preferes que fique em casa? Corre o `hoard-server` no teu PC ou NAS e aponta todas as máquinas para ele. Sem conta connosco, sem telemetria para nós, nada a passar pelos nossos servidores. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard). Para os outros emuladores, vê [saves de emuladores](/guides/back-up-emulator-saves).

<!-- faq -->

## Perguntas frequentes

### O Dolphin tem saves na nuvem?

Não. O Dolphin guarda os saves numa pasta local e deixa a sincronização contigo. O Hoard é uma forma de os sincronizar automaticamente, com histórico de versões por cima.

### Posso sincronizar o Dolphin entre um PC e uma Steam Deck?

Sim. Instala o Hoard nos dois com a mesma conta. O Hoard sabe onde o Dolphin guarda os saves no Windows, no Linux e no Flatpak da Steam Deck, e associa-os entre máquinas.

### Ficheiro de cartão ou pasta GCI?

Para sincronizar, a pasta GCI: cada save é o seu próprio ficheiro, por isso as versões são leves e mostram que jogo mudou. Escolhas o que escolheres, usa o mesmo em todas as máquinas.

### Também sincroniza os saves da Wii?

Sim. A pasta `Wii` guarda a memória da consola emulada, com os saves, e o Hoard faz backup dela e sincroniza-a como os cartões de GameCube.

### O Hoard sincroniza os estados do Dolphin?

Não por omissão, porque os estados partem-se entre versões do Dolphin. Acrescenta a pasta `StateSaves` à mão se os quiseres.

### Funciona com o Dolphin em Android?

Hoje não. O Hoard funciona em Windows, macOS, Linux e Steam Deck.
