---
title: "如何在 Steam Deck 和 PC 之间同步存档"
description: "自动同步 Steam Deck 和 PC 的存档，包括非 Steam 游戏、模拟器和不支持 Steam 云的游戏。设置步骤、存档路径和常见陷阱。"
order: 10
updated: 2026-10-09
---

对于支持 Steam 云的 Steam 游戏，你的 Deck 和 PC 已经在共享存档了。其余的都需要帮忙：开发者从没开启 Steam 云的游戏、用 Heroic 启动的 Epic 和 GOG 游戏、模拟器，以及你作为非 Steam 游戏添加的任何东西。Hoard 会自动处理所有这些。你在一台机器上退出游戏时，它会备份存档，另一台机器随后把它取下来；之前的每个版本都会保留，以防出问题。

## Steam 云在 Deck 上已经做了什么

如果游戏支持 Steam 云，Steam 会在你退出时上传存档，在另一台机器上启动时下载。游戏是否支持可以在商店页面看到，也可以在 **属性 → 通用** 中逐个游戏关闭。

缺口总是那几个：

- **不支持的游戏。** 是否开启由开发者逐个游戏决定，很多 PC 游戏从没开启过。
- **Steam 之外的一切。** Heroic、Lutris、模拟器、手动安装的游戏。
- **无法回退。** Steam 只保存当前的存档，没有历史。如果损坏的存档被同步了，两台机器上的好存档都会消失。

更多内容见 [Steam 云的替代方案](/guides/steam-cloud-alternative) 指南。

## Deck 把存档放在哪里

Deck 通过 Proton 运行 Windows 游戏，所以同一款游戏的存档位置和你的 PC 不一样：

- **Windows 游戏（Proton）：** `~/.local/share/Steam/steamapps/compatdata/<AppID>/pfx/drive_c/users/steamuser/`，后面接常见的 Windows 路径：`Documents`、`AppData/Roaming`、`AppData/Local`、`AppData/LocalLow` 或 `Saved Games`。AppID 是游戏商店网址里的数字。
- **microSD 卡上的游戏：** 卡上有自己的 `steamapps/compatdata/<AppID>`，里面的结构相同。
- **原生 Linux 游戏：** 通常是 `~/.local/share/<游戏>` 或 `~/.config/<游戏>`。Unity 游戏使用 `~/.config/unity3d/<公司>/<游戏>`。
- **Heroic、Lutris 和 Bottles：** 每个都为每款游戏保留一个 Wine 前缀，Windows 目录树位于 `drive_c/users/<你的用户名>/` 下，而不是 `steamuser`。
- **模拟器：** EmuDeck 把它们集中在 `~/Emulation/saves/`。参见 [模拟器存档](/guides/back-up-emulator-saves) 和 [RetroArch](/guides/retroarch-save-sync)。

在你的 PC 上，同一款游戏写入的是 `C:\Users\<你>\...`。同一份存档却有两条不同的路径：这正是手动复制文件夹容易出错的原因。Hoard 会在所有这些位置查找，并按游戏把找到的内容对应起来，于是 Deck 的存档和 PC 的存档成为同一段历史的两个版本。

## 设置步骤

1. 在 Deck 上切换到桌面模式：**Steam 按钮 → 电源 → 切换到桌面**。
2. 打开浏览器，进入 [下载页面](/download)，下载 Linux 版的 **Hoard Setup**。在文件管理器中打开该文件的属性，允许它作为程序运行，然后打开它。
3. 用你在 PC 上使用的同一个账户登录，或者把应用指向你自己的服务器。
4. 打开 **库**，查看 Hoard 找到了什么。缺少的内容可以指定它的文件夹来添加：Heroic 前缀、模拟器、你自己安装的游戏。
5. 在 PC 上用同一个账户安装 Hoard。相同的游戏会自动对应起来。
6. 回到游戏模式。之后不需要再回桌面模式。

Hoard Setup 会把应用放在你的主文件夹里，并把同步引擎放进一个随 Deck 启动的后台服务。它不会向 SteamOS 的只读系统写入任何东西，所以系统更新不会影响它。

## 平常的一天

