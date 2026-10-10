---
title: "PC ゲームのセーブデータはどこにある？主な保存場所の一覧"
description: "Windows、Steam Deck、Linux、Mac で PC ゲームがセーブを保存する場所、独自フォルダーを使うランチャー、どんなゲームのセーブでもすぐ見つける方法。"
order: 11
updated: 2026-10-09
---

決まったフォルダーはひとつではありません。Windows では、ほぼすべてのゲームが六つの場所のどれかに保存します。`Documents`、`Saved Games`、三つの `AppData` フォルダーのどれか、Steam の `userdata`、あるいはゲーム自身のインストールフォルダーです。どこにするかを決めるのはエンジンと開発者で、購入したストアではありません。このページでは主な保存場所、独自の層を加えるランチャー、そして誰も記録していないゲームでもセーブを素早く見つける方法をまとめます。

## Windows：よくある六つの場所

| フォルダー | 典型的なパス | 使うゲーム |
|---|---|---|
| ドキュメント | `%USERPROFILE%\Documents\My Games\<ゲーム>` | Bethesda のゲーム、Rockstar（`Documents\Rockstar Games`）、昔の大作の多く |
| 保存したゲーム | `%USERPROFILE%\Saved Games\<パブリッシャー>\<ゲーム>` | Cyberpunk 2077 と、根強い少数派 |
| AppData\Roaming | `%APPDATA%\<ゲーム>` | Elden Ring、Stardew Valley、Minecraft（`.minecraft`）、多くのインディー |
| AppData\Local | `%LOCALAPPDATA%\<ゲーム>\Saved\SaveGames` | Unreal Engine のゲーム（Palworld は `Pal\Saved\SaveGames`） |
| AppData\LocalLow | `%USERPROFILE%\AppData\LocalLow\<会社>\<ゲーム>` | Unity のゲーム（Hollow Knight ほか多数） |
| Steam の userdata | `C:\Program Files (x86)\Steam\userdata\<UserID>\<AppID>\remote` | Steam 自身のセーブ領域を使うゲーム |

そして、なかなか消えない七つ目が **ゲームのインストールフォルダー** です。古いゲームの多くや一部のインディーは、今もここに書き込みます。

実用的な注意を二つ。`AppData` は隠しフォルダーなので、クリックでたどるより、エクスプローラーのアドレスバーに `%APPDATA%` や `%LOCALAPPDATA%` と入力してください。また、OneDrive が `ドキュメント` フォルダーをバックアップしている場合、実際のパスは `C:\Users\<あなた>\OneDrive\Documents` です。これに驚く人は少なくありません。[OneDrive とゲームのセーブ](/guides/onedrive-game-saves) を参照してください。

## 独自の層を加えるランチャー

ほとんどのランチャーはセーブの置き場所を決めません。決めるのはゲームです。例外がいくつかあります。

- **Steam** は `userdata` にゲームごとのセーブ領域を持っています。`<UserID>` は Steam アカウントに結びついた数字（その PC でサインインしたアカウントごとにフォルダーがひとつ）、`<AppID>` はストアの URL にある数字です。
- **Ubisoft Connect** は `C:\Program Files (x86)\Ubisoft\Ubisoft Game Launcher\savegames\<ユーザー ID>\<ゲーム ID>` に保存し、どちらの階層も名前ではなく数字です。
- **Xbox アプリと PC Game Pass** は `%LOCALAPPDATA%\Packages\<パッケージ>\SystemAppData\wgs` を使い、ファイル名はランダムで、Xbox アプリにしか意味がわかりません。これは Xbox のクラウドセーブに任せてください。手でコピーしてもうまくいくことはまれです。
- **Epic、GOG、EA アプリ** はたいていゲームに任せるので、それらのタイトルは上の表の場所に落ち着きます。クラウドセーブがある場合も、そこからコピーしています。

## レジストリ（まれ）

一部のゲーム、主に小規模な Unity のタイトルは、進行をファイルではなく Windows のレジストリの `HKEY_CURRENT_USER\Software\<会社>\<ゲーム>` に保存します。フォルダーにコピーするものがないため、フォルダー単位のバックアップツールは Hoard も含めてこれを見られません。必要なら `regedit` でそのキーをエクスポートしてください。

## Steam Deck と Linux

