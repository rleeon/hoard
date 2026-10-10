---
title: "Saves na nuvem para o PCSX2: sincroniza os memory cards da PS2 entre PC e Steam Deck"
description: "O PCSX2 não tem saves na nuvem. Sincroniza os memory cards da PS2 entre PC e Steam Deck automaticamente, com histórico: caminhos, cartões de pasta e armadilhas."
order: 16
updated: 2026-10-09
---

O PCSX2 não sincroniza saves por conta própria: o teu progresso da PS2 vive em ficheiros de memory card numa só máquina, e a outra nunca sabe de nada. O Hoard sincroniza-os automaticamente. Quando fechas o PCSX2, faz backup dos teus memory cards, descarrega-os nos teus outros PCs e na tua Steam Deck, e guarda todas as versões para que um save mau nunca te custe uma partida inteira.

## Onde o PCSX2 guarda os teus saves

O PCSX2 guarda como uma PS2 a sério: em memory cards. Por omissão há dois, `Mcd001.ps2` e `Mcd002.ps2`, de 8 MB cada, numa pasta `memcards`. Um só cartão guarda os saves de todos os jogos que jogaste com ele.

- **Windows:** `Documents\PCSX2\memcards`. No modo portátil, a pasta fica ao lado do programa.
- **Linux:** `~/.config/PCSX2/memcards`.
- **Steam Deck** (o Flatpak da Discover): `~/.var/app/net.pcsx2.PCSX2/config/PCSX2/memcards`. Com o EmuDeck, a ligação em `~/Emulation/saves/pcsx2` aponta para uma destas.
- **Mac:** `~/Library/Application Support/PCSX2/memcards`.

No PCSX2, **Settings → Memory Cards** mostra a pasta que está mesmo a usar e que cartão está em cada ranhura. O Hoard encontra sozinho as pastas do Windows, Linux e Steam Deck; num Mac ou numa instalação portátil, aponta uma vez para a pasta `memcards`.

## Cartões de ficheiro e cartões de pasta

O PCSX2 consegue criar dois tipos de memory card, e ao sincronizar a escolha importa mais do que parece.

- **Um cartão de ficheiro** (`.ps2`) é um só ficheiro de 8 MB com os saves de todos os jogos. Gravas em qualquer jogo e muda o ficheiro inteiro, por isso cada nova versão são os 8 MB completos.
- **Um cartão de pasta** é uma pasta em vez de um ficheiro, com cada save na sua subpasta. Gravas num jogo e só mudam os ficheiros desse jogo, por isso as versões ficam leves e o histórico mostra que save mudou.

Podes criar qualquer um deles em **Settings → Memory Cards**. Escolhas o que escolheres, usa **o mesmo tipo, os mesmos nomes e as mesmas ranhuras** em todas as máquinas. Um cartão de ficheiro no desktop e um de pasta na Deck são dois cartões diferentes, e cada máquina vai achar que o save da outra não existe.

## Como funciona a sincronização no dia a dia

Jogas no desktop e fechas o PCSX2. O Hoard espera que o PCSX2 tenha saído e que os cartões deixem de mudar, e envia a versão nova. Mais tarde pegas na Steam Deck. Assim que fica online, o Hoard descarrega os cartões mais recentes, e quando abres o PCSX2 o teu save está lá. Fecha-o na Deck e acontece o mesmo no sentido contrário.

Nenhuma das máquinas tem de estar ligada ao mesmo tempo. Os cartões esperam no servidor até a outra máquina os pedir.

## Armadilhas que convém conhecer

- **Fecha o PCSX2, não só o jogo.** O Hoard faz backup quando o emulador sai, por isso nunca copia um cartão a meio da escrita. Numa Deck, suspender não conta como fechar.
- **Mesmo disco, mesma região.** As versões PAL e NTSC de um jogo têm números de série diferentes (SLES e SLUS, por exemplo) e não veem os saves uma da outra. Usa a mesma imagem de disco em todo o lado.
- **Os estados são outra coisa.** Os estados (ficheiros `.p2s` em `sstates`) são fotografias do emulador e muitas vezes não carregam noutra versão do PCSX2. O Hoard sincroniza os memory cards; se quiseres que os estados também viajem, acrescenta a pasta `sstates` como elemento próprio e mantém o PCSX2 na mesma versão em todas as máquinas.
- **Uma versão é o cartão inteiro.** Restaurar uma versão anterior repõe o cartão inteiro como estava, com todos os jogos. O Hoard mostra o que vai mudar antes de confirmares, e o teu cartão atual é guardado primeiro, por isso um restauro pode sempre ser desfeito.

## Configurar

1. Instala o Hoard em cada máquina e inicia sessão com a mesma conta.
2. Na **Biblioteca**, acrescenta o PCSX2 a partir da lista de emuladores.
3. Confirma que todas as máquinas usam o mesmo tipo de cartão, os mesmos nomes e as mesmas ranhuras.
4. Joga, fecha o PCSX2 e continua na outra máquina.

Preferes que fique em casa? Corre o `hoard-server` no teu PC ou NAS e aponta todas as máquinas para ele. Sem conta connosco, sem telemetria para nós, nada a passar pelos nossos servidores. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard). Para os outros emuladores, vê [saves de emuladores](/guides/back-up-emulator-saves).

<!-- faq -->

## Perguntas frequentes

### O PCSX2 tem saves na nuvem?

Não. O PCSX2 escreve os memory cards numa pasta local e deixa a sincronização contigo. O Hoard é uma forma de o fazer automaticamente, com histórico de versões por cima.

### Posso sincronizar o PCSX2 entre um PC e uma Steam Deck?

Sim. Instala o Hoard nos dois com a mesma conta. O Hoard sabe onde o PCSX2 guarda os cartões no Windows e no Flatpak da Steam Deck, e associa-os entre máquinas.

### Cartão de ficheiro ou cartão de pasta?

Para sincronizar, o cartão de pasta encaixa melhor: só são enviados os saves que mudaram, e o histórico mostra que jogo mudou. Os dois funcionam, desde que todas as máquinas usem o mesmo.

### O Hoard sincroniza os estados do PCSX2?

Não por omissão, porque os estados partem-se entre versões do PCSX2. Acrescenta a pasta `sstates` à mão se os quiseres, e mantém o PCSX2 na mesma versão em todo o lado.

### Restaurar uma versão antiga faz recuar todos os jogos do cartão?

Sim. Uma versão é o cartão inteiro. O Hoard mostra antes o que vai mudar, e guarda também como versão o cartão que substituis.

### Funciona com emuladores de PS2 em Android?

Hoje não. O Hoard funciona em Windows, macOS, Linux e Steam Deck.
