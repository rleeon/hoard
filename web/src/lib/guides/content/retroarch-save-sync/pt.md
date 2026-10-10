---
title: "Sincronizar saves do RetroArch entre PC e Steam Deck"
description: "Sincroniza saves e estados do RetroArch entre PC, Steam Deck e portátil: onde ficam os .srm, Cloud Sync integrado ou sincronização automática, e as armadilhas."
order: 14
updated: 2026-10-09
---

O RetroArch guarda os saves do jogo como ficheiros `.srm` numa pasta `saves` e os estados numa pasta `states`. Para os sincronizar entre dispositivos podes usar o Cloud Sync que vem no RetroArch, com um servidor WebDAV que forneças tu, ou uma ferramenta que vigie as duas pastas. O Hoard faz a segunda coisa automaticamente: faz backup das duas pastas quando sais do RetroArch, descarrega-as nas outras máquinas, guarda todas as versões e percebe as instalações do EmuDeck.

## Onde o RetroArch guarda os saves

- **Windows:** `%APPDATA%\RetroArch\saves` e `\states`, ou `saves` e `states` ao lado de `retroarch.exe` se o instalaste numa pasta própria.
- **Linux:** `~/.config/retroarch/saves`. O Flatpak usa `~/.var/app/org.libretro.RetroArch/config/retroarch/saves`.
- **Steam Deck com EmuDeck:** `~/Emulation/saves/retroarch/`, onde `saves` e `states` são ligações para as pastas reais. O Hoard lê o `retroarch.cfg` para saber para onde apontam de facto.
- **RetroDECK:** `~/retrodeck/saves` e `~/retrodeck/states` por omissão.
- **Noutro sítio qualquer:** **Settings → Directory** mostra as pastas que o RetroArch está mesmo a usar.

## Quando é que o RetroArch escreve mesmo o save

Isto apanha muita gente. O RetroArch mantém o save do jogo em memória e só escreve o `.srm` quando fechas o jogo ou sais do RetroArch, a não ser que tenhas definido **Settings → Saving → SaveRAM Autosave Interval**. Até lá não há nada no disco: um crash ou uma bateria vazia levam tudo desde a última escrita, e nenhuma ferramenta de sincronização consegue mover um save que não foi escrito.

Define um intervalo de gravação automática de poucos segundos. E antes de mudares de dispositivo, **sai do RetroArch**, não só do jogo: o Hoard faz backup quando o RetroArch fecha, por isso nunca copia um save a meio. Num Deck, suspender não conta como sair.

## Configura todos os dispositivos da mesma forma

- **Opções de ordenação.** **Settings → Saving** pode separar saves e estados em subpastas por nome do núcleo ou por pasta de conteúdo. Se um dispositivo ordena e o outro não, o ficheiro sincronizado cai numa pasta onde o RetroArch não procura. Usa as mesmas definições em todos.
- **Nomes das ROMs.** O `.srm` tem o nome da ROM: `Super Metroid (USA).sfc` guarda em `Super Metroid (USA).srm`. Uma ROM com outro nome no outro dispositivo não o encontra.
- **O mesmo núcleo.** Dois núcleos da mesma consola nem sempre leem os saves um do outro. Escolhe um por sistema e usa-o em todo o lado.
- **Versões do núcleo, para os estados.** Um estado é uma fotografia da memória do núcleo e muitas vezes não carrega noutra versão. Os saves normais não têm esse problema.

Outra armadilha com os estados: um estado inclui a memória do jogo, save incluído. Carrega um estado antigo e a escrita seguinte do `.srm` traz de volta esse save antigo. Se usas **Auto Load State**, sincroniza também os estados, para que seja o mais recente a viajar.

## Cloud Sync do RetroArch ou Hoard?

Sendo justo com os dois:

- **O Cloud Sync do RetroArch** vem integrado e sincroniza saves e estados com um servidor WebDAV que geres ou alugas. Funciona também em Android e iOS, o que o Hoard hoje não faz. Se já usas o Nextcloud, que guarda versões dos ficheiros por conta própria, encaixa bem, e é a melhor escolha se o telemóvel faz parte da tua configuração.
- **O Hoard** não precisa de servidor WebDAV. Faz backup e sincroniza automaticamente, guarda um histórico de versões para onde voltar e cobre também os teus emuladores autónomos e os jogos de PC. Trata a pasta `saves` inteira como um só elemento, por isso voltar atrás restaura todos os jogos lá dentro tal como estavam nesse momento. Antes de confirmares, mostra o que vai mudar, e os teus ficheiros atuais são guardados primeiro.

Escolhe um por pasta. Duas ferramentas a escrever os mesmos saves é a receita para conflitos.

## Configurar com o Hoard

1. Instala o Hoard em cada dispositivo e inicia sessão com a mesma conta.
2. Na **Biblioteca**, acrescenta o RetroArch. Os saves e os estados aparecem como dois elementos.
3. Iguala as definições acima em todos os dispositivos.
4. Joga, sai do RetroArch e continua no outro dispositivo.

Preferes que fique em casa? Corre o `hoard-server` no teu PC ou NAS: sem conta connosco, sem telemetria para nós, nada a passar pelos nossos servidores. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard).

<!-- faq -->

## Perguntas frequentes

### O RetroArch tem saves na nuvem?

Sim, um Cloud Sync integrado que precisa de um servidor WebDAV. O Hoard é a alternativa se não quiseres manter um, quiseres um histórico de versões ou também usares emuladores autónomos.

### Porque é que o meu save do RetroArch não sincronizou?

Quase sempre por uma de três razões: o RetroArch ainda não tinha escrito o `.srm` (continuava aberto, sem intervalo de gravação automática), os dois dispositivos separam os saves em subpastas diferentes, ou as ROMs têm nomes diferentes.

### Também posso sincronizar os estados?

Sim. O Hoard segue `states` como um elemento próprio. Se um estado carrega no outro dispositivo depende de ambos usarem a mesma versão do núcleo.

### Funciona com o EmuDeck e o RetroDECK?

Sim. O Hoard lê a configuração do RetroArch para seguir as ligações do EmuDeck até às pastas reais. No RetroDECK, acrescenta `~/retrodeck/saves` e `~/retrodeck/states` se não aparecerem sozinhas.

### O Hoard sincroniza o RetroArch em Android?

Hoje não. O Hoard funciona em Windows, macOS, Linux e Steam Deck.
