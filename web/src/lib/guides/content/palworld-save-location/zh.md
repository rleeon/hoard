---
title: "幻兽帕鲁（Palworld）存档位置（PC 与 Steam Deck）"
description: "Palworld 的世界在 PC 和 Steam Deck 上存放在哪里，每个文件的作用，联机世界如何运作，以及如何备份存档并在 PC 之间同步。"
order: 24
updated: 2026-10-09
---

在 PC（Steam）上，Palworld 把存档放在 `%LOCALAPPDATA%\Pal\Saved\SaveGames\<你的 Steam ID>`，里面每个世界一个文件夹。这是 Pocketpair 在官方 FAQ 中给出的路径。下面是 Steam Deck 上的路径、每个文件的作用、联机时有什么不同，以及如何让你的世界一直有备份，并在 PC 和 Steam Deck 之间保持同步。

## Palworld 的存档位置

- **Windows，Steam 版：**`%LOCALAPPDATA%\Pal\Saved\SaveGames\<你的 Steam ID>\<世界 ID>`
- **Steam Deck 和 Linux**（Proton）：`~/.local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames/`

Steam ID 文件夹是一个与你的 Steam 账号绑定的长数字。里面每个你创建的世界都有自己的文件夹，名字是一长串十六进制字符。在 Steam Deck 上游戏通过 Proton 运行，所以存档位于 Steam 为它维护的 Proton 前缀中；`1623730` 是游戏的 Steam 应用 ID。

Xbox 应用 / Game Pass 版把存档放在另一个打包好的位置，与在 Steam 安装之间复制的文件不是一回事。

## 世界文件夹里有什么

- **`Level.sav`** 就是世界本身：你的据点、地图、放置在其中的帕鲁。
- **`LevelMeta.sav`** 保存世界在菜单中显示的名称和概要。
- **`Players\`** 为每个进入过该世界的玩家保存一个 `.sav`。
- **`LocalData.sav`** 和 **`WorldOption.sav`** 保存本地数据和世界设置。
- **`backup\`** 是游戏自己为世界做的自动备份。

设置在别处：`Pal\Saved\Config\Windows\GameUserSettings.ini` 保存画面和操作设置，`Pal\Saved\Logs` 保存日志。它们都不属于你的进度。

备份时请复制**整个世界文件夹**，而不只是 `Level.sav`。世界和玩家文件是一体的，只恢复其中一个，角色就会和所在的世界对不上。

## 联机与专用服务器

联机时，**世界保存在房主的 PC 上**。你在那个世界里的角色是房主 `Players` 文件夹中的一个文件，而不在你的设备上。如果房主丢了存档，所有人在那个世界的进度都会一起丢失。在专用服务器上，世界保存在服务器上。

所以对于共享的世界，需要备份的是房主的文件夹。

## Palworld 有云存档吗？

Steam 版使用 Steam 云，会在同一账号的设备之间同步世界的最新状态。它不保留旧版本，而游戏自带的 `backup\` 文件夹和存档在同一块硬盘上，硬盘坏了两者会一起没。

## 手动备份

1. 完全关闭游戏。
2. 把 `SaveGames` 中你的 Steam ID 文件夹（包含你所有的世界）复制到安全的地方。
3. 恢复时，关闭游戏，把它复制回原来的位置。

## 用 Hoard 自动备份和同步

[Hoard](/download) 会在你每次停止游戏时备份你的世界，并把每个版本保存在本机之外，所以世界损坏或硬盘丢失，都不会让你花了几周建起来的据点就此完蛋。它还会在你的 PC 和 Steam Deck 之间同步这些世界。

1. 安装 Hoard 并登录，或把它指向[你自己的服务器](/guides/self-host-hoard)。
2. 打开**库**。Palworld 会通过你的 Steam 库和社区存档数据库被识别出来。
3. 开始游戏。退出后，第一个版本就会出现在历史记录里。

如果你是联机房主，最重要的就是这台设备：备份好房主，共享的世界也就有了保障。要把世界回滚，请[恢复旧版本](/guides/restore-a-game-save)。

<!-- faq -->

## 常见问题

### Steam Deck 上的 Palworld 存档在哪里？

在 Proton 前缀里：`~/.local/share/Steam/steamapps/compatdata/1623730/pfx/drive_c/users/steamuser/AppData/Local/Pal/Saved/SaveGames/` 下以你的 Steam ID 命名的文件夹。

### 我在朋友世界里的角色保存在哪里？

在房主的 PC 上，那个世界的 `Players` 文件夹里。你的 PC 不会保留别人托管的世界的副本。

### 哪个文件是我的世界？

`Level.sav`，但请备份整个世界文件夹：玩家文件和世界是一体的。

### Palworld 会自动备份我的世界吗？

它会在世界的 `backup` 文件夹里保留自动备份。但它们和存档在同一块硬盘上，所以只能防存档损坏，防不了硬盘丢失。
