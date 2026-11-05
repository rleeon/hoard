---
title: "Sincronização de saves comparada: Hoard frente a Ludusavi, Syncthing, OpenSave e as outras"
description: "Hoard, Ludusavi, Syncthing, OpenSave, GameSave Manager, SaveState, EmuSync e mais, comparados: os pontos fortes de cada um e uma tabela."
order: 4
updated: 2026-11-05
---

A Steam Cloud só cobre jogos comprados na Steam, e apenas quando o programador se deu ao trabalho de a ligar. Emuladores, GOG, Epic, itch.io, jogos fora da Steam, qualquer coisa com mods: nada disso entra. Se jogas em mais do que uma máquina, um desktop e uma Steam Deck por exemplo, acabas a copiar pastas à mão na esperança de teres apanhado a mais recente.

Há várias ferramentas que resolvem isto, e não fazem todas o mesmo. Umas fazem cópias locais, outras espelham pastas entre dispositivos, outras enviam para uma nuvem. Esta página passa por elas e diz em que é que cada uma é genuinamente boa. O Hoard é o meu projeto, por isso fica para o fim, com o que não faz, e com uma tabela que podes ler sem acreditar numa única linha do texto.

## Ludusavi

O mais conhecido, e com razão. O Ludusavi (de mtkennerly) é uma ferramenta de backup gratuita e open source, com interface e com CLI, construída sobre o manifesto comunitário de localizações de saves que cobre dezenas de milhares de jogos — o mesmo manifesto que quase todas as desta lista usam, o Hoard incluído. Guarda cópias locais versionadas e pode enviá-las para uma nuvem tua através do Rclone.

**Melhor se:** queres cópias locais, controlo total e nenhum servidor em lado nenhum. É a escolha mais segura da lista e não custa nada.

**Onde para:** a sincronização entre máquinas é algo que montas tu. Agendar um backup, configurar um remote do Rclone e lembrares-te de restaurar no outro PC *antes* de jogar. Funciona, mas nada te impede de esquecer o último passo.

## Syncthing

Não é sequer uma ferramenta de jogos: é um espelho de pastas peer-to-peer de uso geral, e muito bom. Apontas-lhe uma pasta de saves e ela aparece nos teus outros dispositivos.

**Melhor se:** já o tens a correr e queres os ficheiros em dois sítios sem nuvem pelo meio.

**Onde para:** espelha, não fotografa. Um save corrompido chega a todos os dispositivos em segundos, exatamente à mesma velocidade de um bom. O versionamento é por ficheiro, sem ideia nenhuma do que é uma sessão de jogo, por isso «voltar a como estava na terça à noite» é algo que reconstróis à mão. Duas máquinas que jogaram offline dão-te ficheiros de conflito, não uma fusão.

## OpenSave

Sincronização peer-to-peer feita de propósito para saves, em Go, com licença MIT, para Windows, Linux, Steam Deck e, desde a versão 2.4, macOS. Sem conta e sem servidor: os dispositivos emparelham entre si e sincronizam pela rede local ou através de um código de sala num relay. Cada alteração vira um snapshot, há branches para partidas paralelas, os conflitos resolvem-se por linhagem de sincronização em vez de pelo relógio, e só viajam os blocos que mudaram. Opcionalmente pode espelhar para Drive, Dropbox, OneDrive ou WebDAV.

**Melhor se:** recusas ter uma conta e os teus dispositivos estão ligados ao mesmo tempo com frequência suficiente.

**Onde para:** peer-to-peer significa que o save só vive nos teus dispositivos. Se morre a Deck com a única cópia recente e o espelho nunca foi configurado, acabou. Os dois dispositivos têm de estar ligados para haver sincronização.

## OpenCloudSaves

Uma interface multiplataforma que sincroniza as tuas pastas de saves para uma nuvem que já pagas — OneDrive, Google Drive, Dropbox, Nextcloud — com o Rclone por baixo.

**Melhor se:** queres os saves numa conta de armazenamento que já tens, com interface em vez de ficheiros de configuração do Rclone.

