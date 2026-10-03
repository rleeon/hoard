---
title: "Onde ficam os saves de Cyberpunk 2077 (PC e Steam Deck)"
description: "Onde o Cyberpunk 2077 guarda os saves no Windows, Steam Deck e Mac, o que há em cada pasta e como fazer backup ou levá-los de um PC para outro."
order: 20
updated: 2026-10-02
---

No Windows, o Cyberpunk 2077 guarda os saves em `%USERPROFILE%\Saved Games\CD Projekt Red\Cyberpunk 2077`, uma pasta por save. Essa é a resposta curta. O resto da página cobre os caminhos na Steam Deck e no Mac, o que há realmente na pasta e como mantê-la sempre com backup.

## Onde o Cyberpunk 2077 guarda os saves

- **Windows** (Steam, GOG ou Epic): `%USERPROFILE%\Saved Games\CD Projekt Red\Cyberpunk 2077`
- **Steam Deck e Linux** (Proton): `~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077`
- **Mac:** `~/Library/Application Support/CD Projekt Red/Cyberpunk 2077/Saves`

Os caminhos do Windows e do Mac são os que a CD Projekt Red indica nas suas páginas de suporte. Na Steam Deck, o jogo corre dentro de um prefixo do Proton, uma pequena árvore de pastas do Windows que o Steam mantém para cada jogo; `1091500` é o ID do Cyberpunk no Steam. Se o jogo estiver instalado no cartão microSD, procura `steamapps/compatdata/1091500` no cartão.

## O que há na pasta

O Cyberpunk não escreve um único ficheiro de save. Escreve **uma pasta por save**: `AutoSave-0`, `AutoSave-1` e seguintes, `ManualSave-0`, `ManualSave-1` e `QuickSave-0`. Cada uma guarda o save em si (`sav.dat`) e a captura e os metadados que o menu de carregamento mostra.

Daqui saem duas coisas:

- **Faz backup da pasta mãe, não de um save solto.** Copiar só o `ManualSave` mais recente deixa de fora os autosaves, que muitas vezes têm o progresso mais novo.
- **Os autosaves rodam.** O jogo reutiliza umas poucas pastas `AutoSave` e escreve por cima da mais antiga. Um autosave de há três horas normalmente já não existe, e é por isso que compensa ter um histórico fora do jogo.

As definições não estão aqui. Gráficos e controlos vivem em `UserSettings.json`, em `%LOCALAPPDATA%\CD Projekt Red\Cyberpunk 2077`, junto a caches e logs. É a pasta que muita gente (e algumas ferramentas) copia por engano: não guarda nada cuja perda te custe progresso.

## O Cyberpunk 2077 tem saves na nuvem?

Sim. A versão Steam usa o Steam Cloud e a da GOG a nuvem do GOG Galaxy. Ambas mantêm em dia o último estado dos teus saves entre máquinas da mesma loja.

O que nenhuma faz:

- **Guardar versões anteriores.** Se um save se corromper, ou um mod o estragar, a nuvem também fica com a cópia estragada.
- **Atravessar lojas.** O Steam Cloud e a nuvem da GOG não falam um com o outro, embora os saves de PC do Steam, GOG e Epic carreguem sem problemas em qualquer um se copiares a pasta.

## Fazer backup à mão

1. Fecha o jogo por completo.
2. Copia a pasta `Cyberpunk 2077` inteira do caminho acima para uma pen USB, outro disco ou uma pasta na nuvem.
3. Para restaurar, fecha o jogo e volta a copiar a pasta, substituindo o que lá estiver.

Funciona, mas só com a frequência com que te lembrares, e só tens a cópia da última vez.

## Backup e sincronização automáticos com o Hoard

O [Hoard](/download) faz backup da pasta de saves sempre que deixas de jogar e guarda todas as versões, por isso um save corrompido ou um autosave que já rodou estão a um clique. Também sincroniza a pasta entre os teus PCs e uma Steam Deck.

1. Instala o Hoard e inicia sessão, ou aponta-o para [o teu próprio servidor](/guides/self-host-hoard).
2. Abre a **Biblioteca**. O Cyberpunk é detetado a partir da tua biblioteca Steam e da base de dados comunitária de saves.
3. Confirma que a pasta mostrada é a de `Saved Games\CD Projekt Red\Cyberpunk 2077`. Se aparecer a de `AppData\Local`, muda-a: essa só tem definições.
4. Joga. Ao sair, a primeira versão aparece no histórico.

O Hoard acompanha a pasta inteira, por isso cada `AutoSave`, `ManualSave` e `QuickSave` entra na mesma versão. Com uma Deck e um PC, a versão mais recente espera por ti no que pegares a seguir — vê [como funciona a sincronização entre PCs](/guides/sync-game-saves-across-pcs).

<!-- faq -->

## Perguntas frequentes

### Onde ficam os saves de Cyberpunk 2077 na Steam Deck?

Dentro do prefixo do Proton do jogo: `~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077`. Se o jogo estiver no cartão microSD, a pasta `compatdata` está no cartão.

### Posso passar os meus saves de Cyberpunk 2077 da GOG para o Steam?

Sim. Os saves de PC são os mesmos no Steam, GOG e Epic. Copia as pastas de save para o mesmo caminho na outra instalação com o jogo fechado e aparecem no menu de carregamento.

### Porque há tantas pastas AutoSave?

O jogo mantém algumas ranhuras de autosave e escreve por cima da mais antiga de cada vez. São saves normais, só que são substituídos sozinhos.

### Porque desapareceu o meu autosave antigo?

Porque a ranhura onde estava foi reutilizada. O jogo só guarda uns poucos. Uma ferramenta de backup que guarde versões é a única forma de o recuperar depois de rodar.

### O Hoard também sincroniza as minhas definições?

Não. As definições vivem noutra pasta que não faz parte do save, por isso cada máquina mantém as suas, que normalmente é o que queres: uma Deck e um PC precisam de definições gráficas diferentes. Mais sobre isto em [sincronizar saves entre PCs](/guides/sync-game-saves-across-pcs).
