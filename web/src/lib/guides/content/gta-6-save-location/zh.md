---
title: "GTA 6 存档位置：PS5、Xbox 与 PC（目前已知的信息）"
description: "GTA 6 将于 2026 年 11 月 19 日登陆 PS5 和 Xbox，暂无 PC 版。主机存档在哪里、如何保护，以及 PC 版可以期待什么。"
order: 25
updated: 2026-10-17
---

GTA 6 将于 **2026 年 11 月 19 日发售，仅限 PS5 和 Xbox Series X|S**。目前还没有 PC 版：Rockstar 没有公布日期，唯一流传的时间来自 2026 年 10 月一份未经证实的报道，称是 2027 年初。在那之前，GTA 6 的存档保存在主机和主机的云端，而不是一个你能打开的文件夹里。下面介绍这些存档如何保存、如何保护，以及参照 Rockstar 最近的 PC 作品，PC 版存档最可能放在哪里。PC 版一推出，这一页就会写上真实路径。

## GTA 6 在主机上的存档位置

- **PS5：**在主机上。订阅 PlayStation Plus 后，PS5 还会自动把存档上传到云存储。没有订阅的话，唯一的一份就在主机上。
- **Xbox Series X|S：**在主机上，并与你 Microsoft 账户的 Xbox 云同步。这是自动的，不需要订阅。

两者都不是能在电脑上浏览的文件夹。PS5 可以把整台主机（包括存档）备份到 USB 驱动器，但这份备份只能在 PS5 上恢复。

## 能用 Hoard 备份 GTA 6 的存档吗？

主机上不能。Hoard 处理的是以文件形式存在于电脑上的存档：Windows、macOS、Linux 和 Steam Deck。主机存档从不离开主机自己的系统，所以任何 PC 应用都碰不到它们。

PC 版推出后，GTA 6 的存档就会像其他 PC 游戏一样是文件，Hoard 可以备份它们，并在你的多台 PC 之间保持同步。

## PC 版存档可能的位置

Rockstar 最近的 PC 作品都存在同一个地方，即 `文档\Rockstar Games` 下每个游戏一个文件夹：

- **GTA V Legacy：**`%USERPROFILE%\Documents\Rockstar Games\GTA V\Profiles\<档案 ID>`
- **GTA V Enhanced：**`%USERPROFILE%\Documents\Rockstar Games\GTAV Enhanced\Profiles\<档案 ID>`
- **Red Dead Redemption 2：**`%USERPROFILE%\Documents\Rockstar Games\Red Dead Redemption 2\Profiles\<档案 ID>`

最后一层文件夹以你的 Rockstar 账户命名，所以每个账户都有自己的文件夹。如果 GTA 6 沿用同样的模式，它的存档会放在 `文档\Rockstar Games\<游戏名>\Profiles\<档案 ID>`。**这是根据先例的推测，不是已确认的路径。**

如果你的电脑把 `文档` 重定向到了 OneDrive，整个 `Rockstar Games` 文件夹也会跟着搬到 `%USERPROFILE%\OneDrive\Documents\Rockstar Games`。这时 OneDrive 会在你游戏时自行同步存档，这本身就会带来问题：参见[游戏存档与 OneDrive](/guides/onedrive-game-saves)。

## Rockstar 自己在 PC 上的云存档

在 PC 上，GTA V 和 Red Dead Redemption 2 通过 Rockstar Games Launcher 同步存档，而不是 Steam Cloud，即使你是在 Steam 上买的。云存档要在启动器设置里逐个游戏开启。和 Steam Cloud 一样，它只让各台机器上的当前存档保持一致，不提供可以退回的旧版本。

Rockstar 还没说 GTA 6 在 PC 上会如何处理存档。沿用同一个启动器的云端是最自然的做法，但在官方公布之前，这只是推测。

## 用 Hoard 为 PC 版做好准备

[Hoard](/download) 会在你每次停止游戏时备份存档文件夹，并保留每一个版本，所以存档损坏了，或者想重打某个任务，恢复一次就能回去。它还会在你的 PC 和 Steam Deck 之间保持文件夹同步，而且已经支持 GTA V 和 Red Dead Redemption 2。

GTA 6 登陆 PC 时：

1. 安装 Hoard 并登录，或把它指向[你自己的服务器](/guides/self-host-hoard)。
2. 打开**库**。刚发售的游戏可能要过一段时间才会进入社区存档数据库，所以如果 GTA 6 还没有列出来，就手动添加它在 `文档\Rockstar Games` 下的文件夹。
3. 开始游戏。退出后，第一个版本就会出现在历史记录里。

<!-- faq -->

## 常见问题

### GTA 6 会出 PC 版吗？

首发没有。2026 年 11 月 19 日，它在 PS5 和 Xbox Series X|S 上发售。Rockstar 没有公布 PC 版日期；2026 年 10 月的一份报道称是 2027 年初，但未经证实。

### PS5 上的 GTA 6 存档在哪里？

在主机上；如果你订阅了 PlayStation Plus 并开启了自动上传，云存储里也有一份。它们不是能在电脑上打开的文件。

### 能把 GTA 6 存档从主机转到 PC 吗？

Rockstar 还没说。GTA V 曾允许玩家通过 Rockstar 自己的云把故事模式存档转到新主机上，但 GTA 6 还没有公布类似的功能。

### 存档文件夹里包含线上进度吗？

在 Rockstar 现有的游戏里不包含：GTA Online 和 Red Dead Online 的进度保存在 Rockstar 的服务器上，不在存档文件夹里。磁盘上没有需要为它备份的东西。

### Hoard 会支持 GTA 6 吗？

在 PC 上会，只要它像 Rockstar 其他 PC 游戏一样把存档存在文件夹里。主机存档无论是 Hoard 还是其他任何 PC 应用都无法触及。
