---
title: "紅の砂漠（Crimson Desert）のセーブデータの場所（PC・Steam Deck）"
description: "Crimson DesertのセーブデータがWindows、Steam Deck、Macのどこにあるか、実際にセーブが入っているフォルダー、バックアップとPC間での同期のしかたを解説。"
order: 21
updated: 2026-10-09
---

Windows では、Crimson Desert のセーブデータは `%LOCALAPPDATA%\Pearl Abyss\CD\save` にあります。Pearl Abyss が自社の FAQ で案内しているフォルダーです。以下では Steam Deck と Mac のパス、中身、そしてバックアップを保ちながら PC と Steam Deck で同期する方法を説明します。

## Crimson Desert のセーブデータの場所

- **Windows:** `%LOCALAPPDATA%\Pearl Abyss\CD\save`（つまり `C:\Users\<ユーザー名>\AppData\Local\Pearl Abyss\CD\save`）
- **Steam Deck と Linux**（Proton）: `~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save`
- **Mac、Steam 版:** `~/Library/Application Support/Pearl Abyss/CD/save`
- **Mac、App Store 版:** `~/Library/Containers/com.pearlabyss.CrimsonDesert/Data/Library/Application Support/Pearl Abyss/CD/save`

ネイティブの Linux 版はないため、Steam Deck ではゲームは Proton で動き、セーブは Steam が用意する Proton プレフィックスの中にあります。`3321460` はこのゲームの Steam アプリ ID です。microSD カードにインストールしている場合、`compatdata` フォルダーはカード上にあります。

`AppData` は Windows では隠しフォルダーです。いちばん早いのは、エクスプローラーのアドレスバーに `%LOCALAPPDATA%\Pearl Abyss\CD\save` を貼り付ける方法です。

## フォルダーの中身

`save` の中にはサブフォルダーが 2 つあります。Pearl Abyss によれば、**数字の名前のほうに、ゲーム内で作ったセーブが入っています**。バックアップするときは、ファイルを選ぶのではなく `save` フォルダーを丸ごとコピーしてください。容量は小さく、ゲームに必要なものを取りこぼしません。

Mac の Steam 版と App Store 版はパスが異なります。両者を行き来する場合は、一度だけ手動でセーブをコピーしてください。

## Crimson Desert にクラウドセーブはありますか？

あります。Steam 版には Steam クラウドがあり、同じ Steam アカウントのマシン間で最新のセーブをそろえてくれます。

しないこと:

- **古いバージョンを残すこと。** Steam クラウドが保持するのは現在の状態です。セーブが壊れると、壊れたものが同期されます。
- **ほかのストアをカバーすること。** Mac App Store 版と Steam 版はクラウドを共有しません。

## 手動でバックアップする

1. ゲームを完全に終了します。
2. 上記のパスから `save` フォルダーを丸ごと、別のドライブや USB メモリ、クラウドフォルダーなど安全な場所にコピーします。
3. 復元するときは、ゲームを終了してコピーし戻し、既存のものと置き換えます。

大型アップデートや再インストールの前に一度だけ取るなら十分です。習慣にするには覚えていることが前提で、手元にあるのは前回のコピーだけです。

## Hoard で自動バックアップと同期

[Hoard](/download) は、プレイをやめるたびにセーブフォルダーをバックアップし、すべてのバージョンを保持します。壊れたセーブや後悔した選択からも戻れます。さらにフォルダーを PC と Steam Deck の間で同期します。

1. Hoard をインストールしてサインインするか、[自分のサーバー](/guides/self-host-hoard)を指定します。
2. **ライブラリ** を開きます。Crimson Desert は Steam ライブラリとコミュニティのセーブデータベースから、上記のパスで検出されます。
3. プレイします。終了すると、最初のバージョンが履歴に表示されます。

それ以降はセッションごとにバージョンが追加され、次に座ったマシンに最新のものが届いています。セーブがおかしくなったら、[以前のバージョンへの復元](/guides/restore-a-game-save)は数クリックで済みます。

<!-- faq -->

## よくある質問

### Steam Deck での Crimson Desert のセーブデータはどこですか？

ゲームの Proton プレフィックスの中です: `~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save`。ゲームが microSD カードにある場合、`compatdata` フォルダーはカード上にあります。

### どのサブフォルダーにセーブが入っていますか？

`save` の中の、数字の名前のフォルダーです。それでも取りこぼしがないよう、`save` フォルダー全体をバックアップしてください。

### AppData フォルダーが見つかりません。どこにありますか？

既定では隠しフォルダーです。エクスプローラーのアドレスバーに `%LOCALAPPDATA%\Pearl Abyss\CD\save` を貼り付けて Enter を押すか、［表示］メニューで隠しファイルを表示してください。

### デスクトップと Steam Deck で同じセーブを使えますか？

はい。Steam クラウドは同じ Steam アカウントで最新のセーブについてそれを行います。Hoard も同じことができ、さらにセッションごとにバージョンを残すので、何か壊れても戻れます。
