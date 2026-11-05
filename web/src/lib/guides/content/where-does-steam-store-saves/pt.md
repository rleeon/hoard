---
title: "Onde ficam os saves da Steam? Todas as localizações no PC e na Steam Deck"
description: "O Steam tem uma pasta userdata, mas a maioria dos jogos grava em AppData, Documentos ou na própria pasta. Onde procurar no Windows, na Steam Deck e no Mac."
order: 30
updated: 2026-11-05
---

O Steam tem uma pasta própria para saves, `userdata\<account-id>\<AppID>\remote` dentro da pasta do Steam, mas a maioria dos jogos nunca a usa. Na base de dados comunitária de saves, que conhece o caminho do Windows de 10 937 jogos do Steam, só **1 em cada 10** grava lá. Os restantes escrevem em AppData, em Documentos ou na própria pasta de instalação, e o Steam Cloud, quando o jogo o tem, vai buscar os ficheiros onde quer que estejam.

## A pasta do Steam: userdata

- **Windows:** `C:\Program Files (x86)\Steam\userdata\<account-id>\<AppID>\remote`
- **Steam Deck e Linux:** `~/.local/share/Steam/userdata/<account-id>/<AppID>/remote`
- **macOS:** `~/Library/Application Support/Steam/userdata/<account-id>/<AppID>/remote`

`<account-id>` é o número que o Steam mostra como o teu **código de amigo** (Amigos, depois Adicionar um amigo). Há uma pasta por cada conta que iniciou sessão nessa máquina. `<AppID>` é o número no endereço do jogo na loja: `store.steampowered.com/app/413150` é o Stardew Valley.

A pasta acima de `remote`, a do `<AppID>` sozinho, guarda também `remotecache.vdf`. É a contabilidade do próprio Steam, não o teu save, e é normal que seja diferente no PC e na Deck.

## Onde a maioria dos jogos grava de facto

Contado nessa mesma base de dados a 5 de novembro de 2026, a percentagem de jogos do Steam que usa cada sítio (alguns usam mais do que um):

- **`AppData\LocalLow`**, 23 %, sobretudo jogos feitos com Unity: `%USERPROFILE%\AppData\LocalLow\<programador>\<jogo>`
- **A pasta de instalação do jogo**, 22 %: `C:\Program Files (x86)\Steam\steamapps\common\<jogo>`
- **`AppData\Local`**, 21 %: `%LOCALAPPDATA%\<programador ou jogo>`
- **`Documentos`**, 15 %, muitas vezes dentro de `My Games`: `%USERPROFILE%\Documents\My Games\<jogo>`
- **`AppData\Roaming`**, 12 %: `%APPDATA%\<jogo>`
- **A pasta `userdata` do Steam**, 10 %: vê acima
- **`Jogos Guardados`**, 3 %: `%USERPROFILE%\Saved Games\<programador>\<jogo>`

## Na Steam Deck: o prefixo do Proton

Os jogos de Windows correm na Deck através do Proton, e cada um tem o seu pequeno Windows dentro de `~/.local/share/Steam/steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/`. AppData, Documentos e Jogos Guardados vivem todos lá dentro, por isso `%APPDATA%\Jogo` num PC passa a ser `compatdata/<AppID>/pfx/drive_c/users/steamuser/AppData/Roaming/Jogo` na Deck. O prefixo aparece da primeira vez que o jogo arranca.

Os jogos com versão nativa para Linux saltam o prefixo e costumam gravar em `~/.local/share` ou `~/.config`. A pasta de instalação é `~/.local/share/Steam/steamapps/common/<jogo>`, ou a mesma árvore `steamapps` no cartão microSD se o jogo estiver instalado lá.

## Encontrar depressa os saves de um jogo

1. Procura o jogo na [lista de localizações de saves](/saves), ou na sua página da PCGamingWiki, em «Save game data location».
2. Se usar a pasta do Steam, abre `userdata\<account-id>\<AppID>\remote` com o AppID da sua página na loja.
3. Se não, grava no jogo e ordena `%LOCALAPPDATA%`, `%USERPROFILE%\AppData\LocalLow`, `%APPDATA%` e `Documentos` por data de modificação. A pasta que acabou de mudar é a certa.

