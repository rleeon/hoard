---
title: "Onde ficam os saves dos jogos de PC? Todas as localizações"
description: "Onde os jogos de PC guardam os saves no Windows, Steam Deck, Linux e Mac, que launchers acrescentam pastas próprias e como encontrar os de qualquer jogo."
order: 11
updated: 2026-10-09
---

Não há uma só pasta. No Windows, quase todos os jogos guardam num de seis sítios: `Documents`, `Saved Games`, uma das três pastas `AppData`, o `userdata` do Steam ou a própria pasta de instalação. Quem decide é o motor e o programador, não a loja onde o compraste. Esta página lista todas as localizações habituais, os launchers que acrescentam uma camada própria e uma forma rápida de encontrar os saves de qualquer jogo, mesmo de um que ninguém documentou.

## Windows: os seis sítios do costume

| Pasta | Caminho típico | Quem a usa |
|---|---|---|
| Documentos | `%USERPROFILE%\Documents\My Games\<Jogo>` | Jogos da Bethesda, Rockstar (`Documents\Rockstar Games`), muitos títulos grandes mais antigos |
| Jogos guardados | `%USERPROFILE%\Saved Games\<Editora>\<Jogo>` | Cyberpunk 2077 e uma minoria teimosa |
| AppData\Roaming | `%APPDATA%\<Jogo>` | Elden Ring, Stardew Valley, Minecraft (`.minecraft`), muitos indies |
| AppData\Local | `%LOCALAPPDATA%\<Jogo>\Saved\SaveGames` | Jogos em Unreal Engine (o Palworld usa `Pal\Saved\SaveGames`) |
| AppData\LocalLow | `%USERPROFILE%\AppData\LocalLow\<Empresa>\<Jogo>` | Jogos em Unity (Hollow Knight e muitos mais) |
| userdata do Steam | `C:\Program Files (x86)\Steam\userdata\<UserID>\<AppID>\remote` | Jogos que usam o espaço de saves do Steam |

E um sétimo que se recusa a desaparecer: **a pasta de instalação do jogo**, onde muitos jogos antigos e alguns indies ainda escrevem.

Duas notas práticas. A `AppData` está oculta, por isso escreve `%APPDATA%` ou `%LOCALAPPDATA%` na barra de endereço do Explorador em vez de ires lá aos cliques. E se o OneDrive faz cópia de segurança da tua pasta `Documentos`, o caminho real é `C:\Users\<tu>\OneDrive\Documents`, o que surpreende muita gente. Vê [OneDrive e os saves dos jogos](/guides/onedrive-game-saves).

## Launchers que acrescentam uma camada própria

A maioria dos launchers não decide onde ficam os saves; decide o jogo. Há algumas exceções:

- **O Steam** tem um espaço de saves por jogo em `userdata`. `<UserID>` é um número ligado à tua conta do Steam (há uma pasta por cada conta que iniciou sessão nesse PC) e `<AppID>` é o número no URL do jogo na loja.
- **O Ubisoft Connect** guarda em `C:\Program Files (x86)\Ubisoft\Ubisoft Game Launcher\savegames\<ID de utilizador>\<ID do jogo>`, com números em vez de nomes nos dois níveis.
- **A app Xbox e o PC Game Pass** usam `%LOCALAPPDATA%\Packages\<pacote>\SystemAppData\wgs`, com ficheiros de nomes aleatórios que só a app Xbox entende. Deixa esses aos saves na nuvem da Xbox; copiá-los à mão raramente funciona.
- **A Epic, a GOG e a app da EA** normalmente deixam isso ao jogo, por isso os seus títulos acabam nos sítios habituais acima. Os seus saves na nuvem, quando existem, copiam a partir daí.

## O registo, raramente

Alguns jogos, sobretudo títulos pequenos em Unity, guardam o progresso no registo do Windows, em `HKEY_CURRENT_USER\Software\<Empresa>\<Jogo>`, e não num ficheiro. Não há nada para copiar numa pasta, e as ferramentas de cópia baseadas em pastas, incluindo o Hoard, não o veem. Se precisares, exporta essa chave com o `regedit`.

## Steam Deck e Linux

- **Os jogos de Windows através do Proton** guardam dentro de um prefixo por jogo: `~/.local/share/Steam/steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/`, seguido do caminho do Windows da tabela (`Documents`, `AppData/Roaming`, etc.). Os jogos num cartão microSD têm a mesma árvore no `steamapps/compatdata` do próprio cartão.
- **Os jogos nativos de Linux** usam `~/.local/share/<jogo>` ou `~/.config/<jogo>`. Os de Unity vão para `~/.config/unity3d/<Empresa>/<Jogo>`.
- **O userdata do Steam** está em `~/.local/share/Steam/userdata/<UserID>/<AppID>/remote`.
- **Heroic, Lutris e Bottles** mantêm um prefixo do Wine por jogo. Lá dentro, a árvore do Windows fica em `drive_c/users/<o teu utilizador>/`, não em `steamuser`.