- **Proton 経由の Windows 用ゲーム** は、ゲームごとのプレフィックスの中に保存します：`~/.local/share/Steam/steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/`、その後に表の Windows のパス（`Documents`、`AppData/Roaming` など）が続きます。microSD カード上のゲームは、カードの `steamapps/compatdata` の下に同じ構造があります。
- **ネイティブ Linux ゲーム** は `~/.local/share/<ゲーム>` か `~/.config/<ゲーム>` を使います。Unity のゲームは `~/.config/unity3d/<会社>/<ゲーム>` です。
- **Steam の userdata** は `~/.local/share/Steam/userdata/<UserID>/<AppID>/remote` にあります。
- **Heroic、Lutris、Bottles** はゲームごとに Wine プレフィックスを持ちます。中の Windows の構造は `steamuser` ではなく `drive_c/users/<あなたのユーザー名>/` の下です。

Deck には専用のガイドがあります：[Steam Deck と PC の間でセーブを同期する方法](/guides/sync-saves-steam-deck-pc)。

## Mac

- **ほとんどのゲーム：** `~/Library/Application Support/<ゲーム>`。Unity のゲームは `~/Library/Application Support/<会社>/<ゲーム>` です。
- **Mac App Store のゲーム** はサンドボックス化されています：`~/Library/Containers/<バンドル ID>/Data/Library/Application Support/`。

`~/Library` も隠れています。Finder で Option キーを押しながら **移動** メニューを開くと表示されます。

## どんなゲームでもセーブを見つける方法

どのリストにも載っていないゲームでも、三つのコツで数分あれば見つかります。

1. **PCGamingWiki で調べる。** ほぼすべてのゲームのページに「Save game data location」という節があります。Hoard や Ludusavi が使うセーブのデータベースも、このソースから作られています。
2. **変化を見る。** ゲーム内でセーブして終了し、直近数分で変更されたファイルをユーザーフォルダーから探します。Windows なら `C:\Users\<あなた>` の中で `datemodified:today` を検索し、日付で並べ替えます。Linux や Deck なら `find ~ -type f -mmin -5 -not -path '*/.cache/*'`。
3. **Steam に聞く。** Steam Cloud 対応のゲームなら、[store.steampowered.com/account/remotestorage](https://store.steampowered.com/account/remotestorage) に Steam がゲームごとに保持しているファイルが名前とサイズつきで並びます。ファイル名がわかれば、フォルダーはすぐ見つかります。

## あるいは、見つけてもらう

Hoard は同じコミュニティのデータベース（数千本のゲームを網羅）を読み、あなたのマシン上の候補パスをひとつずつ確認します。Proton、Heroic、Lutris のプレフィックス、OneDrive の `ドキュメント`、エミュレーター、ポータブル版のインストールまで。見つけたものは遊び終えるたびに自動でバックアップされ、すべてのバージョンが残り、PC と Steam Deck の間で同期されます。見落としたものは、フォルダーを一度指定すれば追加できます。[ゲームのセーブを自動でバックアップする方法](/guides/back-up-game-saves) を参照してください。

人気ゲームのいくつかには、正確なパスをまとめたページもあります：[Cyberpunk 2077](/guides/cyberpunk-2077-save-location)、[Baldur's Gate 3](/guides/baldurs-gate-3-save-location)、[Palworld](/guides/palworld-save-location)、[Crimson Desert](/guides/crimson-desert-save-location)、[Marvel's Spider-Man 2](/guides/spider-man-2-save-location)。

<!-- faq -->

## よくある質問

### Steam はセーブをどこに保存しますか？

ゲームによります。Steam 独自の領域 `Steam\userdata\<UserID>\<AppID>\remote` を使うものもありますが、多くは他のゲームと同じく `Documents`、`AppData`、`Saved Games` に書き込み、Steam Cloud はそこからコピーします。

### AppData フォルダーが見つからないのはなぜ？

隠しフォルダーだからです。エクスプローラーのアドレスバーか「ファイル名を指定して実行」（Win + R）に `%APPDATA%`（Roaming）または `%LOCALAPPDATA%`（Local）と入力してください。`LocalLow` は `Local` の隣にあります。

### Steam 版、GOG 版、Epic 版は同じ場所に保存しますか？

たいていは同じです。決めるのはストアではなくゲームだからです。ただし例外もあります。アカウント ID のフォルダーを挟むゲームや、ストア版によってフォルダー名が違うものもあります。版をまたいでセーブをコピーする前に確認してください。

### Xbox アプリと Game Pass のセーブはどこ？

`%LOCALAPPDATA%\Packages\<パッケージ>\SystemAppData\wgs` に、Xbox アプリにしか読めないランダムな名前のファイルとして置かれています。同期は Xbox のクラウドが行います。手でコピーしてもうまくいくことはまれです。

### ドキュメントフォルダーが OneDrive の中にあります。問題ですか？

問題になることがあります。ゲームはフォルダーを追って OneDrive の中に入り、OneDrive はゲームが書いている最中のセーブを同期します。[OneDrive とゲームのセーブ](/guides/onedrive-game-saves) を参照してください。
