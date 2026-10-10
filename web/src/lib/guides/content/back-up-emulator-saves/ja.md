---
title: "エミュレーターのセーブをバックアップ・同期する方法（RetroArch、Dolphin、PCSX2）"
description: "RetroArch、Dolphin、PCSX2、DuckStationなどのエミュレーターのセーブをPCとSteam Deck間でバックアップ・同期。履歴付き、保存場所の一覧も。"
order: 6
updated: 2026-10-09
---

エミュレーターのセーブは失われやすいものです。セーブファイルとセーブステートはあちこちのフォルダーに散らばり、再インストールや新しい PC で何年分もの進行が消えることがあります。Hoard はそれらを自動でバックアップし、Steam Deck を含むすべてのマシン間で同期し続けます。

## Hoard が対応するエミュレーター

Hoard は標準的なエミュレーターのセーブファイル（`.srm`、`.sav`、メモリーカード、ゲームごとのセーブフォルダー）とセーブステートを扱います。次のエミュレーターについては、セーブの場所を最初から把握しています。

- **ソニー系:** PCSX2（PS2）、DuckStation（PS1）、PPSSPP（PSP）、RPCS3（PS3）、shadPS4（PS4）、Vita3K（PS Vita）
- **任天堂系:** Dolphin（ゲームキューブ / Wii）、Cemu（Wii U）、Ryujinx・yuzu・Eden・Suyu・Citron・Sudachi（Switch）、Citra / Azahar（3DS）、melonDS（DS）、mGBA（GBA）、Project64（N64）
- **その他:** RetroArch（マルチシステム）、xemu（Xbox）、Flycast（ドリームキャスト）

Hoard は Ludusavi と同じコミュニティデータベースを使ってセーブフォルダーを探すため、多くのパスは自動で検出されます。独自の場所は、フォルダーを手動で指定できます。

## エミュレーターのセーブバックアップを設定する

1. Windows、macOS、Linux 用の **Hoard をインストール**してサインインします。
2. **ライブラリ** を開いてエミュレーターを追加します。既定の場所を変更している場合は、セーブ／ステートのフォルダーを手動で追加します。
3. **自動モード** をオンのままにします。Hoard は各セッション後にバックアップし、世代履歴を保持します。
4. 他の PC にも同じアカウントで Hoard をインストールすれば、それらのセーブがどこでも同期されます。[複数の PC 間でセーブを同期する方法](/guides/sync-game-saves-across-pcs)も参照してください。

## エミュレーターに Ludusavi？

Ludusavi もエミュレーターのセーブをローカルにバックアップでき、その用途なら優れた無料の選択肢です。さらにそのセーブをマシン間で自動同期し、Rclone を設定せずにクラウドでバージョン履歴を残したいなら、そこで Hoard が役立ちます。[Ludusavi と Hoard の詳しい比較](/guides/ludusavi-alternative)をどうぞ。

## エミュレーターごとのクラウドセーブ

以下の単体エミュレーターは、どれもマシン間でセーブを自分で同期しません。セーブはディスク上のただのファイルです。これは良い知らせで、正しいフォルダーを監視するツールなら何でも運べるということです。それぞれの保存場所は次のとおりです。「Steam Deck」は Discover ストアからインストールする Flatpak 版を指します。

### PCSX2 のクラウドセーブ（PS2）

PCSX2 はメモリーカード（`.ps2` ファイル）を `memcards/` に書き込みます。

- Windows: `Documents\PCSX2\memcards`
- Linux: `~/.config/PCSX2/memcards`
- Steam Deck: `~/.var/app/net.pcsx2.PCSX2/config/PCSX2/memcards`

1 枚のメモリーカードには、そのカードで遊んだ全ゲームのセーブが入っているため、1 つの単位として移動します。古いバージョンに戻すと、1 本のゲームではなくカード全体が巻き戻ります。

ファイル型とフォルダー型のカードを含む詳しい手順は [PCSX2 のクラウドセーブ](/guides/pcsx2-cloud-saves) を参照してください。

### Dolphin のクラウドセーブ（ゲームキューブと Wii）

ゲームキューブのセーブは `GC/`（メモリーカードのイメージ、またはカードごとのフォルダー）に、Wii のセーブはエミュレートされた NAND の `Wii/` にあります。

- Windows: `Documents\Dolphin Emulator\GC` と `\Wii`
- Linux: `~/.local/share/dolphin-emu/GC` と `/Wii`
- Steam Deck: `~/.var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu/`