晚上你在 PC 上玩完并退出。Hoard 会等游戏关闭、存档不再变化，然后上传。第二天早上你拿起 Deck。一联网，Hoard 就会发现更新的版本，并把它写进 Proton 前缀。你启动游戏，接着玩。在 Deck 上退出时，同样的事情会反方向发生一次。

两台机器不需要同时开机。存档会在服务器上等着，直到另一台来取。

## 需要了解的陷阱

### 睡眠不等于退出

在 Deck 上，按下电源键、让游戏开着就走开，实在太容易了。Hoard 只会在游戏关闭后备份存档，因为正在运行的游戏可能正写到一半。它也从不替换正在运行的游戏的存档。所以如果你让 Deck 睡眠，然后去 PC 上玩，Deck 上的进度还没有上传，而 PC 的新存档会一直等到你在 Deck 上关闭游戏。

能避免这一切的习惯只有一个：**换机器之前先退出游戏。** 退出后 Proton 经常留下一个已经死掉的进程，Hoard 会察觉游戏已经不在了，并继续工作。

### 唤醒后给它几秒钟

Deck 从睡眠中唤醒时，Wi-Fi 需要一点时间恢复，之后 Hoard 才能检查是否有更新的存档。如果你在最初几秒内启动游戏，下载会一直等到你关闭它。开始玩之前，先让它联网一会儿。

### microSD 卡

如果游戏在卡上而卡没有插着，Hoard 不会把存档下载到一个不存在的文件夹里。它会等卡回来。

### 设置留在各自的机器上

Deck 用掌机 GPU 以 1280×800 运行，你的台式机大概不是。Hoard 会把 `graphics.ini` 这类设置文件和存档一起备份，但不会覆盖另一台机器上的设置，所以 Deck 保留自己的设置。如果你仍想复制，还原时有专门的选项。更多内容见 [如何在多台 PC 之间同步存档](/guides/sync-game-saves-across-pcs)。

### Steam 的 `remote` 文件夹

对于 Steam 游戏，存档位于 `userdata/<UserID>/<AppID>/remote/`。它的上一级文件夹里还有 `remotecache.vdf`，以及在 Deck 和 PC 上本就应该不同的游玩时间和成就文件。手动同步上一级文件夹，每次启动都会看起来像冲突。Hoard 只跟踪 `remote/`。

## 同时使用 Steam 云和 Hoard

两者互不干扰。支持 Steam 云的游戏，就让 Steam 继续同步。Hoard 在这里补上的是版本历史，这样一台机器上的存档损坏不会把你的进度一起带走。对于其他所有游戏，同步也由 Hoard 负责。

## 不用我们的服务器

如果你希望存档不出家门，可以在 PC 或 NAS 上运行 `hoard-server`，然后把 Deck 和 PC 都指向它。不需要我们的账户，没有发给我们的遥测，没有任何东西经过我们的服务器。参见 [如何自托管 Hoard](/guides/self-host-hoard)。

<!-- faq -->

## 常见问题

### Hoard 能在游戏模式下工作吗？

能。同步引擎是随 Deck 启动的后台服务，不需要打开任何窗口就能备份和还原。只有安装以及手动添加文件夹时才需要桌面模式。

### SteamOS 更新会删掉它吗？

不会。Hoard 安装的一切都在你的主文件夹里，SteamOS 更新不会碰它。

### 能同步 Heroic、Lutris 或 EmuDeck 的游戏吗？

能。Hoard 会查看 Heroic、Lutris 和 Bottles 的前缀以及 EmuDeck 的文件夹。如果某款游戏没被检测到，指定一次它的存档文件夹，之后就会像其他游戏一样被跟踪。

### 如果我没同步就在两边都玩了怎么办？

Hoard 从不盲目覆盖。它会比较版本，保留被替换内容的副本，之前的每个版本都留在历史里。它无法把两段不同的游玩合并成一个存档（任何工具都做不到），但你可以选择保留哪一个。

### Deck 算一台设备吗？

算。免费方案包含三台设备，所以一台 PC、一台笔记本和一台 Deck 正好够用。Pro 和自托管服务器没有设备数量限制。

### 我可以在 Deck 上改用命令行版本吗？

可以。`hoard` 命令在没有窗口的情况下运行同一个引擎，有些人在掌机上更喜欢这样。参见 [CLI 页面](/cli)。
