---
title: "在 PC 和 Steam Deck 之间同步 RetroArch 存档"
description: "在 PC、Steam Deck 和笔记本之间同步 RetroArch 的存档和即时存档：.srm 文件在哪里，内置云同步与自动同步的区别，以及常见陷阱。"
order: 14
updated: 2026-10-09
---

RetroArch 把游戏内存档以 `.srm` 文件保存在 `saves` 文件夹里，把即时存档保存在 `states` 文件夹里。要在设备之间同步它们，你可以用 RetroArch 内置的 Cloud Sync 配合你自己提供的 WebDAV 服务器，也可以用一个同时监视这两个文件夹的工具。Hoard 自动做的是后者：你退出 RetroArch 时备份两个文件夹，把它们带到其他机器上，保留每个版本，并且能识别 EmuDeck 的安装方式。

## RetroArch 的存档在哪里

- **Windows：** `%APPDATA%\RetroArch\saves` 和 `\states`；如果你把它装在单独的文件夹里，则是 `retroarch.exe` 旁边的 `saves` 和 `states`。
- **Linux：** `~/.config/retroarch/saves`。Flatpak 版使用 `~/.var/app/org.libretro.RetroArch/config/retroarch/saves`。
- **装了 EmuDeck 的 Steam Deck：** `~/Emulation/saves/retroarch/`，其中的 `saves` 和 `states` 是指向真实文件夹的链接。Hoard 会读取 `retroarch.cfg` 来确定它们真正指向哪里。
- **RetroDECK：** 默认是 `~/retrodeck/saves` 和 `~/retrodeck/states`。
- **其他位置：** **Settings → Directory** 会显示 RetroArch 实际使用的文件夹。

## RetroArch 什么时候才真正写入存档

很多人在这里栽跟头。RetroArch 把游戏内存档放在内存里，只有在你关闭游戏或退出 RetroArch 时才写入 `.srm`，除非设置了 **Settings → Saving → SaveRAM Autosave Interval**。在那之前硬盘上什么都没有：一次崩溃或电量耗尽，就会丢掉上次写入之后的一切，而任何同步工具都搬不动一个还没写出来的存档。

把自动保存间隔设为几秒。换设备之前，**退出 RetroArch**，而不只是关闭游戏：Hoard 会在 RetroArch 关闭后备份，所以永远不会复制写了一半的存档。在 Deck 上，睡眠不算退出。

## 让所有设备的设置保持一致

- **分类选项。** **Settings → Saving** 可以按核心名称或内容文件夹，把存档和即时存档分到子文件夹里。如果一台设备分类而另一台不分，同步过来的文件就会落在 RetroArch 不去找的文件夹里。所有设备用同样的设置。
- **ROM 文件名。** `.srm` 和 ROM 同名：`Super Metroid (USA).sfc` 的存档是 `Super Metroid (USA).srm`。另一台设备上的 ROM 名字不同，就找不到它。
- **同一个核心。** 同一台主机的两个核心不一定能读对方的存档。每个平台选一个核心，在所有设备上都用它。
- **即时存档要看核心版本。** 即时存档是核心内存的快照，在另一个版本里常常无法加载。普通存档没有这个问题。

即时存档还有一个陷阱：即时存档包含游戏的内存，游戏内存档也在其中。加载一个旧的即时存档，下一次写入 `.srm` 就会把那个旧存档带回来。如果你用了 **Auto Load State**，也把即时存档一起同步，让传过去的是最新的那个。

## RetroArch 云同步还是 Hoard？

对两者都公平地说：

- **RetroArch 的 Cloud Sync** 是内置的，会把存档和即时存档同步到你自己运行或租用的 WebDAV 服务器。它在 Android 和 iOS 上也能用，而 Hoard 目前不行。如果你已经在用 Nextcloud（它自己会保留文件版本），那它很合适；如果手机也是你游戏设备的一部分，它是更好的选择。
- **Hoard** 不需要 WebDAV 服务器。它自动备份和同步，保留可以回退的版本历史，还能一并照顾独立模拟器和 PC 游戏。它把整个 `saves` 文件夹当作一个条目，所以回退会把其中每款游戏都恢复到那个时刻的样子。确认之前它会显示将要改变什么，而且你当前的文件会先被保存。

每个文件夹只选一个工具。两个工具写同一批存档，就是在制造冲突。

## 用 Hoard 设置

1. 在每台设备上安装 Hoard，并用同一个账户登录。
2. 在 **库** 中添加 RetroArch。存档和即时存档会显示为两个条目。
3. 在所有设备上把上面的设置统一好。
4. 玩，退出 RetroArch，然后在另一台设备上继续。

想让一切都留在家里？在你的 PC 或 NAS 上运行 `hoard-server`：不需要我们的账户，没有发给我们的遥测，没有任何东西经过我们的服务器。参见 [如何自托管 Hoard](/guides/self-host-hoard)。

<!-- faq -->

## 常见问题

### RetroArch 有云存档吗？

有，内置的 Cloud Sync，需要一台 WebDAV 服务器。如果你不想自己维护服务器、想要版本历史，或者还用独立模拟器，Hoard 是替代方案。

### 为什么我的 RetroArch 存档没有同步？

通常是三种原因之一：RetroArch 还没写出 `.srm`（它仍开着，而且没设自动保存间隔），两台设备把存档分到了不同的子文件夹，或者 ROM 的名字不一样。

### 即时存档也能同步吗？

能。Hoard 把 `states` 作为单独的条目跟踪。即时存档能否在另一台设备上加载，取决于两边是否使用同一版本的核心。

### 支持 EmuDeck 和 RetroDECK 吗？

支持。Hoard 会读取 RetroArch 的配置，顺着 EmuDeck 的链接找到真实的文件夹。对于 RetroDECK，如果 `~/retrodeck/saves` 和 `~/retrodeck/states` 没被自动发现，就手动添加它们。

### Hoard 能同步 Android 上的 RetroArch 吗？

目前不能。Hoard 运行在 Windows、macOS、Linux 和 Steam Deck 上。
