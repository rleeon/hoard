---
title: "博德之门 3（Baldur's Gate 3）存档位置（PC 与 Steam Deck）"
description: "Baldur's Gate 3 在 Windows、Steam Deck 和 Mac 上的存档位置，哪些是存档、哪些是 Mod 或设置，荣誉模式，以及如何备份和同步存档。"
order: 23
updated: 2026-10-09
---

在 Windows 上，Baldur's Gate 3 把存档放在 `%LOCALAPPDATA%\Larian Studios\Baldur's Gate 3\PlayerProfiles\Public\Savegames\Story`，每个存档一个文件夹。这是 Larian 在官方支持 FAQ 中给出的路径。下面是 Steam Deck 和 Mac 上的路径、存档旁边都有什么、荣誉模式，以及如何把一切都备份好，并在 PC 和 Steam Deck 之间保持同步。

## Baldur's Gate 3 的存档位置

- **Windows：**`%LOCALAPPDATA%\Larian Studios\Baldur's Gate 3\PlayerProfiles\Public\Savegames\Story`
- **Steam Deck 和 Linux**（Proton）：`~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story`
- **Mac：**`~/Documents/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story`

Larian 已经停止了原生 Linux 版，所以在 Steam Deck 上游戏通过 Proton 运行，存档位于 Steam 为它维护的 Proton 前缀中；`1086940` 是游戏的 Steam 应用 ID。如果装在 microSD 卡上，`compatdata` 文件夹就在卡上。

## 哪些是存档，哪些不是

每个存档都是 `Story` 里的**一个文件夹**，内含一个 `.lsv` 文件和一张缩略图。周围的其他东西都不是存档：

- **`Mods`**（在 `Baldur's Gate 3` 下）存放 Mod 文件。
- **`modsettings.lsx`**（在 `PlayerProfiles\Public` 下）是已启用 Mod 的列表及其加载顺序。
- **设置**，例如画面和操作，是放在档案旁边的配置文件，不属于存档。

最容易出问题的是 Mod。用 Mod 创建的存档在读取时，需要同样的 Mod 处于启用状态。如果你把带 Mod 的存档搬到另一台 PC，请把 Mod 列表一起带过去，否则游戏会提示缺少 Mod，存档也可能无法按预期读取。

## 荣誉模式

荣誉模式只有一个存档，游戏会在你游玩时不断覆盖它；如果队伍全灭，这次荣誉之旅就结束了（你可以在失去荣誉的前提下以自定义模式继续）。要不要备份这个存档由你决定：在一场硬仗之前留一份副本，从技术上说就是一条退路；有些玩家正是在崩溃或 bug 之后需要它，也有人认为这是在对这个模式作弊。备份工具无论如何都会保留版本；要不要恢复，就是你和骰子之间的事了。

## Baldur's Gate 3 有云存档吗？

有。在 Steam 上它使用 Steam 云，会在同一账号的设备之间同步最新的存档。它只保存当前状态：如果存档损坏，或被某个 Mod 的更新弄坏，同步过去的就是那个版本。

## 手动备份

1. 完全关闭游戏。
2. 复制上面路径中的整个 `PlayerProfiles` 文件夹（其中包含 `Savegames` 和 `modsettings.lsx`）。
3. 恢复时，关闭游戏，把它复制回去。

## 用 Hoard 自动备份和同步

[Hoard](/download) 会在你每次停止游戏时备份存档文件夹，并保留每一个版本，所以被 Mod 更新或补丁弄坏的存档，只需一次恢复就能找回。它还会在你的 PC 和 Steam Deck 之间保持这个文件夹同步。

1. 安装 Hoard 并登录，或把它指向[你自己的服务器](/guides/self-host-hoard)。
2. 打开**库**。Baldur's Gate 3 会通过你的 Steam 库和社区存档数据库被识别出来。
3. 开始游戏。退出后，第一个版本就会出现在历史记录里。

每次游玩都会增加一个包含所有存档的版本。想退回去时，打开历史记录并[恢复旧版本](/guides/restore-a-game-save)；你电脑上现有的内容会先被备份，所以试一个旧版本从来不是单程票。

<!-- faq -->

## 常见问题

### Steam Deck 上的 Baldur's Gate 3 存档在哪里？

在游戏的 Proton 前缀里：`~/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/users/steamuser/AppData/Local/Larian Studios/Baldur's Gate 3/PlayerProfiles/Public/Savegames/Story`。

### 能把带 Mod 的存档搬到另一台 PC 吗？

能，前提是另一台 PC 装了同样的 Mod，并以相同顺序启用。把 `modsettings.lsx` 和存档一起复制过去，并安装同样的 Mod 文件。

### 能备份荣誉模式的存档吗？

存档就是一个普通文件夹，所以可以，任何备份工具都能复制它。恢复它是否符合这个模式的精神，由你自己决定。

### 为什么我的存档提示缺少 Mod？

因为它是用现在没有启用的 Mod 创建的。按相同顺序重新启用同样的 Mod，就能正常读取。
