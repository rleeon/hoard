---
title: "Onde ficam os saves de Elden Ring (PC e Steam Deck)"
description: "Onde o Elden Ring guarda o ER0000.sl2 no PC e na Steam Deck, o que contém, Seamless Co-op e Nightreign, e como fazer backup de todas as personagens."
order: 26
updated: 2026-10-18
---

No PC, o Elden Ring guarda os saves em `%APPDATA%\EldenRing\<o teu ID do Steam>`, num único ficheiro, `ER0000.sl2`, que contém todas as personagens que criaste, até dez. Abaixo tens o caminho na Steam Deck, o que mais há na pasta, onde o Seamless Co-op e o Nightreign guardam os saves, e como manter tudo com backup e sincronizado entre o PC e a Steam Deck.

## Onde o Elden Ring guarda os saves

- **Windows (Steam):** `%APPDATA%\EldenRing\<o teu ID do Steam>\ER0000.sl2`
- **Steam Deck e Linux** (Proton): `~/.local/share/Steam/steamapps/compatdata/1245620/pfx/drive_c/users/steamuser/AppData/Roaming/EldenRing/<o teu ID do Steam>/`

`%APPDATA%` é `C:\Users\<o teu utilizador>\AppData\Roaming`: cola `%APPDATA%\EldenRing` na barra de endereço do Explorador de Ficheiros para ires diretamente. A pasta do ID do Steam é um número longo ligado à tua conta Steam. Na Steam Deck o jogo corre com o Proton, por isso o save está dentro do prefixo do Proton que o Steam mantém para ele; `1245620` é o ID do jogo no Steam.

## O que há na pasta de saves

- **`ER0000.sl2`** é o teu progresso: todas as personagens, o equipamento e onde estão no mundo, Shadow of the Erdtree incluído.
- **`ER0000.sl2.bak`** é uma cópia de segurança que o jogo guarda ao lado.
- **`ER0000.co2`**, se o vires, é do mod Seamless Co-op (mais abaixo).

As definições gráficas não estão nesta pasta: ficam um nível acima, em `%APPDATA%\EldenRing\GraphicsConfig.xml`.

Da forma como o ficheiro funciona resultam duas coisas:

- **As personagens não se copiam nem se restauram uma a uma.** Restaurar um `ER0000.sl2` antigo leva todas as personagens de volta a esse momento, não só a que querias corrigir.
- **O save está ligado à tua conta Steam.** O teu ID do Steam está escrito dentro do ficheiro, e o jogo não carrega um save criado noutra conta. Passar o teu próprio save entre os teus PCs e uma Steam Deck funciona sem problemas.

## Seamless Co-op e Nightreign

### Seamless Co-op

O mod Seamless Co-op não mexe no `ER0000.sl2`: guarda o seu próprio save, `ER0000.co2`, na mesma pasta, por isso as tuas personagens com e sem mod são ficheiros diferentes. A comunidade do mod desaconselha copiar um `.co2` por cima do save normal para jogar online.

### Elden Ring Nightreign

O Nightreign é outro jogo, com a sua própria pasta: `%APPDATA%\Nightreign\<o teu ID do Steam>\NR0000.sl2` no Windows, e o mesmo caminho dentro do seu próprio prefixo do Proton na Steam Deck (`compatdata/2622380`). Nunca toca nas tuas personagens de Elden Ring.

## O Elden Ring tem saves na nuvem?

Sim, através do Steam Cloud, que mantém o `ER0000.sl2` em dia entre as máquinas da mesma conta. Só guarda o último estado, sem versões anteriores para onde voltar, e o `.bak` do jogo fica no mesmo disco que o save: um disco morto leva os dois.

## Fazer backup à mão

1. Fecha o jogo por completo.
2. Copia a tua pasta do ID do Steam de `%APPDATA%\EldenRing` para um sítio seguro. Contém o `ER0000.sl2`, o seu `.bak` e o save do Seamless Co-op, se o tiveres.
3. Para restaurar, fecha o jogo e volta a copiar a pasta para o mesmo sítio.

## Backup e sincronização automáticos com o Hoard

O [Hoard](/download) é gratuito e de código aberto. Faz backup dos teus saves de Elden Ring sempre que o jogo grava um e outra vez quando sais, guarda um histórico para onde podes voltar e sincroniza a pasta entre os teus PCs e uma Steam Deck. Só viajam os ficheiros que mudaram, comprimidos, e cada ficheiro é verificado com o seu hash SHA-256 quando regressa.

1. Instala o Hoard e inicia sessão no Hoard One, recomendado para todos, ou aponta-o para [o teu próprio servidor](/guides/self-host-hoard).
2. Abre a **Biblioteca**. O Elden Ring é detetado a partir da tua biblioteca Steam e da base de dados comunitária de saves.
3. Joga. Ao sair, a primeira versão aparece no histórico.

O Hoard leva a pasta do ID do Steam inteira, por isso um save do Seamless Co-op também vai em cada versão. Cada versão são todas as tuas personagens nesse momento: para voltar atrás, [restaura uma versão anterior](/guides/restore-a-game-save), e as versões mais recentes ficam no histórico se mudares de ideias.

## Hoard One, ou o teu próprio servidor

O Hoard é gratuito das duas maneiras, e a aplicação é a mesma em ambas:

- **Hoard One**, recomendado para todos. Nada para configurar nem para manter a correr: um servidor gerido na UE, gratuito com 2 GB e três dispositivos, sem cartão. O [Pro](/pricing) só acrescenta espaço, 100 GB e dispositivos ilimitados, para quando os teus saves já não couberem.
- **O teu próprio servidor.** Corres o `hoard-server` num PC, num NAS ou num Raspberry Pi e todas as tuas máquinas sincronizam através dele. Sem conta connosco, sem telemetria para nós e sem quota nossa: nada passa pelos nossos servidores, porque não há nada nosso no caminho. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard).

O mesmo programa, a mesma deteção, o mesmo histórico. O Hoard One guarda todas as versões para sempre; no teu próprio servidor decides tu quanto histórico guardar.

<!-- faq -->

## Perguntas frequentes

### Posso usar o meu save de Elden Ring noutra conta Steam?

Tal como está, não. O teu ID do Steam está escrito dentro do `ER0000.sl2`, e o jogo não carrega um save criado noutra conta. O teu próprio save, na tua própria conta, passa sem problemas entre os teus PCs e uma Steam Deck.

### Restaurar um save antigo faz recuar todas as minhas personagens?

Sim. Todas as personagens partilham o `ER0000.sl2`, por isso qualquer restauro leva-as todas de volta a esse momento. Com o Hoard as versões mais recentes ficam no histórico, por isso voltar atrás nunca é uma viagem só de ida.

### O Hoard faz backup dos saves do Seamless Co-op?

Sim. O mod guarda o `ER0000.co2` na mesma pasta do ID do Steam que o save normal, e o Hoard faz backup da pasta inteira: os dois vão em cada versão.

### O Hoard é gratuito?

Sim. A aplicação, a linha de comandos e o servidor são gratuitos e de código aberto (AGPL-3.0), e correr o teu próprio servidor não custa mais do que a máquina. O Hoard One, o servidor gerido que recomendamos a todos, também é gratuito com 2 GB e três dispositivos; o Pro acrescenta espaço por 1,99 € por mês.

### O self-hosting envia alguma coisa ao Hoard?

Não. No teu próprio servidor não há conta connosco nem telemetria para nós: os teus saves, os teus utilizadores e os teus logs ficam na tua máquina e nunca tocam nos nossos.
