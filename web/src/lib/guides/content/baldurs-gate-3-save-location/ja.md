---
title: "バルダーズ・ゲート3（Baldur's Gate 3）のセーブデータの場所（PC・Steam Deck）"
description: "Baldur's Gate 3のセーブデータがWindows、Steam Deck、Macのどこにあるか、セーブとModや設定の違い、オナーモード、バックアップ方法を解説。"
order: 23
updated: 2026-10-02
---

Windows では、Baldur's Gate 3 のセーブデータは `%LOCALAPPDATA%\Larian Studios\Baldur's Gate 3\PlayerProfiles\Public\Savegames\Story` にあり、セーブ 1 つにつきフォルダーが 1 つです。Larian が自社のサポート FAQ で案内しているパスです。以下では Steam Deck と Mac のパス、セーブの隣にあるもの、オナーモード、そしてすべてをバックアップしておく方法を説明します。

## Baldur's Gate 3 のセーブデータの場所

- **Windows:** `%LOCALAPPDATA%\Larian Studios\Baldur's Gate 3\PlayerProfiles\Public\Savegames\Story`
- **Steam Deck と Linux**（Proton）: `~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story`
- **Mac:** `~/Documents/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story`

Larian はネイティブの Linux 版を廃止したため、Steam Deck ではゲームは Proton で動き、セーブは Steam が用意する Proton プレフィックスの中にあります。`1086940` はこのゲームの Steam アプリ ID です。microSD カードにインストールしている場合、`compatdata` フォルダーはカード上にあります。

## セーブとそれ以外

各セーブは `Story` の中の **フォルダー** で、`.lsv` ファイルとサムネイルが入っています。周りにあるものは別物です。

- **`Mods`**（`Baldur's Gate 3` の下）には Mod のファイルが入っています。
- **`modsettings.lsx`**（`PlayerProfiles\Public` の下）は、有効な Mod の一覧と読み込み順です。
- **設定**（グラフィックや操作など）はプロファイルの隣にある設定ファイルで、セーブの一部ではありません。

つまずきやすいのは Mod です。Mod を入れて作ったセーブは、読み込むときに同じ Mod が有効であることを前提にします。Mod 入りのセーブを別の PC に移すなら Mod の一覧も一緒に移してください。そうしないと Mod 不足の警告が出て、セーブが期待どおりに読み込めないことがあります。

## オナーモード

オナーモードではセーブは 1 つだけで、プレイ中にゲームが上書きしていきます。パーティーが全滅するとオナーでの冒険は終わりです（オナーを失ったうえでカスタムモードとして続けることはできます）。このセーブをバックアップするかはあなた次第です。難しい戦闘の前のコピーは技術的には「戻る手段」で、クラッシュやバグの後にまさにそれを求める人もいれば、モードへのズルだと考える人もいます。バックアップツールはどちらにせよバージョンを残します。それを復元するかどうかは、あなたとダイスの問題です。

## Baldur's Gate 3 にクラウドセーブはありますか？

あります。Steam では Steam クラウドを使い、同じアカウントのマシン間で最新のセーブをそろえます。保持するのは現在の状態だけなので、セーブが壊れたり Mod の更新で壊れたりすると、そのバージョンが同期されます。

## 手動でバックアップする

1. ゲームを完全に終了します。
2. 上記のパスから `PlayerProfiles` フォルダーを丸ごとコピーします（`Savegames` と `modsettings.lsx` を含みます）。
3. 復元するときは、ゲームを終了してコピーし戻します。

## Hoard で自動バックアップと同期

[Hoard](/download) は、プレイをやめるたびにセーブフォルダーをバックアップし、すべてのバージョンを保持します。Mod の更新やパッチで壊れたセーブも、復元 1 回で戻せます。さらにフォルダーを PC と Steam Deck の間で同期します。

1. Hoard をインストールしてサインインするか、[自分のサーバー](/guides/self-host-hoard)を指定します。
2. **ライブラリ** を開きます。Baldur's Gate 3 は Steam ライブラリとコミュニティのセーブデータベースから検出されます。
3. プレイします。終了すると、最初のバージョンが履歴に表示されます。

セッションごとに、すべてのセーブを含むバージョンが追加されます。戻りたいときは履歴を開いて[以前のバージョンを復元](/guides/restore-a-game-save)してください。今 PC にあるものは先にバックアップされるので、古いバージョンを試しても片道切符にはなりません。

<!-- faq -->

## よくある質問

### Steam Deck での Baldur's Gate 3 のセーブデータはどこですか？

ゲームの Proton プレフィックスの中です: `~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story`。

### Mod 入りのセーブを別の PC に移せますか？

はい。移し先の PC に同じ Mod がインストールされ、同じ順番で有効になっていれば可能です。セーブと一緒に `modsettings.lsx` をコピーし、同じ Mod ファイルをインストールしてください。

### オナーモードのセーブをバックアップできますか？

セーブは普通のフォルダーなので、どのバックアップツールでもコピーできます。それを復元することがモードの精神に合うかどうかは、あなたが決めてください。

### セーブに「Mod が不足している」と表示されるのはなぜですか？

今は有効になっていない Mod を使って作られたセーブだからです。同じ Mod を同じ順番で有効にすれば、普通に読み込めます。