GCI フォルダーと Wii のメモリーを含む詳しい手順は [Dolphin のクラウドセーブ](/guides/dolphin-cloud-saves) を参照してください。

### DuckStation のクラウドセーブ（PS1）

DuckStation はメモリーカードを `memcards/` に置き、既定ではゲームごとに別のカードを作ります。これは同期と相性抜群です。

- Windows: `Documents\DuckStation\memcards`（新しいバージョンは `%LOCALAPPDATA%\DuckStation\memcards`）
- Linux: `~/.local/share/duckstation/memcards`
- Steam Deck: `~/.var/app/org.duckstation.DuckStation/` の `data/` または `config/` 内

### RetroArch のセーブ同期

RetroArch は `saves/`（ゲーム内セーブ）と `states/`（セーブステート）を分けています。Hoard はセーブフォルダーを追跡します。ステートも使うなら `states/` を別のエントリーとして追加してください。

- Windows: `%APPDATA%\RetroArch`、ポータブル版なら `retroarch.exe` の隣
- Linux: `~/.config/retroarch`
- Steam Deck: `~/.var/app/org.libretro.RetroArch/config/retroarch`、EmuDeck で導入した場合は `~/Emulation/saves/retroarch`

RetroArch には、自分で用意した WebDAV サーバーとやり取りする Cloud Sync も組み込まれています。RetroArch だけを使い、すでに WebDAV があるなら妥当な選択です。Hoard は WebDAV が不要で、巻き戻せるバージョン履歴を保持し、単体エミュレーターもカバーします。

### PPSSPP（PSP）

セーブは `PSP/SAVEDATA`、ステートは `PSP/PPSSPP_STATE` に入ります。

- Windows: `Documents\PPSSPP\PSP\SAVEDATA`、ポータブル版なら実行ファイルの隣の `memstick\PSP\SAVEDATA`
- Linux: `~/.config/ppsspp/PSP/SAVEDATA`
- Steam Deck: `~/.var/app/org.ppsspp.PPSSPP/config/ppsspp/PSP/SAVEDATA`

### RPCS3（PS3）

セーブは `dev_hdd0/home/00000001/savedata` にあります。Windows では RPCS3 のフォルダー内、Linux と Steam Deck では `~/.config/rpcs3/` の下です。

### Switch エミュレーター: Ryujinx、yuzu、Eden、Suyu、Citron、Sudachi

Ryujinx はセーブを `bis/user/save`（`%APPDATA%\Ryujinx` または `~/.config/Ryujinx` の下）に置きます。yuzu 系は `%APPDATA%` または `~/.local/share` にある各自のフォルダー内の `nand/user/save` を使います。

ここには落とし穴があります。yuzu 系のツリーは `save/<アカウント>/<プロファイル>/<タイトルID>/` という構造で、プロファイル ID はエミュレーターの初回起動時に生成されるため、インストールごとに異なります。`save/` フォルダー全体を 2 台のマシンで同期すると、それぞれに相手のプロファイルが自分のものと並んで置かれ、どちらのゲームも相手の進行を認識しません。Hoard は代わりに各ゲーム自身のフォルダーまで降りていくので、プロファイル名が何であっても同じタイトルがマシン間で一致します。

### Citra と Azahar（3DS）

セーブは `sdmc/Nintendo 3DS/<id0>/<id1>/title/…` の深い場所にあり、`id0`/`id1` はエミュレートされた本体の鍵から作られるため、これもインストールごとに異なります。Hoard は Switch のツリーと同じように扱い、ゲームごとに 1 つのエントリーとしてマシン間で対応付けます。

### その他

- **Cemu（Wii U）:** `mlc01/usr/save`。`%APPDATA%\Cemu` または `~/.local/share/Cemu` の下。
- **shadPS4（PS4）:** `savedata`。`%APPDATA%\shadPS4` または `~/.local/share/shadPS4` の下。
- **Vita3K（PS Vita）:** データフォルダー内の `ux0/user/00/savedata`。
- **mGBA、melonDS など、カートリッジ時代のほとんどのエミュレーター:** 設定を変えていなければ ROM の隣に `.sav` が作られます。ROM フォルダーのセーブを手動で追加してください。

## Steam Deck でのエミュレーターのセーブ

