---
title: "Save corrompido? Como o recuperar"
description: "Um save que não carrega nem sempre está perdido. Onde encontrar uma cópia boa (backups do jogo, Steam Cloud, Windows, OneDrive) e como não voltar a perdê-lo."
order: 12
updated: 2026-10-09
---

Um save que não carrega raramente está perdido de vez. Na maior parte das vezes existe uma cópia boa algures: um backup feito pelo próprio jogo, a cópia do Steam Cloud, uma versão anterior guardada pelo Windows ou pelo OneDrive, ou o save noutro PC. Mas a ordem importa, porque um passo errado pode escrever a cópia estragada por cima da boa. Começa por aqui.

## Primeiro: para e copia a pasta

1. **Fecha o jogo** e não comeces um jogo novo nesse espaço. Cada gravação a partir de agora pode empurrar uma cópia mais antiga para fora.
2. **Copia a pasta de saves inteira** para o ambiente de trabalho ou para uma pen USB. Assim, tudo o que tentares a seguir pode ser desfeito. Se não sabes onde fica a pasta, vê [onde os jogos de PC guardam os saves](/guides/where-are-pc-game-saves-stored).
3. **Põe em pausa tudo o que sincroniza essa pasta.** Steam Cloud (jogo a jogo, em **Propriedades → Geral**), OneDrive, Syncthing. Caso contrário, o ficheiro estragado pode chegar ao único sítio que ainda tem uma cópia boa.

## Confirma que está mesmo corrompido

Há coisas que parecem corrupção e não são:

- **O jogo foi atualizado** e os saves antigos não carregam, ou precisam de um patch. Vê as notícias ou o fórum do jogo.
- **Faltam mods.** Os jogos da Bethesda, sobretudo, avisam quando faltam plugins e podem recusar um save que os usava. Reinstala os mods primeiro.
- **Estás noutra conta.** Alguns jogos arquivam os saves sob o ID da tua conta do Steam ou da Ubisoft, por isso outra conta vê o espaço vazio.
- **O ficheiro só está online.** Com o OneDrive, um save com o ícone da nuvem foi tirado do disco para libertar espaço. Clica com o botão direito e escolhe **Manter sempre neste dispositivo**.

Um save com **0 KB**, ou muito mais pequeno do que os do lado, está mesmo estragado: a escrita foi cortada a meio.

## Onde pode estar uma cópia boa

Vai por ordem. As primeiras são mais rápidas e têm mais hipóteses de resultar.

### 1. Os backups do próprio jogo

Muitos jogos guardam uma cópia de reserva sem avisar. Procura na pasta de saves ficheiros que acabem em `.bak`, `_old` ou `.backup`, e espaços de gravação automática extra. Alguns casos conhecidos:

- **Elden Ring** escreve `ER0000.sl2.bak` ao lado do save.
- **Stardew Valley** guarda uma cópia `_old` de cada quinta, que é o dia anterior do jogo.
- **Terraria** guarda ficheiros `.bak` de personagens e mundos.
- **Minecraft Java** guarda `level.dat_old` dentro de cada mundo.

Para usar um, põe de lado o ficheiro estragado (já copiaste a pasta) e muda o nome do backup para o nome original.

### 2. Steam Cloud

