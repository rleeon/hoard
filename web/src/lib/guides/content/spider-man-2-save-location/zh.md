---
title: "漫威蜘蛛侠 2（Marvel's Spider-Man 2）存档位置（PC 与 Steam Deck）"
description: "Marvel's Spider-Man 2 的 PC 存档位置、那个长数字文件夹是什么、OneDrive 的陷阱、Steam Deck 上的路径，以及如何备份存档。"
order: 22
updated: 2026-10-02
---

在 PC 上，Marvel's Spider-Man 2 把存档放在 `文档\Marvel's Spider-Man 2\` 里一个以长数字命名的子文件夹中。在 Steam 上，这个数字就是你的 Steam ID。下面介绍这在实际中意味着什么、OneDrive 的陷阱、Steam Deck 上的路径，以及如何让存档一直有备份。

## Marvel's Spider-Man 2 的存档位置

- **Windows：**`%USERPROFILE%\Documents\Marvel's Spider-Man 2\<长数字>`
- **Steam Deck 和 Linux**（Proton）：`~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/<长数字>`

PC 版只支持 Windows，所以在 Steam Deck 上通过 Proton 运行，存档位于 Steam 为该游戏维护的 Proton 前缀中；`2651280` 是它的 Steam 应用 ID。如果装在 microSD 卡上，`compatdata` 文件夹就在卡上。

负责 PC 移植的工作室 Nixxes 把存档文件夹描述为 `文档\Marvel's Spider-Man 2\` 下“一个以长数字或字母数字组合命名的子文件夹”。

## 长数字文件夹

子文件夹以你的账号命名：在 Steam 上是你的 **64 位 Steam ID**；Epic 版则用字母和数字的组合。无论哪种，每个账号都不一样。由此有两个后果：

- 如果两个人在同一台 PC 上用不同的 Steam 账号玩，每人都有自己的存档文件夹。
- 如果你手动把存档复制到另一台 PC，要放进**那台**设备上 Steam 账号对应的文件夹。放进 ID 不同的文件夹，游戏是看不到的。

上一级的 `Marvel's Spider-Man 2` 文件夹里还有游戏日志和崩溃转储（`.log`、`.mdmp`）。它们不是存档，不需要备份。

## OneDrive 的陷阱

很多 Windows PC 会把“文档”重定向到 OneDrive。如果你也是这样，真实路径是 `%USERPROFILE%\OneDrive\Documents\Marvel's Spider-Man 2\`，而且在你游玩时 OneDrive 会自行同步这个文件夹。这会带来两个问题：OneDrive 可能上传写到一半的存档，“释放空间”还可能把存档变成仅在线的占位文件。如果你在这里依赖 OneDrive，请把文件夹设为**始终保留在此设备上**。

## Spider-Man 2 有云存档吗？

有，Steam 云会在同一 Steam 账号的设备之间同步最新的存档。它不保留旧版本：存档一旦损坏，同步过去的就是损坏的那份。

## 手动备份

1. 完全关闭游戏。
2. 把“文档”里的 `Marvel's Spider-Man 2` 文件夹复制到安全的地方。
3. 恢复时，关闭游戏，把长数字文件夹复制回同一位置，使用同一个 Steam 账号。

## 用 Hoard 自动备份和同步

[Hoard](/download) 会在你每次停止游戏时备份存档文件夹，并保留每一个版本。它还会在你的 PC 和 Steam Deck 之间同步，让游戏在任意一台上都能接着玩。

1. 安装 Hoard 并登录，或把它指向[你自己的服务器](/guides/self-host-hoard)。
2. 打开**库**，确认 Spider-Man 2 显示的文件夹是“文档”（或 `OneDrive\Documents`）下的那个。如果指向别处，请改成该文件夹。
3. 开始游戏。退出后，第一个版本就会出现在历史记录里。

如果之后存档出了问题，[恢复旧版本](/guides/restore-a-game-save)就能把它找回来。

<!-- faq -->

## 常见问题

### 存档文件夹里的长数字是什么？

在 Steam 上是你的 64 位 Steam ID；在 Epic 上是你的账号 ID。每个账号都有自己的文件夹，游戏只读取当前登录账号的那个。

### Steam Deck 上的 Spider-Man 2 存档在哪里？

在 Proton 前缀里：`~/.local/share/Steam/steamapps/compatdata/2651280/pfx/drive_c/users/steamuser/Documents/Marvel's Spider-Man 2/` 下的长数字文件夹。

### 我在“文档”里找不到这个文件夹，它在哪里？

去 `OneDrive\Documents\Marvel's Spider-Man 2` 找找。在大多数新装的 Windows 上，“文档”位于 OneDrive 里。

### 我能把存档复制到朋友的 PC 上吗？

文件可以复制，但要放进那台 PC 上账号的 Steam ID 文件夹里。游戏是否接受用其他账号创建的存档取决于游戏本身，所以尝试前先保留一份原始存档。
