---
title: "Onde ficam os saves de Palworld (PC e Steam Deck)"
description: "Onde o Palworld guarda os mundos no PC e na Steam Deck, o que é cada ficheiro, como funcionam os mundos em co-op e como fazer backup e sincronizar os saves."
order: 24
updated: 2026-10-09
---

No PC (Steam), o Palworld guarda os saves em `%LOCALAPPDATA%\Pal\Saved\SaveGames\<o teu ID do Steam>`, com uma pasta por mundo lá dentro. É o caminho que a Pocketpair indica no seu FAQ oficial. Abaixo tens o caminho na Steam Deck, o que faz cada ficheiro, o que muda no co-op e como manter os teus mundos com backup e sincronizados entre o PC e a Steam Deck.

## Onde o Palworld guarda os saves

- **Windows, versão Steam:** `%LOCALAPPDATA%\Pal\Saved\SaveGames\<o teu ID do Steam>\<ID do mundo>`
- **Steam Deck e Linux** (Proton): `~/.local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames/`

A pasta do ID do Steam é um número longo ligado à tua conta Steam. Lá dentro, cada mundo que criaste tem a sua pasta com um nome hexadecimal longo. Na Steam Deck o jogo corre com o Proton, por isso os saves estão dentro do prefixo do Proton que o Steam mantém para ele; `1623730` é o ID do jogo no Steam.

A versão da app Xbox / Game Pass guarda os saves noutro sítio, empacotado, e não são os mesmos ficheiros que copiarias entre instalações do Steam.

## O que há na pasta de um mundo

- **`Level.sav`** é o mundo em si: a tua base, o mapa, os Pals lá colocados.
- **`LevelMeta.sav`** guarda o nome e o resumo do mundo para o menu.
- **`Players\`** guarda um `.sav` por cada jogador que esteve nesse mundo.
- **`LocalData.sav`** e **`WorldOption.sav`** guardam dados locais e as definições do mundo.
- **`backup\`** são os backups automáticos do mundo feitos pelo próprio jogo.

As definições estão noutro sítio: `Pal\Saved\Config\Windows\GameUserSettings.ini` guarda gráficos e controlos, e `Pal\Saved\Logs` os logs. Nenhum faz parte do teu progresso.

Ao fazer o backup, leva **a pasta do mundo inteira**, não só o `Level.sav`. O mundo e os ficheiros de jogador andam juntos, e restaurar um sem o outro é como as personagens acabam dessincronizadas do mundo em que estão.

## Co-op e servidores dedicados

No co-op, **o mundo vive no PC do anfitrião**. A tua personagem nesse mundo é um ficheiro na pasta `Players` do anfitrião, não na tua máquina. Se o anfitrião perder o save, o progresso de todos nesse mundo vai com ele. Num servidor dedicado, o mundo vive no servidor.

Por isso, num mundo partilhado, é a pasta do anfitrião que precisa de backup.

## O Palworld tem saves na nuvem?

A versão Steam usa o Steam Cloud, que mantém em dia o último estado dos teus mundos entre máquinas com a mesma conta. Não guarda versões anteriores, e a pasta `backup\` do jogo está no mesmo disco que o save, por isso um disco morto leva os dois.

## Fazer backup à mão

1. Fecha o jogo por completo.
2. Copia a tua pasta do ID do Steam de `SaveGames` (contém todos os teus mundos) para um sítio seguro.
3. Para restaurar, fecha o jogo e volta a copiá-la para o mesmo sítio.

## Backup e sincronização automáticos com o Hoard

O [Hoard](/download) faz backup dos teus mundos sempre que deixas de jogar e guarda todas as versões fora da máquina, por isso um mundo corrompido ou um disco perdido não são o fim de uma base em que andas há semanas. Também sincroniza os mundos entre os teus PCs e uma Steam Deck.

1. Instala o Hoard e inicia sessão, ou aponta-o para [o teu próprio servidor](/guides/self-host-hoard).
2. Abre a **Biblioteca**. O Palworld é detetado a partir da tua biblioteca Steam e da base de dados comunitária de saves.
3. Joga. Ao sair, a primeira versão aparece no histórico.

Se és anfitrião no co-op, esta é a máquina que importa: faz backup do anfitrião e o mundo partilhado fica coberto. Para recuar um mundo, [restaura uma versão anterior](/guides/restore-a-game-save).

<!-- faq -->

## Perguntas frequentes

### Onde ficam os saves de Palworld na Steam Deck?

Dentro do prefixo do Proton: `~/.local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames/`, na pasta com o teu ID do Steam.

### Onde fica guardada a minha personagem no mundo de um amigo?

No PC do anfitrião, na pasta `Players` desse mundo. O teu PC não guarda cópia dos mundos alojados por outra pessoa.

### Que ficheiro é o meu mundo?

O `Level.sav`, mas faz backup da pasta do mundo inteira: os ficheiros de jogador e o mundo andam juntos.

### O Palworld faz backup do meu mundo sozinho?

Guarda backups automáticos na pasta `backup` do mundo. Estão no mesmo disco que o save, por isso protegem-te de um save mau, não de perder o disco.
