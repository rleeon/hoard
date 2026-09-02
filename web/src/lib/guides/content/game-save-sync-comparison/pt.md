---
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

O Hoard toma a sessão de jogo como unidade. O motor corre como serviço em segundo plano — `hoardd`, sem janela, por isso funciona no modo de jogo do SteamOS —, dá-se conta de que paraste de jogar e faz o snapshot nessa altura, em vez de reagir a cada escrita de ficheiro a meio da partida.

- **Histórico versionado por sessão.** Cada sessão é uma versão à qual podes voltar, mesmo depois de uma falha de disco ou de uma instalação limpa.
- **Desduplicação por hash de conteúdo.** Dez versões de um save de 2 GB custam cerca de 2 GB, não 20 GB. As transferências vão comprimidas com zstd.
- **SHA-256 à subida e à descida.** A corrupção é apanhada antes de poder sobrescrever um save bom. Nada é sobrescrito em silêncio: é esse o desenho todo.
- **Nuvem ou auto-alojado, o mesmo binário.** O Hoard Cloud tem plano gratuito (2 GB, 3 dispositivos, histórico completo). Ou levantas o `hoard-server` tu mesmo com Docker Compose contra qualquer armazenamento compatível com S3 — MinIO, Garage, Backblaze B2 — sem conta e sem quota. AGPL-3.0.
- **Windows, Linux, macOS**, mais uma CLI sem interface para uma Steam Deck ou um servidor.
- **Emuladores em beta:** PCSX2, RPCS3, Dolphin, Cemu, Ryujinx, RetroArch, DuckStation, PPSSPP e outros como predefinições.

## O detalhe que decide a sincronização Steam Deck ↔ PC

Vale a pena saber, escolhas a ferramenta que escolheres. O save na nuvem de um jogo da Steam vive em `<AppID>/remote/`, e a pasta *acima* guarda o `remotecache.vdf`, o estado das conquistas, estatísticas e contadores de horas jogadas — coisas que legitimamente diferem entre a tua Deck e o teu desktop.

Sincroniza a pasta-mãe e ficas com um conflito permanente entre duas máquinas que nunca discordaram sobre um único save. O Hoard segue `remote/`, não a pasta-mãe. A qualquer ferramenta a que apontes uma pasta à mão pode dizer-se o mesmo, e é a primeira coisa a verificar quando uma configuração de sincronização assinala conflitos sem motivo visível.

## Onde o Hoard perde

- **Quer um servidor.** Conta na nuvem ou máquina tua, de qualquer forma é infraestrutura, e o OpenSave ou o Ludusavi não precisam de nenhuma.
- **O suporte a emuladores está em beta.** As instalações portáteis e as manias de cada emulador ainda o apanham, e hoje o Aletheia e o OpenSave cobrem melhor alguns casos limite de launchers e emuladores.
- **O macOS está mal testado em hardware real.** Compila e funciona, mas ninguém viveu lá durante meses.
- **É jovem.** O Ludusavi e o Game Backup Monitor têm anos de relatos de bugs atrás deles. O Hoard não, e isso pesa em algo que guarda um save de 200 horas.
- **Não faz partilha cooperativa.** Se queres passar um mundo a um amigo, o SaveSync foi feito para isso e o Hoard não.

## A distinção entre Hoard Cloud e self-hosting

As comparações sobre o Hoard quase sempre fundem os dois num só, e o resultado sai errado. Por isso, de forma clara:

- **O Hoard Cloud** é a opção gerida: inicias sessão e os teus saves ficam nos nossos servidores, na UE.
- **Um Hoard self-hosted é inteiramente teu.** Corres o `hoard-server` no teu PC ou NAS e os saves vão da tua máquina para o teu disco. **Não há conta connosco, nem telemetria para nós, nem quota, nem retransmissão**: não passa nada pelos nossos servidores, porque não há nada nosso no caminho. Não vemos um save, o nome de um jogo ou um email, porque nada disso nos chega. Se o Hoard Cloud fechasse amanhã, uma instalação self-hosted continuaria igual.

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

Sincronizar a pasta acima de `<AppID>/remote/` no `userdata` da Steam. A de cima guarda `remotecache.vdf` e ficheiros de proezas e tempo de jogo que devem ser diferentes em cada máquina, por isso cada arranque parece um conflito mesmo sem nenhum save se ter mexido.
