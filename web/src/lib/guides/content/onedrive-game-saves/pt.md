---
title: "OneDrive e os saves dos jogos: o que se estraga e como resolver"
description: "O OneDrive mudou os teus Documentos e agora os saves falham, desaparecem ou aparecem a dobrar. Porque acontece, como resolver e uma forma melhor de sincronizar."
order: 15
updated: 2026-10-09
---

Em muitos PCs com Windows, o OneDrive faz cópia de segurança da pasta `Documentos`, muitas vezes ativada durante a instalação sem ninguém dar por isso. Os jogos que guardam em `Documentos`, ou seja, quase tudo em `My Games`, seguem-na para dentro do OneDrive. E aí começam os problemas: saves que não são escritos, saves que têm de ser descarregados antes de carregarem, e cópias com o nome do teu PC que o jogo nunca lê. Eis porque acontece e como resolver.

## Como os teus saves foram parar ao OneDrive

A cópia de segurança de pastas do OneDrive move `Documentos`, `Ambiente de Trabalho` e `Imagens` para `C:\Users\<tu>\OneDrive\...`. Os jogos perguntam ao Windows onde está `Documentos`, por isso seguem-na sem dizer nada. Para confirmar, clica com o botão direito em `Documentos` e abre **Propriedades → Localização**: se o caminho tiver `OneDrive`, os teus saves estão lá dentro.

A `AppData` e a `Saved Games` não fazem parte dessa cópia, por isso os jogos que guardam aí não são afetados.

## O que corre mal

- **Escritas que chocam.** O OneDrive envia os ficheiros assim que mudam. Um jogo que escreve o save no mesmo instante pode encontrar o ficheiro em uso.
- **Saves só online.** O OneDrive pode libertar espaço guardando ficheiros só na nuvem (o ícone da nuvem). O jogo tem então de descarregar o save antes de o carregar, e offline não há nada para carregar.
- **Cópias em conflito.** Usa o OneDrive em dois PCs com a mesma conta e os dois sincronizam o mesmo `My Games`. Joga nos dois antes de um apanhar o outro e o OneDrive guarda as duas versões, mudando o nome de uma para incluir o nome do PC. O jogo ignora esse ficheiro.
- **Espaço.** O plano gratuito tem 5 GB, e alguns jogos também põem mods, caches ou gravações em `Documentos`.
- **Nenhuma noção de sessão de jogo.** O OneDrive sincroniza ficheiro a ficheiro, a meio do jogo, e guarda versões de cada ficheiro em separado em vez do save inteiro.

## Como resolver

### Rápido: manter os saves no dispositivo

Clica com o botão direito em `Documentos\My Games` (ou na pasta do jogo) e escolhe **Manter sempre neste dispositivo**. Isso acaba com o problema dos ficheiros só online. Não impede o OneDrive de sincronizar enquanto jogas.

### Limpo: deixar de fazer cópia de segurança dos Documentos

No OneDrive, abre **Definições → Sincronização e cópia de segurança → Gerir cópia de segurança** e desativa `Documentos`. O Windows volta a apontar `Documentos` para a pasta local, mas os ficheiros que já foram copiados ficam na pasta do OneDrive. Antes de desativares, marca-os como **Manter sempre neste dispositivo** para que estejam mesmo no disco. Depois, com os jogos fechados, move as pastas dos jogos de volta para os `Documentos` locais, senão os jogos começam do zero.

## Uma divisão melhor

O OneDrive é bom com documentos. Os saves precisam de outra coisa: um backup feito quando o jogo já fechou, versões do save inteiro em vez de ficheiros soltos, e sincronização com os teus outros PCs e com um Steam Deck.

É isso que o Hoard faz. Encontra os teus saves quer `Documentos` esteja no OneDrive quer não, porque pergunta ao Windows onde a pasta está de facto. Faz backup deles automaticamente depois de cada sessão, guarda todas as versões e sincroniza-os com as tuas outras máquinas.

Uma regra: a sincronização entre PCs deve ficar a cargo de uma só ferramenta. Se o OneDrive faz cópia de `Documentos` em vários PCs de jogo com a mesma conta, já está a sincronizar esses saves entre eles. Desativa a cópia de `Documentos` nesses PCs e deixa os saves ao Hoard.

Preferes nada de nuvem? Corre o `hoard-server` no teu PC ou NAS: sem conta connosco, sem telemetria para nós, nada a passar pelos nossos servidores. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard).

<!-- faq -->

## Perguntas frequentes

### Devo deixar o OneDrive fazer cópia dos meus saves?

Num só PC, como cópia, é melhor do que nada. Como forma de sincronizar saves entre PCs cria conflitos, porque não sabe quando um jogo está aberto.

### Desativei a cópia e os meus saves desapareceram. Onde estão?

Continuam na pasta do OneDrive: `C:\Users\<tu>\OneDrive\Documents\My Games`. Fecha os jogos e move-os de volta para os `Documentos` locais.

### O que são os ficheiros de save com o nome do meu PC?

Cópias em conflito do OneDrive. Dois PCs alteraram o mesmo ficheiro antes de sincronizarem, e o OneDrive guardou os dois. Descobre qual é o mais recente, dá-lhe o nome original e põe o outro de lado.

### O OneDrive pode devolver-me um save mais antigo?

Às vezes. Em onedrive.com, clica com o botão direito no ficheiro e escolhe **Histórico de versões**. Funciona ficheiro a ficheiro e só durante um tempo limitado. Vê [como recuperar um save corrompido](/guides/recover-corrupted-game-save).

### O Hoard funciona se a minha pasta Documentos estiver no OneDrive?

Sim. O Hoard lê onde o Windows diz que está `Documentos`, por isso encontra os saves em qualquer dos dois sítios.
