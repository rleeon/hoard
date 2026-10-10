---
title: "サイバーパンク2077（Cyberpunk 2077）のセーブデータの場所（PC・Steam Deck）"
description: "Cyberpunk 2077のセーブデータがWindows、Steam Deck、Macのどこにあるか、各フォルダーの中身、バックアップとPC間での同期のしかたを解説。"
order: 20
updated: 2026-10-09
---

Windows では、Cyberpunk 2077 のセーブデータは `%USERPROFILE%\Saved Games\CD Projekt Red\Cyberpunk 2077` にあり、セーブ 1 つにつきフォルダーが 1 つ作られます。これが短い答えです。以下では Steam Deck と Mac のパス、フォルダーの実際の中身、そしてバックアップを保ちながら PC と Steam Deck で同期する方法を説明します。

## Cyberpunk 2077 のセーブデータの場所

- **Windows**（Steam、GOG、Epic）: `%USERPROFILE%\Saved Games\CD Projekt Red\Cyberpunk 2077`
- **Steam Deck と Linux**（Proton）: `~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077`
- **Mac:** `~/Library/Application Support/CD Projekt Red/Cyberpunk 2077/Saves`

Windows と Mac のパスは、CD Projekt Red が自社のサポートページで案内しているものです。Steam Deck ではゲームは Proton プレフィックスの中で動きます。これは Steam がゲームごとに用意する小さな Windows のフォルダーツリーで、`1091500` は Cyberpunk の Steam アプリ ID です。ゲームを microSD カードにインストールしている場合は、カード上の `steamapps/compatdata/1091500` を探してください。

## フォルダーの中身

Cyberpunk はセーブを 1 つのファイルとして書き込むのではなく、**セーブごとにフォルダー**を作ります。`AutoSave-0`、`AutoSave-1` と続き、`ManualSave-0`、`ManualSave-1`、`QuickSave-0` などです。それぞれにセーブ本体（`sav.dat`）と、ロード画面に表示されるスクリーンショットとメタデータが入っています。

ここから 2 つのことが言えます。

- **個別のセーブではなく、親フォルダーをバックアップする。** 最新の `ManualSave` だけをコピーすると、最新の進行が入っていることの多いオートセーブが漏れます。
- **オートセーブはローテーションする。** ゲームは少数の `AutoSave` フォルダーを使い回し、いちばん古いものを上書きします。3 時間前のオートセーブはたいてい消えているので、ゲームの外に履歴を持つ価値があります。

設定はここにはありません。グラフィックや操作の設定は `%LOCALAPPDATA%\CD Projekt Red\Cyberpunk 2077` の `UserSettings.json` にあり、キャッシュやログと一緒に置かれています。多くの人（や一部のツール）が誤ってバックアップするのがこのフォルダーですが、失っても進行に影響するものは入っていません。

## Cyberpunk 2077 にクラウドセーブはありますか？

あります。Steam 版は Steam クラウドを、GOG 版は GOG Galaxy のクラウドを使います。どちらも同じストアのマシン間で、セーブの最新状態をそろえてくれます。

どちらもしないこと:

- **古いバージョンを残すこと。** セーブが壊れたり Mod で壊れたりすると、クラウドにも壊れたコピーが残ります。
- **ストアをまたぐこと。** Steam クラウドと GOG のクラウドは連携しません。ただし Steam、GOG、Epic の PC 版セーブは、フォルダーをコピーすればどれでも問題なく読み込めます。

## 手動でバックアップする

1. ゲームを完全に終了します。
2. 上記のパスから `Cyberpunk 2077` フォルダーを丸ごと、USB メモリや別のドライブ、クラウドフォルダーにコピーします。
3. 復元するときは、ゲームを終了してフォルダーをコピーし戻し、既存のものと置き換えます。

これでも使えますが、覚えているときにしかできず、手元にあるのは前回のコピーだけです。

## Hoard で自動バックアップと同期

[Hoard](/download) は、プレイをやめるたびにセーブフォルダーをバックアップし、すべてのバージョンを保持します。壊れたセーブも、ローテーションで消えたオートセーブも、ワンクリックで戻せます。さらにフォルダーを PC と Steam Deck の間で同期します。

1. Hoard をインストールしてサインインするか、[自分のサーバー](/guides/self-host-hoard)を指定します。
2. **ライブラリ** を開きます。Cyberpunk は Steam ライブラリとコミュニティのセーブデータベースから検出されます。
3. 表示されているフォルダーが `Saved Games\CD Projekt Red\Cyberpunk 2077` であることを確認します。`AppData\Local` のフォルダーが表示されていたら変更してください。そちらには設定しか入っていません。
4. プレイします。終了すると、最初のバージョンが履歴に表示されます。

Hoard はフォルダー全体を追跡するので、すべての `AutoSave`、`ManualSave`、`QuickSave` が同じバージョンに入ります。Deck とデスクトップなら、次に手に取ったほうで最新バージョンが待っています。詳しくは[PC 間の同期のしくみ](/guides/sync-game-saves-across-pcs)をご覧ください。

<!-- faq -->

## よくある質問

### Steam Deck での Cyberpunk 2077 のセーブデータはどこですか？

ゲームの Proton プレフィックスの中です: `~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077`。ゲームが microSD カードにある場合、`compatdata` フォルダーはカード上にあります。

### Cyberpunk 2077 のセーブを GOG から Steam に移せますか？

はい。PC 版のセーブは Steam、GOG、Epic で共通です。ゲームを閉じた状態で、もう一方のインストールの同じパスにセーブフォルダーをコピーすれば、ロード画面に表示されます。

### なぜ AutoSave フォルダーがたくさんあるのですか？

ゲームはいくつかのオートセーブ枠を持ち、毎回いちばん古いものを上書きします。普通のセーブですが、自動で置き換えられていくのです。

### 古いオートセーブが消えたのはなぜですか？

それが入っていた枠が再利用されたからです。ゲームが残すのは少数だけです。ローテーションで消えた後に取り戻すには、バージョンを保持するバックアップツールしかありません。

### Hoard は設定も同期しますか？

いいえ。設定はセーブに含まれない別のフォルダーにあるので、各マシンが自分の設定を保ちます。たいていはそれが望ましく、Deck とデスクトップでは必要なグラフィック設定が違います。詳しくは[PC 間でセーブを同期する](/guides/sync-game-saves-across-pcs)をご覧ください。
