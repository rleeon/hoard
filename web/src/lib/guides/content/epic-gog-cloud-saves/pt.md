---
title: "Saves na nuvem da Epic e da GOG: o que cobrem e como sincronizar o resto"
description: "A nuvem da Epic e da GOG só funciona para alguns jogos, só no seu launcher e sem histórico. O que cobre e como sincronizar o resto automaticamente."
order: 19.5
updated: 2026-10-09
---

Tanto a Epic como a GOG têm saves na nuvem, com as mesmas limitações do Steam: o programador tem de os suportar jogo a jogo, só funcionam através do launcher da própria loja e guardam a última cópia em vez de um histórico. Aqui tens o que cada uma cobre, onde estão as falhas e como manter todos os teus jogos sincronizados entre os teus PCs e uma Steam Deck, comprados onde forem.

## Epic Games Store

O launcher da Epic tem um interruptor de saves na nuvem nas definições, e os jogos que os suportam sincronizam-se através dele quando jogas noutro PC. O suporte é jogo a jogo: o programador tem de o implementar, e muitos jogos da loja nunca o fizeram.

No Linux e na Steam Deck não há launcher oficial da Epic. O Heroic consegue sincronizar a nuvem da Epic nos jogos que a suportam, mas tens de a ativar jogo a jogo.

## GOG

O GOG Galaxy sincroniza os saves na nuvem dos jogos que incluem «Cloud saves» nas características da página da loja. Há duas limitações próprias da GOG:

- **Só através do Galaxy.** Os instaladores offline da GOG, a parte sem DRM que é o grande atrativo, não têm nuvem nenhuma. Joga com o instalador e os teus saves ficam nesse PC.
- **Jogo a jogo e plataforma a plataforma.** Um jogo só se sincroniza entre as plataformas para as quais o programador o preparou.

Tal como na Epic, o Heroic consegue sincronizar a nuvem da GOG no Linux e na Steam Deck se a ativares jogo a jogo.

## Ubisoft, EA e as restantes

O Ubisoft Connect e a app da EA têm saves na nuvem para muitos dos seus próprios jogos, cada um só dentro do seu launcher. Na Amazon Games e nas lojas mais pequenas varia de jogo para jogo.

## O que nenhuma faz

- **Histórico.** Todos os launchers guardam o save atual. Se um save se estraga e é sincronizado, o bom desaparece em todo o lado.
- **Entre lojas.** O mesmo jogo comprado no Steam para um PC e na GOG para outro tem duas nuvens separadas que nunca falam.
- **Tudo o que está fora do launcher.** Emuladores, instaladores sem DRM, jogos instalados à mão.
- **Jogos sem suporte.** Se o programador não o implementou, o launcher não pode fazer nada.

## Sincronizar o resto

O Hoard trabalha por jogo, não por loja. Encontra a pasta de saves de cada jogo com uma base de dados comunitária que cobre milhares de títulos, venha o jogo de onde vier, faz backup dela automaticamente quando deixas de jogar e sincroniza-a com os teus outros PCs e a tua Steam Deck, guardando todas as versões.

Isso cobre as falhas acima:

- **Qualquer launcher, ou nenhum.** Epic, GOG, Galaxy ou o instalador offline, o Heroic no Linux, um jogo que descompactaste numa pasta.
- **Entre lojas.** A maioria dos jogos guarda no mesmo sítio seja qual for a loja que os vendeu, normalmente em `AppData` ou `Documents`, por isso uma instalação da GOG num PC e uma do Steam noutro podem partilhar um save. Alguns acrescentam uma pasta com o ID da conta ou mudam de nome consoante a loja; confirma antes de contares com isso.
- **Um histórico.** Cada sessão é uma versão para onde podes voltar.

Onde a nuvem de um launcher já sincroniza um jogo, deixa-a continuar. O Hoard acrescenta o histórico e sincroniza tudo o resto.

Se preferes não usar a nuvem de ninguém, corre o `hoard-server` no teu PC ou NAS. Sem conta connosco, sem telemetria para nós, nada a passar pelos nossos servidores. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard).

<!-- faq -->

## Perguntas frequentes

### A Epic tem saves na nuvem?

Sim, nos jogos cujo programador os implementou, através do launcher da Epic. Muitos jogos da loja não os suportam, e o launcher não guarda histórico de saves anteriores.

### A GOG tem saves na nuvem?

Sim, através do GOG Galaxy, nos jogos que o indicam na página da loja. Os instaladores offline não sincronizam nada.

### A Epic ou a GOG guardam versões antigas dos meus saves?

Não. As duas guardam só a última cópia. Para voltar a um save anterior precisas de um backup que guarde versões.

### Posso passar um save da versão da GOG para a do Steam?

Muitas vezes, sim: a maioria dos jogos guarda na mesma pasta seja qual for a loja. Alguns acrescentam uma pasta com o ID da conta ou usam outro nome de pasta, por isso confirma primeiro os caminhos.

### Os instaladores offline da GOG sincronizam os saves?

Não através da GOG, porque a nuvem dela só funciona no Galaxy. O Hoard sincroniza-os como qualquer outro jogo, porque segue a pasta de saves e não o launcher.
