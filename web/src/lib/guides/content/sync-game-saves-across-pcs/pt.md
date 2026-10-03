---
title: "Como sincronizar saves entre vários PCs"
description: "Joga no fixo, no portátil e na Steam Deck sem perder progresso: sincroniza os saves entre PCs automaticamente, com histórico. Passo a passo."
order: 2
updated: 2026-10-01
---

Se jogas em mais de um computador — um fixo em casa e um portátil em viagem — o Hoard mantém os teus saves sincronizados para que retomes sempre onde paraste.

## Como funciona a sincronização

O Hoard faz backup de cada save para a tua nuvem e descarrega a versão mais recente nas tuas outras máquinas. Quando acabas de jogar num PC, o save mais recente espera-te no seguinte.

## Configurar a sincronização

1. Instala o **Hoard** em cada PC onde jogas (Windows, macOS ou Linux).
2. Inicia sessão com a **mesma conta** em cada máquina, ou liga-as ao mesmo servidor self-hosted.
3. Adiciona os mesmos jogos à **Biblioteca** em cada PC. O Hoard associa-os por jogo, por isso um save feito num aparece nos outros.
4. Mantém o **modo automático** ligado. O Hoard envia depois de jogares e descarrega a versão mais recente antes de começares.

## Vens do Ludusavi?

O Ludusavi é uma excelente ferramenta open source para fazer backup e restaurar saves localmente, e pode enviar esses backups para uma nuvem que configuras tu mesmo com o Rclone. Mas a sincronização entre dispositivos montas tu à mão: agendar o backup, configurar o remoto, e depois restaurar no outro PC antes de jogar.

O Hoard transforma isso em sincronização gerida. Usa os mesmos dados comunitários de localização do Ludusavi para encontrar os teus saves, depois envia após cada sessão e descarrega a versão mais recente antes da seguinte — em cada PC da tua conta, com histórico versionado na nuvem. Sem remotos de Rclone, sem scripts. E como o Ludusavi, o Hoard é open source e pode ser self-hosted. Vê a [comparação completa com o Ludusavi](/guides/ludusavi-alternative).

## Evitar conflitos

O Hoard tem em conta os conflitos: compara as datas de modificação e guarda uma cópia local de qualquer save substituído, por isso uma sincronização nunca destrói progresso em silêncio. Se um jogo ainda estiver aberto ou um save foi tocado nos últimos minutos, o Hoard espera.

## Steam Deck e desktop

A montagem de duas máquinas mais comum é também a que mais se estraga quando é feita à mão, e quase sempre pelo mesmo motivo.

No Windows, o save de um jogo pode estar em `Documentos\My Games\…` ou dentro do `userdata` da Steam. Numa Steam Deck, esse mesmo jogo de Windows corre com Proton, por isso o save vive dentro de um prefixo de compatibilidade: `steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/…`. Dois caminhos muito diferentes, um só jogo, um só progresso. O Hoard lê os prefixos Proton além das localizações nativas e associa o que encontra por jogo, por isso o save da Deck e o do desktop passam a ser duas versões do mesmo histórico em vez de duas pastas sem relação.

O detalhe de que tudo depende: nos jogos da Steam, o Hoard segue `<AppID>/remote/` dentro de `userdata`, e **não** a pasta acima. A pasta acima guarda também `remotecache.vdf` e ficheiros de proezas e tempo de jogo próprios de cada máquina, que devem ser diferentes entre a tua Deck e o teu desktop. Se sincronizares a de cima, cada arranque parece um conflito mesmo sem nenhum save se ter mexido. É esse único erro que faz parecerem avariadas quase todas as montagens caseiras entre Deck e PC.

## Jogos que a Steam Cloud não cobre

Se todos os jogos que jogas suportassem Steam Cloud, não precisarias de nada disto. Na prática:

- **Jogos vindos de qualquer sítio que não a Steam.** GOG, Epic, itch, Battle.net, a app da Xbox e tudo o que instalaste à mão.
- **Jogos da Steam em que o programador nunca a ativou**, ou ativou só para uma plataforma.
- **Emuladores.** RetroArch, Dolphin, PCSX2, RPCS3 e companhia guardam onde lhes apetece, e a Steam não sabe nada disso.
- **Jogos que escrevem fora da pasta vigiada pela Steam**, e são mais do que se imagina.

Ao Hoard tanto lhe faz quem publicou o jogo ou de onde veio: segue a pasta que muda quando jogas.

## Quando dois PCs mexem no mesmo save

Jogas no portátil sem deixar o fixo acabar de sincronizar e tens o problema clássico: dois saves, ambos mais recentes do que a última versão comum.

