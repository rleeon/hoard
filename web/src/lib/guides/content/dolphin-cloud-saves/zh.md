---
title: "Dolphin 云存档：在 PC 和 Steam Deck 之间同步 GameCube 和 Wii 存档"
description: "Dolphin 没有云存档。在 PC 和 Steam Deck 之间自动同步 GameCube 和 Wii 存档，并保留版本历史：存档路径、记忆卡类型和常见陷阱。"
order: 17
updated: 2026-10-09
---

Dolphin 不会在机器之间同步存档：你的 GameCube 记忆卡和模拟的 Wii 都在一台 PC 的文件夹里。Hoard 会自动同步它们。你关闭 Dolphin 时，它会备份 GameCube 和 Wii 的存档，把它们带到你的其他 PC 和 Steam Deck 上，并保留每个版本，让你随时都能回退。

## Dolphin 把存档放在哪里

一切都在 Dolphin 的用户文件夹里。最快的找法是在 Dolphin 里点 **File → Open User Folder**。里面有：

- `GC`：GameCube 记忆卡。
- `Wii`：模拟 Wii 的内部存储，包括存档。
- `StateSaves`：即时存档。

这个文件夹的位置：

- **Windows：** `Documents\Dolphin Emulator`。较新的安装可能改用 `%APPDATA%\Dolphin Emulator`，便携版则在 `Dolphin.exe` 旁边有一个 `User` 文件夹。
- **Linux：** `~/.local/share/dolphin-emu`。
- **Steam Deck**（Discover 里的 Flatpak）：`~/.var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu`。
- **Mac：** `~/Library/Application Support/Dolphin`。

Hoard 能自动找到 `Documents`、Linux 和 Steam Deck 上的文件夹。对于 `%APPDATA%`、便携版或 Mac，指定一次 `GC` 和 `Wii` 文件夹即可。

## GameCube：记忆卡文件还是 GCI 文件夹

在 **Options → Configuration → GameCube** 中，每个记忆卡槽可以是以下两种之一：

- **记忆卡文件**：像 `MemoryCardA.USA.raw` 这样的原始镜像，里面装着所有游戏的存档。任何一次保存都会重写整个文件。
- **GCI 文件夹**：每个存档都是独立的 `.gci` 文件，放在像 `GC/USA/Card A` 这样的文件夹里。只有变化的那个存档是新的，所以版本又小又好读。

同步时 GCI 文件夹更合适。不管选哪种，**所有机器都要用相同的设置**：一台 PC 用记忆卡文件、另一台用 GCI 文件夹，两边看到的都会是空卡。如果需要在两种之间转移存档，Dolphin 的 **Tools → Memory Card Manager** 可以导入和导出 `.gci` 文件。

记忆卡还按 **区域** 分开（USA、EUR、JAP）。同一款游戏的 PAL 版和 NTSC 版看不到彼此的存档，所以所有地方都用同一个光盘镜像。

## Wii：模拟主机的存储

Wii 存档在 `Wii` 文件夹里，光盘游戏位于 `Wii/title/00010000/<游戏 ID>/data`。这个文件夹就是整台模拟主机的存储：存档、Mii、系统设置以及你安装的频道。Hoard 把它作为一个条目备份，所以还原一个版本会把主机存储恢复到那个时刻的样子。确认之前 Hoard 会显示将要改变什么，而且你当前的文件会先被保存。

如果只想手动转移一个 Wii 存档，Dolphin 可以导出它：在列表中右键点击游戏，选择 **Export Wii Save**。

## 日常同步是怎样的

你在台式机上玩完，关闭 Dolphin。Hoard 会等 Dolphin 退出、文件夹安静下来，然后上传新版本。之后你拿起 Steam Deck；一联网，Hoard 就会把更新的存档取下来。在 Deck 上关闭 Dolphin，同样的事会反方向发生一次。两台机器不需要同时开机。

## 需要了解的陷阱

- **关闭 Dolphin，而不只是游戏。** Hoard 在模拟器退出后才备份。在 Deck 上，睡眠不算关闭。
- **即时存档很脆弱。** Dolphin 的即时存档经常在版本之间失效。Hoard 同步的是真正的存档；如果你也想要 `StateSaves`，就把它作为单独的条目添加，并让所有地方的 Dolphin 保持同一版本。
- **自定义路径。** 如果你在 **Options → Configuration → Paths** 中修改过 Wii NAND 根目录或 GCI 文件夹路径，就把那些文件夹指定给 Hoard。

## 设置步骤

1. 在每台机器上安装 Hoard，并用同一个账户登录。
2. 在 **库** 中，从模拟器列表里添加 Dolphin。
3. 所有机器使用相同的记忆卡设置和相同的区域。
4. 玩，关闭 Dolphin，然后在另一台机器上继续。

想让一切都留在家里？在你的 PC 或 NAS 上运行 `hoard-server`，把所有机器都指向它。不需要我们的账户，没有发给我们的遥测，没有任何东西经过我们的服务器。参见 [如何自托管 Hoard](/guides/self-host-hoard)。其他模拟器参见 [模拟器存档](/guides/back-up-emulator-saves)。

<!-- faq -->

## 常见问题

### Dolphin 有云存档吗？

没有。Dolphin 把存档放在本地文件夹里，同步交给你自己。Hoard 是自动同步、并额外保留版本历史的一种方式。

### 能在 PC 和 Steam Deck 之间同步 Dolphin 存档吗？

能。用同一个账户在两边都安装 Hoard。Hoard 知道 Dolphin 在 Windows、Linux 和 Steam Deck 的 Flatpak 中把存档放在哪里，并在机器之间对应起来。

### 该用记忆卡文件还是 GCI 文件夹？

同步时用 GCI 文件夹：每个存档都是独立的文件，所以版本很小，也能看出是哪个游戏变了。不管选哪种，所有机器都用同一种。

### Wii 存档也会同步吗？

会。`Wii` 文件夹装着模拟主机的存储，包括存档，Hoard 会像 GameCube 记忆卡一样备份并同步它。

### Hoard 会同步 Dolphin 的即时存档吗？

默认不会，因为即时存档在不同 Dolphin 版本之间会失效。需要的话手动添加 `StateSaves` 文件夹。

### 能用于 Android 上的 Dolphin 吗？

目前不能。Hoard 运行在 Windows、macOS、Linux 和 Steam Deck 上。
