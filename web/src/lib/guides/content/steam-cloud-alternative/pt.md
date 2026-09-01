---
title: "Alternativa à Steam Cloud: guarda os saves que a Steam não guarda"
description: "A Steam Cloud só cobre jogos da Steam cujo programador a ativou, e não guarda histórico. O Hoard copia todos os jogos a que jogas, venham de onde vierem, com um histórico versionado a que podes voltar — na nuvem ou no teu próprio servidor."
order: 7
updated: 2026-09-01
---

A Steam Cloud faz muito bem o trabalho estreito que faz, e a maioria das pessoas só lhe descobre os limites no dia em que perde alguma coisa. Este guia explica onde estão esses limites e o que fazer com os jogos que ficam de fora.

## O que a Steam Cloud cobre mesmo

A Steam Cloud sincroniza a pasta de um jogo quando **o programador a configurou**: ou declarando que ficheiros sincronizar, ou chamando a API da Steam de dentro do jogo. É todo o modelo, e daí saem três consequências:

- Só funciona com jogos comprados e lançados pela Steam.
- Se funciona ou não é decisão do programador, jogo a jogo, e às vezes por plataforma.
- Cada jogo tem a sua própria quota de espaço, definida por esse programador.

Quando funciona é invisível e excelente: fechas o jogo num PC, abres noutro, e o progresso está lá.

## Onde te deixa exposto

- **Tudo o que não seja um jogo da Steam.** GOG, Epic, itch, Battle.net, a app da Xbox, emuladores, tudo o que instalaste à mão. A Steam nem sabe que existem.
- **Jogos da Steam onde nunca foi ativada.** Muitos títulos, sobretudo antigos ou pequenos, simplesmente não a têm. A página da loja di-lo, mas ninguém verifica antes de começar uma campanha de 60 horas.
- **Não há volta atrás.** É o ponto grande. A Steam guarda o estado atual do save, não o seu histórico. Se o ficheiro se corrompe, se uma mod te come o mundo, ou se escreves por cima de um save bom com um mau, a cópia na nuvem já é a má. Podes ver os ficheiros que a Steam guarda de um jogo, mas não há versão anterior para restaurar.
- **A janela de conflito.** Quando a Steam acha que o local e o remoto divergem, pede-te para escolher com pouco mais do que duas datas à frente. Escolhes mal e a outra cópia desapareceu.

## O que o Hoard acrescenta

O Hoard vigia a pasta onde o jogo realmente escreve e captura uma **versão nova sempre que acabas de jogar**:

- **Não lhe interessa de onde veio o jogo.** Steam, GOG, Epic, itch, emuladores ou uma pasta que lhe apontes à mão.
- **Todas as versões ficam guardadas**, por isso recuperar de um save corrompido ou de uma má decisão são dois cliques e não uma campanha perdida.
- **Sincroniza entre as tuas máquinas** da mesma forma, Steam Deck e desktop incluídos.
- **Nada é destruído em silêncio.** O save substituído é capturado primeiro, por isso até uma restauração errada é reversível.

Os snapshots são guardados por hash de conteúdo, por isso dez versões de um save de 2 GB ocupam cerca de 2 GB e não 20 — é isso que torna prático manter o histórico inteiro.

## Usar os dois ao mesmo tempo

Não se atropelam, e não tens de escolher. Num jogo da Steam com suporte de nuvem, deixa a Steam sincronizar o que já sincroniza; o que o Hoard acrescenta aí é o histórico, exatamente aquilo que a Steam não guarda. Para tudo o resto, é o Hoard que também trata da sincronização.

Um detalhe que conta se tens uma Steam Deck além do fixo: o Hoard segue `<AppID>/remote/` dentro de `userdata`, e não a pasta acima, porque a de cima guarda `remotecache.vdf` e ficheiros de proezas e tempo de jogo próprios de cada máquina. É a distinção que uma sincronização caseira falha com mais frequência, e a razão pela qual essas montagens parecem entrar em conflito a cada arranque.

## Quando a Steam Cloud chega

Convém dizê-lo com clareza: se todos os jogos a que jogas são da Steam e com suporte de nuvem, jogas num só PC e nunca precisaste de desfazer um save, a Steam Cloud já faz o trabalho e não precisas de mais nada. O que justifica juntar o Hoard é o histórico de versões, os jogos de fora da Steam e as máquinas onde a Steam Cloud não chega.

## Sem a nuvem de ninguém

Se o que te atrai é não depender de plataforma nenhuma, o Hoard pode correr inteiramente no teu hardware: `hoard-server` num PC ou num NAS, e os teus saves vão da tua máquina para o teu disco. **Não há conta connosco, nem telemetria para nós, nem retransmissão**: não passa nada pelos nossos servidores, porque não há nada nosso no caminho. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard).

O mesmo programa, a mesma deteção, o mesmo histórico. A única coisa que muda é de quem é o armazenamento.

<!-- faq -->

## Perguntas frequentes

### O Hoard substitui a Steam Cloud?

Não tem de substituir. A Steam Cloud mantém o teu save atual sincronizado nos jogos que a suportam; o Hoard acrescenta o histórico de versões e cobre os jogos que não. Ter os dois é o normal.

### A Steam Cloud consegue voltar a um save mais antigo?

Não. A Steam guarda o estado atual dos ficheiros, não o histórico deles. Assim que um save mau sincroniza, é esse que está na nuvem. Para voltar atrás é preciso uma ferramenta com versões.

### Porque é que nem todos os meus jogos da Steam sincronizam?

Porque quem a ativa é o programador, jogo a jogo e às vezes por plataforma. A página do jogo na loja indica a Steam Cloud entre as funcionalidades quando é suportada — e muitos títulos simplesmente não são.

### O Hoard funciona com jogos que não são da Steam?

Sim, e é boa parte do sentido. Localiza os saves através de uma base de dados comunitária que cobre mais de 20.000 títulos, de qualquer loja, e para o que for invulgar podes apontar-lhe a pasta à mão.

### Ter os dois a correr provoca conflitos?

Não. O Hoard captura uma versão depois de parares e de a pasta ficar quieta, e nunca escreve por cima sem capturar primeiro aquilo que substitui.

### Posso manter os meus saves fora das duas nuvens?

Sim. Aloja o servidor tu mesmo e os teus saves nunca saem de hardware teu, sem conta e sem telemetria para lado nenhum.