Steam Deck ではエミュレーターは通常 Flatpak で入るため、フォルダーはいつもの `~/.config` や `~/.local/share` ではなく `~/.var/app/<id>/` の下にあります。EmuDeck はすべてを `~/Emulation/saves/` にエミュレーターごとのフォルダーでまとめます。いずれの場合も、フォルダーを一度追加すれば Hoard が監視します。

携帯機で大事なのは次の点です。Hoard のエンジンはバックグラウンドサービスとして動くため、ゲームモードでゲームを終了すると、ウィンドウを開かなくてもバックアップされます。デスクトップで遊んだ後に Deck を手に取れば、セーブはもう届いています。

## セーブデータとセーブステートは別物

移動するときの振る舞いが違うので、分けて考える価値があります。

- **セーブファイル**（`.srm`、メモリーカード、`SAVEDATA` フォルダー）は、エミュレートされたゲーム機が書き込むゲーム自身のセーブです。マシン間やエミュレーターのバージョン間を問題なく移動できます。
- **セーブステート**はエミュレーターのメモリのダンプです。エミュレーターのビルド、多くの場合は特定のコアに縛られるため、あるバージョンで作ったステートが別のバージョンでは読み込めないことがあります。

Hoard は両方をバックアップします。ただ、更新済みのマシンのステートが古いマシンで開けなくても驚かないでください。エミュレーターのバージョンを揃え、大事なものはセーブファイルに頼りましょう。

## エミュレーターは 1 つ、ゲームは多数

エミュレーターは数十本のタイトルを抱える 1 つのプロセスで、「実行中のゲーム」という単位で考えるツールにとって、これがエミュレーターのセーブを扱いにくくしています。Hoard はエミュレーター全体を 1 つの塊として扱わずにタイトルを分けるので、何かを起動するたびに変わる共通の山ではなく、ゲームごとに独自の履歴が残ります。セーブが壊れても、[以前のバージョンに戻す](/guides/restore-a-game-save)ことができます。

## 当方のサーバーを介さないエミュレーターのバックアップ

ここで説明したことはすべて、自分のサーバーでも同じように動きます。`hoard-server` を起動してアプリをそこに向ければ、セーブはあなたのマシンからあなたのディスクへ届きます。当社のアカウントも、当社へのテレメトリーも不要で、当社のサーバーを何も通りません。[Hoard をセルフホストする方法](/guides/self-host-hoard)を参照してください。

## ヒント

セーブステートは特定のエミュレーターのバージョンに依存します。同期したステートがどこでも正しく読み込めるよう、各 PC のエミュレーターを揃えて更新しましょう。

<!-- faq -->

## よくある質問

### Hoard は ROM もバックアップしますか？

いいえ。追跡するのはセーブフォルダーで、ゲームファイルではありません。ROM は大きく、変化せず、すでに手元にあるので、バージョン管理するものがありません。

### PCSX2、Dolphin、DuckStation にクラウドセーブは組み込まれていますか？

いいえ。セーブをローカルのフォルダーに書き込み、同期はユーザーに任せています。上に挙げたフォルダーに同期ツールを向ければ、セーブはマシン間でついてきます。

### RetroArch にクラウド同期はありますか？

はい。自分で運用またはレンタルする WebDAV サーバーが必要な Cloud Sync が組み込まれています。WebDAV を設定したくない、巻き戻せるバージョン履歴がほしい、単体エミュレーターも使う、という場合は Hoard が代わりになります。

### ゲームモードの Steam Deck でも動きますか？

はい。エンジンがバックグラウンドサービスとして動くので、ゲームを終了したときにウィンドウなしでセーブがバックアップされます。Flatpak や EmuDeck のフォルダーも他と同じように使えます。

### エミュレーターがポータブル版です。使えますか？

はい。実行ファイルの隣にあるフォルダーを手動で追加すれば、Hoard は他のセーブ場所と同じように追跡します。携帯機ではこれが一般的な構成です。

### 2 台の PC 間でセーブステートを同期できますか？

できますし、Hoard はそうします。ステートが読み込めるかどうかは両方のマシンのエミュレーターが同じバージョンかどうかにかかっており、これは同期ではなくエミュレーター側の制約です。セーブファイルにはこの問題がありません。

### リストにないエミュレーターでも使えますか？

ほぼ確実に使えます。一般的なものは自動で検出され、それ以外はセーブフォルダーを Hoard に指定すれば追加できます。

### セルフホストするとエミュレーターで何か変わりますか？

いいえ。検出も、バージョンも、同期も同じです。ストレージがあなたのものになるだけです。
