---
title: "PC 游戏存档在哪里？所有常见位置"
description: "PC 游戏在 Windows、Steam Deck、Linux 和 Mac 上把存档放在哪里，哪些启动器会加上自己的文件夹，以及如何快速找到任何游戏的存档。"
order: 11
updated: 2026-10-09
---

没有统一的文件夹。在 Windows 上，几乎每款游戏都把存档放在六个位置之一：`Documents`、`Saved Games`、三个 `AppData` 文件夹之一、Steam 的 `userdata`，或者游戏自己的安装文件夹。决定位置的是引擎和开发者，而不是你购买游戏的商店。本页列出所有常见位置、会加上自己一层的启动器，以及一个快速找到任何游戏存档的方法，即使是没人记录过的游戏。

## Windows：六个常见位置

| 文件夹 | 典型路径 | 谁在用 |
|---|---|---|
| 文档 | `%USERPROFILE%\Documents\My Games\<游戏>` | Bethesda 的游戏、Rockstar（`Documents\Rockstar Games`）、许多较老的大作 |
| 保存的游戏 | `%USERPROFILE%\Saved Games\<发行商>\<游戏>` | Cyberpunk 2077 和一小撮顽固的游戏 |
| AppData\Roaming | `%APPDATA%\<游戏>` | Elden Ring、Stardew Valley、Minecraft（`.minecraft`）、大量独立游戏 |
| AppData\Local | `%LOCALAPPDATA%\<游戏>\Saved\SaveGames` | 虚幻引擎游戏（Palworld 用的是 `Pal\Saved\SaveGames`） |
| AppData\LocalLow | `%USERPROFILE%\AppData\LocalLow\<公司>\<游戏>` | Unity 游戏（Hollow Knight 等许多游戏） |
| Steam userdata | `C:\Program Files (x86)\Steam\userdata\<UserID>\<AppID>\remote` | 使用 Steam 自带存档空间的游戏 |

还有第七个怎么也不肯消失的位置：**游戏的安装文件夹**，许多老游戏和一些独立游戏至今仍往这里写。

两条实用提示。`AppData` 是隐藏文件夹，所以与其一路点进去，不如在资源管理器的地址栏里输入 `%APPDATA%` 或 `%LOCALAPPDATA%`。另外，如果 OneDrive 在备份你的 `文档` 文件夹，它的真实路径是 `C:\Users\<你>\OneDrive\Documents`，这让很多人意外。参见 [OneDrive 与游戏存档](/guides/onedrive-game-saves)。

## 会加上自己一层的启动器

大多数启动器不决定存档放在哪里，由游戏决定。有几个例外：

- **Steam** 在 `userdata` 中为每款游戏保留一块存档空间。`<UserID>` 是与你的 Steam 账户绑定的数字（每个在这台 PC 上登录过的账户各有一个文件夹），`<AppID>` 是游戏商店网址里的数字。
- **Ubisoft Connect** 把存档放在 `C:\Program Files (x86)\Ubisoft\Ubisoft Game Launcher\savegames\<用户 ID>\<游戏 ID>`，两层都用数字而不是名字。
- **Xbox 应用和 PC Game Pass** 使用 `%LOCALAPPDATA%\Packages\<包>\SystemAppData\wgs`，里面的文件名是随机的，只有 Xbox 应用看得懂。这些就交给 Xbox 云存档吧，手动复制很少能成功。
- **Epic、GOG 和 EA 应用** 一般交给游戏自己决定，所以它们的游戏最终落在上面那些常见位置。它们的云存档（如果有的话）也是从那里复制。

## 注册表（少见）

少数游戏，主要是小型 Unity 游戏，会把进度存在 Windows 注册表的 `HKEY_CURRENT_USER\Software\<公司>\<游戏>` 下，而不是文件里。这时文件夹里没有东西可复制，基于文件夹的备份工具（包括 Hoard）也看不到它。如果需要，可以用 `regedit` 导出这个键。

## Steam Deck 和 Linux

