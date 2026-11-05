---
title: "Steam のセーブデータはどこにある？PC と Steam Deck の保存場所まとめ"
description: "Steam には userdata フォルダーがありますが、多くのゲームは AppData、ドキュメント、ゲーム自身のフォルダーに保存します。Windows、Steam Deck、Mac での探し方。"
order: 30
updated: 2026-11-05
---

Steam にはセーブ用の専用フォルダー、Steam フォルダー内の `userdata\<account-id>\<AppID>\remote` がありますが、ほとんどのゲームはこれを使いません。10,937 本の Steam ゲームの Windows のセーブ場所を収録したコミュニティのセーブデータベースでは、ここに保存するのは **約 10 本に 1 本** だけです。残りは AppData、ドキュメント、またはゲーム自身のインストールフォルダーに書き込み、Steam クラウドに対応していれば、Steam はファイルがある場所からそのまま拾います。

## Steam 自身のフォルダー：userdata

- **Windows：** `C:\Program Files (x86)\Steam\userdata\<account-id>\<AppID>\remote`
- **Steam Deck と Linux：** `~/.local/share/Steam/userdata/<account-id>/<AppID>/remote`
- **macOS：** `~/Library/Application Support/Steam/userdata/<account-id>/<AppID>/remote`

`<account-id>` は、Steam が **フレンドコード** として表示する番号です（フレンド →「フレンドを追加」）。そのマシンでサインインしたアカウントごとにフォルダーが 1 つあります。`<AppID>` はストアのゲームのアドレスにある番号で、`store.steampowered.com/app/413150` なら Stardew Valley です。

`remote` の一つ上、`<AppID>` だけのフォルダーには `remotecache.vdf` もあります。これは Steam 自身の管理ファイルでセーブではなく、PC と Deck で中身が違うのが正常です。

## ほとんどのゲームが実際に保存する場所

同じデータベースで 2026 年 11 月 5 日に数えた、各場所を使う Steam ゲームの割合です（複数を使うゲームもあります）。

- **`AppData\LocalLow`**：23%、主に Unity 製のゲーム。`%USERPROFILE%\AppData\LocalLow\<開発元>\<ゲーム>`
- **ゲームのインストールフォルダー**：22%。`C:\Program Files (x86)\Steam\steamapps\common\<ゲーム>`
- **`AppData\Local`**：21%。`%LOCALAPPDATA%\<開発元またはゲーム>`
- **`ドキュメント`**：15%、`My Games` の下が多い。`%USERPROFILE%\Documents\My Games\<ゲーム>`
- **`AppData\Roaming`**：12%。`%APPDATA%\<ゲーム>`
- **Steam の `userdata`**：10%。上を参照
- **`保存したゲーム`**：3%。`%USERPROFILE%\Saved Games\<開発元>\<ゲーム>`

## Steam Deck では：Proton プレフィックス

Deck では Windows 用ゲームは Proton で動き、ゲームごとに `~/.local/share/Steam/steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/` の中に小さな Windows を持ちます。AppData、ドキュメント、保存したゲームはすべてこの中にあるので、PC の `%APPDATA%\Game` は Deck では `compatdata/<AppID>/pfx/drive_c/users/steamuser/AppData/Roaming/Game` になります。プレフィックスはゲームを初めて起動したときに作られます。

ネイティブ Linux 版のあるゲームはプレフィックスを使わず、たいてい `~/.local/share` か `~/.config` に保存します。インストールフォルダーは `~/.local/share/Steam/steamapps/common/<ゲーム>`、microSD カードにインストールした場合はカード上の同じ `steamapps` の中です。

## あるゲームのセーブを素早く見つける

1. [セーブデータの場所の一覧](/saves) か、そのゲームの PCGamingWiki のページの「Save game data location」で探します。
2. Steam のフォルダーを使うゲームなら、ストアページの AppID で `userdata\<account-id>\<AppID>\remote` を開きます。
3. そうでなければ、ゲーム内でセーブしてから `%LOCALAPPDATA%`、`%USERPROFILE%\AppData\LocalLow`、`%APPDATA%`、ドキュメントを更新日時で並べ替えます。いま変わったフォルダーがそれです。

