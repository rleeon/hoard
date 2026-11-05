---
title: "GameSave Manager 替代方案：自动备份与同步，Linux 也能用"
description: "在找 GameSave Manager 的替代品？Hoard 会自动备份每个存档，在多台 PC 和 Steam Deck 之间同步，并且支持 Linux 和 macOS。"
order: 27
updated: 2026-11-05
---

叫 Game Save Manager 的程序有三个，而且三个都只在一台 Windows 电脑上备份存档。如果你想要自动进行、并能跟着你去其他电脑、Steam Deck 或 Linux 的备份，[Hoard](/download) 免费、开源，正是为此而做的。

## 你用的是哪个 Game Save Manager

- **GameSave Manager**：Windows 上的老牌闭源工具。它有自己的游戏数据库，可以手动或按计划备份到一个归档文件里；它的 **Sync & Link** 会把存档文件夹移到 Dropbox、OneDrive 或 Google Drive，并在原处留下一个链接。
- **Game Save Manager (GSM)**：dyang886 开发，GPL-3.0，Windows 平台。它借助 PCGamingWiki 的数据库找到已安装的游戏，为每款游戏轮换保留一组本地备份，并能用 `.gsmr` 归档把历史带到新电脑上。
- **game-save-manager**：mcthesw 开发，AGPL-3.0，Windows 平台，在中文社区很受欢迎。它按计划备份你添加的文件夹，并可通过 WebDAV 在本机之外再存一份。

## 它们做得好的地方

- **简单、本地。** 选好游戏，点一下备份，就得到一个归档。不需要账号，也没什么要设置。
- **Windows 上检测得好。** 庞大的游戏数据库能找到大多数已安装的游戏，不用你自己去找路径。
- **换新电脑时搬得干净。** 一个导出的归档就能把一切带走。

如果你只在一台 Windows 电脑上玩，也不介意点按钮，哪个都能胜任。

## Hoard 多做了什么

| | Game Save Manager 类工具 | Hoard |
|---|---|---|
| 何时备份 | 你点按钮时，或按计划 | 游戏每次存档时，退出时再备份一次 |
| 多台 PC 同步 | 导出再导入，或通过 Sync & Link 用云盘文件夹 | 自动：最新的存档已在下一台机器上等你 |
| 另一台 PC 可以关机 | 只能通过云盘文件夹 | 可以，存档在服务器上等着 |
| 历史 | 本地备份，或云盘保留多少就有多少 | 所有版本，带去重，所有机器上都有 |
| 写到一半的存档 | 游戏还在写入时，云盘客户端可能就同步了链接的文件夹 | 绝不上传写到一半的文件，也从不替换正在运行的游戏的存档 |
| 平台 | Windows | Windows、Linux、macOS 和 SteamOS |
| 许可 | 闭源、GPL-3.0 或 AGPL-3.0 | AGPL-3.0，免费 |

大家搜的其实是 Linux 那一行。Game Save Manager 类工具没有一个能在 Linux 或 Steam Deck 上运行。Hoard 可以，而且是原生的：四个系统用同一个应用，还能识别 Proton 和 Wine 前缀，所以同一款游戏在 Windows 电脑上的存档和在 Deck 上的存档，会成为同一段历史里的两个版本。

## 如何迁移

1. 安装 [Hoard](/download)，登录我们推荐给所有人的 Hoard One，或者把它指向[你自己的服务器](/guides/self-host-hoard)。
2. 如果你用过 **Sync & Link**，先把存档文件夹放回原来的位置，免得云盘客户端和 Hoard 同步同一批文件。
3. 打开**库**。Hoard 会借助社区存档数据库和你的启动器找到游戏；它漏掉的文件夹可以手动添加。
4. 开始玩。游戏一存档，第一个版本就会出现在历史里。在其他机器上用同一账号安装 Hoard，[多台 PC 之间的同步](/guides/sync-game-saves-across-pcs)就会自动进行。

旧的 Game Save Manager 归档可以原样留着，当作冷备份。

## Hoard One，或者你自己的服务器

无论哪种方式 Hoard 都免费，两种方式用的也是同一个应用：

- **Hoard One**，推荐所有人使用。无需架设，也无需维护：一台位于欧盟的托管服务器，2 GB、三台设备免费，无需绑卡。[Pro](/pricing) 只是增加空间，提供 100 GB 和不限数量的设备，供存档装不下时使用。
- **你自己的服务器。** 在 PC、NAS 或树莓派上运行 `hoard-server`，你所有的机器都通过它同步。不需要我们的账号，没有发往我们的遥测，也没有我们设的配额：什么都不经过我们的服务器，因为路径上根本没有我们的东西。参见[如何自托管 Hoard](/guides/self-host-hoard)。

同一个程序，同样的识别，同样的历史记录。Hoard One 永久保留每一个版本；在你自己的服务器上，保留多少历史由你决定。

<!-- faq -->

## 常见问题

### 有 Linux 版的 GameSave Manager 吗？

没有：GameSave Manager 只支持 Windows，两个开源的 Game Save Manager 也一样。Hoard 在 Linux 和 SteamOS 上原生运行，能找到 Proton 和 Wine 前缀里的存档，并与你的 Windows 电脑同步。

### Ludusavi 还是 Game Save Manager？

两者都是本地备份工具。Ludusavi 开源，支持 Windows、Linux 和 macOS，基于社区存档清单；Game Save Manager 类工具只支持 Windows。如果你想要备份加同步，又不想自己接 Rclone，Hoard 使用同样的社区数据，并会自动同步。另有一篇完整的 [Ludusavi 对比](/guides/ludusavi-alternative)。

### 旧的备份还能留着吗？

能。Hoard 不会碰它们。把这些归档留作冷备份；从第一次存档开始，由 Hoard 自己的历史接手。

### Hoard 免费吗？

免费。应用、命令行和服务器都免费且开源（AGPL-3.0），运行你自己的服务器除了机器本身不花一分钱。我们推荐所有人使用的托管服务器 Hoard One 同样免费，提供 2 GB 和三台设备；Pro 每月 1.99 欧元，增加空间。

### 自托管会向 Hoard 发送任何东西吗？

不会。在你自己的服务器上，没有我们的账号，也没有发往我们的遥测：你的存档、你的用户和你的日志都留在你的机器上，从不接触我们的机器。
