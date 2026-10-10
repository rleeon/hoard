---
title: "DockerでHoardをセルフホストする方法"
description: "Docker ComposeでHoardサーバーを自前で運用し、すべてのデバイスのセーブをそこで同期。無料・オープンソースで、当社のアカウントも容量制限も不要です。"
order: 0
featured: true
updated: 2026-10-09
---

Hoard はオープンソースでセルフホスト可能です。Hoard Cloud を使う代わりに、同じ `hoard-server` を自分のマシンで動かし、すべての端末をそこへ接続できます。アカウントは不要で、容量制限は与えたディスク容量だけです。このガイドでは Docker を使って数分でサーバーを立ち上げます。

## なぜ Hoard をセルフホストするのか

- **完全な所有権。** セーブデータは他人のクラウドではなく、自分が管理するハードウェアに保存されます。
- **容量制限なし。** 容量は自分のディスクだけが上限です。
- **同じアプリ、同じ機能。** バージョン履歴とバックグラウンド同期は Hoard Cloud とまったく同じように動作し、変わるのはバックエンドだけです。
- **オープンソース。** サーバーを読み、監査し、改変できます。

これが [Ludusavi](/guides/ludusavi-alternative) のようなツールとの決定的な違いです。Ludusavi はローカルバックアップや Rclone 経由の「自分のクラウドを持ち込む」方式に優れていますが、同期は自分で組む必要があります。Hoard は一度立ち上げればすべての端末が接続できる、管理された同期サーバーを提供します。

## セルフホストがデータにとって何を意味するか

多くの比較が Hoard について誤解している点なので、はっきり書きます。

**Hoard Cloud** はマネージドな選択肢です。サインインすると、セーブは EU にある当方のサーバーに置かれます。

**セルフホストした Hoard は完全にあなたのものです。** あなたの端末は自分のサーバーとだけ通信し、他のどこにも接続しません。**当方のアカウントも、当方へのテレメトリも、容量制限も、中継もありません。** 経路上に当方のものが何一つないため、当方のサーバーを何も通りません。セーブもゲーム名もメールアドレスも見えません。そもそも届かないからです。仮に明日 Hoard Cloud が終了しても、あなたの構成はそのまま動き続けます。

正確を期して 1 点だけ。あなたのサーバーには確かにログインがあります。下で作成するユーザーと、端末ごとのトークンです。それらはあなたのもので、あなたのマシンの、あなたのデータベースの中にあります。存在しないのは「当方のアカウント」です。

## 必要なもの

- 常時稼働するマシン（自宅サーバー、Docker が動く NAS、または小さな VPS）。
- Docker と Docker Compose がインストール済みであること（Synology NAS なら Container Manager パッケージ）。
- 任意で、HTTPS 用のドメインとリバースプロキシ（LAN を越える用途では推奨）。

## Docker Compose でインストール

リポジトリをクローンする必要はありません。サーバーはビルド済みのイメージ（`ghcr.io/rleeon/hoard`、amd64 と arm64）なので、ダウンロードするのは `docker-compose.yml` だけです。

```sh
mkdir hoard && cd hoard
curl -O https://raw.githubusercontent.com/rleeon/hoard/main/deploy/docker/docker-compose.yml

# Used only on the first start: they create that admin and print a device token in the log, once
HOARD_ADMIN_USERNAME=alice HOARD_ADMIN_PASSWORD='CHANGE_ME' docker compose up -d
docker compose logs -f server                  # wait for "listening", then copy the token
```

初回起動時に、コンテナは動作する `config.toml` を compose ファイルの隣の `./config/` に書き出します。素の `docker compose` なら事前の準備は何もいりません。データは名前付き Docker ボリューム（`hoard-data`）に保存されるので、他のボリュームと同様にバックアップしてください。コンテナは内部でポート `12421` を待ち受けます。別のホストポートを使うには `HOARD_PORT=9000 docker compose up -d` とします。

