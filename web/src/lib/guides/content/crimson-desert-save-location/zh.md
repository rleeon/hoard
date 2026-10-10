---
title: "红色沙漠（Crimson Desert）存档位置（PC 与 Steam Deck）"
description: "Crimson Desert 在 Windows、Steam Deck 和 Mac 上的存档位置，哪个文件夹真正存放存档，以及如何备份存档并在 PC 之间同步。"
order: 21
updated: 2026-10-09
---

在 Windows 上，Crimson Desert 把存档放在 `%LOCALAPPDATA%\Pearl Abyss\CD\save`。这是 Pearl Abyss 在其官方 FAQ 中给出的文件夹。下面是 Steam Deck 和 Mac 上的路径、里面有什么，以及如何让它一直有备份，并在 PC 和 Steam Deck 之间保持同步。

## Crimson Desert 的存档位置

- **Windows：**`%LOCALAPPDATA%\Pearl Abyss\CD\save`（即 `C:\Users\<你的用户名>\AppData\Local\Pearl Abyss\CD\save`）
- **Steam Deck 和 Linux**（Proton）：`~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save`
- **Mac，Steam 版：**`~/Library/Application Support/Pearl Abyss/CD/save`
- **Mac，App Store 版：**`~/Library/Containers/com.pearlabyss.CrimsonDesert/Data/Library/Application Support/Pearl Abyss/CD/save`

游戏没有原生 Linux 版，所以在 Steam Deck 上通过 Proton 运行，存档位于 Steam 为它维护的 Proton 前缀中；`3321460` 是游戏的 Steam 应用 ID。如果装在 microSD 卡上，`compatdata` 文件夹就在卡上。

`AppData` 在 Windows 上是隐藏文件夹。最快的办法是把 `%LOCALAPPDATA%\Pearl Abyss\CD\save` 粘贴到文件资源管理器的地址栏。

## 文件夹里有什么

`save` 里有两个子文件夹。根据 Pearl Abyss 的说法，**名字是数字的那个存放你在游戏里创建的存档**。备份时请复制整个 `save` 文件夹，而不是挑文件：它不大，也不会漏掉游戏需要的东西。

Mac 上的 Steam 版和 App Store 版路径不同。如果在两者之间切换，请手动复制一次存档。

## Crimson Desert 有云存档吗？

有，Steam 版支持 Steam 云，会在同一 Steam 账号的设备之间同步最新的存档。

它做不到的：

- **保留旧版本。** Steam 云只保存当前状态。存档一旦损坏，同步过去的就是损坏的那份。
- **覆盖其他商店。** Mac App Store 版和 Steam 版不共享云。

## 手动备份

1. 完全关闭游戏。
2. 把上面路径中的整个 `save` 文件夹复制到安全的地方：另一块硬盘、U 盘或云盘文件夹。
3. 恢复时，关闭游戏，把它复制回去并替换原有内容。

在大版本更新或重装前做一次没问题。但要当成习惯，就得靠你记得，而且手上永远只有上一次的那份。

## 用 Hoard 自动备份和同步

[Hoard](/download) 会在你每次停止游戏时备份存档文件夹，并保留每一个版本，所以存档坏了或做了后悔的选择都能退回去。它还会在你的 PC 和 Steam Deck 之间保持这个文件夹同步。

1. 安装 Hoard 并登录，或把它指向[你自己的服务器](/guides/self-host-hoard)。
2. 打开**库**。Crimson Desert 会通过你的 Steam 库和社区存档数据库，在上面的路径被识别出来。
3. 开始游戏。退出后，第一个版本就会出现在历史记录里。

此后每次游玩都会增加一个版本，最新的那个会出现在你下一次坐下来的设备上。如果存档出了问题，[恢复旧版本](/guides/restore-a-game-save)只需点几下。

<!-- faq -->

## 常见问题

### Steam Deck 上的 Crimson Desert 存档在哪里？

在游戏的 Proton 前缀里：`~/.local/share/Steam/steamapps/compatdata/3321460/pfx/drive_c/users/steamuser/AppData/Local/Pearl Abyss/CD/save`。如果游戏装在 microSD 卡上，`compatdata` 文件夹就在卡上。

### 哪个子文件夹是我的存档？

`save` 里名字是数字的那个。不过还是请备份整个 `save` 文件夹，免得漏掉东西。

### 我找不到 AppData 文件夹，它在哪里？

它默认是隐藏的。把 `%LOCALAPPDATA%\Pearl Abyss\CD\save` 粘贴到文件资源管理器的地址栏并按回车，或者在“查看”菜单里显示隐藏的项目。

### 我能在台式机和 Steam Deck 上用同一个存档吗？

能。Steam 云会在同一 Steam 账号下同步最新的存档。Hoard 也能做到，而且每次游玩保留一个版本，出了问题可以退回去。
