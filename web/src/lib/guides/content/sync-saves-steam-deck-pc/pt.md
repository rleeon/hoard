---
title: "Como sincronizar saves entre Steam Deck e PC"
description: "Sincroniza automaticamente os saves do Steam Deck e do PC, incluindo jogos fora do Steam, emuladores e jogos sem Steam Cloud. Passos, caminhos e armadilhas."
order: 10
updated: 2026-10-09
---

Nos jogos do Steam com Steam Cloud, o teu Deck e o teu PC já partilham os saves. Tudo o resto precisa de ajuda: jogos em que o programador nunca ativou o Steam Cloud, jogos da Epic e da GOG que abres com o Heroic, emuladores e qualquer coisa que tenhas adicionado como jogo fora do Steam. O Hoard trata de todos automaticamente. Quando fechas um jogo numa máquina, faz backup do save, e a outra máquina descarrega-o, com todas as versões anteriores guardadas para o caso de algo correr mal.

## O que o Steam Cloud já faz no Deck

Se um jogo suporta o Steam Cloud, o Steam envia o save quando sais e descarrega-o quando abres o jogo noutra máquina. Na página da loja vês se o jogo o tem, e podes desativá-lo jogo a jogo em **Propriedades → Geral**.

As falhas são as do costume:

- **Jogos que não o têm.** É o programador que decide, jogo a jogo, e muitos jogos de PC nunca o ativaram.
- **Tudo o que está fora do Steam.** Heroic, Lutris, emuladores, um jogo que instalaste à mão.
- **Não há volta atrás.** O Steam guarda o save atual, não um histórico. Se um save estragado for sincronizado, o bom desaparece nas duas máquinas.

Há mais sobre isto no guia [alternativa ao Steam Cloud](/guides/steam-cloud-alternative).

## Onde o Deck guarda os teus saves

O Deck corre os jogos de Windows através do Proton, por isso o mesmo jogo guarda num sítio diferente do teu PC:

- **Jogos de Windows (Proton):** `~/.local/share/Steam/steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/`, seguido do caminho habitual do Windows: `Documents`, `AppData/Roaming`, `AppData/Local`, `AppData/LocalLow` ou `Saved Games`. O AppID é o número que aparece no URL do jogo na loja.
- **Jogos no cartão microSD:** o cartão tem o seu próprio `steamapps/compatdata/<AppID>`, com a mesma árvore lá dentro.
- **Jogos nativos de Linux:** normalmente `~/.local/share/<jogo>` ou `~/.config/<jogo>`. Os jogos feitos em Unity usam `~/.config/unity3d/<Empresa>/<Jogo>`.
- **Heroic, Lutris e Bottles:** cada um mantém um prefixo do Wine por jogo, com a árvore do Windows em `drive_c/users/<o teu utilizador>/` em vez de `steamuser`.
- **Emuladores:** o EmuDeck junta-os em `~/Emulation/saves/`. Vê [saves de emuladores](/guides/back-up-emulator-saves) e [RetroArch](/guides/retroarch-save-sync).

No teu PC, o mesmo jogo escreve em `C:\Users\<tu>\...`. Dois caminhos diferentes para um só save: é por isso que copiar pastas à mão corre mal. O Hoard procura em todos estes sítios e associa o que encontra ao jogo certo, de modo que o save do Deck e o do PC passam a ser duas versões de um mesmo histórico.

## Configurar

1. No Deck, muda para o modo de ambiente de trabalho: **botão Steam → Energia → Mudar para o ambiente de trabalho**.
2. Abre um navegador, vai à [página de transferência](/download) e descarrega o **Hoard Setup** para Linux. No gestor de ficheiros, abre as propriedades do ficheiro, permite que corra como programa e abre-o.
3. Inicia sessão com a mesma conta que usas no PC, ou aponta a app para o teu próprio servidor.
4. Abre a **Biblioteca** e vê o que o Hoard encontrou. Acrescenta o que faltar apontando para a pasta: um prefixo do Heroic, um emulador, um jogo que instalaste tu.
5. Instala o Hoard no PC com a mesma conta. Os mesmos jogos associam-se sozinhos.
6. Volta ao modo de jogo. Não precisas de voltar ao ambiente de trabalho.

O Hoard Setup põe a app na tua pasta pessoal e o motor de sincronização num serviço em segundo plano que arranca com o Deck. Nada é escrito no sistema só de leitura do SteamOS, por isso as atualizações do sistema não lhe tocam.

## Como é um dia normal