## Steam クラウドが残すもの

Steam クラウドは、開発元が有効にしたゲームについて、最新のセーブを Steam アカウントに残します。Steam の [リモートストレージのページ](https://store.steampowered.com/account/remotestorage) には、Steam クラウドにファイルがあるゲームがすべて並び、ダウンロードもできます。

残るのは最新の状態だけで履歴はないので、壊れたセーブも正常なセーブと同じように忠実に同期されます。また対象は Steam のゲームだけで、GOG、Epic、エミュレーター、Steam 以外のショートカットは含まれません。詳しくは [Steam クラウドの代わりのガイド](/guides/steam-cloud-alternative) をどうぞ。

## Hoard で自動バックアップと同期

[Hoard](/download) は無料のオープンソースです。userdata、AppData、ドキュメント、インストールフォルダー、Deck の Proton プレフィックスなど、ゲームが置いた場所でセーブを見つけ、ゲームがセーブを書くたびと終了時にバックアップし、いつでも戻れる履歴を残し、PC と Steam Deck の間で同期します。Steam のフォルダーでは `<AppID>/remote/` を追跡して `remotecache.vdf` には触れないので、PC と Deck の内容が食い違って見えることはありません。

1. Hoard をインストールして、すべての人におすすめの Hoard One にサインインするか、[自分のサーバー](/guides/self-host-hoard) を指定します。
2. **ライブラリ** を開きます。Steam のゲームは Steam ライブラリとコミュニティのセーブデータベースから検出されます。
3. プレイします。終了すると、最初のバージョンが履歴に表示されます。

## Hoard One か、自分のサーバーか

どちらを選んでも Hoard は無料で、アプリも同じです。

- **Hoard One**：すべての人におすすめです。設定することも、動かし続けるものもありません。EU にある管理サーバーで、2 GB・3 台まで無料、カードは不要です。[Pro](/pricing) は容量を増やすだけのプランで、セーブが収まらなくなったときのために 100 GB と台数無制限を用意しています。
- **自分のサーバー。** PC、NAS、Raspberry Pi で `hoard-server` を動かせば、すべてのマシンがそこを通して同期します。当方のアカウントは不要で、当方へのテレメトリーも当方による容量制限もありません。経路上に当方のものが何もないので、当方のサーバーを通るものは何もありません。[Hoard をセルフホストする方法](/guides/self-host-hoard) を参照してください。

同じプログラム、同じ検出、同じ履歴です。Hoard One はすべてのバージョンをずっと保持し、自分のサーバーではどれだけ履歴を残すかを自分で決められます。

<!-- faq -->

## よくある質問

### Steam の userdata フォルダーはどこですか？

Steam フォルダーの中です。Steam が既定の場所にあれば Windows では `C:\Program Files (x86)\Steam\userdata`、Steam Deck や Linux では `~/.local/share/Steam/userdata`、Mac では `~/Library/Application Support/Steam/userdata` です。

### ゲームをアンインストールするとセーブは消えますか？

インストールフォルダー内のセーブは一緒に消えます。AppData、ドキュメント、userdata のセーブはたいてい残ります。Deck ではアンインストールでゲームの Proton プレフィックスも消えることがあり、その中のセーブもすべて消えるので、先にバックアップしてください。

### Steam のセーブを手作業で別の PC にコピーできますか？

できます。ゲームを閉じ、セーブフォルダーをもう一台の PC の同じ場所にコピーして、そちらでゲームを起動します。Steam クラウドを使うゲームなら、次の起動時にどちらのコピーを残すか聞かれることがあるので、日付の新しいほうを選んでください。

### Hoard は無料ですか？

はい。アプリ、コマンドライン、サーバーは無料のオープンソース（AGPL-3.0）で、自分のサーバーを動かすのにかかるのはマシン代だけです。すべての人におすすめの管理サーバーである Hoard One も 2 GB・3 台まで無料で、Pro は月 1.99 ユーロで容量を増やします。

### セルフホストすると Hoard に何か送られますか？

いいえ。自分のサーバーでは当方のアカウントも当方へのテレメトリーもありません。セーブ、ユーザー、ログはあなたのマシンに留まり、当方のマシンに触れることはありません。