起動前に設定を読んでおきたい場合や、イメージを自分でビルドしたい場合は、[リポジトリのセルフホストガイド](https://github.com/rleeon/hoard/blob/main/SELF-HOST_GUIDE.md)にクローンする手順があります。

### Synology NAS の場合（Container Manager）

Container Manager には、この 2 つの変数を渡すコマンドラインがありません。また、バインドマウントするフォルダーが存在しないとプロジェクトを起動しません。これが `Bind mount failed: '…/config' does not exist` エラーの原因です。次の 4 ステップで両方とも解決します。

1. File Station で Hoard 用のフォルダー（例：`docker/hoard`）を作り、その中に空の `config` フォルダーを作ります。
2. Container Manager で **Project** → **Create** を開き、パスにそのフォルダーを指定して `docker-compose.yml` の作成を選び、ファイルの内容を貼り付けます（内容は、上の `curl` 行の URL をブラウザーで開くと確認できます）。
3. 貼り付けたファイルで `${HOARD_ADMIN_USERNAME:-}` と `${HOARD_ADMIN_PASSWORD:-}` を管理者のユーザー名とパスワードに置き換えます。各行のそれ以外の部分はそのままにして、次のようにします。

   ```yaml
   HOARD_ADMIN_USERNAME: alice
   HOARD_ADMIN_PASSWORD: 'CHANGE_ME'
   ```

4. ウィザードを完了して起動したら、**Container** で `hoard-server` のログを開きます。端末トークンはそこに一度だけ表示されます。

ファイルをそのまま使うと、セーブは共有フォルダーの外にある Docker 自身のストレージに保存されます。すでにバックアップしている共有フォルダーに置きたい場合は、`config` の隣に `data` フォルダーも作り、`hoard-data:/var/lib/hoard` の行を `./data:/var/lib/hoard` に変更してください。

## ユーザーと端末トークンを作成

`HOARD_ADMIN_USERNAME` と `HOARD_ADMIN_PASSWORD` を付けて起動した場合、これはもう済んでいます。ユーザーは作成済みで、トークンはログにあります。そうでない場合は、コマンドラインで作成します（サーバーにサインアップ画面はありません）。

```sh
docker compose exec server hoard-admin --config /etc/hoard/config.toml \
    user create alice --admin --password 'CHANGE_ME'
docker compose exec server hoard-admin --config /etc/hoard/config.toml \
    token create alice --device 'desktop'
```

トークンは一度だけ表示され、**後から取得することはできません**。今すぐコピーしてください。あとから端末を追加するときはターミナル不要です。ブラウザーでサーバーのアドレスを開き、そのユーザー名とパスワードで Web パネルにログインして、**ユーザー** → **トークンを発行** を使います。

## デスクトップアプリを接続

各マシンに [Hoard デスクトップアプリ](/download) をインストールします。オンボーディングで **セルフホスト** を選び、サーバーの URL と作成したトークンを貼り付けます。あとは Hoard Cloud とまったく同じで、ゲームを検出し、自動でバックアップし、バージョン履歴を保持します。日常的な使い方は [複数の PC 間でセーブを同期する](/guides/sync-game-saves-across-pcs) を参照してください。

## サーバーを最新に保つ

更新の方法はインストールの仕方によって変わります。しかも間違ったコマンドはエラーにならず、ただ何も起きないだけなので、自分がどれに当てはまるかを知っておく価値があります。

**Docker Compose.** 新しいイメージを取得し、コンテナを作り直します。次の順番で、両方とも実行してください。

```sh
docker compose pull
docker compose up -d
```

最初のコマンドで止めると、古いコンテナがそのまま動き続けます。`/v1/health` は古いバージョンを返し続け、更新が黙って失敗したように見えます。`git pull` はどちらも更新しません。動いているのは公開イメージであって、あなたのチェックアウトではないからです。新しいイメージが来るタイミングを自分で決めたい場合は、`:latest` の代わりにバージョンを固定してください（`ghcr.io/rleeon/hoard:1.1`）。

**Unraid.** *Docker* タブ → Hoard → 更新が出たら *Apply update*。入力するものはありません。

**ベアメタル（systemd）.** `sudo hoard-server upgrade` を実行し、続けて `sudo systemctl restart hoard-server`。バイナリをアトミックに入れ替えますが、進行中の同期を切らないよう、サービスの再起動は意図的に行いません。

`hoard-server upgrade` はベアメタルのインストール専用です。コンテナ内では意図的に実行を拒否し（入れ替えたバイナリは次の `docker compose up -d` で消えてしまうため）、代わりに上の 2 つのコマンドを表示します。実際に確かめたい場合は `docker compose exec server hoard-server upgrade` を実行してください。データベースのマイグレーションは起動時にサーバーが適用するので、そのための別の手順はありません。

## 本番運用

ローカルネットワークを越えて公開する場合は、リバースプロキシ（Caddy、nginx、Traefik）で TLS を終端します。ベアメタルがよい場合は、リポジトリに `systemd` インストールスクリプトと、進行中の同期を止めずにバイナリをアトミックに入れ替える `hoard-server upgrade` コマンドも含まれています。

## セルフホストと Hoard Cloud のどちら？

すでにサーバーを運用していて容量制限なしの完全な管理を望むなら、セルフホストが最適です。インフラの保守をしたくない場合は、[Hoard Cloud](/pricing) が同じ同期をこちらで管理して提供し、無料プランから始められます。どちらでもアプリとセーブデータは可搬性を保つので、後から切り替えられます。

<!-- faq -->

## よくある質問

### セルフホストした Hoard は外部に通信しますか？

いいえ。デスクトップアプリは、あなたが指定したサーバーのアドレスとだけ通信します。セーブもユーザーもログもあなたのマシンにとどまり、そのいずれも当方には届きません。

### セルフホストのサーバーは Hoard Cloud と同じコードですか？

はい。AGPL-3.0 の同じ `hoard-server` バイナリです。機能を削ったコミュニティ版もなければ、ホスト版だけの機能もありません。

### セーブは実際どこに保存されますか？

既定では、コンテナに与えた Docker ボリューム、つまりあなた自身のディスクです。すでにオブジェクトストレージを運用しているなら、サーバーは S3 も話せるので、MinIO、Garage、Backblaze B2 を保存先にできます。いずれの場合も、端末が通信する相手はあなたのサーバーだけです。

### NAS で動かせますか？

はい、Docker が動く NAS なら動きます。Synology では上の Container Manager の手順に従ってください。Unraid 用にはリポジトリにテンプレートが同梱されています。いずれの場合も、イメージは指定した `PUID`/`PGID` に降格するので、バインドマウントしたフォルダーの所有者が root ではなく適切なユーザーになります。

### ドメインと HTTPS は必要ですか？

自宅の LAN 内だけなら不要です。サーバーが外部から到達可能になった時点で、前段にリバースプロキシを置いて TLS を終端してください。Caddy、nginx、Traefik のいずれでも構いません。

### プレイ終了時にサーバーが落ちていたら？

スナップショットはローカルで作られるので、失われるものはありません。サーバーが応答を再開すると自動でアップロードされます。

### Hoard Cloud で始めて、後から移れますか？

はい、双方向に移れます。アカウントページからすべてをエクスポートでき、アプリは再インストールなしで別のサーバーを指すように変更できます。
