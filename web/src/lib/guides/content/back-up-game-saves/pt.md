---
title: "Como fazer backup dos teus saves automaticamente"
description: "Configura backups na nuvem automáticos e versionados dos teus saves de PC com o Hoard — para que uma falha, uma reinstalação ou um mod com problemas nunca apaguem o teu progresso."
order: 1
updated: 2026-09-01
---

Perder um save significa perder horas de progresso. O Hoard faz backup dos teus saves de PC automaticamente e guarda um histórico completo de versões, para que possas sempre voltar atrás.

## O que o Hoard guarda

O Hoard deteta as pastas de save dos jogos a que jogas e copia-as para a tua própria nuvem — Hoard Cloud ou um servidor que alojes tu mesmo. Cada backup é versionado, por isso as cópias antigas nunca são sobrescritas.

Para encontrar onde cada jogo guarda os saves, o Hoard usa a mesma base de dados comunitária de localizações que alimenta o Ludusavi, por isso a deteção funciona logo para milhares de títulos. A diferença está no que acontece a seguir: em vez de deixar o backup no teu disco, o Hoard versiona-o automaticamente na nuvem.

## Configurar backups automáticos

1. **Descarrega e instala o Hoard** para Windows, macOS ou Linux a partir da página de download.
2. Inicia sessão, ou aponta a app para o teu servidor self-hosted.
3. Abre a **Biblioteca**. O Hoard procura jogos instalados e lista os saves que encontra.
4. Adiciona os jogos que queres proteger. O Hoard localiza cada pasta de save automaticamente; podes adicionar um caminho à mão se um jogo não for detetado.
5. Deixa o **modo automático** ligado. O Hoard vigia as pastas de save e faz backup quando paras de jogar.

A partir daí cada sessão é capturada sem que faças nada.

## Onde os jogos de PC guardam mesmo os saves

Não há um sítio único, e é exatamente por isso que uma ferramenta destas existe. Na prática, um save acaba num destes lugares:

- **Dentro da Steam**, em `userdata/<UserID>/<AppID>/remote/` — a pasta que a própria Steam Cloud sincroniza.
- **`Documentos\My Games\…`**, o mais parecido com uma convenção que o Windows tem.
- **`%APPDATA%`, `%LOCALAPPDATA%` ou `LocalLow`**, onde escrevem a maioria dos jogos Unity e Unreal.
- **`%USERPROFILE%\Saved Games`**, usada por um grupo menor mas teimoso de títulos.
- **A própria pasta de instalação do jogo**, onde ainda guardam surpreendentemente muitos títulos antigos.
- **No Linux**, `~/.local/share` ou `~/.config` para jogos nativos, e dentro do prefixo Proton — `steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…` — para os de Windows.
- **No macOS**, `~/Library/Application Support`.

De onde veio o jogo pouco importa: os títulos de GOG, Epic e itch caem no mesmo punhado de sítios, porque quem decide é o motor e o programador, não a loja.

## O que é copiado e o que não é

Uma pasta de saves raramente contém só saves, por isso o Hoard separa o que encontra em três montes:

- **Os dados de save** são copiados e restaurados. Isso é o teu progresso.
- **Os ficheiros que pertencem a uma máquina concreta** — configuração, registos e afins — são enviados para fazerem parte da cópia, mas nunca escritos por cima da cópia de outro PC. As tuas definições gráficas continuam tuas.
- **O lixo** — caches, despejos de erro, temporários — é ignorado, para que uma cópia não inche com coisas que nunca quererias de volta.

## Quando é feita a cópia

O Hoard vigia a pasta e captura-a **depois de parares de jogar**, não enquanto o jogo tem ficheiros abertos. Se o save foi escrito há segundos, espera que as coisas acalmem: um ficheiro a ser escrito não é um ficheiro que valha a pena capturar a meio.

Cada captura é uma versão. Os snapshots são guardados por hash de conteúdo, por isso um ficheiro que não muda é guardado uma só vez: dez versões de um save de 2 GB ocupam cerca de 2 GB, não 20.

## Cópias sem passar pelos nossos servidores

Se preferes não usar a nuvem de ninguém, corre o `hoard-server` tu mesmo e aponta a aplicação para lá. Os teus saves vão do teu PC para o teu disco: sem conta connosco, sem telemetria para nós e sem nada a passar pelos nossos servidores. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard).

## Dica: verifica o teu histórico

Abre o separador **Histórico** de um jogo para ver cada backup com data e tamanho. A partir daí podes restaurar qualquer versão anterior com um clique. Os teus saves viajam cifrados, são guardados na UE, e podes exportá-los ou apagá-los quando quiseres.

Já usas uma ferramenta de backup local como o Ludusavi? Podes mantê-la — mas se queres que esses backups cheguem à nuvem e sincronizem entre máquinas sem configurares o Rclone tu mesmo, é exatamente isso que o Hoard automatiza. Vê [Ludusavi vs Hoard](/guides/ludusavi-alternative) para uma comparação justa.

<!-- faq -->

## Perguntas frequentes

### O Hoard faz cópias enquanto jogo?

Não. Espera que saias e que a pasta de saves fique quieta, por isso uma cópia nunca é um ficheiro escrito a meio.

### Quanto espaço ocupam os meus saves?

Menos do que imaginas. As versões são desduplicadas por hash de conteúdo, por isso só ocupa espaço novo aquilo que mudou mesmo entre sessões: a maioria das coleções cabe à vontade em dois gigabytes.

### E se um dos meus jogos não for detetado?

Aponta o Hoard para a pasta à mão e ele segue-a como qualquer outra. A deteção cobre milhares de títulos, mas um jogo que guarde num sítio invulgar, ou que tenhas instalado à mão, às vezes precisa da pista.

### Também copia as minhas mods?

O Hoard segue a pasta de saves, por isso mods que vivam noutro sítio não entram na cópia. É de propósito: as mods são grandes, voltam a descarregar-se, e uma pasta de mods a sincronizar entre máquinas dá mais problemas do que resolve.

### O self-hosting muda a forma como as cópias funcionam?

Nada. A mesma deteção, as mesmas versões, a mesma captura automática. Só o armazenamento é teu.
