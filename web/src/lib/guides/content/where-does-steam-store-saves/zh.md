---
title: "Steam 游戏存档位置在哪里？PC 和 Steam Deck 上的所有位置"
description: "Steam 有一个 userdata 文件夹，但大多数游戏把存档放在 AppData、文档或自己的文件夹里。教你在 Windows、Steam Deck 和 Mac 上去哪里找。"
order: 30
updated: 2026-11-05
---

Steam 有一个自己的存档文件夹，即 Steam 文件夹里的 `userdata\<account-id>\<AppID>\remote`，但大多数游戏根本不用它。社区存档数据库收录了 10,937 款 Steam 游戏在 Windows 上的存档路径，其中只有**大约十分之一**把存档放在这里。其余的写进 AppData、文档或自己的安装文件夹；如果游戏支持 Steam 云，Steam 会直接从文件所在的位置同步。

## Steam 自己的文件夹：userdata

- **Windows：** `C:\Program Files (x86)\Steam\userdata\<account-id>\<AppID>\remote`
- **Steam Deck 与 Linux：** `~/.local/share/Steam/userdata/<account-id>/<AppID>/remote`
- **macOS：** `~/Library/Application Support/Steam/userdata/<account-id>/<AppID>/remote`

`<account-id>` 就是 Steam 显示的你的**好友代码**（好友 → 添加好友）。在这台机器上登录过的每个账号各有一个文件夹。`<AppID>` 是游戏商店地址里的数字：`store.steampowered.com/app/413150` 就是 Stardew Valley。

`remote` 上一层、只以 `<AppID>` 命名的文件夹里还有 `remotecache.vdf`。那是 Steam 自己的记录文件，不是你的存档，在 PC 和 Deck 上内容不同是正常的。

## 大多数游戏实际把存档放在哪里

2026 年 11 月 5 日在同一个数据库里统计的、使用各位置的 Steam 游戏占比（有些游戏同时用好几个）：

- **`AppData\LocalLow`**，23%，大多是 Unity 制作的游戏：`%USERPROFILE%\AppData\LocalLow\<开发商>\<游戏>`
- **游戏安装文件夹**，22%：`C:\Program Files (x86)\Steam\steamapps\common\<游戏>`
- **`AppData\Local`**，21%：`%LOCALAPPDATA%\<开发商或游戏>`
- **`文档`**，15%，常在 `My Games` 下：`%USERPROFILE%\Documents\My Games\<游戏>`
- **`AppData\Roaming`**，12%：`%APPDATA%\<游戏>`
- **Steam 的 `userdata`**，10%：见上文
- **`保存的游戏`**，3%：`%USERPROFILE%\Saved Games\<开发商>\<游戏>`

## 在 Steam Deck 上：Proton 前缀

Windows 游戏在 Deck 上通过 Proton 运行，每款游戏都在 `~/.local/share/Steam/steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/` 里有一套自己的小 Windows。AppData、文档和保存的游戏都在里面，所以 PC 上的 `%APPDATA%\Game` 在 Deck 上就是 `compatdata/<AppID>/pfx/drive_c/users/steamuser/AppData/Roaming/Game`。前缀会在游戏第一次启动时创建。

有 Linux 原生版本的游戏不走前缀，通常存档在 `~/.local/share` 或 `~/.config` 下。安装文件夹是 `~/.local/share/Steam/steamapps/common/<游戏>`；如果游戏装在 microSD 卡上，就是卡上同样的 `steamapps` 目录。

## 快速找到某款游戏的存档

1. 在[存档位置列表](/saves)里查这款游戏，或者看它在 PCGamingWiki 页面上的“Save game data location”一节。
2. 如果它用 Steam 的文件夹，就用商店页面上的 AppID 打开 `userdata\<account-id>\<AppID>\remote`。
3. 如果不是，先在游戏里存一次档，再把 `%LOCALAPPDATA%`、`%USERPROFILE%\AppData\LocalLow`、`%APPDATA%` 和文档按修改日期排序。刚刚变过的那个文件夹就是。