**Onde para:** o projeto foi arquivado em janeiro de 2025 e já não tem manutenção, por isso não vai receber correções para jogos novos nem para atualizações do sistema. Além disso, não há desduplicação ao nível do conteúdo. Dez cópias de um save de 2 GB são 20 GB da tua quota do Drive, e as nuvens de ficheiros sincronizam ficheiros, não sessões de jogo, por isso o que recuperas é como a pasta estava naquele momento.

## Game Backup Monitor

Windows primeiro, e o original de todo este género. O GBM vigia o processo do jogo e, quando sais, comprime o save com 7-Zip e guarda um histórico numerado.

**Melhor se:** estás num único PC com Windows e queres um arquivo local comprimido sem pensar nisso.

**Onde para:** é uma ferramenta de backup, não de sincronização. Levar o arquivo para uma segunda máquina é problema teu, e a Steam Deck / SteamOS não é o seu terreno.

## GameSave Manager

Uma ferramenta gratuita e veterana para Windows, de código fechado, com a sua própria base de dados de jogos. Faz backup e restauro, e a sua função mais conhecida, **Sync & Link**, move a pasta de saves para uma pasta na nuvem como o Dropbox ou o OneDrive e deixa um link no lugar, para que o cliente da nuvem a mantenha sincronizada.

**Melhor se:** estás no Windows, já vives no Dropbox ou no OneDrive e queres simplesmente que o save esteja lá.

**Onde para:** só Windows, e o Sync & Link significa que um cliente de nuvem genérico sincroniza a pasta ativa enquanto o jogo escreve nela, o mesmo padrão que torna [o Syncthing arriscado para saves](/guides/syncthing-game-saves). O histórico de versões é o que a tua nuvem guardar.

## Game Save Manager (GSM)

Apesar do nome, não é o mesmo programa que o de cima. O GSM, de dyang886, é uma aplicação para Windows com licença GPL-3.0 que encontra os jogos instalados no Steam, Epic, Battle.net e outros launchers com a base de dados da PCGamingWiki, guarda para cada jogo uma série de backups locais em rotação até ao limite que definires, e leva todo o histórico para um PC novo num arquivo `.gsmr` exportado. Há até um terceiro homónimo: o game-save-manager de mcthesw, popular em comunidades chinesas, que faz backup agendado das pastas que adicionas à mão, com WebDAV para uma cópia fora da máquina.

**Melhor se:** jogas num só PC com Windows e queres backups locais com versões quase sem configuração, mais uma mudança fácil para uma máquina nova.

**Onde para:** não sincroniza entre dispositivos: o segundo PC recebe os teus saves quando exportas um arquivo e o importas lá. Só Windows.

## SaveState

O SaveState, de Matteo842, é um gestor de backups com licença GPL-3.0 para Windows e Linux, com uma aplicação Android à parte. Largas em cima dele o atalho de um launcher e ele encontra a pasta de saves, e o suporte de emuladores é muito completo: RetroArch, Dolphin, os cartões de memória do PCSX2, xemu e mais de vinte outros. Os backups são arquivos `.zip` comprimidos em rotação, e uma sincronização opcional copia-os para o Google Drive, WebDAV, FTP, uma partilha de rede ou Git.

**Melhor se:** queres uma ferramenta de backup simpática, com muita cobertura de emuladores, e não te importas de restaurar à mão na outra máquina.

**Onde para:** gira em torno de arquivos de backup, não do save vivo. A sincronização move esses arquivos entre máquinas, mas pô-los na pasta do jogo do outro lado continua a ser um restauro que lanças tu. Não há macOS.

## EmuSync

O EmuSync é uma ferramenta de sincronização com licença GPL-3.0 para saves de PC e de emuladores, pensada primeiro para um PC com Windows e uma Steam Deck. Associas a localização dos saves de cada jogo, em cada dispositivo, a um único ponto de sincronização no teu próprio Google Drive, Dropbox ou OneDrive, ou numa pasta partilhada, e ele sincroniza sozinho ou quando pedes. Um plugin do Decky mostra o estado no modo Jogo.

