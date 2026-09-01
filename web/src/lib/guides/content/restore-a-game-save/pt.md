---
title: "Como restaurar um save antigo"
description: "Tomaste uma má decisão, corrompeste um ficheiro ou queres recomeçar? Volta a qualquer versão anterior do teu save com o histórico na nuvem do Hoard — incluindo saves feitos com ferramentas como o Ludusavi."
order: 3
updated: 2026-09-01
---

Uma má decisão no jogo, um ficheiro corrompido ou um mod que parte tudo — às vezes só precisas de voltar atrás. Como o Hoard guarda um histórico completo de versões de cada save, restaurar um anterior leva segundos.

## Restaurar uma versão anterior

1. Abre o **Hoard** e vai ao jogo na tua **Biblioteca**.
2. Abre o separador **Histórico**. Verás cada backup com data e tamanho.
3. Escolhe a versão que queres e carrega em **Restaurar**.
4. O Hoard volta a escrever esse snapshot na pasta de save do jogo. O teu save atual é guardado primeiro, por isso a restauração é reversível.

## Restaurar num PC novo ou reinstalado

1. Instala o Hoard e inicia sessão com a tua conta.
2. Adiciona o jogo à Biblioteca — o Hoard encontra o backup na nuvem correspondente.
3. Restaura a versão mais recente, ou uma mais antiga, e continua a jogar.

Como o Hoard localiza as pastas de save com a mesma base de dados comunitária do Ludusavi, sabe onde colocar um save restaurado mesmo numa instalação limpa — sem procurares caminhos à mão.

## Quando um save fica corrompido ou uma mod o parte

Um jogo que rebenta ao carregar, uma mod que reescreveu o que não devia, um autosave que caiu a meio de uma escrita: a solução é a mesma. Abre o **Histórico** do jogo, escolhe a última versão anterior ao problema e restaura-a. As datas e os tamanhos costumam chegar para ver onde correu mal — uma queda súbita de tamanho é bom sinal de que um save ficou truncado.

Se não tens a certeza de qual é a boa, restaura a candidata mais provável e confirma dentro do jogo. Tentar de novo não custa nada, porque a versão que acabaste de substituir também ficou guardada.

## O que uma restauração faz mesmo

Três coisas que vale a pena saber, porque são as que tornam seguro experimentar:

1. **O teu save atual é capturado primeiro.** A restauração é reversível: o que substituíste passa a ser uma versão do histórico como outra qualquer.
2. **Só se descarrega o que falta.** Os ficheiros já em disco com o conteúdo certo são aproveitados tal como estão, por isso restaurar um save grande depois de uma pequena alteração move alguns megabytes e não a pasta inteira.
3. **Os ficheiros próprios desta máquina ficam intactos.** A configuração e os registos ao lado do save são copiados, mas não escritos por cima das tuas cópias locais: os teus controlos e as tuas definições gráficas sobrevivem a uma restauração vinda de outro PC.

## Restaurar sem passar pelos nossos servidores

Se corres o teu próprio `hoard-server`, as restaurações funcionam exatamente da mesma maneira, só que as versões vêm da tua máquina e não da nossa. Não há conta connosco, nem telemetria para nós, nem nada que passe pelos nossos servidores. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard).

## Dica

As restaurações nunca são destrutivas: o save que substituis é primeiro capturado como nova versão, por isso podes sempre desfazer uma restauração restaurando a entrada anterior. Se até agora só guardavas backups locais (por exemplo com o Ludusavi), passar para o Hoard acrescenta um histórico versionado fora da máquina, a partir do qual podes restaurar mesmo depois de uma falha de disco.

<!-- faq -->

## Perguntas frequentes

### Restaurar sobrescreve o meu progresso atual?

Só depois de o teu save atual ter sido capturado como uma nova versão. Se restaurares a errada, restaura a entrada anterior e ficas onde estavas.

### Até onde vai o histórico?

Até onde permitir o limite de versões do teu plano, e uma versão que fixes nunca é apagada para abrir espaço. Num servidor self-hosted o único limite é o teu disco.

### Posso restaurar num PC onde o jogo ainda não está instalado?

Instala primeiro o jogo para que a pasta de saves exista, e depois restaura. O Hoard sabe onde cada jogo espera os seus saves, por isso escreve o snapshot no sítio certo sem teres de procurar o caminho.

### Funciona entre Windows e uma Steam Deck?

Sim. O mesmo jogo guarda em sítios diferentes em cada um — na Deck, dentro do prefixo Proton — e o Hoard escreve a versão restaurada onde essa máquina a espera.

### Restaurar é diferente num servidor self-hosted?

Não. Mesma aplicação, mesmo histórico, mesma restauração num clique. Só o armazenamento é teu.