O Hoard nunca escreve por cima às cegas. Compara datas de modificação, guarda uma cópia local do que substitui, e espera enquanto houver um jogo aberto ou o save tiver sido tocado nos últimos minutos: um ficheiro a ser escrito não é um ficheiro que queiras enviar a meio. Todas as versões anteriores ficam no histórico da nuvem, por isso enganares-te na versão custa dois cliques e não um fim de semana.

O limite honesto: **o Hoard não funde dois saves divergentes.** Nenhuma ferramenta o consegue — um ficheiro de save é opaco, e não há forma correta de misturar duas tardes de jogo diferentes. O que tens em troca é todas as versões, em todas as máquinas, e a possibilidade de escolher.

## As definições sincronizam entre PCs?

Os saves sim. As definições, por predefinição, não, e é de propósito. A Steam Deck mostra porquê: corre a 1280×800 com uma GPU de portátil, enquanto o teu PC talvez corra a 4K em algo muito maior. Copia o `graphics.ini` do PC para a Deck e o jogo arranca numa resolução que o ecrã não consegue mostrar, com definições que o hardware não aguenta.

Por isso o Hoard classifica o que encontra numa pasta de saves:

- **Os saves** sincronizam entre máquinas, em cada sessão.
- **Os ficheiros de definições** (`graphics.ini`, `settings.cfg` e afins) ficam em cada backup, por isso nunca se perdem, mas um restauro não os escreve por cima dos de outra máquina. Se a entrada do jogo na base de dados de saves disser que um `.ini` *é* o save, é tratado como save.
- **O lixo**, como crash dumps, ficheiros temporários, logs do motor tipo `Player.log` e a contabilidade própria do Steam, nem sequer entra no backup, para que abrir um jogo não crie por si só uma versão nova.

Quando quiseres mesmo que as definições viajem, marca **Restaurar também os ficheiros de definições** ao restaurar uma versão a partir do histórico, ou configura o jogo com **Restaurar também as definições** para que cada restauro as traga, incluindo os automáticos. Útil quando os dois PCs têm o mesmo ecrã e já afinaste o jogo uma vez.

## Sincronizar sem passar pelos nossos servidores

Vale a pena dizê-lo de forma explícita, porque é o ponto em que quase todas as comparações se enganam. Há duas formas de o usar:

- **O Hoard Cloud** é a opção gerida: inicias sessão e os teus saves ficam nos nossos servidores, na UE.
- **O self-hosting é inteiramente teu.** Corres o `hoard-server` no teu PC ou no teu NAS e as tuas máquinas sincronizam através dele. **Não há conta connosco, nem telemetria para nós, nem quota, nem retransmissão**: não passa nada pelos nossos servidores, porque não há nada nosso no caminho. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard).

O mesmo programa, a mesma deteção, o mesmo histórico de versões. A única coisa que muda é de quem é o armazenamento.

## Dica

Dá a cada máquina um momento para terminar a sincronização antes de abrires um jogo — o painel mostra o estado em tempo real, por isso sabes que o save mais recente já está no sítio.

<!-- faq -->

## Perguntas frequentes

### Quantos PCs posso sincronizar?

Três no plano gratuito, ilimitados no Pro e ilimitados em self-hosting: o teu servidor, as tuas regras.

### As duas máquinas têm de estar ligadas ao mesmo tempo?

Não. O teu save sobe para o servidor quando acabas de jogar e desce quando a outra máquina o pede, por isso o segundo PC pode estar desligado uma semana e mesmo assim receber a versão mais recente ao ligar.

### E se jogar sem ligação?

Sem problema. O snapshot é tirado localmente quando páras de jogar, e sobe sozinho assim que a máquina volta a ter ligação.

### Também sincroniza mods e definições?

Os saves, sim. Os ficheiros que pertencem a uma máquina em concreto — configuração, registos e afins — são enviados para ficarem no backup, mas não são escritos por cima da cópia de outro PC: uma definição gráfica que serve ao teu fixo raramente é a que o teu portátil quer.

### O self-hosting envia alguma coisa para o Hoard?

Não. Em modo self-hosted não há conta connosco nem telemetria para nós: os teus saves, os teus utilizadores e os teus registos vivem no teu próprio servidor e nunca tocam no nosso.

### Posso copiar as minhas definições para o outro PC mesmo assim?

Sim. Marca **Restaurar também os ficheiros de definições** ao restaurar uma versão, ou configura o jogo com **Restaurar também as definições** para as trazer em cada restauro. Na linha de comandos, `hoard restore --allow-ini` faz o mesmo.