O Steam Cloud guarda a cópia **mais recente**, não um histórico. Só ajuda se o save se estragou depois do último envio, por exemplo porque o jogo foi abaixo e o ficheiro mau nunca chegou a ser sincronizado. Podes ver e descarregar o que o Steam guarda de cada jogo em [store.steampowered.com/account/remotestorage](https://store.steampowered.com/account/remotestorage) e voltar a pôr o ficheiro no sítio à mão.

### 3. Versões anteriores do Windows

Clica com o botão direito na pasta de saves e abre **Propriedades → Versões anteriores**. Se o Histórico de Ficheiros ou a Proteção do Sistema estavam ativos nessa unidade, aparecem aqui cópias antigas da pasta, e podes abri-las para tirar os ficheiros de que precisas. Se a lista estiver vazia, nenhum dos dois estava ativo.

### 4. O histórico de versões do OneDrive

Se o OneDrive faz cópia de segurança da tua pasta `Documentos`, muitos saves estão lá sem tu saberes. Em onedrive.com, clica com o botão direito no ficheiro do save e escolhe **Histórico de versões** para descarregar uma versão anterior. O OneDrive guarda-as durante um tempo limitado, e os ficheiros apagados também passam algum tempo na reciclagem dele.

### 5. As tuas outras máquinas

Jogaste há pouco num portátil ou num Steam Deck? A cópia dele pode ser anterior ao problema. Copia-a antes que essa máquina sincronize a estragada.

### 6. Se o ficheiro foi apagado, e não estragado

Vê primeiro a Reciclagem. Depois, uma ferramenta de recuperação de ficheiros pode encontrá-lo, desde que deixes de escrever nessa unidade. Cada instalação e cada transferência reduzem as hipóteses.

## Quando não aparece nada

Para alguns jogos populares, a comunidade tem editores de saves ou ferramentas de reparação capazes de reconstruir um ficheiro danificado: procura o nome do jogo com "save repair". Caso contrário, a resposta honesta é que a única cópia que conta é a que foi feita antes do problema.

## Porque é que os saves se estragam

- **Um crash ou um corte de luz a meio da escrita.** O jogo estava a gravar quando morreu.
- **O disco cheio.** O jogo não conseguiu acabar de escrever e deixou um ficheiro truncado.
- **Uma ferramenta de sincronização copiou-o a meio da escrita**, ou dois PCs editaram o mesmo save e uma das cópias ganhou.
- **Um mod** escreveu algo que o jogo não consegue voltar a ler.
- **Um disco a falhar**, o que costuma notar-se também noutros ficheiros.

## Nunca mais ficar sem save

Todas as recuperações acima dependem da sorte: que o jogo tenha feito um backup, ou que o Steam ainda não tenha sincronizado. Um backup com versões tira a sorte da equação. É o que faz o Hoard: faz backup de cada save automaticamente depois de deixares de jogar, assim que a pasta fica parada, por isso um backup nunca é um ficheiro a meio. Todas as versões ficam guardadas. Quando algo se estraga, abres o **Histórico** do jogo e restauras a última boa com um clique; o teu save atual é guardado primeiro, por isso até isso se pode desfazer. E como o Hoard também mantém os saves sincronizados, a cópia no portátil ou na Steam Deck nunca é uma cópia antiga e esquecida: todas as tuas máquinas trabalham com o mesmo histórico.

Uma dica para perceber quando se estragou: uma queda brusca de tamanho entre duas versões costuma indicar um save truncado. Mais em [como restaurar um save antigo](/guides/restore-a-game-save).

Se preferes que os backups fiquem em casa, corre o `hoard-server` no teu PC ou NAS. Sem conta connosco, sem telemetria para nós, nada a passar pelos nossos servidores. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard).

<!-- faq -->

## Perguntas frequentes

### Um save corrompido pode ser reparado?

Raramente no próprio ficheiro. Alguns jogos têm ferramentas de reparação da comunidade, mas quase sempre recuperar significa encontrar uma cópia mais antiga: o backup do jogo, o Steam Cloud, o Windows ou o OneDrive, ou outro PC.

### O Steam Cloud guarda versões antigas dos meus saves?

Não. Só guarda o ficheiro atual. Se um save estragado já foi enviado, o Steam Cloud também tem o estragado.

### Verificar os ficheiros do jogo repara um save corrompido?

Não. A verificação compara os ficheiros do jogo com os do Steam, não os teus saves. Ajuda se o próprio jogo estiver danificado, mas não te devolve o progresso.

### Porque é que o meu save tem 0 KB?

O jogo começou a escrevê-lo e nunca acabou: um crash, um corte de luz ou o disco cheio. Procura ao lado um ficheiro `.bak` ou `_old`, ou uma versão anterior noutro sítio.

### Como evito que volte a acontecer?

Com backups com versões feitos quando o jogo não está aberto. O Hoard faz isso automaticamente depois de cada sessão e guarda cada versão, por isso podes voltar a qualquer uma.
