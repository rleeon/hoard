---
title: "Alternativa ao OpenSave: direto entre máquinas ou com um servidor teu"
description: "O OpenSave sincroniza saves diretamente entre os teus PCs, sem nada pelo meio. O Hoard sincroniza através de um servidor — o nosso ou um teu — e guarda histórico versionado. Um olhar honesto sobre quando cada desenho ganha."
order: 8
updated: 2026-09-01
---

As duas ferramentas resolvem o mesmo problema e discordam quanto à arquitetura, que é a única coisa que vale a pena comparar. Esta página põe os dois desenhos lado a lado, incluindo os casos em que o outro é a melhor resposta.

## A diferença a sério: direto ou com servidor

**O OpenSave** é ponto a ponto. As tuas máquinas falam diretamente umas com as outras e no meio não há nada. Sem conta e sem armazenamento a pagar, e opcionalmente pode espelhar uma cópia para uma nuvem que já tenhas.

**O Hoard** sincroniza através de um servidor. Esse servidor é o Hoard Cloud, gerido por nós, ou o `hoard-server` a correr no teu PC ou no teu NAS. O teu save sobe quando paras de jogar e desce quando outra máquina o pede.

Tudo o resto sai dessa única escolha.

## O que um servidor te dá

- **A outra máquina não tem de estar ligada.** Acabas no fixo, o portátil fica fechado uma semana, e o save mais recente está à espera quando o abres. O ponto a ponto precisa das duas pontas acordadas ao mesmo tempo: perfeito numa secretária, chato com uma consola portátil que pegas duas vezes por mês.
- **Um histórico de versões, não só o último estado.** Cada sessão passa a ser uma versão à qual podes voltar. É a parte que conta no dia em que uma mod te come o mundo ou um save fica escrito a meio: uma sincronização direta copia fielmente o ficheiro partido para o outro PC.
- **Uma cópia que sobrevive ao hardware.** Os dois PCs morrerem na mesma casa não é um cenário exótico. Um save que só existiu nessas duas máquinas morre com elas.
- **Nada para preparar na rede.** Sem NAT para atravessar, sem porta para abrir, sem a condição de estarem os dois na mesma LAN.

## O que o ponto a ponto te dá

Sendo justos com o outro lado:

- **Nunca há armazenamento a pagar.** Não há quota para esgotar porque não há depósito. O plano gratuito do Hoard são 2 GB; acima disso pagas ou alojas tu.
- **Nada pelo meio, por desenho.** Se o objetivo é que um ficheiro nunca toque no disco de terceiros, a transferência direta é a resposta mais curta possível.
- **Nada para manter.** Nenhum servidor de pé, nem sequer o teu.

Se jogas em dois fixos ambos ligados, nunca queres voltar atrás e preferes não pensar em armazenamento, esse desenho encaixa bem e o Hoard é mais maquinaria do que precisas.

## A questão da privacidade, com precisão

É aqui que as comparações ao Hoard costumam falhar, por isso sejamos exatos: há duas formas de o usar e são genuinamente diferentes.

- **O Hoard Cloud** é a opção gerida: inicias sessão e os saves ficam nos nossos servidores, na UE.
- **O self-hosting é inteiramente teu.** Corres o `hoard-server` no teu PC ou NAS e os saves vão da tua máquina para o teu disco. **Não há conta connosco, nem telemetria para nós, nem quota, nem retransmissão**: não passa nada pelos nossos servidores, porque não há nada nosso no caminho. Não vemos um save, o nome de um jogo ou um email, porque nada disso nos chega. Se o Hoard Cloud fechasse amanhã, uma instalação self-hosted continuaria igual.

Ou seja, "servidor" não significa "o computador de outra pessoa" a não ser que o escolhas. Um Hoard self-hosted mantém os saves em hardware teu, tal como uma transferência direta, e ainda te dá o histórico e o caso da máquina desligada.

## Deteção e cobertura

Ambas as ferramentas encontram automaticamente os saves de um catálogo grande. O Hoard lê o mesmo manifesto comunitário de localizações que o ecossistema open source partilha, com mais de 20.000 títulos, e junta-lhe as bibliotecas da Steam, os processos em execução e uma varredura do disco. Nos jogos da Steam segue `<AppID>/remote/` dentro de `userdata` e não a pasta acima, porque a de cima guarda `remotecache.vdf` e ficheiros de proezas e tempo de jogo próprios de cada máquina: sincronizá-los é ver um conflito a cada arranque. Para o que for invulgar, apontas-lhe a pasta.

## Qual deves usar?

- **Ponto a ponto** se as tuas máquinas estão ligadas ao mesmo tempo, não queres armazenamento na equação e o último save é tudo o que alguma vez precisaste.
- **O Hoard** se queres um histórico ao qual voltar, uma máquina que possa estar desligada uma semana e uma cópia que sobreviva aos dois PCs — com a escolha entre a nossa nuvem e o teu próprio servidor.

Há uma [comparação de todas as ferramentas de sincronização](/guides/game-save-sync-comparison) para o panorama completo, e uma [comparação com o Ludusavi](/guides/ludusavi-alternative) para o lado das cópias locais.

<!-- faq -->

## Perguntas frequentes

### O Hoard precisa de conta?

Para o Hoard Cloud sim, é a isso que a sincronização está ligada. Em self-hosted não há conta nenhuma connosco: o teu servidor tem os seus utilizadores e um token por dispositivo, e não saem da tua máquina.

### O Hoard funciona sem nuvem nenhuma?

Sim. Corre o `hoard-server` num PC ou num NAS e os teus saves vão da tua máquina para o teu disco, sem nada a passar pelos nossos servidores.

### Os dois PCs têm de estar online ao mesmo tempo?

Não, e essa é a vantagem prática de sincronizar através de um servidor. O save sobe quando paras de jogar e desce quando a outra máquina o pedir.

### Uma transferência direta guarda histórico de versões?

Por si só não: copiar um ficheiro para outra máquina dá-te o estado atual nas duas. O Hoard captura cada sessão como uma versão, e é isso que torna possível voltar atrás a partir de um save corrompido.

### O Hoard também é open source?

Sim, AGPL-3.0, servidor incluído. O servidor self-hosted é o mesmo binário que nós corremos, não uma edição reduzida.
