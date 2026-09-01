---
title: "Alternativa ao Ludusavi: sincronização automática de saves na nuvem"
description: "Uma comparação justa entre o Ludusavi e o Hoard. O Ludusavi é uma excelente ferramenta open source de backup local; o Hoard acrescenta sincronização na nuvem gerida e histórico versionado em todos os teus PCs — usando os mesmos dados de localização."
order: 5
updated: 2026-09-01
---

Se procuras uma forma de fazer backup e sincronizar os teus saves, é provável que tenhas encontrado o **Ludusavi** — e é excelente. Este guia é uma comparação honesta para te ajudar a escolher a ferramenta certa, e explica onde o Hoard se encaixa se quiseres sincronização na nuvem automática entre máquinas.

## O que o Ludusavi faz bem

O Ludusavi é uma ferramenta gratuita e open source (criada por mtkennerly) para fazer backup e restaurar saves de PC em Windows, macOS e Linux. Tem uma GUI limpa e uma CLI, encontra automaticamente os saves de milhares de jogos, guarda backups locais versionados e pode enviar esses backups para uma nuvem tua configurando o **Rclone** (Google Drive, Dropbox e muitos outros). Se queres controlo total e uma configuração faz-tu-mesmo, o Ludusavi é uma escolha fantástica — e completamente gratuita.

O Hoard não vem substituir isso. Na verdade, **o Hoard usa a mesma base de dados comunitária de localizações em que o Ludusavi se apoia** para localizar onde cada jogo guarda os saves, por isso a qualidade da deteção está ao mesmo nível.

## Em que o Hoard é diferente

O ponto onde a maioria esbarra com qualquer ferramenta local é a **sincronização entre dispositivos**. Com o Ludusavi fá-lo tu: agendar um backup, configurar um remoto Rclone, e depois restaurar no outro PC antes de jogar. Funciona, mas é manual.

O Hoard transforma isso em **sincronização na nuvem gerida**:

- **Inicia sessão e pronto.** Sem remotos Rclone, sem scripts. O Hoard envia o teu save depois de jogares e descarrega a versão mais recente antes de começares, em cada PC da tua conta.
- **Histórico versionado na nuvem.** Cada backup é guardado, por isso podes voltar a qualquer save anterior — mesmo depois de uma falha de disco ou de uma instalação limpa.
- **Tem em conta os conflitos.** O Hoard compara os timestamps e guarda uma cópia local de tudo o que substitui, por isso uma sincronização nunca destrói progresso em silêncio.
- **Continua open source e self-hostable.** Como o Ludusavi, não há aprisionamento — usa o Hoard Cloud ou aloja o servidor tu mesmo.

## Frente a frente

| | Ludusavi | Hoard |
|---|---|---|
| Backups locais | Sim | Sim |
| Deteção de saves | Manifesto comunitário | O mesmo manifesto, mais bibliotecas Steam, processos em execução e uma varredura do disco |
| Armazenamento na nuvem | O teu, via Rclone | Incluído, ou o teu próprio servidor |
| Sincronização entre PCs | Manual: backup aqui, restauro ali | Automática, depois de jogares e antes de começares |
| Histórico de versões | Backups locais que limpas tu | Todas as versões na nuvem, deduplicadas por hash de conteúdo |
| Emuladores | Sim | Sim |
| Interfaces | App de ambiente de trabalho e CLI | App de ambiente de trabalho, CLI e overlay dentro do jogo |
| Preço | Gratuito | Plano gratuito de 2 GB e 3 dispositivos, Pro acima disso, sem qualquer quota em self-hosting |
| Licença | MIT | AGPL-3.0 |

## Quando o Ludusavi é a melhor escolha

Esta é a parte que quase nenhuma página de comparação inclui. O Ludusavi é a melhor ferramenta quando:

- **Só jogas num PC.** A sincronização na nuvem resolve um problema que não tens. Um backup local chega, e o Ludusavi faz backups locais muito bem.
- **Já tens um remoto Rclone em que confias.** Se o teu armazenamento está montado e a funcionar, a principal vantagem do Hoard é um passo de configuração que já pagaste.
- **Queres usá-lo a partir do modo de jogo de uma Steam Deck.** O Ludusavi tem um plugin Decky, por isso podes lançar um backup sem sair da interface de consola.
- **Queres uma licença permissiva.** O Ludusavi é MIT e o Hoard é AGPL-3.0. Se pensas construir algo por cima e não publicar o resultado, essa diferença pesa.
- **Não queres nada a correr em pano de fundo.** Alojar o Hoard tu mesmo implica manter um pequeno servidor de pé, mesmo que seja no próprio PC. O Ludusavi é uma aplicação que abres quando precisas.

## Passar do Ludusavi para o Hoard

Não há importador, e é de propósito. Os passos:

1. **Deixa os teus backups do Ludusavi exatamente onde estão.** Nada é migrado nem apagado. Guarda-os como rede de segurança nas primeiras semanas.
2. **Instala o Hoard e inicia sessão**, ou aponta-o ao teu próprio servidor.
3. **Deixa-o analisar.** Lê o mesmo manifesto, por isso a lista de jogos detetados deve ser-te familiar.
4. **Não apontes o Hoard para a pasta de backups do Ludusavi.** Segue a pasta onde o jogo escreve. Uma pasta de backups é uma cópia que muda por horário e não quando jogas, e sincronizar a cópia de uma cópia é como se acaba a restaurar o progresso de ontem. O Hoard tenta detetá-lo sozinho — `hoard doctor` assinala uma pasta seguida que parece um espelho de backups — mas é mais simples nunca a seguir.
5. **Joga uma vez.** Ao sair, a primeira versão aparece no histórico.
6. **Repete no segundo PC.** Inicias sessão e as versões já lá estão.

