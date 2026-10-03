---
title: "如何备份和同步模拟器存档（RetroArch、Dolphin、PCSX2）"
description: "在 PC 和 Steam Deck 之间备份与同步 RetroArch、Dolphin、PCSX2、DuckStation 等模拟器存档，保留版本历史，并列出各自的存档位置。"
order: 6
updated: 2026-10-01
---

模拟器存档很容易丢失：存档文件和即时存档散落在各处的文件夹里，一次重装或换一台新电脑就可能抹掉多年的进度。Hoard 会自动备份它们，并在你的所有设备之间保持同步，包括 Steam Deck。

## Hoard 支持的模拟器

Hoard 处理标准的模拟器存档文件（`.srm`、`.sav`、记忆卡、按游戏划分的存档文件夹）以及即时存档。下面这些模拟器的存档位置它都已内置：

- **索尼：** PCSX2（PS2）、DuckStation（PS1）、PPSSPP（PSP）、RPCS3（PS3）、shadPS4（PS4）、Vita3K（PS Vita）
- **任天堂：** Dolphin（GameCube / Wii）、Cemu（Wii U）、Ryujinx、yuzu、Eden、Suyu、Citron 和 Sudachi（Switch）、Citra / Azahar（3DS）、melonDS（DS）、mGBA（GBA）、Project64（N64）
- **其他：** RetroArch（多平台）、xemu（Xbox）、Flycast（Dreamcast）

由于 Hoard 使用与 Ludusavi 相同的社区数据库来定位存档文件夹，许多路径都能自动识别。对于自定义的位置，你也可以手动指定文件夹。

## 设置模拟器存档备份

1. **安装 Hoard**（Windows、macOS 或 Linux）并登录。
2. 打开**库**并添加你的模拟器；如果你更改了默认位置，请手动添加它的存档／即时存档文件夹。
3. 保持**自动模式**开启。Hoard 会在每次会话后备份，并保留版本历史。
4. 在其他 PC 上用同一账号安装 Hoard，这些存档就会在所有设备间同步——参见[如何在多台 PC 之间同步存档](/guides/sync-game-saves-across-pcs)。

## 模拟器用 Ludusavi？

Ludusavi 也能在本地备份模拟器存档，这方面它是很棒的免费选择。如果你还希望这些存档在设备之间自动同步，并在云端保留版本历史，而不用配置 Rclone，那就是 Hoard 派上用场的地方——请阅读完整的 [Ludusavi 与 Hoard 对比](/guides/ludusavi-alternative)。

## 各模拟器的云存档

下面这些独立模拟器都不会自己在设备之间同步存档：存档只是你磁盘上的普通文件。这其实是好事，因为任何监视正确文件夹的工具都能把它们带走。下面是各自的存档位置。“Steam Deck”指的是从 Discover 商店安装的 Flatpak 版本。

### PCSX2 云存档（PS2）

PCSX2 把记忆卡（`.ps2` 文件）写到 `memcards/`：

- Windows：`Documents\PCSX2\memcards`
- Linux：`~/.config/PCSX2/memcards`
- Steam Deck：`~/.var/app/net.pcsx2.PCSX2/config/PCSX2/memcards`

一张记忆卡里存着你在上面玩过的所有游戏的存档，因此它作为一个整体迁移：恢复旧版本会回滚整张卡，而不是单个游戏。

### Dolphin 云存档（GameCube 和 Wii）

GameCube 存档位于 `GC/`（记忆卡镜像或每张卡一个文件夹），Wii 存档位于模拟 NAND 的 `Wii/` 中：

- Windows：`Documents\Dolphin Emulator\GC` 和 `\Wii`
- Linux：`~/.local/share/dolphin-emu/GC` 和 `/Wii`
- Steam Deck：`~/.var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu/`

### DuckStation 云存档（PS1）

DuckStation 把记忆卡放在 `memcards/`，并且默认为每个游戏单独建一张卡，这非常适合同步：

