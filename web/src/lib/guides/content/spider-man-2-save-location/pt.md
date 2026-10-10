---
title: "Onde ficam os saves de Marvel's Spider-Man 2 (PC e Steam Deck)"
description: "Onde o Marvel's Spider-Man 2 guarda os saves no PC, a pasta do número longo, a armadilha do OneDrive, o caminho na Steam Deck, backup e sincronização."
order: 22
updated: 2026-10-09
---

No PC, o Marvel's Spider-Man 2 guarda os saves em `Documentos\Marvel's Spider-Man 2\`, dentro de uma subpasta com um número longo. No Steam, esse número é o teu ID do Steam. Abaixo tens o que isso significa na prática, a armadilha do OneDrive, o caminho na Steam Deck e como manter os saves com backup e sincronizados entre o PC e a Steam Deck.

## Onde o Marvel's Spider-Man 2 guarda os saves

- **Windows:** `%USERPROFILE%\Documents\Marvel's Spider-Man 2\<número longo>`
- **Steam Deck e Linux** (Proton): `~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/<número longo>`

A versão de PC é só para Windows, por isso na Steam Deck corre com o Proton e os saves estão dentro do prefixo do Proton que o Steam mantém para o jogo; `2651280` é o seu ID no Steam. Se estiver instalado no cartão microSD, a pasta `compatdata` está no cartão.

A Nixxes, o estúdio responsável pela versão de PC, descreve a pasta de saves como «uma subpasta com um número longo ou uma combinação de letras e números» dentro de `Documentos\Marvel's Spider-Man 2\`.

## A pasta do número longo

A subpasta tem o nome da tua conta: no Steam é o teu **ID do Steam de 64 bits**; a versão da Epic usa antes uma mistura de letras e números. Em qualquer caso, é diferente para cada conta. Duas consequências:

- Se duas pessoas jogarem no mesmo PC com contas Steam diferentes, cada uma tem a sua pasta de saves.
- Se copiares saves à mão para outro PC, põe-nos na pasta da conta Steam **dessa** máquina. Numa pasta com outro ID, o jogo não os vê.

A pasta mãe `Marvel's Spider-Man 2` também guarda o log do jogo e os crash dumps (`.log`, `.mdmp`). Não são saves e não precisam de backup.

## A armadilha do OneDrive

Muitos PCs com Windows redirecionam `Documentos` para o OneDrive. Se for o teu caso, o caminho real é `%USERPROFILE%\OneDrive\Documents\Marvel's Spider-Man 2\`, e o OneDrive sincroniza a pasta por conta própria enquanto jogas. Isso traz dois problemas: o OneDrive pode enviar um save escrito a meio, e «Libertar espaço» pode transformar o save num marcador só online. Se dependes do OneDrive aqui, marca a pasta como **Manter sempre neste dispositivo**.

## O Spider-Man 2 tem saves na nuvem?

Sim, Steam Cloud, que mantém em dia os últimos saves entre máquinas com a mesma conta Steam. Não guarda versões anteriores: se um save se estragar, é o estragado que se sincroniza.

## Fazer backup à mão

1. Fecha o jogo por completo.
2. Copia a pasta `Marvel's Spider-Man 2` de `Documentos` para um sítio seguro.
3. Para restaurar, fecha o jogo e volta a copiar a pasta do número longo para o mesmo sítio, com a mesma conta Steam.

## Backup e sincronização automáticos com o Hoard

O [Hoard](/download) faz backup da pasta de saves sempre que deixas de jogar e guarda todas as versões. Também a sincroniza entre os teus PCs e uma Steam Deck, para que o jogo continue onde o deixaste em qualquer um deles.

1. Instala o Hoard e inicia sessão, ou aponta-o para [o teu próprio servidor](/guides/self-host-hoard).
2. Abre a **Biblioteca** e confirma que a pasta mostrada para o Spider-Man 2 é a de `Documentos` (ou `OneDrive\Documents`). Se apontar para outro sítio, muda-a para essa pasta.
3. Joga. Ao sair, a primeira versão aparece no histórico.

Se mais tarde um save se estragar, [restaurar uma versão anterior](/guides/restore-a-game-save) devolve-o.

<!-- faq -->

## Perguntas frequentes

### O que é o número longo da pasta de saves?

No Steam, o teu ID do Steam de 64 bits; na Epic, o ID da tua conta. Cada conta tem a sua pasta, e o jogo só lê a da conta com sessão iniciada.

### Onde ficam os saves de Spider-Man 2 na Steam Deck?

Dentro do prefixo do Proton: `~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/`, na pasta do número longo.

### Não encontro a pasta em Documentos. Onde está?

Procura em `OneDrive\Documents\Marvel's Spider-Man 2`. Na maioria das instalações recentes do Windows, Documentos fica dentro do OneDrive.

### Posso copiar os meus saves para o PC de um amigo?

Os ficheiros copiam-se, mas vão para a pasta com o ID do Steam da conta desse PC. Se o jogo aceita saves feitos noutra conta depende do jogo, por isso guarda uma cópia do original antes de experimentar.
