---
title: "Alternativa ao OpenCloudSaves: está arquivado, eis o que usar no lugar dele"
description: "O OpenCloudSaves está arquivado desde janeiro de 2025. O Hoard é um substituto mantido e open source que sincroniza sozinho e guarda todas as versões."
order: 28
updated: 2026-11-05
---

O OpenCloudSaves foi arquivado a 20 de janeiro de 2025. O código e a última versão continuam lá, mas o projeto já não aceita alterações, por isso nada o vai corrigir para jogos novos, launchers novos ou atualizações do sistema. Se sincronizava os teus saves através de uma nuvem, o [Hoard](/download) é um substituto gratuito e open source, mantido ativamente, que encontra os teus saves sozinho e os sincroniza sempre que o jogo escreve um.

## O que o OpenCloudSaves fazia bem

- **A tua própria nuvem.** Os saves iam para o OneDrive, Google Drive, Dropbox ou Nextcloud que já tinhas, através do Rclone, com uma interface a sério em vez de ficheiros de configuração.
- **Cauteloso de origem.** Fazia um ensaio antes de cada sincronização, por isso vias o que ia mudar antes de mudar.
- **Filtros e jogos próprios.** Podias sincronizar só certos ficheiros, deixar de fora as definições gráficas e definir jogos tu mesmo, até entre lojas.
- **Todos os ambientes de trabalho.** Windows, macOS e Linux, Steam Deck incluída.

## O que um substituto precisa

- **Alguém que o mantenha.** As localizações dos saves mudam e os launchers também; uma ferramenta de sincronização que ninguém atualiza acaba a sincronizar a pasta errada.
- **Um histórico, não só a última cópia.** Uma nuvem sincroniza ficheiros, por isso um save corrompido chega a todas as máquinas tão fielmente como um bom.
- **Nada de ficheiros escritos a meio.** Uma sincronização genérica pode apanhar um save enquanto o jogo ainda o escreve; o [guia do OneDrive](/guides/onedrive-game-saves) mostra o que isso parte.

## O Hoard como substituto

| | OpenCloudSaves | Hoard |
|---|---|---|
| Mantido | Arquivado desde janeiro de 2025 | Sim, open source (AGPL-3.0) |
| Encontrar saves | Definições de saves, mais pastas que adicionas | Automático: base de dados comunitária, Steam, Epic, GOG, prefixos do Proton e do Wine, 19 emuladores |
| Quando sincroniza | Quando corre uma sincronização, depois de um ensaio | Sempre que o jogo grava, e outra vez quando sais |
| Onde vivem os saves | A tua nuvem | Hoard One, ou o teu próprio servidor em disco local ou qualquer armazenamento compatível com S3 |
| Histórico | O que a nuvem guardar | Todas as versões, desduplicadas |
| Dez versões de um save de 2 GB | Cerca de 20 GB de quota da nuvem | Cerca de 2 GB, porque as versões partilham o que não mudou |
| Plataformas | Windows, macOS, Linux | Windows, Linux, macOS e SteamOS |

A única coisa que o Hoard não faz é escrever no teu Google Drive ou OneDrive. Sincroniza através de um servidor, e é isso que lhe dá o histórico e deixa a outra máquina estar desligada. Se o que querias era armazenamento que controlas, mais do que essa nuvem em concreto, um Hoard self-hosted escreve num NAS, num disco local ou em qualquer bucket compatível com S3, como o Backblaze B2. Se tem de ser essa nuvem, o Ludusavi com um remoto Rclone é a opção mantida mais próxima.

## Como mudar

1. Antes de mexer em alguma coisa, copia as tuas pastas de saves para um sítio seguro.
2. Deixa de sincronizar essas pastas no OpenCloudSaves, para que duas ferramentas nunca escrevam no mesmo save.
3. Instala o Hoard e inicia sessão no Hoard One, recomendado para todos, ou aponta-o para o [teu próprio servidor](/guides/self-host-hoard).
4. Abre a **Biblioteca**. O Hoard encontra os teus jogos; adiciona à mão qualquer pasta própria. A primeira versão aparece no histórico assim que o jogo grava.
5. Instala o Hoard nas tuas outras máquinas com a mesma conta.

## Hoard One, ou o teu próprio servidor

O Hoard é gratuito das duas maneiras, e a aplicação é a mesma em ambas:

- **Hoard One**, recomendado para todos. Nada para configurar nem para manter a correr: um servidor gerido na UE, gratuito com 2 GB e três dispositivos, sem cartão. O [Pro](/pricing) só acrescenta espaço, 100 GB e dispositivos ilimitados, para quando os teus saves já não couberem.
- **O teu próprio servidor.** Corres o `hoard-server` num PC, num NAS ou num Raspberry Pi e todas as tuas máquinas sincronizam através dele. Sem conta connosco, sem telemetria para nós e sem quota nossa: nada passa pelos nossos servidores, porque não há nada nosso no caminho. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard).

O mesmo programa, a mesma deteção, o mesmo histórico. O Hoard One guarda todas as versões para sempre; no teu próprio servidor decides tu quanto histórico guardar.

<!-- faq -->

## Perguntas frequentes

### O OpenCloudSaves ainda é seguro de usar?

A última versão continua disponível, mas o projeto está arquivado: não há correções para jogos novos nem para mudanças do sistema. Além disso saiu como beta, e o autor pedia que fizesses backup dos saves antes de o usares. Se continuares com ele, guarda uma cópia à mão.

### O Hoard pode guardar os saves no Google Drive ou no OneDrive?

Não. O Hoard sincroniza através de um servidor: o Hoard One, ou o teu em disco local ou armazenamento compatível com S3. A tua pasta `Documentos` pode continuar no OneDrive; o Hoard pergunta ao Windows onde ela está de facto e encontra lá os saves.

### O Hoard é gratuito?

Sim. A aplicação, a linha de comandos e o servidor são gratuitos e de código aberto (AGPL-3.0), e correr o teu próprio servidor não custa mais do que a máquina. O Hoard One, o servidor gerido que recomendamos a todos, também é gratuito com 2 GB e três dispositivos; o Pro acrescenta espaço por 1,99 € por mês.

### O self-hosting envia alguma coisa ao Hoard?

Não. No teu próprio servidor não há conta connosco nem telemetria para nós: os teus saves, os teus utilizadores e os teus logs ficam na tua máquina e nunca tocam nos nossos.
