---
title: "Onde ficam os saves de Crimson Desert (PC e Steam Deck)"
description: "Onde o Crimson Desert guarda os saves no Windows, Steam Deck e Mac, que pasta os contém realmente e como fazer backup e sincronizá-los entre PCs."
order: 21
updated: 2026-10-09
---

No Windows, o Crimson Desert guarda os saves em `%LOCALAPPDATA%\Pearl Abyss\CD\save`. É a pasta que a Pearl Abyss indica no seu próprio FAQ. Abaixo tens os caminhos na Steam Deck e no Mac, o que há lá dentro e como a manter com backup e sincronizada entre o PC e a Steam Deck.

## Onde o Crimson Desert guarda os saves

- **Windows:** `%LOCALAPPDATA%\Pearl Abyss\CD\save` (ou seja, `C:\Users\<tu>\AppData\Local\Pearl Abyss\CD\save`)
- **Steam Deck e Linux** (Proton): `~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save`
- **Mac, versão Steam:** `~/Library/Application Support/Pearl Abyss/CD/save`
- **Mac, versão App Store:** `~/Library/Containers/com.pearlabyss.CrimsonDesert/Data/Library/Application Support/Pearl Abyss/CD/save`

Não há versão nativa para Linux, por isso na Steam Deck o jogo corre com o Proton e os saves estão dentro do prefixo do Proton que o Steam mantém para ele; `3321460` é o ID do jogo no Steam. Se estiver no cartão microSD, a pasta `compatdata` está no cartão.

`AppData` é uma pasta oculta no Windows. O mais rápido é colar `%LOCALAPPDATA%\Pearl Abyss\CD\save` na barra de endereço do Explorador de Ficheiros.

## O que há na pasta

Dentro de `save` há duas subpastas. Segundo a Pearl Abyss, **a que tem um nome numérico guarda os saves que crias no jogo**. Ao fazer o backup, leva a pasta `save` inteira em vez de escolher ficheiros: ocupa pouco e não deixas nada de que o jogo precise.

As versões Steam e App Store no Mac usam caminhos diferentes. Se mudares de uma para a outra, copia os saves à mão uma vez.

## O Crimson Desert tem saves na nuvem?

Sim, a versão Steam tem Steam Cloud, que mantém em dia os últimos saves entre máquinas com a mesma conta Steam.

O que não faz:

- **Guardar versões anteriores.** O Steam Cloud guarda o estado atual. Se um save se corromper, é o corrompido que se sincroniza.
- **Cobrir outras lojas.** Uma cópia da App Store do Mac e uma do Steam não partilham nuvem.

## Fazer backup à mão

1. Fecha o jogo por completo.
2. Copia a pasta `save` inteira do caminho acima para um sítio seguro: outro disco, uma pen USB, uma pasta na nuvem.
3. Para restaurar, fecha o jogo e volta a copiá-la, substituindo o que lá estiver.

Serve como cópia pontual antes de uma grande atualização ou de uma reinstalação. Como rotina depende de te lembrares, e só tens a cópia da última vez.

## Backup e sincronização automáticos com o Hoard

O [Hoard](/download) faz backup da pasta de saves sempre que deixas de jogar e guarda todas as versões, por isso podes voltar atrás depois de um save estragado ou de uma escolha de que te arrependes. Também mantém a pasta sincronizada entre os teus PCs e uma Steam Deck.

1. Instala o Hoard e inicia sessão, ou aponta-o para [o teu próprio servidor](/guides/self-host-hoard).
2. Abre a **Biblioteca**. O Crimson Desert é detetado a partir da tua biblioteca Steam e da base de dados comunitária de saves, no caminho acima.
3. Joga. Ao sair, a primeira versão aparece no histórico.

A partir daí cada sessão junta uma versão, e a mais recente está na máquina em que te sentares a seguir. Se um save se estragar, [restaurar um anterior](/guides/restore-a-game-save) são dois cliques.

<!-- faq -->

## Perguntas frequentes

### Onde ficam os saves de Crimson Desert na Steam Deck?

Dentro do prefixo do Proton do jogo: `~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save`. Se o jogo estiver no cartão microSD, a pasta `compatdata` está no cartão.

### Que subpasta tem os meus saves?

A do nome numérico, dentro de `save`. Mesmo assim, faz backup da pasta `save` inteira para não deixares nada de fora.

### Não encontro a pasta AppData. Onde está?

Está oculta por predefinição. Cola `%LOCALAPPDATA%\Pearl Abyss\CD\save` na barra de endereço do Explorador de Ficheiros e carrega em Enter, ou ativa os itens ocultos no menu Ver.

### Posso jogar no PC e na Steam Deck com o mesmo save?

Sim. O Steam Cloud fá-lo com o último save na mesma conta Steam. O Hoard também, e guarda uma versão por sessão, por isso podes voltar atrás se algo se estragar.
