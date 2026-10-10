---
title: "Onde ficam os saves de Baldur's Gate 3 (PC e Steam Deck)"
description: "Onde o Baldur's Gate 3 guarda os saves no Windows, Steam Deck e Mac, o que é save e o que são mods ou definições, o modo Honra, backup e sincronização."
order: 23
updated: 2026-10-09
---

No Windows, o Baldur's Gate 3 guarda os saves em `%LOCALAPPDATA%\Larian Studios\Baldur's Gate 3\PlayerProfiles\Public\Savegames\Story`, uma pasta por save. É o caminho que a Larian indica no seu FAQ de suporte. Abaixo tens os caminhos na Steam Deck e no Mac, o que está ao lado dos saves, o modo Honra e como manter tudo com backup e sincronizado entre o PC e a Steam Deck.

## Onde o Baldur's Gate 3 guarda os saves

- **Windows:** `%LOCALAPPDATA%\Larian Studios\Baldur's Gate 3\PlayerProfiles\Public\Savegames\Story`
- **Steam Deck e Linux** (Proton): `~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story`
- **Mac:** `~/Documents/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story`

A Larian retirou a versão nativa para Linux, por isso na Steam Deck o jogo corre com o Proton e os saves estão dentro do prefixo do Proton que o Steam mantém para ele; `1086940` é o ID do jogo no Steam. Se estiver instalado no cartão microSD, a pasta `compatdata` está no cartão.

## O que é save e o que não é

Cada save é **uma pasta** dentro de `Story`, com um ficheiro `.lsv` e uma miniatura. O que está à volta é outra coisa:

- **`Mods`** (dentro de `Baldur's Gate 3`) guarda os ficheiros dos mods.
- **`modsettings.lsx`** (dentro de `PlayerProfiles\Public`) é a lista de mods ativos e a sua ordem de carregamento.
- **As definições**, como gráficos e controlos, são ficheiros de configuração junto ao perfil, não fazem parte de um save.

A armadilha são os mods. Um save feito com mods espera os mesmos mods ativos ao carregar. Se levares um save com mods para outro PC, leva também a lista de mods, ou o jogo avisa que faltam mods e o save pode não carregar como esperas.

## O modo Honra

O modo Honra tem um único save que o jogo sobrescreve enquanto jogas, e se o teu grupo cair, a partida em Honra acaba (podes continuar no modo Personalizado, sem a Honra). Fazer backup desse save é decisão tua: uma cópia antes de um combate difícil é, tecnicamente, uma forma de voltar atrás, e há quem a queira precisamente depois de um crash ou de um bug, enquanto outros o veem como fazer batota ao modo. Uma ferramenta de backup guarda as versões na mesma; se alguma vez restaurares uma, é entre ti e os dados.

## O Baldur's Gate 3 tem saves na nuvem?

Sim. No Steam usa o Steam Cloud, que mantém em dia os últimos saves entre máquinas com a mesma conta. Só guarda o estado atual: se um save se corromper ou a atualização de um mod o estragar, é essa a versão que se sincroniza.

## Fazer backup à mão

1. Fecha o jogo por completo.
2. Copia a pasta `PlayerProfiles` inteira do caminho acima (contém `Savegames` e `modsettings.lsx`).
3. Para restaurar, fecha o jogo e volta a copiá-la.

## Backup e sincronização automáticos com o Hoard

O [Hoard](/download) faz backup da pasta de saves sempre que deixas de jogar e guarda todas as versões, por isso um save estragado pela atualização de um mod ou por um patch está a um restauro de distância. Também mantém a pasta sincronizada entre os teus PCs e uma Steam Deck.

1. Instala o Hoard e inicia sessão, ou aponta-o para [o teu próprio servidor](/guides/self-host-hoard).
2. Abre a **Biblioteca**. O Baldur's Gate 3 é detetado a partir da tua biblioteca Steam e da base de dados comunitária de saves.
3. Joga. Ao sair, a primeira versão aparece no histórico.

Cada sessão junta uma versão com todos os saves lá dentro. Para voltar atrás, abre o histórico e [restaura uma versão anterior](/guides/restore-a-game-save); o que tens agora no PC é copiado antes, por isso experimentar uma antiga nunca é uma viagem sem volta.

<!-- faq -->

## Perguntas frequentes

### Onde ficam os saves de Baldur's Gate 3 na Steam Deck?

Dentro do prefixo do Proton do jogo: `~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story`.

### Posso levar um save com mods para outro PC?

Sim, desde que o outro PC tenha os mesmos mods instalados e ativos pela mesma ordem. Copia o `modsettings.lsx` junto com o save e instala os mesmos ficheiros de mods.

### Posso fazer backup de um save do modo Honra?

O save é uma pasta normal, por isso sim, qualquer ferramenta de backup o pode copiar. Se restaurá-lo combina com o espírito do modo, decides tu.

### Porque é que o meu save diz que faltam mods?

Foi feito com mods que agora não estão ativos. Volta a ativar os mesmos mods, pela mesma ordem, e carrega normalmente.