- Windows：`Documents\DuckStation\memcards`（较新的版本使用 `%LOCALAPPDATA%\DuckStation\memcards`）
- Linux：`~/.local/share/duckstation/memcards`
- Steam Deck：`~/.var/app/org.duckstation.DuckStation/` 下的 `data/` 或 `config/`

### RetroArch 存档同步

RetroArch 把 `saves/`（游戏内存档）和 `states/`（即时存档）分开。Hoard 跟踪存档文件夹；如果你也用即时存档，把 `states/` 作为单独的条目添加：

- Windows：`%APPDATA%\RetroArch`，便携版则在 `retroarch.exe` 旁边
- Linux：`~/.config/retroarch`
- Steam Deck：`~/.var/app/org.libretro.RetroArch/config/retroarch`；如果是用 EmuDeck 配置的，则在 `~/Emulation/saves/retroarch`

RetroArch 还内置了 Cloud Sync，需要连接你自己提供的 WebDAV 服务器。如果你只用 RetroArch 且已有 WebDAV，这是个合理的选择。Hoard 不需要 WebDAV，保留可回滚的版本历史，并且也覆盖各独立模拟器。

### PPSSPP（PSP）

存档放在 `PSP/SAVEDATA`，即时存档放在 `PSP/PPSSPP_STATE`：

- Windows：`Documents\PPSSPP\PSP\SAVEDATA`，便携版则在可执行文件旁的 `memstick\PSP\SAVEDATA`
- Linux：`~/.config/ppsspp/PSP/SAVEDATA`
- Steam Deck：`~/.var/app/org.ppsspp.PPSSPP/config/ppsspp/PSP/SAVEDATA`

### RPCS3（PS3）

存档位于 `dev_hdd0/home/00000001/savedata`：Windows 上在 RPCS3 文件夹内，Linux 和 Steam Deck 上在 `~/.config/rpcs3/` 下。

### Switch 模拟器：Ryujinx、yuzu、Eden、Suyu、Citron、Sudachi

Ryujinx 把存档放在 `bis/user/save`（位于 `%APPDATA%\Ryujinx` 或 `~/.config/Ryujinx` 下）。yuzu 系列则使用各自文件夹中的 `nand/user/save`，位于 `%APPDATA%` 或 `~/.local/share` 下。

这里有个陷阱。yuzu 式的目录结构是 `save/<账户>/<用户档案>/<游戏ID>/`，而用户档案 ID 是在模拟器首次运行时生成的，所以每次安装都不一样。如果在两台设备之间同步整个 `save/` 文件夹，每台都会在自己的档案旁边多出对方的档案，两边的游戏都看不到对方的进度。Hoard 则会深入到每个游戏自己的文件夹，因此无论档案叫什么，同一款游戏都能在设备之间对应上。

### Citra 和 Azahar（3DS）

存档藏得很深，位于 `sdmc/Nintendo 3DS/<id0>/<id1>/title/…`，而 `id0`/`id1` 由模拟主机的密钥生成，所以同样因安装而异。Hoard 的处理方式与 Switch 相同：每个游戏一个条目，在设备之间配对。

### 其他

- **Cemu（Wii U）：**`mlc01/usr/save`，位于 `%APPDATA%\Cemu` 或 `~/.local/share/Cemu` 下。
- **shadPS4（PS4）：**`savedata`，位于 `%APPDATA%\shadPS4` 或 `~/.local/share/shadPS4` 下。
- **Vita3K（PS Vita）：**其数据文件夹中的 `ux0/user/00/savedata`。
- **mGBA、melonDS 以及大多数卡带时代的模拟器：**除非另有设置，存档是 ROM 旁边的一个 `.sav`。请手动添加 ROM 文件夹中的存档。

## Steam Deck 上的模拟器存档