Jogas no PC à noite e sais. O Hoard espera que o jogo feche e que o save deixe de mudar, e depois envia-o. Na manhã seguinte pegas no Deck. Assim que fica online, o Hoard vê a versão mais recente e escreve-a no prefixo do Proton. Abres o jogo e continuas. Quando sais no Deck, acontece o mesmo no sentido inverso.

Nenhuma das máquinas tem de estar ligada ao mesmo tempo que a outra. O save espera no servidor até a outra o pedir.

## As armadilhas que convém conhecer

### Suspender não é sair

O Deck torna muito fácil carregar no botão de ligar e deixar o jogo aberto. O Hoard só faz backup de um save depois de o jogo fechar, porque um jogo a correr pode estar a meio de o escrever. E nunca troca o save de um jogo aberto. Por isso, se suspendes o Deck e depois jogas no PC, o progresso do Deck ainda não foi enviado, e o save novo do PC fica à espera de que feches o jogo no Deck.

O hábito que evita tudo isto: **fecha o jogo antes de mudar de máquina.** Se o Proton deixar um processo morto para trás depois de saíres, o que acontece muitas vezes, o Hoard percebe que o jogo já não está lá e segue em frente.

### Dá-lhe uns segundos ao acordar

Quando o Deck acorda, o Wi-Fi demora um momento a voltar, e só então o Hoard pode procurar um save mais recente. Se abrires um jogo nesses primeiros segundos, a transferência espera até o fechares. Dá-lhe um momento online antes de começares a jogar.

### O cartão microSD

Se um jogo está no cartão e o cartão não está inserido, o Hoard não descarrega um save para uma pasta que não existe. Espera até o cartão voltar.

### As definições ficam em cada máquina

O Deck corre a 1280×800 numa GPU de consola portátil. O teu desktop, provavelmente não. O Hoard faz backup dos ficheiros de definições como `graphics.ini` junto com o save, mas não os escreve por cima dos da outra máquina, por isso o Deck mantém os seus. Se mesmo assim os quiseres copiar, há uma opção para isso ao restaurar. Mais em [sincronizar saves entre PCs](/guides/sync-game-saves-across-pcs).

### A pasta `remote` do Steam

Nos jogos do Steam, o save fica em `userdata/<UserID>/<AppID>/remote/`. A pasta acima também guarda `remotecache.vdf` e ficheiros de tempo de jogo e conquistas que devem ser diferentes no Deck e no PC. Sincroniza a pasta de cima à mão e cada arranque parece um conflito. O Hoard segue só `remote/`.

## Steam Cloud e Hoard ao mesmo tempo

Não se atrapalham. Num jogo com Steam Cloud, deixa o Steam continuar a sincronizá-lo. O que o Hoard acrescenta aí é o histórico de versões, para que um save estragado numa máquina não leve o teu progresso com ele. Em todos os outros jogos, o Hoard também trata da sincronização.

## Sem os nossos servidores

Se preferes que os saves não saiam de casa, corre o `hoard-server` no teu PC ou num NAS e aponta para ele tanto o Deck como o PC. Sem conta connosco, sem telemetria para nós, nada a passar pelos nossos servidores. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard).

<!-- faq -->

## Perguntas frequentes

### O Hoard funciona no modo de jogo?

Sim. O motor de sincronização é um serviço em segundo plano que arranca com o Deck, por isso faz backup e restaura sem nenhuma janela aberta. Só precisas do ambiente de trabalho para o instalar e para acrescentar pastas à mão.

### Uma atualização do SteamOS remove-o?

Não. Tudo o que o Hoard instala fica na tua pasta pessoal, e as atualizações do SteamOS não lhe tocam.

### Sincroniza jogos do Heroic, Lutris ou EmuDeck?

Sim. O Hoard procura dentro dos prefixos do Heroic, Lutris e Bottles e nas pastas do EmuDeck. Se um jogo não for detetado, aponta uma vez para a pasta de saves e fica seguido como qualquer outro.

### E se joguei nos dois sem sincronizar?

O Hoard nunca sobrescreve às cegas. Compara versões, guarda uma cópia do que substitui e todas as versões anteriores ficam no histórico. Não consegue juntar duas sessões de jogo diferentes num só save (nada consegue), mas podes escolher com qual ficas.

### O Deck conta como dispositivo?

Sim. O plano gratuito inclui três dispositivos, por isso cabem um PC, um portátil e um Deck. O Pro e os servidores alojados por ti não têm limite de dispositivos.

### Posso usar a versão de linha de comandos no Deck?

Sim. O comando `hoard` corre o mesmo motor sem janela, e há quem o prefira numa consola portátil. Vê [a página da CLI](/cli).