## Steam 云会保留什么

对于开发商开启了 Steam 云的游戏，Steam 云会把最新的存档保存在你的 Steam 账号里。Steam 的[远程存储页面](https://store.steampowered.com/account/remotestorage)列出了你 Steam 云里有文件的所有游戏，并且可以下载。

它只保留最新状态，没有历史，所以损坏的存档会和好存档一样被忠实同步；而且它只覆盖 Steam 游戏：GOG、Epic、模拟器或非 Steam 快捷方式都不在内。详见 [Steam 云替代方案指南](/guides/steam-cloud-alternative)。

## 用 Hoard 自动备份和同步

[Hoard](/download) 免费且开源。无论游戏把存档放在哪里，userdata、AppData、文档、安装文件夹，还是 Deck 上的 Proton 前缀，它都能找到；每次游戏写入存档时备份，退出时再备份一次，保留可以随时回退的历史记录，并在你的 PC 和 Steam Deck 之间同步。在 Steam 的文件夹里，它追踪的是 `<AppID>/remote/`，不碰 `remotecache.vdf`，所以你的 PC 和 Deck 永远不会看起来互相冲突。

1. 安装 Hoard 并登录 Hoard One（推荐所有人使用），或者把它指向[你自己的服务器](/guides/self-host-hoard)。
2. 打开**游戏库**。你的 Steam 游戏会通过你的 Steam 库和社区存档数据库被识别出来。
3. 开始游戏。退出后，第一个版本就会出现在历史记录里。

## Hoard One，或者你自己的服务器

无论哪种方式 Hoard 都免费，两种方式用的也是同一个应用：

- **Hoard One**，推荐所有人使用。无需架设，也无需维护：一台位于欧盟的托管服务器，2 GB、三台设备免费，无需绑卡。[Pro](/pricing) 只是增加空间，提供 100 GB 和不限数量的设备，供存档装不下时使用。
- **你自己的服务器。** 在 PC、NAS 或树莓派上运行 `hoard-server`，你所有的机器都通过它同步。不需要我们的账号，没有发往我们的遥测，也没有我们设的配额：什么都不经过我们的服务器，因为路径上根本没有我们的东西。参见[如何自托管 Hoard](/guides/self-host-hoard)。

同一个程序，同样的识别，同样的历史记录。Hoard One 永久保留每一个版本；在你自己的服务器上，保留多少历史由你决定。

<!-- faq -->

## 常见问题

### Steam 的 userdata 文件夹在哪里？

在 Steam 文件夹里：如果 Steam 装在默认位置，Windows 上是 `C:\Program Files (x86)\Steam\userdata`，Steam Deck 或 Linux 上是 `~/.local/share/Steam/userdata`，Mac 上是 `~/Library/Application Support/Steam/userdata`。

### 卸载游戏会删除存档吗？

放在安装文件夹里的存档会跟着一起删掉。AppData、文档或 userdata 里的存档通常会留下。在 Deck 上，卸载还可能删除游戏的 Proton 前缀，连同里面的所有存档，所以请先备份。

### 可以手动把 Steam 存档复制到另一台电脑吗？

可以。关闭游戏，把存档文件夹复制到另一台电脑的同一位置，然后在那边启动游戏。如果游戏使用 Steam 云，Steam 下次启动时可能会问你保留哪一份；选日期较新的那份。

### Hoard 免费吗？

免费。应用、命令行和服务器都免费且开源（AGPL-3.0），运行你自己的服务器除了机器本身不花一分钱。我们推荐所有人使用的托管服务器 Hoard One 同样免费，提供 2 GB 和三台设备；Pro 每月 1.99 欧元，增加空间。

### 自托管会向 Hoard 发送任何东西吗？

不会。在你自己的服务器上，没有我们的账号，也没有发往我们的遥测：你的存档、你的用户和你的日志都留在你的机器上，从不接触我们的机器。
