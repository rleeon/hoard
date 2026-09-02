---
title: "Syncthing para saves de jogos: o que funciona e o que parte"
description: "O Syncthing é um excelente sincronizador de ficheiros genérico, mas os saves de jogos partem três dos seus pressupostos. O que corre mal, como as pessoas contornam, e quando compensa uma ferramenta que sabe o que é um save."
order: 9
updated: 2026-09-01
---

O Syncthing é a resposta a que muita gente chega primeiro, e com razão: é gratuito, open source, ponto a ponto, e funciona. Mas os saves de jogos partem três dos pressupostos em que assenta um sincronizador genérico, e as falhas são silenciosas. Este guia é sobre o que corre mal a sério, e sobre quando vale a pena usar algo que saiba o que é um save.

## Porque se acaba aí

É software genuinamente bom. Sem conta, sem subscrição, os teus ficheiros nunca ficam no disco de uma empresa, e sincroniza qualquer coisa: documentos, fotos, uma pasta de saves. Se já o tens a correr para outras coisas, apontá-lo a uma pasta de saves custa-te trinta segundos. É um argumento real, e para certas montagens é o correto.

## As três coisas que partem

**Sincroniza com o jogo aberto.** O Syncthing reage à alteração de um ficheiro, que é o comportamento certo para um documento. Um jogo escreve o save a meio da sessão, às vezes em várias passagens, e um ficheiro apanhado a meio da escrita propaga-se incompleto. A outra máquina fica com um save que o jogo pode recusar carregar.

**Os conflitos tornam-se ficheiros, não decisões.** Quando ambas as máquinas mudam o mesmo save, o Syncthing faz o seguro e guarda os dois, renomeando um para `algo.sync-conflict-20260901-143022-ABCDEFG.sav`. Não se perde nada, mas o jogo não sabe o que é esse ficheiro, e ficas a comparar datas num explorador para decidir que tarde de jogo manténs. Repete umas quantas vezes e a pasta enche-se de ficheiros de conflito que ninguém se atreve a apagar.

**O versionamento é por ficheiro, não por sessão.** O Syncthing pode guardar cópias antigas em `.stversions`, e é melhor do que nada. Mas um save é muitas vezes vários ficheiros que só fazem sentido juntos, e restaurar significa encontrar à mão a data certa de cada um. Não existe um "põe este jogo como estava na terça".

E um quarto ponto, específico da Steam: se o apontares a `userdata/<UserID>/<AppID>/` em vez da pasta `remote/` lá dentro, também estás a sincronizar `remotecache.vdf` e ficheiros de proezas e tempo de jogo que **devem** ser diferentes entre máquinas. A partir daí cada arranque parece um conflito mesmo sem nenhum save se ter mexido. É o motivo mais comum para uma montagem caseira entre Steam Deck e desktop parecer avariada.

## O que acabas por construir

Nada disto é insolúvel. As pessoas safam-se com padrões de exclusão por jogo, uma política de versionamento, e o hábito de fechar o jogo e esperar antes de tocar no outro PC. Funciona, e é manutenção que passa a ser tua para sempre: um jogo novo são caminhos novos, e o dia em que te esqueces de esperar é o dia em que dás por isso.

## O que faz em vez disso uma ferramenta que percebe de saves

O Hoard captura **quando paras de jogar**, assim que a pasta fica quieta, por isso um snapshot nunca é um ficheiro escrito a meio. Cada captura é uma versão do save inteiro, e não de ficheiros soltos, por isso restaurar é um clique e devolve tudo junto. Sabe que pasta é de que jogo — lê o mesmo manifesto comunitário de localizações que o ecossistema open source partilha, com mais de 20.000 títulos — por isso não há caminhos para manter, e segue `<AppID>/remote/` em vez da pasta acima.

## Quando o Syncthing é a melhor resposta

Sendo justos:

- **Já o tens a correr**, e acrescentar uma pasta sai grátis.
- **Queres ponto a ponto sem servidor nenhum**, nem sequer o teu.
- **Sincronizas muito mais do que saves** e preferes uma só ferramenta para tudo.
- **Nunca voltas atrás.** Se o último save é tudo o que alguma vez precisaste, um histórico de versões é maquinaria que não vais usar.

## Usar os dois

Convivem sem se atropelar, e é uma montagem razoável: o sincronizador genérico trata dos teus documentos e do resto, e das pastas de saves trata uma ferramenta que as perceba. A única regra é não apontar os dois à mesma pasta — dois programas a escrever os mesmos ficheiros é a forma de fabricar precisamente os conflitos que querias evitar.

## Sem os nossos servidores também

Se parte do apelo é que nada toque no disco de uma empresa, o Hoard pode ser usado da mesma forma: `hoard-server` no teu PC ou NAS, e os teus saves vão da tua máquina para o teu disco. **Não há conta connosco, nem telemetria para nós, nem retransmissão**: não passa nada pelos nossos servidores, porque não há nada nosso no caminho. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard).

O mesmo binário, a mesma deteção, o mesmo histórico. A única coisa que muda é de quem é o armazenamento. Há também uma [comparação completa de todas as ferramentas de sincronização](/guides/game-save-sync-comparison).

<!-- faq -->

## Perguntas frequentes

### O Syncthing consegue sincronizar saves de jogos?

Consegue, e em casos simples fá-lo bem. Os problemas começam com jogos que escrevem enquanto jogas, saves feitos de vários ficheiros, e qualquer montagem em que as duas máquinas sejam editadas entre sincronizações.

### O que são os ficheiros .sync-conflict na minha pasta de saves?

É o sincronizador a guardar as duas versões depois de um conflito, em vez de escolher uma. Não se perde nada, mas o jogo não os consegue ler, e decidir qual ficar é trabalho manual de cada vez.

### Porque é que o meu save da Steam dá conflito a cada arranque?

Quase sempre porque a pasta sincronizada é a que está acima de `remote/`. Contém `remotecache.vdf` e ficheiros de proezas e tempo de jogo que são legitimamente diferentes em cada máquina, por isso as duas pontas nunca coincidem.

### Tenho de fechar o jogo antes de sincronizar?

Com um sincronizador genérico, sim: é esse o hábito que evita saves escritos a meio. Uma ferramenta que percebe de saves espera sozinha que a pasta fique quieta.

### Posso continuar a usar os dois?

Sim. Só não apontes os dois à mesma pasta, ou vão andar à luta pelos mesmos ficheiros.