## Dois detalhes que vale a pena saber

**Os saves da Steam vivem uma pasta mais abaixo do que parece.** Nos jogos da Steam, o Hoard segue `<AppID>/remote/` dentro de `userdata`, não a pasta acima. A pasta acima guarda também `remotecache.vdf` e ficheiros de proezas e tempo de jogo, que são legitimamente diferentes em cada máquina. Se sincronizares a pasta acima, cada arranque parece um conflito mesmo sem nenhum save se ter mexido. É o motivo mais comum para uma montagem caseira entre Steam Deck e desktop acabar a lutar contra si própria.

**As versões são baratas.** Os snapshots são guardados por hash de conteúdo, por isso um ficheiro que não muda é guardado uma só vez. Dez versões de um save de 2 GB ocupam cerca de 2 GB, não 20 — e é isso que torna prático manter o histórico inteiro em vez de o ir cortando.

## O que self-hosting quer mesmo dizer

É o ponto em que quase todas as comparações se enganam sobre o Hoard, por isso convém ser exato. Há duas formas de o usar, e são genuinamente diferentes:

- **O Hoard Cloud** é a opção gerida: inicias sessão e os teus saves ficam nos nossos servidores, na UE.
- **O self-hosting é inteiramente teu.** Corres o `hoard-server` no teu PC ou no teu NAS, e os teus saves vão da tua máquina para o teu disco. **Não há conta connosco, nem telemetria para nós, nem quota, nem retransmissão**: não passa nada pelos nossos servidores, porque não há nada nosso no caminho. Não conseguimos ver um save, o nome de um jogo ou um endereço de email, pela simples razão de que nada disso nos chega. Se o Hoard Cloud desaparecesse amanhã, uma instalação self-hosted continuaria igual.

O mesmo programa, a mesma deteção, o mesmo histórico de versões. A única coisa que muda é de quem é o armazenamento.

## Qual deves escolher?

- Escolhe o **Ludusavi** se queres uma ferramenta de backup gratuita e local e não te importas de montar a tua própria nuvem com o Rclone.
- Escolhe o **Hoard** se queres que o backup *e* a sincronização entre PCs simplesmente funcionem, com um histórico na nuvem versionado, mantendo a opção de self-hosting.

Muita gente começa com o Ludusavi para backups locais e passa para o Hoard quando joga os mesmos jogos em mais de uma máquina. Se é o teu caso, vê [como sincronizar saves entre PCs](/guides/sync-game-saves-across-pcs) ou simplesmente [descarrega o Hoard](/download) e inicia sessão. Para o panorama completo, há uma [comparação de todas as ferramentas de sincronização](/guides/game-save-sync-comparison).

<!-- faq -->

## Perguntas frequentes

### Posso usar o Ludusavi e o Hoard ao mesmo tempo?

Sim. Leem as mesmas localizações e nenhum dos dois bloqueia os ficheiros. Muita gente mantém o Ludusavi para backups de arquivo locais e deixa o Hoard tratar da sincronização entre máquinas. A única regra é não apontar uma ferramenta para a pasta de backups da outra.

### O Hoard importa os meus backups do Ludusavi?

Não, e é deliberado. Uma pasta de backups é uma cópia que muda segundo o seu próprio horário, por isso segui-la sincronizaria um espelho desatualizado em vez do teu save real. O Hoard segue a pasta onde o jogo escreve e começa o seu próprio histórico a partir da tua sessão seguinte. Guarda o arquivo do Ludusavi como rede de segurança.

### O Hoard é gratuito?

O Hoard Cloud tem um plano gratuito com 2 GB de armazenamento e 3 dispositivos, o que cobre a maioria das coleções; o Pro sobe ambos. Alojar o servidor tu mesmo é gratuito e não tem quota nenhuma. Tudo é open source sob AGPL-3.0.

### O Hoard funciona na Steam Deck?

Sim, na Steam Deck e em qualquer ambiente de trabalho Linux, além de Windows e macOS. A Deck é exatamente o caso que precisa do detalhe do `remote/` acima, porque uma Deck e um desktop escrevem ficheiros de proezas e tempo de jogo diferentes ao lado do mesmo save.

### Preciso de Rclone ou de uma conta de nuvem minha?

Não. É essa a principal diferença prática: com o Hoard Cloud o armazenamento já está pronto quando inicias sessão. Se preferes ser dono do armazenamento, corre o servidor tu mesmo contra um bucket compatível com S3 ou uma pasta normal da tua máquina.

### O self-hosting envia alguma coisa para o Hoard?

Não. Em modo self-hosted não há conta connosco nem telemetria para nós: os teus saves, os teus utilizadores e os teus registos vivem no teu próprio servidor e nunca tocam no nosso. É esse o sentido do modo, e é por isso que o servidor é o mesmo binário open source que nós corremos e não uma versão reduzida.