## O que o Steam Cloud guarda

O Steam Cloud guarda o save mais recente na tua conta Steam, nos jogos cujo programador o ativou. A [página de armazenamento remoto](https://store.steampowered.com/account/remotestorage) do Steam lista todos os jogos com ficheiros no teu Steam Cloud e deixa-te descarregá-los.

Guarda o último estado, não um histórico, por isso um save corrompido sincroniza tão fielmente como um bom, e só cobre jogos do Steam: nada da GOG, da Epic, de um emulador ou de um atalho fora do Steam. Há mais no [guia da alternativa ao Steam Cloud](/guides/steam-cloud-alternative).

## Backup e sincronização automáticos com o Hoard

O [Hoard](/download) é gratuito e de código aberto. Encontra os saves de cada jogo onde o jogo os pôs, userdata, AppData, Documentos, a pasta de instalação ou um prefixo do Proton na Deck, faz backup deles sempre que o jogo grava e outra vez quando sais, guarda um histórico para onde podes voltar e sincroniza-os entre os teus PCs e uma Steam Deck. Na pasta do Steam segue `<AppID>/remote/` e deixa `remotecache.vdf` em paz, para que o teu PC e a tua Deck nunca pareçam estar em desacordo.

1. Instala o Hoard e inicia sessão no Hoard One, recomendado para todos, ou aponta-o para [o teu próprio servidor](/guides/self-host-hoard).
2. Abre a **Biblioteca**. Os teus jogos do Steam são detetados a partir da tua biblioteca Steam e da base de dados comunitária de saves.
3. Joga. Ao sair, a primeira versão aparece no histórico.

## Hoard One, ou o teu próprio servidor

O Hoard é gratuito das duas maneiras, e a aplicação é a mesma em ambas:

- **Hoard One**, recomendado para todos. Nada para configurar nem para manter a correr: um servidor gerido na UE, gratuito com 2 GB e três dispositivos, sem cartão. O [Pro](/pricing) só acrescenta espaço, 100 GB e dispositivos ilimitados, para quando os teus saves já não couberem.
- **O teu próprio servidor.** Corres o `hoard-server` num PC, num NAS ou num Raspberry Pi e todas as tuas máquinas sincronizam através dele. Sem conta connosco, sem telemetria para nós e sem quota nossa: nada passa pelos nossos servidores, porque não há nada nosso no caminho. Vê [como alojar o Hoard tu mesmo](/guides/self-host-hoard).

O mesmo programa, a mesma deteção, o mesmo histórico. O Hoard One guarda todas as versões para sempre; no teu próprio servidor decides tu quanto histórico guardar.

<!-- faq -->

## Perguntas frequentes

### Onde fica a pasta userdata do Steam?

Dentro da pasta do Steam: `C:\Program Files (x86)\Steam\userdata` no Windows se o Steam estiver no sítio predefinido, `~/.local/share/Steam/userdata` numa Steam Deck ou no Linux, e `~/Library/Application Support/Steam/userdata` num Mac.

### Desinstalar um jogo apaga os saves?

Os saves na pasta de instalação vão com ela. Os de AppData, Documentos ou userdata costumam ficar. Na Deck, desinstalar também pode apagar o prefixo do Proton do jogo, e com ele todos os saves lá dentro, por isso faz backup antes.

### Posso copiar à mão saves do Steam para outro PC?

Sim. Fecha o jogo, copia a pasta de saves para o mesmo sítio no outro PC e abre lá o jogo. Se usar o Steam Cloud, o Steam pode perguntar que cópia manter no arranque seguinte; escolhe a de data mais recente.

### O Hoard é gratuito?

Sim. A aplicação, a linha de comandos e o servidor são gratuitos e de código aberto (AGPL-3.0), e correr o teu próprio servidor não custa mais do que a máquina. O Hoard One, o servidor gerido que recomendamos a todos, também é gratuito com 2 GB e três dispositivos; o Pro acrescenta espaço por 1,99 € por mês.

### O self-hosting envia alguma coisa ao Hoard?

Não. No teu próprio servidor não há conta connosco nem telemetria para nós: os teus saves, os teus utilizadores e os teus logs ficam na tua máquina e nunca tocam nos nossos.
