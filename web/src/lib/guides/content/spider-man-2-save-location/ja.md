---
title: "Marvel's Spider-Man 2 のセーブデータの場所（PC・Steam Deck）"
description: "Marvel's Spider-Man 2のPC版セーブデータの場所、長い数字のフォルダーの正体、OneDriveの落とし穴、Steam Deckでのパス、バックアップ方法を解説。"
order: 22
updated: 2026-10-02
---

PC 版の Marvel's Spider-Man 2 は、セーブデータを `ドキュメント\Marvel's Spider-Man 2\` の中の、長い数字の名前のサブフォルダーに保存します。Steam では、その数字はあなたの Steam ID です。以下では、それが実際に意味すること、OneDrive の落とし穴、Steam Deck でのパス、そしてセーブのバックアップを保つ方法を説明します。

## Marvel's Spider-Man 2 のセーブデータの場所

- **Windows:** `%USERPROFILE%\Documents\Marvel's Spider-Man 2\<長い数字>`
- **Steam Deck と Linux**（Proton）: `~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/<長い数字>`

PC 版は Windows 専用なので、Steam Deck では Proton で動き、セーブは Steam がこのゲーム用に用意する Proton プレフィックスの中にあります。`2651280` はこのゲームの Steam アプリ ID です。microSD カードにインストールしている場合、`compatdata` フォルダーはカード上にあります。

PC 版を手がけたスタジオ Nixxes は、セーブフォルダーを `ドキュメント\Marvel's Spider-Man 2\` の下にある「長い数字、または英数字の組み合わせのサブフォルダー」と説明しています。

## 長い数字のフォルダー

サブフォルダーの名前はアカウントによって決まります。Steam では **64 ビットの Steam ID**、Epic 版では英数字の組み合わせです。いずれにせよ、アカウントごとに異なります。ここから 2 つのことが言えます。

- 同じ PC で 2 人が別々の Steam アカウントで遊ぶ場合、それぞれに専用のセーブフォルダーがあります。
- 手動で別の PC にセーブをコピーするときは、**その**マシンの Steam アカウントのフォルダーに入れてください。別の ID のフォルダーに置いても、ゲームは認識しません。

親フォルダーの `Marvel's Spider-Man 2` には、ゲームのログやクラッシュダンプ（`.log`、`.mdmp`）も入っています。これらはセーブではないので、バックアップは不要です。

## OneDrive の落とし穴

多くの Windows PC では `ドキュメント` が OneDrive にリダイレクトされています。その場合、実際のパスは `%USERPROFILE%\OneDrive\Documents\Marvel's Spider-Man 2\` で、プレイ中に OneDrive がフォルダーを勝手に同期します。問題は 2 つです。書き込み途中のセーブを OneDrive がアップロードすることがあり、「空き領域を増やす」でセーブがオンライン専用のプレースホルダーになることがあります。ここで OneDrive に頼るなら、フォルダーを **このデバイス上に常に保持する** に設定してください。

## Spider-Man 2 にクラウドセーブはありますか？

あります。Steam クラウドが、同じ Steam アカウントのマシン間で最新のセーブをそろえます。古いバージョンは残しません。セーブが壊れると、壊れたものが同期されます。

## 手動でバックアップする

1. ゲームを完全に終了します。
2. `ドキュメント` の `Marvel's Spider-Man 2` フォルダーを安全な場所にコピーします。
3. 復元するときは、ゲームを終了し、長い数字のフォルダーを同じ場所、同じ Steam アカウントにコピーし戻します。

## Hoard で自動バックアップと同期

[Hoard](/download) は、プレイをやめるたびにセーブフォルダーをバックアップし、すべてのバージョンを保持します。さらに PC と Steam Deck の間で同期するので、どちらでも続きから遊べます。

1. Hoard をインストールしてサインインするか、[自分のサーバー](/guides/self-host-hoard)を指定します。
2. **ライブラリ** を開き、Spider-Man 2 に表示されているフォルダーが `ドキュメント`（または `OneDrive\Documents`）の下のものか確認します。別の場所を指していたら、そのフォルダーに変更してください。
3. プレイします。終了すると、最初のバージョンが履歴に表示されます。

後でセーブがおかしくなっても、[以前のバージョンを復元](/guides/restore-a-game-save)すれば元に戻せます。

<!-- faq -->

## よくある質問

### セーブフォルダーの長い数字は何ですか？

Steam では 64 ビットの Steam ID、Epic ではアカウントの ID です。アカウントごとにフォルダーがあり、ゲームはサインイン中のアカウントのものだけを読み込みます。

### Steam Deck での Spider-Man 2 のセーブデータはどこですか？

Proton プレフィックスの中です: `~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/` の、長い数字のフォルダーです。

### ドキュメントにフォルダーが見つかりません。どこにありますか？

`OneDrive\Documents\Marvel's Spider-Man 2` を確認してください。最近の Windows の多くでは、ドキュメントは OneDrive の中にあります。

### 友達の PC にセーブをコピーできますか？

ファイル自体はコピーできますが、その PC のアカウントの Steam ID のフォルダーに入れる必要があります。別のアカウントで作ったセーブを受け付けるかはゲーム次第なので、試す前に元のデータのコピーを取っておいてください。
