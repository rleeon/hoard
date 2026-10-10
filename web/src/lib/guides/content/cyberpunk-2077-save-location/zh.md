---
title: "赛博朋克 2077（Cyberpunk 2077）存档位置（PC 与 Steam Deck）"
description: "Cyberpunk 2077 在 Windows、Steam Deck 和 Mac 上的存档位置，各文件夹里有什么，以及如何备份存档并在 PC 之间同步。"
order: 20
updated: 2026-10-09
---

在 Windows 上，Cyberpunk 2077 把存档放在 `%USERPROFILE%\Saved Games\CD Projekt Red\Cyberpunk 2077`，每个存档一个文件夹。这是简短的答案。本页其余部分介绍 Steam Deck 和 Mac 上的路径、文件夹里实际有什么，以及如何让它一直有备份，并在 PC 和 Steam Deck 之间保持同步。

## Cyberpunk 2077 的存档位置

- **Windows**（Steam、GOG 或 Epic）：`%USERPROFILE%\Saved Games\CD Projekt Red\Cyberpunk 2077`
- **Steam Deck 和 Linux**（Proton）：`~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077`
- **Mac：**`~/Library/Application Support/CD Projekt Red/Cyberpunk 2077/Saves`

Windows 和 Mac 的路径来自 CD Projekt Red 自己的支持页面。在 Steam Deck 上，游戏运行在一个 Proton 前缀里，也就是 Steam 为每款游戏单独维护的一套小型 Windows 目录树；`1091500` 是 Cyberpunk 的 Steam 应用 ID。如果游戏装在 microSD 卡上，请到卡上的 `steamapps/compatdata/1091500` 查找。

## 文件夹里有什么

Cyberpunk 不是写一个存档文件，而是**每个存档一个文件夹**：`AutoSave-0`、`AutoSave-1` 依次往后，`ManualSave-0`、`ManualSave-1`，以及 `QuickSave-0`。每个文件夹里都有存档本体（`sav.dat`），以及读档菜单显示的截图和元数据。

由此可以得出两点：

- **备份上一级文件夹，而不是单个存档。** 只复制最新的 `ManualSave` 会漏掉自动存档，而它们往往才是最新的进度。
- **自动存档会轮换。** 游戏只循环使用少数几个 `AutoSave` 文件夹，并覆盖最旧的那个。三小时前的自动存档通常已经没了，所以在游戏之外保留历史是值得的。

设置不在这里。画面和操作设置在 `%LOCALAPPDATA%\CD Projekt Red\Cyberpunk 2077` 下的 `UserSettings.json` 中，和缓存、日志放在一起。很多人（以及一些工具）会误备份这个文件夹：里面没有任何丢了会损失进度的东西。

## Cyberpunk 2077 有云存档吗？

有。Steam 版用 Steam 云，GOG 版用 GOG Galaxy 的云。两者都会在同一商店的设备之间同步存档的最新状态。

两者都做不到的：

- **保留旧版本。** 如果存档损坏或被 Mod 弄坏，云端也会是坏掉的那份。
- **跨商店。** Steam 云和 GOG 的云互不相通，尽管 Steam、GOG 和 Epic 的 PC 存档只要复制文件夹过去，在任何一个版本里都能正常读取。

## 手动备份

1. 完全关闭游戏。
2. 把上面路径中的整个 `Cyberpunk 2077` 文件夹复制到 U 盘、另一块硬盘或云盘文件夹。
3. 恢复时，关闭游戏，把文件夹复制回去并替换原有内容。

这样可行，但只能在你记得的时候做，而且手上只有上一次的那份。

## 用 Hoard 自动备份和同步

[Hoard](/download) 会在你每次停止游戏时备份存档文件夹，并保留每一个版本，所以损坏的存档或已经被轮换掉的自动存档只需点一下就能找回。它还会在你的 PC 和 Steam Deck 之间同步这个文件夹。

1. 安装 Hoard 并登录，或把它指向[你自己的服务器](/guides/self-host-hoard)。
2. 打开**库**。Cyberpunk 会通过你的 Steam 库和社区存档数据库被识别出来。
3. 确认显示的文件夹是 `Saved Games\CD Projekt Red\Cyberpunk 2077`。如果显示的是 `AppData\Local` 下的文件夹，请改掉：那里只有设置。
4. 开始游戏。退出后，第一个版本就会出现在历史记录里。

Hoard 跟踪整个文件夹，所以每个 `AutoSave`、`ManualSave` 和 `QuickSave` 都会进入同一个版本。有 Deck 和台式机时，最新版本会在你下一台拿起的设备上等着你——参见[PC 之间同步的原理](/guides/sync-game-saves-across-pcs)。

<!-- faq -->

## 常见问题

### Steam Deck 上的 Cyberpunk 2077 存档在哪里？

在游戏的 Proton 前缀里：`~/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/users/steamuser/Saved Games/CD Projekt Red/Cyberpunk 2077`。如果游戏装在 microSD 卡上，`compatdata` 文件夹就在卡上。

### 能把 Cyberpunk 2077 的存档从 GOG 转到 Steam 吗？

能。Steam、GOG 和 Epic 的 PC 存档是通用的。在游戏关闭时，把存档文件夹复制到另一个安装的相同路径下，它们就会出现在读档菜单里。

### 为什么有这么多 AutoSave 文件夹？

游戏保留几个自动存档槽位，每次覆盖最旧的那个。它们是普通存档，只是会被自动替换。

### 为什么我较早的自动存档不见了？

因为它所在的槽位被重新使用了。游戏只保留少数几个。一旦被轮换掉，只有能保留版本的备份工具才能把它找回来。

### Hoard 也会同步我的设置吗？

不会。设置位于另一个不属于存档的文件夹，所以每台设备保留各自的设置，这通常正是你想要的：Deck 和台式机需要不同的画面设置。详见[在 PC 之间同步存档](/guides/sync-game-saves-across-pcs)。