O Deck tem um guia só seu: [sincronizar saves entre Steam Deck e PC](/guides/sync-saves-steam-deck-pc).

## Mac

- **A maioria dos jogos:** `~/Library/Application Support/<Jogo>`. Os de Unity usam `~/Library/Application Support/<Empresa>/<Jogo>`.
- **Os jogos da Mac App Store** ficam isolados em `~/Library/Containers/<id do pacote>/Data/Library/Application Support/`.

A `~/Library` também está oculta. No Finder, abre o menu **Ir** com a tecla Option premida e ela aparece.

## Como encontrar os saves de qualquer jogo

Quando um jogo não está em nenhuma lista, três truques encontram-no em poucos minutos:

1. **Procura-o no PCGamingWiki.** Quase todas as páginas de jogos têm uma secção "Save game data location". É a mesma fonte de onde saem as bases de dados de saves usadas pelo Hoard e pelo Ludusavi.
2. **Vê o que muda.** Guarda no jogo, sai e procura na tua pasta de utilizador os ficheiros alterados nos últimos minutos. No Windows, procura `datemodified:today` dentro de `C:\Users\<tu>` e ordena por data. Em Linux ou num Deck: `find ~ -type f -mmin -5 -not -path '*/.cache/*'`.
3. **Pergunta ao Steam.** Num jogo com Steam Cloud, [store.steampowered.com/account/remotestorage](https://store.steampowered.com/account/remotestorage) lista os ficheiros que o Steam guarda de cada jogo, com nome e tamanho. Sabendo como se chama o ficheiro, encontrar a pasta é fácil.

## Ou deixa alguém encontrá-los por ti

O Hoard lê essa mesma base de dados comunitária, que cobre milhares de jogos, e verifica cada caminho possível na tua máquina: prefixos do Proton, Heroic e Lutris, a pasta `Documentos` do OneDrive, emuladores, instalações portáteis. O que encontra é guardado automaticamente sempre que deixas de jogar, com todas as versões guardadas, e mantido sincronizado entre os teus PCs e a Steam Deck. O que lhe escapar, acrescentas apontando para a pasta uma vez. Vê [como fazer backup automático dos teus saves](/guides/back-up-game-saves).

Também há páginas com os caminhos exatos de alguns jogos populares: [Cyberpunk 2077](/guides/cyberpunk-2077-save-location), [Baldur's Gate 3](/guides/baldurs-gate-3-save-location), [Palworld](/guides/palworld-save-location), [Crimson Desert](/guides/crimson-desert-save-location) e [Marvel's Spider-Man 2](/guides/spider-man-2-save-location).

<!-- faq -->

## Perguntas frequentes

### Onde é que o Steam guarda os saves?

Depende do jogo. Alguns usam o espaço próprio do Steam, `Steam\userdata\<UserID>\<AppID>\remote`; a maioria escreve em `Documents`, `AppData` ou `Saved Games` como qualquer outro jogo, e o Steam Cloud copia-os a partir daí.

### Porque não encontro a pasta AppData?

Porque está oculta. Escreve `%APPDATA%` (Roaming) ou `%LOCALAPPDATA%` (Local) na barra de endereço do Explorador ou em Executar (Win + R). A `LocalLow` fica ao lado da `Local`.

### As versões do Steam, GOG e Epic guardam no mesmo sítio?

Normalmente sim, porque quem decide é o jogo e não a loja. Há exceções: alguns jogos acrescentam uma pasta com o ID da tua conta, e uma ou outra versão de loja usa outro nome de pasta. Confirma antes de copiar saves de uma versão para outra.

### Onde estão os saves da app Xbox e do Game Pass?

Em `%LOCALAPPDATA%\Packages\<pacote>\SystemAppData\wgs`, em ficheiros com nomes aleatórios que só a app Xbox entende. São sincronizados pela nuvem da Xbox; copiá-los à mão raramente funciona.

### A minha pasta Documentos está dentro do OneDrive. É um problema?

Pode ser. Os jogos seguem a pasta para dentro do OneDrive, e o OneDrive sincroniza os saves enquanto os jogos os escrevem. Vê [OneDrive e os saves dos jogos](/guides/onedrive-game-saves).
