---
title: "Como transferir saves de jogos para um PC novo"
description: "PC novo ou vais reinstalar o Windows? Leva todos os saves contigo: o que o Steam Cloud cobre, como fazer à mão, como fazer automaticamente e as armadilhas."
order: 13
updated: 2026-10-09
---

Os jogos com Steam Cloud voltam sozinhos quando inicias sessão no PC novo. Tudo o resto é uma pasta que tens de levar tu: jogos sem saves na nuvem, emuladores, qualquer coisa fora do Steam. Podes copiar essas pastas à mão, ou deixar o Hoard fazer backup delas no PC antigo e pôr cada uma no sítio certo no novo. Aqui estão as duas formas, e as armadilhas que custam os saves a muita gente.

## Antes de apagares seja o que for

- **Faz uma lista do que jogas**, incluindo jogos em que não tocas há meses. São esses que ficam esquecidos.
- **Vê que jogos têm saves na nuvem.** No Steam, a página da loja diz-te, e **Propriedades → Geral** mostra se está ativo. A Epic e a GOG também o mostram jogo a jogo.
- **Faz backup do resto, e de preferência de tudo.** Os saves na nuvem guardam uma só cópia, a última. Se essa cópia estiver estragada, está estragada em todo o lado.

## À mão

1. **Encontra a pasta de cada jogo.** A maioria está em `Documents\My Games`, `Saved Games` ou nas pastas `AppData` (`Roaming`, `Local`, `LocalLow`). A lista completa está em [onde os jogos de PC guardam os saves](/guides/where-are-pc-game-saves-stored).
2. **Copia-as para um disco externo**, mantendo a estrutura de pastas. Leva também a pasta `userdata` inteira do Steam: é pequena e cobre os jogos que guardam através do Steam sem terem o Steam Cloud ativo.
3. **No PC novo, instala primeiro o jogo.** Se o jogo precisar de criar as pastas, abre-o uma vez e sai no menu principal. Não comeces um jogo novo.
4. **Copia os saves para o sítio** e abre o jogo. Confirma que o progresso está lá antes de apagares alguma coisa do disco antigo.

Funciona. O problema é que é uma cópia de uma só vez: tens de te lembrar de todas as pastas, e se continuares a jogar no PC antigo, os dois afastam-se a partir desse dia.

## As armadilhas

- **Saves ligados a uma conta.** Alguns jogos põem o ID da tua conta no nome da pasta ou dentro do save: o Elden Ring guarda sob o teu SteamID, e os jogos da Ubisoft sob o teu ID da Ubisoft. Mesma conta nos dois PCs: sem problema. Outra conta: o jogo vê o espaço vazio.
- **O OneDrive mudou os Documentos.** Se um PC faz cópia de segurança de `Documentos` com o OneDrive e o outro não, a "mesma" pasta está em dois sítios diferentes. Clica com o botão direito em `Documentos` e abre **Propriedades → Localização** para veres onde está de facto. Mais em [OneDrive e os saves dos jogos](/guides/onedrive-game-saves).
- **Versões do jogo.** Um save de uma versão mais recente do jogo pode não carregar numa mais antiga. Atualiza o jogo no PC novo antes de copiar.
- **Mods.** Um save com mods (sobretudo nos jogos da Bethesda) pode recusar-se a carregar sem os mesmos mods. Reinstala-os primeiro.
- **Do Windows para um Steam Deck ou Linux.** O save vai para dentro do prefixo do Proton do jogo, que só existe depois de o jogo ter sido aberto uma vez. Vê [sincronizar saves entre Steam Deck e PC](/guides/sync-saves-steam-deck-pc).

## Automaticamente

O Hoard transforma a mudança naquilo que faz todos os dias: backup numa máquina, restauro noutra.

1. **No PC antigo**, instala o Hoard e inicia sessão. Abre a **Biblioteca**: o Hoard lista os saves que encontrou para os teus jogos, com a mesma base de dados comunitária do Ludusavi. Acrescenta o que faltar apontando para a pasta.
2. **Confirma que cada jogo tem uma versão** no seu **Histórico**. É a tua rede de segurança antes de apagares o disco antigo.
3. **No PC novo**, instala o Hoard, inicia sessão com a mesma conta e instala os teus jogos. O Hoard associa-os aos seus backups jogo a jogo e restaura a última versão na pasta que esta máquina espera, mesmo que o caminho seja outro (outro disco, outro nome de utilizador, um prefixo do Proton num Deck).
4. **Antes de começares um jogo novo**, deixa o Hoard acabar de repor os teus saves. A app mostra o estado de cada jogo.

Dois pormenores tornam isto mais seguro do que uma cópia. Os ficheiros de definições como `graphics.ini` entram no backup mas não são escritos por cima dos do PC novo, por isso o hardware novo arranca com definições que lhe servem (podes trazê-las ao restaurar, se as duas máquinas forem parecidas). E nada é definitivo: todas as versões ficam no histórico, por isso um restauro errado desfaz-se restaurando o anterior.

Se o PC antigo continuar em uso, simplesmente continua a sincronizar com o novo. Se for de vez, tira-o dos teus dispositivos. O plano gratuito inclui três.

## Sem os nossos servidores

Podes fazer tudo isto contra o teu próprio servidor: corre o `hoard-server` num PC ou num NAS, aponta as duas máquinas para ele e os saves nunca saem de casa. Sem conta connosco, sem telemetria para nós. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard).

<!-- faq -->

## Perguntas frequentes

### Os saves do Steam passam sozinhos?

Só os dos jogos com Steam Cloud. Inicia sessão no PC novo, instala o jogo e o save é descarregado. Os jogos sem Steam Cloud precisam que copies a pasta, ou de uma ferramenta que o faça por ti.

### Posso simplesmente copiar a minha pasta de utilizador inteira?

Funciona para a maioria dos saves, mas também arrasta gigabytes de caches, definições pensadas para o hardware antigo e dados de aplicações que podem dar problemas numa instalação nova. Copiar só as pastas de saves é mais limpo.

### Os meus saves funcionam se o meu utilizador do Windows tiver outro nome?

Sim, quase sempre. Os saves ficam dentro da tua pasta de utilizador, por isso o nome no caminho não interessa. O Hoard trata disso sozinho.

### Posso passar saves do Windows para um Steam Deck?

Sim. Abre o jogo uma vez no Deck para que o prefixo do Proton exista e põe o save lá dentro, ou deixa o Hoard fazê-lo. Vê [o guia do Steam Deck](/guides/sync-saves-steam-deck-pc).

### Tenho de guardar o PC antigo até o novo estar pronto?

Se copiares à mão, guarda o disco externo até teres confirmado todos os jogos. Com o Hoard, os saves já estão no servidor, por isso o PC antigo pode ir embora assim que cada jogo mostrar uma versão no histórico.