**Melhor se:** jogas entre um PC com Windows e uma Deck e queres os saves numa nuvem que já tens.

**Onde para:** a nuvem guarda o estado atual, e o backup e o restauro são locais a cada dispositivo, em vez de um histórico partilhado entre eles. O macOS não está testado, e a última versão é de fevereiro de 2026.

## Aletheia

Um dos mais recentes, AGPL, e vai exatamente à parte que os outros cobrem pela metade: os launchers. Heroic, itch.io, Lutris, Steam, GOG Galaxy e Xbox, em Windows, Linux e macOS.

**Melhor se:** a tua biblioteca está espalhada por launchers que as outras ferramentas detetam mal, sobretudo Xbox/Game Pass e Heroic.

**Onde para:** é um projeto jovem com um âmbito propositadamente estreito. Fazer cópia e restaurar é todo o conjunto de funcionalidades; não há uma nuvem versionada por trás.

## SaveSync

O comercial, vendido na Steam como compra única, virado para Windows. O truque dele é que não aponta a ti-em-dois-PC, mas ao cooperativo: os saves vão para entradas privadas e não listadas da Steam Workshop para que um amigo possa puxar o teu mundo de Valheim ou de Factorio, e também há sincronização por rede local.

**Melhor se:** o problema que resolves é «o meu amigo aloja e preciso do save dele», não «que os meus saves me sigam».

**Onde para:** código fechado, Windows, preso à Steam como meio de transporte, e uma lista de jogos cooperativos suportados em vez de tudo o que tens.

## Tachyon

O nome mais recente da lista, em beta gratuita para Windows. O Tachyon deteta os saves de mais de 5 000 jogos de PC, sincroniza-os através da sua própria nuvem e guarda um histórico de versões com várias «linhas temporais».

**Melhor se:** estás no Windows, queres algo que funcione logo à primeira e não te importas que seja uma beta.

**Onde para:** por agora só Windows (Linux e macOS aparecem como «em breve»), por isso ainda sem Steam Deck; sem código-fonte publicado; sem forma de usar o teu próprio servidor. O preço depois da beta não foi anunciado.

## Plugins para Steam Deck (Decky)

Na própria Deck, alguns plugins do Decky Loader tratam de parte do trabalho a partir do modo Jogo. O Decky Cloud Save sincroniza pastas com o OneDrive, Google Drive ou Dropbox através do rclone depois de um jogo fechar; o autor já não o mantém e aponta para dois sucessores, o Decky Ludusavi (uma interface do Ludusavi para o modo Jogo) e o SDH-GameSync (um fork que descarrega os saves ao abrir cada jogo e os envia ao fechá-lo). O Steamback guarda localmente os dez snapshots mais recentes dos jogos que usam o Steam Cloud, tirados quando sais.

**Melhor se:** jogas sobretudo na Deck e queres algo que se use sem sair do modo Jogo.

**Onde para:** só funcionam na Deck, e a parte entre dispositivos depende de outra coisa: o rclone e a tua nuvem, o Ludusavi ou o próprio Steam Cloud. Os snapshots do Steamback ficam na Deck e só cobrem jogos com Steam Cloud.

## Uma nota sobre o EmuDeck

O EmuDeck aparece nestas conversas e não é um concorrente no sentido normal: é um instalador e configurador de emuladores para a Steam Deck, e a sincronização que oferece é uma comodidade acoplada a esse trabalho (Rclone contra uma nuvem de ficheiros, só para saves de emulador). Sobrepõe-se às ferramentas acima sem ser a mesma coisa: o EmuDeck deixa-te os emuladores montados, e as daqui tomam conta dos saves da biblioteca toda. Há quem use o EmuDeck ao lado de uma destas, e é uma montagem sensata, não redundante.

## Hoard

