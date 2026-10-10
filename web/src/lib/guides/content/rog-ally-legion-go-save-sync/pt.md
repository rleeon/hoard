---
title: "Sincronizar saves entre ROG Ally, Legion Go, MSI Claw e o teu PC"
description: "As consolas portáteis com Windows como a ROG Ally, a Legion Go e a MSI Claw são PCs. Sincroniza os saves delas com o teu desktop automaticamente, com histórico."
order: 19
updated: 2026-10-09
---

A ROG Ally, a Legion Go e a MSI Claw correm Windows, por isso para um jogo são só mais um PC. E esse é precisamente o problema: o teu desktop e a tua portátil guardam cada um os seus próprios saves. O Steam Cloud cobre parte da tua biblioteca e a nuvem da Xbox cobre o Game Pass, mas tudo o resto fica na máquina onde jogaste. O Hoard mantém os saves sincronizados entre a portátil e o desktop automaticamente: deixas de jogar numa e o jogo espera-te na outra, com todas as versões anteriores guardadas.

## O que já te segue

- **Os jogos do Steam com Steam Cloud** sincronizam-se sozinhos.
- **Os jogos do Game Pass e da app Xbox** usam a nuvem da Xbox, desde que jogues a versão da Xbox nas duas máquinas.
- **Epic, GOG, Ubisoft e EA** têm saves na nuvem para alguns dos seus jogos, dentro dos seus próprios launchers. Vê [os saves na nuvem da Epic e da GOG](/guides/epic-gog-cloud-saves).

O que fica de fora: jogos em que o programador nunca ativou a nuvem, emuladores, jogos instalados à mão e qualquer jogo em que o desktop e a portátil não usam o mesmo launcher.

## Configurar

1. **Na portátil**, muda para o ambiente de trabalho do Windows, abre a [página de transferência](/download) e instala o Hoard para Windows.
2. **Inicia sessão** com a conta que usas no desktop, ou aponta a app para o teu próprio servidor.
3. Abre a **Biblioteca** e vê o que o Hoard encontrou. Acrescenta o que faltar apontando para a pasta, por exemplo um emulador.
4. **No desktop**, instala o Hoard com a mesma conta. Os mesmos jogos associam-se sozinhos.

O motor de sincronização é um serviço em segundo plano que arranca com o Windows, por isso continua a funcionar enquanto estás no Armoury Crate, no Legion Space, no MSI Center M ou no modo Big Picture do Steam. Não precisas de abrir a janela do Hoard para jogar.

## Armadilhas das portáteis

### Suspender não é sair

Numa portátil é muito fácil carregar no botão de ligar e arrumá-la com o jogo aberto. O Hoard só faz backup depois de o jogo fechar, porque um jogo a correr pode estar a meio de escrever o save, e nunca troca o save de um jogo aberto. Se suspendes a portátil e depois jogas no desktop, o progresso da portátil ainda não foi enviado. **Fecha o jogo antes de mudares de máquina.**

### Jogos no cartão microSD

Instalar os jogos no cartão é normal numa portátil, e quase nunca afeta os saves: a maioria dos jogos guarda na tua pasta de utilizador no disco interno, esteja instalada onde estiver. A exceção são os jogos que guardam ao lado da pasta de instalação; se algum não for detetado, acrescenta a pasta à mão.

### Ecrã e definições

A tua portátil corre a uma resolução mais baixa e com uma GPU mais pequena do que o desktop. O Hoard faz backup dos ficheiros de definições como `graphics.ini` junto com o save, mas não os escreve por cima dos da outra máquina, por isso cada uma mantém as definições que lhe servem. Se mesmo assim os quiseres copiar, há uma opção para isso ao restaurar.

### Mesmo jogo, loja diferente

Um jogo comprado no Steam para o desktop e jogado com o Game Pass na portátil são duas instalações diferentes, e a versão da Xbox guarda os saves num formato que só a app Xbox entende. Para partilhar um save, joga a versão da mesma loja nas duas.

### SteamOS ou Bazzite em vez de Windows?

Então a tua portátil é uma máquina Linux e os saves vivem dentro de prefixos do Proton, tal como numa Steam Deck. Vê [sincronizar saves entre Steam Deck e PC](/guides/sync-saves-steam-deck-pc).

## Sem os nossos servidores

Se preferes que os saves fiquem em casa, corre o `hoard-server` no teu PC ou num NAS e aponta as duas máquinas para ele. Sem conta connosco, sem telemetria para nós, nada a passar pelos nossos servidores. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard).

<!-- faq -->

## Perguntas frequentes

### A ROG Ally tem saves na nuvem?

Tem o que cada launcher tiver: Steam Cloud, a nuvem da Xbox e as da Epic, GOG, Ubisoft ou EA nos jogos que a suportam. Não há uma sincronização de saves para todo o sistema. O Hoard acrescenta uma para os jogos que esses deixam de fora.

### O Hoard funciona com o Armoury Crate ou o Legion Space?

Sim. O motor de sincronização do Hoard é um serviço em segundo plano do Windows, independente do launcher com que abres os jogos.

### A portátil conta como dispositivo?

Sim. O plano gratuito inclui três dispositivos, por isso cabem um desktop, um portátil e uma consola portátil. O Pro e os servidores alojados por ti não têm limite de dispositivos.

### E os saves do Game Pass?

Deixa-os à nuvem da Xbox, que os sincroniza entre as instalações da app Xbox das duas máquinas. O Hoard cobre os jogos que não têm nuvem própria.

### Posso sincronizar a portátil também com uma Steam Deck?

Sim. O Hoard funciona nas duas e associa cada jogo entre o Windows e os prefixos do Proton da Deck.
