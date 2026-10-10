---
title: "在 ROG Ally、Legion Go、MSI Claw 和 PC 之间同步存档"
description: "ROG Ally、Legion Go 和 MSI Claw 这类 Windows 掌机本质上就是 PC。自动把它们的游戏存档与台式机同步，并保留版本历史。"
order: 19
updated: 2026-10-09
---

ROG Ally、Legion Go 和 MSI Claw 运行的是 Windows，所以在游戏看来它们就是又一台 PC。问题恰恰在这里：你的台式机和掌机各自保存着自己的存档。Steam 云覆盖了你游戏库的一部分，Xbox 云存档覆盖了 Game Pass，但其余的一切都留在你玩过的那台机器上。Hoard 会自动在掌机和台式机之间同步存档：在一台上停止游戏，另一台上游戏已经在等你，之前的每个版本也都会保留。

## 已经会跟着你走的

- **支持 Steam 云的 Steam 游戏** 会自动同步。
- **Game Pass 和 Xbox 应用的游戏** 使用 Xbox 云，前提是两台机器上玩的都是 Xbox 版本。
- **Epic、GOG、Ubisoft 和 EA** 在各自的启动器里为部分游戏提供云存档。参见 [Epic 和 GOG 的云存档](/guides/epic-gog-cloud-saves)。

剩下的：开发者从未开启云存档的游戏、模拟器、手动安装的游戏，以及台式机和掌机用的不是同一个启动器的任何游戏。

## 设置步骤

1. **在掌机上** 切换到 Windows 桌面，打开 [下载页面](/download)，安装 Windows 版 Hoard。
2. 用你在台式机上使用的账户 **登录**，或者把应用指向你自己的服务器。
3. 打开 **库**，查看 Hoard 找到了什么。缺少的内容（例如模拟器）可以指定文件夹来添加。
4. **在台式机上** 用同一个账户安装 Hoard。相同的游戏会自动对应起来。

同步引擎是随 Windows 启动的后台服务，所以你在 Armoury Crate、Legion Space、MSI Center M 或 Steam 的大屏幕模式里时，它也照常工作。玩游戏不需要打开 Hoard 的窗口。

## 掌机的陷阱

### 睡眠不等于退出

在掌机上，按下电源键、让游戏开着就收起来，实在太容易了。Hoard 只会在游戏关闭后备份，因为正在运行的游戏可能正写到一半；它也从不替换正在运行的游戏的存档。如果你让掌机睡眠，然后去台式机上玩，掌机上的进度还没有上传。**换机器之前先退出游戏。**

### microSD 卡上的游戏

在掌机上把游戏装在卡上很常见，但这几乎不影响存档：大多数游戏不管装在哪里，都会把存档写进内置硬盘上的用户文件夹。例外是那些把存档放在安装文件夹旁边的游戏；如果有这样的游戏没被检测到，就手动添加它的文件夹。

### 屏幕与设置

你的掌机分辨率更低、GPU 更小。Hoard 会把 `graphics.ini` 这类设置文件和存档一起备份，但不会覆盖另一台机器上的设置，所以两边都保留适合自己的设置。如果你仍想复制，还原时有专门的选项。

### 同一款游戏，不同的商店

台式机上在 Steam 买的游戏，和掌机上通过 Game Pass 玩的同一款游戏，是两套不同的安装，而 Xbox 版的存档格式只有 Xbox 应用看得懂。要共享一份存档，两边就得玩同一家商店的版本。

### 用的是 SteamOS 或 Bazzite 而不是 Windows？

那你的掌机就是一台 Linux 机器，存档放在 Proton 前缀里，和 Steam Deck 完全一样。参见 [如何在 Steam Deck 和 PC 之间同步存档](/guides/sync-saves-steam-deck-pc)。

## 不用我们的服务器

如果你希望存档留在家里，可以在 PC 或 NAS 上运行 `hoard-server`，把两台机器都指向它。不需要我们的账户，没有发给我们的遥测，没有任何东西经过我们的服务器。参见 [如何自托管 Hoard](/guides/self-host-hoard)。

<!-- faq -->

## 常见问题

### ROG Ally 有云存档吗？

它有各个启动器各自提供的：Steam 云、Xbox 云，以及 Epic、GOG、Ubisoft 或 EA 对支持的游戏提供的云存档。没有覆盖整个系统的存档同步。Hoard 为这些遗漏的游戏补上一套。

### Hoard 能和 Armoury Crate 或 Legion Space 一起用吗？

能。Hoard 的同步引擎是 Windows 后台服务，和你用哪个启动器开游戏无关。

### 掌机算一台设备吗？

算。免费方案包含三台设备，所以一台台式机、一台笔记本和一台掌机正好够用。Pro 和自托管服务器没有设备数量限制。

### Game Pass 的存档怎么办？

交给 Xbox 云，它会在两台机器的 Xbox 应用之间同步。Hoard 负责那些没有自己云存档的游戏。

### 掌机也能和 Steam Deck 同步吗？

能。Hoard 在两者上都能运行，并会在 Windows 和 Deck 的 Proton 前缀之间把每款游戏对应起来。