O Hoard faz backup de cada save assim que o jogo acaba de o escrever e mantém-no sincronizado em todas as máquinas onde jogas. O motor corre como serviço em segundo plano, `hoardd`, sem janela, por isso continua a funcionar no modo Jogo do SteamOS. Espera que a pasta fique quieta uns segundos, por isso nunca envia um ficheiro escrito a meio; faz mais uma cópia quando sais; e nunca troca o save de um jogo aberto: o que chega de outra máquina espera que feches o jogo.

- **A outra máquina pode estar desligada.** O save mais recente espera no servidor, por isso a Deck em que pegas na próxima semana recebe-o sem o PC estar ligado. As ferramentas ponto a ponto precisam dos dois dispositivos acordados ao mesmo tempo.
- **Todas as versões, guardadas.** Cada save é uma versão à qual podes voltar na aplicação ou com `hoard restore`, também depois de uma falha de disco ou de uma instalação limpa. O Hoard One guarda todas as versões para sempre; no teu próprio servidor decides tu quanto histórico guardar.
- **Só viaja o que muda.** Os envios mandam só os ficheiros alterados, comprimidos com zstd, e as versões partilham tudo o que não muda: dez versões de um save de 2 GB que muda pouco ocupam cerca de 2 GB, não 20.
- **SHA-256 à ida e à volta.** A corrupção é detetada antes de poder escrever por cima de um save bom, e o Hoard guarda uma cópia de tudo o que substitui.
- **Gratuito e open source, com o teu próprio servidor se quiseres.** A aplicação, a CLI e o servidor são AGPL-3.0. O Hoard One, o servidor gerido que recomendamos a toda a gente, é gratuito com 2 GB e três dispositivos, e não há nada para montar. Ou corres o `hoard-server` com Docker num PC, num NAS ou num Raspberry Pi, em disco local ou em qualquer armazenamento compatível com S3: sem conta connosco, sem telemetria para nós e sem quota nossa.
- **Windows, Linux, macOS e SteamOS**, com a mesma aplicação de ambiente de trabalho nos quatro e uma CLI para servidores e máquinas sem ecrã.
- **Uma deteção que cobre a biblioteca inteira:** Steam, Epic, GOG, prefixos do Proton e do Wine, milhares de jogos graças à base de dados comunitária de saves, e predefinições para 19 emuladores, do PCSX2, Dolphin e RPCS3 ao Ryujinx, Cemu, DuckStation e RetroArch.

## O detalhe que decide a sincronização Steam Deck ↔ PC

Vale a pena saber, escolhas a ferramenta que escolheres. O save na nuvem de um jogo da Steam vive em `<AppID>/remote/`, e a pasta *acima* guarda o `remotecache.vdf`, o estado das conquistas, estatísticas e contadores de horas jogadas — coisas que legitimamente diferem entre a tua Deck e o teu desktop.

Sincroniza a pasta-mãe e ficas com um conflito permanente entre duas máquinas que nunca discordaram sobre um único save. O Hoard segue `remote/`, não a pasta-mãe. A qualquer ferramenta a que apontes uma pasta à mão pode dizer-se o mesmo, e é a primeira coisa a verificar quando uma configuração de sincronização assinala conflitos sem motivo visível.

## O que o Hoard não faz

- **Sincronizar sem servidor.** O Hoard sincroniza sempre através de um: o Hoard One se preferires não montar nada, ou o teu. É precisamente isso que deixa o outro dispositivo estar desligado, e que deixa uma cópia que sobrevive às duas máquinas. Se não queres um servidor em caso nenhum, o OpenSave, o Syncthing ou o Ludusavi foram feitos para isso.
- **Partilhar mundos de cooperativo.** Passar um mundo a um amigo é o trabalho do SaveSync; o Hoard trata dos teus próprios saves.
- **Juntar dois saves que divergiram.** Nenhuma ferramenta consegue fundir duas partidas numa. O Hoard guarda as duas como versões, em todas as tuas máquinas, e escolhes tu.

## A distinção entre Hoard One e self-hosting

As comparações sobre o Hoard quase sempre fundem os dois num só, e o resultado sai errado. Por isso, de forma clara:

- **O Hoard One** é a opção gerida, a que recomendamos a todos: inicias sessão e os teus saves ficam nos nossos servidores, na UE.
- **Um Hoard self-hosted é inteiramente teu.** Corres o `hoard-server` no teu PC ou NAS e os saves vão da tua máquina para o teu disco. **Não há conta connosco, nem telemetria para nós, nem quota, nem retransmissão**: não passa nada pelos nossos servidores, porque não há nada nosso no caminho. Não vemos um save, o nome de um jogo ou um email, porque nada disso nos chega. Se o Hoard One fechasse amanhã, uma instalação self-hosted continuaria igual.

O mesmo código, a mesma deteção. O que muda é de quem é o armazenamento, e uma predefinição: um servidor self-hosted vai desbastando as versões antigas com o tempo, a não ser que o desligues. E, para ser exato num detalhe: o teu servidor tem sim os seus próprios acessos — um utilizador e um token por dispositivo — mas vivem na tua base de dados, não na nossa.

## A tabela

| Ferramenta | Sincronização automática | Funciona com o outro dispositivo desligado | Onde vivem os saves | Histórico | Plataformas | Licença |
|---|---|---|---|---|---|---|
| **Hoard** | Sim, sempre que o jogo grava | Sim | Hoard One ou servidor teu (disco ou compatível com S3) | Todas as versões, desduplicadas | Win · Linux · macOS · Deck | AGPL-3.0, gratuito |
| **Ludusavi** | Manual, ou Rclone que ligas tu | Só com um remoto Rclone | Local, mais o teu remoto Rclone | Backups locais versionados | Win · Linux · macOS | MIT, gratuito |
| **Syncthing** | Sim, espelho contínuo | Não, os dois têm de estar ligados | Só nos teus dispositivos | Versões por ficheiro | Tudo | MPL-2.0, gratuito |
| **OpenSave** | Sim, ponto a ponto | Não, a não ser que configures um espelho na nuvem | Os teus dispositivos, espelho na nuvem opcional | Snapshots e ramos | Win · Linux · macOS · Deck | MIT, gratuito |
| **OpenCloudSaves** | Através da tua nuvem | Sim, através da nuvem | OneDrive / Drive / Dropbox / Nextcloud | O que a nuvem guardar | Win · Linux · macOS | Open source, arquivado |
| **Game Backup Monitor** | Não | Não sincroniza | Arquivos 7-Zip locais | Backups numerados | Windows · Linux | GPL-3.0, gratuito |
| **GameSave Manager** | Através da tua nuvem (Sync & Link) | Sim, através da nuvem | Local, mais Dropbox / OneDrive | O que a nuvem guardar | Windows | Gratuito, código fechado |
| **Game Save Manager (GSM)** | Não, exportar e importar | Não sincroniza | Backups locais | Backups locais em rotação | Windows | GPL-3.0, gratuito |
| **SaveState** | Opcional, dos arquivos de backup | Sim, através do teu armazenamento | Zips locais, mais Drive / WebDAV / FTP / SMB / Git | Arquivos em rotação | Windows · Linux | GPL-3.0, gratuito |
| **EmuSync** | Sim, através da tua nuvem | Sim, através da nuvem | Drive / Dropbox / OneDrive ou uma pasta partilhada | Backups locais | Windows · Linux · Deck | GPL-3.0, gratuito |
| **Aletheia** | Backup e restauro por launcher | Não sincroniza | O teu armazenamento | Backups | Win · Linux · macOS | AGPL-3.0 |
| **SaveSync** | Sim, e com amigos | Sim, através do Steam | Entradas privadas do Steam Workshop | Conforme a aplicação | Windows | Pago, código fechado |
| **Tachyon** | Sim, através da sua própria nuvem | Sim | A nuvem do Tachyon | Versões e linhas temporais | Windows (beta) | Beta gratuita, sem código publicado |

## Então qual

Se queres que os backups e a sincronização aconteçam sozinhos, entre PCs e uma Steam Deck, com todas as versões guardadas, a outra máquina livre para estar desligada e a opção de tudo ser teu, isso é o Hoard. [Descarrega-o](/download), entra no Hoard One, ou lê primeiro [como o alojar com Docker](/guides/self-host-hoard). É gratuito das duas maneiras.

