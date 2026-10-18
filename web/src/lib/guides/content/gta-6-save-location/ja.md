---
title: "GTA 6 のセーブデータの場所：PS5・Xbox・PC（現時点でわかっていること）"
description: "GTA 6 は 2026 年 11 月 19 日に PS5 と Xbox で発売、PC 版はまだありません。コンソールのセーブの場所、守り方、PC 版で予想されることを解説。"
order: 25
updated: 2026-10-17
---

GTA 6 は **2026 年 11 月 19 日、PS5 と Xbox Series X|S のみ**で発売されます。PC 版はまだありません。Rockstar は日付を発表しておらず、出回っている時期は 2026 年 10 月の未確認の報道が示す 2027 年初頭だけです。それまで GTA 6 のセーブデータはコンソール本体とそのクラウドにあり、開けるフォルダーにはありません。以下では、セーブの保存のされ方、守り方、そして Rockstar の最近の PC 作品から見て PC 版のセーブがおそらく置かれる場所を説明します。PC 版が出たら、このページに実際のパスを載せます。

## コンソールでの GTA 6 のセーブの場所

- **PS5:** コンソール本体です。PlayStation Plus に加入していれば、PS5 はセーブをオンラインストレージにも自動でアップロードします。未加入なら、コピーは本体にあるものだけです。
- **Xbox Series X|S:** コンソール本体にあり、Microsoft アカウントの Xbox クラウドと同期されます。自動で、サブスクリプションは不要です。

どちらもパソコンから開けるフォルダーではありません。PS5 はセーブを含む本体全体を USB ドライブにバックアップできますが、そのコピーを復元できるのは PS5 だけです。

## Hoard で GTA 6 のセーブをバックアップできますか？

コンソールではできません。Hoard が扱うのは、パソコン上のファイルとして存在するセーブです（Windows、macOS、Linux、Steam Deck）。コンソールのセーブはコンソール自身のシステムから出ないので、PC アプリからは届きません。

PC 版が出れば、GTA 6 のセーブも他の PC ゲームと同じくファイルになり、Hoard でバックアップして PC 間で同期できます。

## PC 版のセーブがありそうな場所

Rockstar の最近の PC 作品は、どれも同じ場所に保存しています。`ドキュメント\Rockstar Games` の下にゲームごとのフォルダーがあります。

- **GTA V Legacy:** `%USERPROFILE%\Documents\Rockstar Games\GTA V\Profiles\<プロフィール ID>`
- **GTA V Enhanced:** `%USERPROFILE%\Documents\Rockstar Games\GTAV Enhanced\Profiles\<プロフィール ID>`
- **Red Dead Redemption 2:** `%USERPROFILE%\Documents\Rockstar Games\Red Dead Redemption 2\Profiles\<プロフィール ID>`

最後のフォルダーは Rockstar アカウントにちなんだ名前で、アカウントごとに別のフォルダーになります。GTA 6 が同じパターンに従うなら、セーブは `ドキュメント\Rockstar Games\<ゲーム名>\Profiles\<プロフィール ID>` に置かれるはずです。**これは前例からの推測で、確認されたパスではありません。**

PC が `ドキュメント` を OneDrive にリダイレクトしている場合、`Rockstar Games` フォルダーごと `%USERPROFILE%\OneDrive\Documents\Rockstar Games` に移ります。すると OneDrive がプレイ中に勝手にセーブを同期し、それ自体が問題を起こします。[ゲームのセーブと OneDrive](/guides/onedrive-game-saves) を参照してください。

## PC での Rockstar 自身のクラウドセーブ

PC では、GTA V と Red Dead Redemption 2 は Steam で買った場合でも、Steam Cloud ではなく Rockstar Games Launcher でセーブを同期します。クラウドセーブはランチャーの設定でゲームごとにオンにします。Steam Cloud と同じく、現在のセーブをマシン間でそろえるだけで、戻れる古いバージョンは残りません。

GTA 6 が PC でセーブをどう扱うかは、Rockstar はまだ明らかにしていません。同じランチャーのクラウドを使うのが自然ですが、発表があるまでは推測です。

## Hoard で PC 版に備える

[Hoard](/download) はプレイをやめるたびにセーブフォルダーをバックアップし、すべてのバージョンを保持します。壊れたセーブも、もう一度やり直したいミッションも、復元一回で戻れます。さらにフォルダーを PC と Steam Deck の間で同期し、GTA V と Red Dead Redemption 2 にはすでに対応しています。

GTA 6 が PC に来たら:

1. Hoard をインストールしてサインインするか、[自分のサーバー](/guides/self-host-hoard)を指定します。
2. **ライブラリ** を開きます。発売直後のゲームはコミュニティのセーブデータベースに載るまで時間がかかることがあります。GTA 6 がまだ一覧にない場合は、`ドキュメント\Rockstar Games` 内のフォルダーを手動で追加してください。
3. プレイします。終了すると、最初のバージョンが履歴に表示されます。

<!-- faq -->

## よくある質問

### GTA 6 は PC で出ますか？

発売時には出ません。2026 年 11 月 19 日に PS5 と Xbox Series X|S で発売されます。Rockstar は PC 版の日付を発表しておらず、2026 年 10 月の報道が 2027 年初頭としていますが、確認はされていません。

### PS5 での GTA 6 のセーブはどこにありますか？

コンソール本体と、PlayStation Plus に加入していて自動アップロードがオンならオンラインストレージにあります。パソコンで開けるファイルではありません。

### GTA 6 のセーブをコンソールから PC に移せますか？

Rockstar は明らかにしていません。GTA V では自社のクラウド経由でストーリーモードのセーブを新しいコンソールに引き継げましたが、GTA 6 で同様の仕組みは発表されていません。

### セーブフォルダーにオンラインの進行状況は含まれますか？

Rockstar の現行作品では含まれません。GTA Online と Red Dead Online の進行状況は Rockstar のサーバーに保存され、セーブフォルダーにはありません。ディスク上にバックアップするものはありません。

### Hoard は GTA 6 に対応しますか？

PC では対応します。Rockstar の他の PC ゲームと同じようにフォルダーに保存する限りです。コンソールのセーブは、Hoard からも他のどの PC アプリからも扱えません。
