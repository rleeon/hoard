---
title: "艾尔登法环（Elden Ring）存档位置（PC 与 Steam Deck）"
description: "艾尔登法环的 ER0000.sl2 在 PC 和 Steam Deck 上的位置、里面有什么、Seamless Co-op 与黑夜君临的存档，以及如何备份所有角色。"
order: 26
updated: 2026-10-18
---

在 PC 上，艾尔登法环把存档放在 `%APPDATA%\EldenRing\<你的 Steam ID>`，全部在一个名为 `ER0000.sl2` 的文件里，其中包含你创建的所有角色，最多十个。下面是 Steam Deck 上的路径、文件夹里还有什么、Seamless Co-op 和黑夜君临把存档放在哪里，以及如何让这一切都有备份，并在 PC 和 Steam Deck 之间保持同步。

## 艾尔登法环的存档位置

- **Windows（Steam）：**`%APPDATA%\EldenRing\<你的 Steam ID>\ER0000.sl2`
- **Steam Deck 和 Linux**（Proton）：`~/.local/share/Steam/steamapps/compatdata/1245620/pfx/drive_c/users/steamuser/AppData/Roaming/EldenRing/<你的 Steam ID>/`

`%APPDATA%` 就是 `C:\Users\<你的用户名>\AppData\Roaming`：把 `%APPDATA%\EldenRing` 粘贴到文件资源管理器的地址栏，就能直接打开。Steam ID 文件夹是一个与你的 Steam 账号绑定的长数字。在 Steam Deck 上游戏通过 Proton 运行，所以存档位于 Steam 为它维护的 Proton 前缀中；`1245620` 是游戏的 Steam 应用 ID。

## 存档文件夹里有什么

- **`ER0000.sl2`** 就是你的进度：所有角色、他们的装备和在世界中的位置，黄金树幽影的内容也在里面。
- **`ER0000.sl2.bak`** 是游戏放在旁边的一份备份。
- **`ER0000.co2`** 如果存在，属于 Seamless Co-op 模组（见下文）。

画面设置不在这个文件夹里，而是在上一级的 `%APPDATA%\EldenRing\GraphicsConfig.xml`。

由这个文件的工作方式可以得出两点：

- **角色无法单独备份或还原。** 还原一个旧的 `ER0000.sl2`，会让所有角色都回到那个时刻，而不只是你想修的那一个。
- **存档与你的 Steam 账号绑定。** 你的 Steam ID 写在文件里，游戏不会加载用其他账号创建的存档。在你自己的 PC 和 Steam Deck 之间搬运你自己的存档则完全没问题。

## Seamless Co-op 与黑夜君临

### Seamless Co-op

Seamless Co-op 模组不会动 `ER0000.sl2`，而是在同一个文件夹里保存自己的存档 `ER0000.co2`，所以装模组和不装模组的角色是两个不同的文件。模组社区建议不要为了联网游玩把 `.co2` 复制覆盖到普通存档上。

### 艾尔登法环 黑夜君临

黑夜君临是另一款游戏，有自己的文件夹：Windows 上是 `%APPDATA%\Nightreign\<你的 Steam ID>\NR0000.sl2`，Steam Deck 上则是它自己的 Proton 前缀（`compatdata/2622380`）里的同一路径。它从不会动你的艾尔登法环角色。

## 艾尔登法环有云存档吗？

有，通过 Steam 云，它会在同一账号的机器之间同步 `ER0000.sl2`。但它只保留最新状态，没有可以回退的旧版本；而且游戏的 `.bak` 和存档在同一块硬盘上，硬盘一坏，两者一起丢。

## 手动备份

1. 完全关闭游戏。
2. 把 `%APPDATA%\EldenRing` 中你的 Steam ID 文件夹复制到安全的地方。里面有 `ER0000.sl2`、它的 `.bak`，以及 Seamless Co-op 的存档（如果有）。
3. 恢复时，关闭游戏，把文件夹复制回原来的位置。

## 用 Hoard 自动备份和同步

[Hoard](/download) 免费且开源。每次游戏写入存档时它都会备份你的艾尔登法环存档，退出时再备份一次，保留可以随时回退的历史记录，并在你的 PC 和 Steam Deck 之间同步该文件夹。只有改动过的文件会被传输，而且经过压缩；每个文件取回时都会用 SHA-256 哈希校验。

1. 安装 Hoard 并登录 Hoard One（推荐所有人使用），或者把它指向[你自己的服务器](/guides/self-host-hoard)。
2. 打开**游戏库**。艾尔登法环会通过你的 Steam 库和社区存档数据库被识别出来。
3. 开始游戏。退出后，第一个版本就会出现在历史记录里。

Hoard 会带走整个 Steam ID 文件夹，所以 Seamless Co-op 的存档也在每个版本里。每个版本都是你所有角色在那一刻的状态：想回退，就[还原一个早先的版本](/guides/restore-a-game-save)；如果改变主意，更新的版本仍留在历史记录里。

## Hoard One，或者你自己的服务器

无论哪种方式 Hoard 都免费，两种方式用的也是同一个应用：

- **Hoard One**，推荐所有人使用。无需架设，也无需维护：一台位于欧盟的托管服务器，2 GB、三台设备免费，无需绑卡。[Pro](/pricing) 只是增加空间，提供 100 GB 和不限数量的设备，供存档装不下时使用。
- **你自己的服务器。** 在 PC、NAS 或树莓派上运行 `hoard-server`，你所有的机器都通过它同步。不需要我们的账号，没有发往我们的遥测，也没有我们设的配额：什么都不经过我们的服务器，因为路径上根本没有我们的东西。参见[如何自托管 Hoard](/guides/self-host-hoard)。

同一个程序，同样的识别，同样的历史记录。Hoard One 永久保留每一个版本；在你自己的服务器上，保留多少历史由你决定。

<!-- faq -->

## 常见问题

### 能在另一个 Steam 账号上使用我的艾尔登法环存档吗？

原样不行。你的 Steam ID 写在 `ER0000.sl2` 里，游戏不会加载用其他账号创建的存档。你自己账号上的你自己的存档，可以在你的 PC 和 Steam Deck 之间自由搬运。

### 还原旧存档会让我所有角色都回退吗？

会。所有角色共用 `ER0000.sl2`，所以任何一次还原都会让他们全部回到那个时刻。用 Hoard 的话，更新的版本仍留在历史记录里，回退永远不是单程票。

### Hoard 会备份 Seamless Co-op 的存档吗？

会。模组把 `ER0000.co2` 放在和普通存档同一个 Steam ID 文件夹里，而 Hoard 备份整个文件夹，所以两者都在每个版本里。

### Hoard 免费吗？

免费。应用、命令行和服务器都免费且开源（AGPL-3.0），运行你自己的服务器除了机器本身不花一分钱。我们推荐所有人使用的托管服务器 Hoard One 同样免费，提供 2 GB 和三台设备；Pro 每月 1.99 欧元，增加空间。

### 自托管会向 Hoard 发送任何东西吗？

不会。在你自己的服务器上，没有我们的账号，也没有发往我们的遥测：你的存档、你的用户和你的日志都留在你的机器上，从不接触我们的机器。