在 Steam Deck 上，模拟器通常来自 Flatpak，所以它们的文件夹位于 `~/.var/app/<id>/`，而不是常见的 `~/.config` 或 `~/.local/share`。EmuDeck 会把一切集中到 `~/Emulation/saves/`，每个模拟器一个文件夹。无论哪种方式，添加一次文件夹，Hoard 就会持续监视。

在掌机上最关键的一点：Hoard 的引擎作为后台服务运行，所以在游戏模式下退出游戏时，无需打开任何窗口就会完成备份。在台式机上玩完一局后拿起 Deck，存档已经在那里了。

## 存档文件和即时存档不是一回事

值得把两者分开，因为它们在迁移时表现不同：

- **存档文件**（`.srm`、记忆卡、`SAVEDATA` 文件夹）是游戏自己的存档，由被模拟的主机写入。它可以在设备之间、模拟器版本之间顺利迁移。
- **即时存档**是模拟器内存的快照。它依赖于模拟器的构建版本，往往还依赖于具体的核心，所以某个版本生成的即时存档可能在另一个版本中无法加载。

Hoard 会备份两者。只是如果更新过的设备上的即时存档在旧版本设备上打不开，不必惊讶——保持各设备上模拟器版本一致，重要的进度依靠存档文件。

## 一个模拟器，许多游戏

一个模拟器就是承载几十款游戏的单个进程，这正是模拟器存档让“以正在运行的游戏为单位”思考的工具难以处理的原因。Hoard 会把各个游戏分开，而不是把整个模拟器当成一个整体，因此每款游戏都有自己的历史，而不是一堆每次启动任何游戏都会变化的混合数据。如果某个存档出了问题，你可以[回滚到更早的版本](/guides/restore-a-game-save)。

## 不经过我们服务器的模拟器备份

以上内容在你自己的服务器上同样适用：运行 `hoard-server`，把应用指向它，你的存档就会从你的设备直接存到你的磁盘。无需我们的账号，不向我们发送遥测，不经过我们的任何服务器。参见[如何自托管 Hoard](/guides/self-host-hoard)。

## 提示

即时存档依赖于特定的模拟器版本。请在各台 PC 上一致地更新模拟器，让同步过来的即时存档在任何地方都能正常加载。

<!-- faq -->

## 常见问题

### Hoard 也会备份我的 ROM 吗？

不会。它跟踪的是存档文件夹，而不是游戏文件。ROM 体积大、不会变化，而且你本来就有——没有需要版本管理的东西。

### PCSX2、Dolphin 或 DuckStation 内置云存档吗？

没有。它们把存档写到本地文件夹，同步交给你自己。把同步工具指向上面列出的文件夹，存档就会跟着你在设备之间流转。

### RetroArch 有云同步吗？

有，内置的 Cloud Sync，但需要一台你自己运行或租用的 WebDAV 服务器。如果你不想配置 WebDAV、想要可回滚的版本历史，或者也在用独立模拟器，Hoard 就是替代方案。

### 在游戏模式下的 Steam Deck 上能用吗？

能。引擎作为后台服务运行，所以退出游戏时会自动备份存档，无需打开窗口。Flatpak 和 EmuDeck 的文件夹与其他文件夹用法相同。

### 我的模拟器是便携版，能用吗？

能。手动添加可执行文件旁边的文件夹，Hoard 会像对待其他存档位置一样跟踪它。这在掌机上是常见配置。

### 能在两台 PC 之间同步即时存档吗？

能，Hoard 会同步。即时存档能否加载取决于两台设备上的模拟器版本是否一致，这是模拟器的限制，而不是同步的问题。存档文件没有这个问题。

### 不在列表里的模拟器也能用吗？

几乎可以肯定。常见模拟器会自动识别，其他的只要把 Hoard 指向其存档文件夹即可添加。

### 自托管对模拟器有什么影响吗？

没有。同样的识别、同样的版本、同样的同步。只是存储归你所有。