- **通过 Proton 运行的 Windows 游戏** 会存档在每款游戏各自的前缀中：`~/.local/share/Steam/steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/`，后面接表格中的 Windows 路径（`Documents`、`AppData/Roaming` 等）。microSD 卡上的游戏在卡上自己的 `steamapps/compatdata` 下有相同的结构。
- **原生 Linux 游戏** 使用 `~/.local/share/<游戏>` 或 `~/.config/<游戏>`。Unity 游戏放在 `~/.config/unity3d/<公司>/<游戏>`。
- **Steam userdata** 位于 `~/.local/share/Steam/userdata/<UserID>/<AppID>/remote`。
- **Heroic、Lutris 和 Bottles** 为每款游戏保留一个 Wine 前缀。里面的 Windows 目录树在 `drive_c/users/<你的用户名>/` 下，而不是 `steamuser`。

Deck 有专门的指南：[如何在 Steam Deck 和 PC 之间同步存档](/guides/sync-saves-steam-deck-pc)。

## Mac

- **大多数游戏：** `~/Library/Application Support/<游戏>`。Unity 游戏使用 `~/Library/Application Support/<公司>/<游戏>`。
- **Mac App Store 的游戏** 运行在沙盒中：`~/Library/Containers/<包 ID>/Data/Library/Application Support/`。

`~/Library` 同样是隐藏的。在访达中按住 Option 键打开 **前往** 菜单，它就会出现。

## 如何找到任何游戏的存档

当某款游戏不在任何列表里时，三个技巧几分钟就能找到：

1. **在 PCGamingWiki 上查。** 几乎每个游戏页面都有 “Save game data location” 一节。Hoard 和 Ludusavi 使用的存档数据库也是从这个来源生成的。
2. **看什么在变化。** 在游戏里存档、退出，然后在你的用户文件夹里搜索最近几分钟修改过的文件。在 Windows 上，于 `C:\Users\<你>` 中搜索 `datemodified:today` 并按日期排序。在 Linux 或 Deck 上：`find ~ -type f -mmin -5 -not -path '*/.cache/*'`。
3. **问 Steam。** 对于支持 Steam 云的游戏，[store.steampowered.com/account/remotestorage](https://store.steampowered.com/account/remotestorage) 会列出 Steam 为每款游戏保存的文件，包括名称和大小。知道了文件名，找文件夹就容易了。

## 或者让别人替你找

Hoard 读取同一个社区数据库（覆盖数千款游戏），并逐一检查你电脑上的每个候选路径：Proton、Heroic 和 Lutris 的前缀、OneDrive 里的 `文档`、模拟器、便携版安装。找到的内容会在你每次停止游戏后自动备份，保留每个版本，并在你的 PC 和 Steam Deck 之间保持同步。它漏掉的，指定一次文件夹就能添加。参见 [如何自动备份游戏存档](/guides/back-up-game-saves)。

一些热门游戏还有列出确切路径的页面：[Cyberpunk 2077](/guides/cyberpunk-2077-save-location)、[Baldur's Gate 3](/guides/baldurs-gate-3-save-location)、[Palworld](/guides/palworld-save-location)、[Crimson Desert](/guides/crimson-desert-save-location) 和 [Marvel's Spider-Man 2](/guides/spider-man-2-save-location)。

<!-- faq -->

## 常见问题

### Steam 把存档放在哪里？

取决于游戏。有些使用 Steam 自己的空间 `Steam\userdata\<UserID>\<AppID>\remote`；大多数则和其他游戏一样写入 `Documents`、`AppData` 或 `Saved Games`，Steam 云再从那里复制。

### 为什么找不到 AppData 文件夹？

因为它是隐藏的。在资源管理器的地址栏或“运行”（Win + R）中输入 `%APPDATA%`（Roaming）或 `%LOCALAPPDATA%`（Local）。`LocalLow` 就在 `Local` 旁边。

### Steam、GOG 和 Epic 版本的存档在同一个地方吗？

通常是，因为决定位置的是游戏而不是商店。但也有例外：有些游戏会加一个以你的账户 ID 命名的文件夹，个别商店版本会用不同的文件夹名。在不同版本之间复制存档前先确认一下。

### Xbox 应用和 Game Pass 的存档在哪里？

在 `%LOCALAPPDATA%\Packages\<包>\SystemAppData\wgs`，是一些名字随机、只有 Xbox 应用看得懂的文件。它们由 Xbox 云同步，手动复制很少能成功。

### 我的文档文件夹在 OneDrive 里，这有问题吗？

可能有。游戏会跟着文件夹进入 OneDrive，而 OneDrive 会在游戏写入存档的同时同步它们。参见 [OneDrive 与游戏存档](/guides/onedrive-game-saves)。