Para tarefas mais estreitas: um só PC com Windows e arquivos locais, Game Backup Monitor ou GSM. Backups locais com controlo total e sem servidor em lado nenhum, Ludusavi. Sem conta em caso nenhum, com dispositivos que costumam estar ligados ao mesmo tempo, OpenSave ou Syncthing. Saves que têm de viver numa pasta do Drive que já pagas, Ludusavi com um remoto Rclone, ou EmuSync; o OpenCloudSaves fazia isso, mas está arquivado. Partilhar um mundo de cooperativo com amigos, SaveSync.

Se vens do OpenCloudSaves e o que querias era ter os saves num armazenamento que controlas, mais do que no Drive em si, um Hoard self-hosted faz isso, com o histórico por cima: escreve em disco local ou em qualquer bucket compatível com S3, como o Backblaze B2 ou o MinIO. Se é o Ludusavi que estás a pesar, há uma [comparação mais longa com o Ludusavi](/guides/ludusavi-alternative).

## Comparações um para um

Cada uma vai mais fundo do que o bloco acima, incluindo onde a outra ferramenta ganha:

- [Hoard frente ao Ludusavi](/guides/ludusavi-alternative)
- [Hoard como alternativa à Steam Cloud](/guides/steam-cloud-alternative)
- [Sincronização ponto a ponto frente a um servidor teu](/guides/opensave-alternative)
- [Syncthing para saves: o que parte](/guides/syncthing-game-saves)
- [Hoard frente ao GameSave Manager e aos outros Game Save Manager](/guides/game-save-manager-alternative)
- [Substituir o OpenCloudSaves agora que está arquivado](/guides/opencloudsaves-alternative)
- [Uma alternativa gratuita ao SaveSync](/guides/savesync-alternative)

<!-- faq -->

## Perguntas frequentes

### Qual destas ferramentas guarda histórico de versões?

O Hoard conserva cada save como uma versão à qual podes voltar, em todas as tuas máquinas. O Ludusavi, o GSM e o SaveState guardam cópias locais versionadas ou em rotação, e o OpenSave guarda snapshots nos teus dispositivos. A maioria das restantes sincroniza ou copia o estado atual, o que significa que um save corrompido é propagado fielmente para a outra máquina.

### Qual funciona sem servidor nem conta?

O Ludusavi com cópias locais, e qualquer ferramenta ponto a ponto. O Hoard também entra se fizeres self-hosting: sem conta connosco e sem nada a passar pelos nossos servidores.

### Qual sincroniza com o meu outro dispositivo desligado?

Qualquer ferramenta que guarde o save num sítio que não sejam os teus dispositivos: o Hoard, através do Hoard One ou do teu próprio servidor, e as ferramentas que escrevem numa nuvem, como o EmuSync ou o Ludusavi com Rclone. A sincronização ponto a ponto (o OpenSave sem o seu espelho, o Syncthing) precisa dos dois dispositivos ligados ao mesmo tempo.

### Qual cobre jogos que não estão na Steam?

Todos os gestores de saves aqui listados, porque localizam os ficheiros pela mesma base de dados comunitária e não através de uma loja. A exceção é a Steam Cloud: só cobre jogos da Steam cujo programador a ativou.

### Tenho de escolher só uma?

Não, e muita gente não escolhe. Uma ferramenta de cópia local e uma de sincronização resolvem metades diferentes do problema. A única regra é nunca apontar uma para a pasta de cópias da outra, ou acabas a sincronizar um espelho desatualizado em vez do teu save real.

### Qual é o detalhe que parte a maioria das montagens caseiras?

Sincronizar a pasta acima de `<AppID>/remote/` no `userdata` da Steam. A de cima guarda `remotecache.vdf` e ficheiros de proezas e tempo de jogo que devem ser diferentes em cada máquina, por isso cada arranque parece um conflito mesmo sem nenhum save se ter mexido.
