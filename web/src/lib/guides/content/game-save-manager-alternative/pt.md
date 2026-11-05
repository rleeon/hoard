---
title: "Alternativa ao GameSave Manager: backup e sincronização automáticos, também no Linux"
description: "Procuras uma alternativa ao GameSave Manager? O Hoard faz backup de cada save sozinho, sincroniza-o entre PCs e uma Steam Deck, e corre no Linux e macOS."
order: 27
updated: 2026-11-05
---

Há três programas diferentes chamados Game Save Manager, e os três fazem backup de saves num só PC com Windows. Se queres backups que aconteçam sozinhos e te sigam para os teus outros PCs, uma Steam Deck ou o Linux, o [Hoard](/download) é gratuito, open source e feito precisamente para isso.

## Que Game Save Manager tens

- **GameSave Manager**, o veterano de código fechado para Windows. Tem a sua própria base de dados de jogos, faz backup para um único arquivo quando pedes ou de forma agendada, e o seu **Sync & Link** move uma pasta de saves para o Dropbox, OneDrive ou Google Drive e deixa um atalho no lugar.
- **Game Save Manager (GSM)**, de dyang886, GPL-3.0, Windows. Encontra os jogos instalados com a base de dados da PCGamingWiki, guarda para cada jogo uma série de backups locais em rotação e leva o histórico para um PC novo num arquivo `.gsmr`.
- **game-save-manager**, de mcthesw, AGPL-3.0, Windows, popular em comunidades chinesas. Faz backup agendado das pastas que adicionas, com WebDAV para uma cópia fora da máquina.

## O que fazem bem

- **Simples e locais.** Escolhes os jogos, carregas em backup e tens um arquivo. Sem conta e sem nada para configurar.
- **Boa deteção no Windows.** Bases de dados grandes encontram a maioria dos jogos instalados sem andares à procura de caminhos.
- **Uma mudança limpa para um PC novo.** Um só arquivo exportado leva tudo.

Se só jogas num PC com Windows e não te importas de carregar num botão, qualquer um deles serve.

## Onde o Hoard vai mais longe

| | Ferramentas Game Save Manager | Hoard |
|---|---|---|
| Quando faz backup | Quando carregas no botão, ou de forma agendada | Sempre que o jogo grava, e outra vez quando sais |
| Sincronização entre PCs | Exportar e importar, ou uma pasta na nuvem com o Sync & Link | Automática: o save mais recente espera-te na máquina seguinte |
| O outro PC pode estar desligado | Só através de uma pasta na nuvem | Sim, o save espera no servidor |
| Histórico | Backups locais, ou o que a nuvem guardar | Todas as versões, desduplicadas, em todas as tuas máquinas |
| Saves escritos a meio | Um cliente de nuvem pode sincronizar uma pasta ligada enquanto o jogo escreve | Nunca envia um ficheiro escrito a meio nem troca o save de um jogo aberto |
| Plataformas | Windows | Windows, Linux, macOS e SteamOS |
| Licença | Código fechado, GPL-3.0 ou AGPL-3.0 | AGPL-3.0, gratuito |

A linha do Linux é a que as pessoas procuram. Nenhuma das ferramentas Game Save Manager corre no Linux nem numa Steam Deck. O Hoard corre, de forma nativa: a mesma aplicação nos quatro sistemas, com os prefixos do Proton e do Wine detetados, por isso o save de um jogo no teu PC com Windows e o do mesmo jogo na Deck passam a ser duas versões de um mesmo histórico.

## Como mudar

1. Instala o [Hoard](/download) e inicia sessão no Hoard One, recomendado para todos, ou aponta-o para o [teu próprio servidor](/guides/self-host-hoard).
2. Se usavas o **Sync & Link**, põe primeiro a pasta de saves de volta no sítio original, para que um cliente de nuvem e o Hoard não sincronizem os mesmos ficheiros.
3. Abre a **Biblioteca**. O Hoard encontra os teus jogos com a base de dados comunitária de saves e os teus launchers; adiciona à mão qualquer pasta que lhe escape.
4. Joga. A primeira versão aparece no histórico assim que o jogo grava. Instala o Hoard nas tuas outras máquinas com a mesma conta e a [sincronização entre PCs](/guides/sync-game-saves-across-pcs) acontece sozinha.

Os teus arquivos antigos do Game Save Manager podem ficar onde estão, como cópia a frio.

## Hoard One, ou o teu próprio servidor

O Hoard é gratuito das duas maneiras, e a aplicação é a mesma em ambas:

- **Hoard One**, recomendado para todos. Nada para configurar nem para manter a correr: um servidor gerido na UE, gratuito com 2 GB e três dispositivos, sem cartão. O [Pro](/pricing) só acrescenta espaço, 100 GB e dispositivos ilimitados, para quando os teus saves já não couberem.
- **O teu próprio servidor.** Corres o `hoard-server` num PC, num NAS ou num Raspberry Pi e todas as tuas máquinas sincronizam através dele. Sem conta connosco, sem telemetria para nós e sem quota nossa: nada passa pelos nossos servidores, porque não há nada nosso no caminho. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard).

O mesmo programa, a mesma deteção, o mesmo histórico. O Hoard One guarda todas as versões para sempre; no teu próprio servidor decides tu quanto histórico guardar.

<!-- faq -->

## Perguntas frequentes

### Há um GameSave Manager para Linux?

Não: o GameSave Manager só existe para Windows, tal como os dois Game Save Manager open source. O Hoard corre de forma nativa no Linux e no SteamOS, encontra os saves dentro dos prefixos do Proton e do Wine, e sincroniza-os com os teus PCs com Windows.

### Ludusavi ou Game Save Manager?

São ambos ferramentas de backup local. O Ludusavi é open source, corre no Windows, Linux e macOS e assenta no manifesto comunitário de saves; os Game Save Manager só correm no Windows. Se queres backup e sincronização sem ligares tu o Rclone, o Hoard usa os mesmos dados comunitários e sincroniza sozinho. Há uma [comparação completa com o Ludusavi](/guides/ludusavi-alternative).

### Posso manter os meus backups antigos?

Sim. O Hoard não lhes toca. Guarda os arquivos como cópia a frio; a partir do primeiro save, o histórico do Hoard assume.

### O Hoard é gratuito?

Sim. A aplicação, a linha de comandos e o servidor são gratuitos e de código aberto (AGPL-3.0), e correr o teu próprio servidor não custa mais do que a máquina. O Hoard One, o servidor gerido que recomendamos a todos, também é gratuito com 2 GB e três dispositivos; o Pro acrescenta espaço por 1,99 € por mês.

### O self-hosting envia alguma coisa ao Hoard?

Não. No teu próprio servidor não há conta connosco nem telemetria para nós: os teus saves, os teus utilizadores e os teus logs ficam na tua máquina e nunca tocam nos nossos.
