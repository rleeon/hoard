---
title: "如何用 Docker 自托管 Hoard"
description: "用 Docker Compose 几分钟搭建你自己的 Hoard 服务器。开源、免费、运行在你自己的硬件上——一个完全自托管的游戏存档云，无需账号、没有容量限制。"
order: 0
featured: true
updated: 2026-09-03
---

Hoard 是开源且可自托管的。你可以不使用 Hoard Cloud，而是在自己的机器上运行同一个 `hoard-server`，让每台设备都连接到它——无需账号，容量只受你分配的磁盘大小限制。本指南用 Docker 在几分钟内把服务器跑起来。

## 为什么自托管 Hoard

- **完全掌控。** 你的存档保存在你自己掌控的硬件上，而不是别人的云端。
- **没有容量限制。** 空间仅受你自己的磁盘限制。
- **同一个应用，同样的功能。** 版本历史和后台同步与 Hoard Cloud 完全一致，改变的只有后端。
- **开源。** 你可以阅读、审计并修改服务器代码。

这正是它与 [Ludusavi](/guides/ludusavi-alternative) 这类工具的关键区别：Ludusavi 在本地备份和通过 Rclone「自带云」方面很出色，但同步需要你自己搭建。Hoard 则提供一个托管式的同步服务器，启动一次后每台设备都能连接。

## 自托管对你的数据意味着什么

这一点值得直说，因为多数对比在 Hoard 上正是弄错了这里。

**Hoard Cloud** 是托管方案：你登录，存档存放在我们位于欧盟的服务器上。

**自托管的 Hoard 完全属于你。** 你的设备只与你自己的服务器通信，不与任何其他地方通信。**没有我们这边的账号，没有发往我们的遥测，没有配额，也没有中转**——不经过我们的任何服务器，因为这条路径上根本没有我们的东西。我们看不到任何存档、游戏名或邮箱地址，原因很简单：这些从未到达我们这里。就算 Hoard Cloud 明天关停，你的部署照常运行。

有一点需要说准确：你的服务器确实有它自己的登录——下面你要创建的用户，以及每台设备一个令牌。它们是你的，在你的机器上、你的数据库里。不存在的是"我们这边的账号"。

## 你需要准备

- 一台保持开机的机器（家庭服务器、运行 Docker 的 NAS，或一台小型 VPS）。
- 已安装 Docker 和 Docker Compose。
- 可选：一个域名和用于 HTTPS 的反向代理（超出本地局域网的场景推荐）。

## 用 Docker Compose 安装

克隆仓库，从示例创建配置，然后启动整套服务：

```sh
git clone https://github.com/rleeon/hoard.git && cd hoard
mkdir -p deploy/docker/config
cp deploy/config.toml.example deploy/docker/config/config.toml
$EDITOR deploy/docker/config/config.toml      # Use nano or vim or something lol

cd deploy/docker
docker compose up -d
docker compose logs -f                         # wait for "listening"
```

等待日志显示服务器正在监听。数据保存在一个命名的 Docker 卷（`hoard-data`）中——像备份其他卷一样备份它。容器内部监听 `12421` 端口；用 `HOARD_PORT=9000 docker compose up -d` 可映射到其他主机端口。

## 创建用户和设备令牌

服务器没有注册页面——用户通过命令行创建：

```sh
docker compose exec server hoard-admin --config /etc/hoard/config.toml \
    user create alice --admin --password 'CHANGE_ME'
docker compose exec server hoard-admin --config /etc/hoard/config.toml \
    token create alice --device 'desktop'
```

令牌只显示一次，**之后无法找回**，请立即复制。

## 连接桌面应用

在每台机器上安装 [Hoard 桌面应用](/download)。在初始引导中选择 **自托管**，然后粘贴你的服务器 URL 和刚创建的令牌。之后它的行为与 Hoard Cloud 完全相同：检测你的游戏、自动备份存档、保留版本历史。日常用法请参见[在多台 PC 之间同步存档](/guides/sync-game-saves-across-pcs)。

## 保持服务器更新

怎么更新取决于你是怎么安装的，而且用错命令不会报错，只是什么都不做 —— 所以值得先弄清楚哪一种是你的情况。

**Docker Compose.** 拉取新镜像并重建容器。两条都要执行，按顺序：

```sh
docker compose pull
docker compose up -d
```

只执行第一条的话，旧容器会原封不动地继续运行：`/v1/health` 仍然报告旧版本，看起来就像更新悄悄失败了。`git pull` 两者都更新不了 —— 运行的是已发布的镜像，不是你的代码副本。如果你想自己决定什么时候用上新版本，把 `:latest` 换成固定版本（`ghcr.io/rleeon/hoard:1.1`）。

**Unraid.** *Docker* 标签页 → Hoard → 出现更新时点 *Apply update*。不需要输入任何命令。

**裸机（systemd）.** 先 `sudo hoard-server upgrade`，再 `sudo systemctl restart hoard-server`。它会原子地替换二进制文件，并且故意不自己重启服务，以免中断正在进行的同步。

`hoard-server upgrade` 只适用于裸机安装。在容器里它会故意拒绝执行 —— 替换后的二进制文件撑不过下一次 `docker compose up -d` —— 并改为打印上面那两条命令；想亲眼看看的话，执行 `docker compose exec server hoard-server upgrade`。数据库迁移由服务器在启动时应用，所以永远不需要单独的步骤。

## 在生产环境中运行

对于任何暴露到本地网络之外的部署，请在反向代理（Caddy、nginx 或 Traefik）上终止 TLS。更喜欢裸机部署？仓库还提供了 `systemd` 安装脚本，以及一个 `hoard-server upgrade` 命令，它会原子地替换二进制文件而不会中断正在进行的同步。

## 自托管还是 Hoard Cloud？

如果你已经在运行服务器并希望完全掌控、没有容量限制，自托管是理想选择。如果你不想维护基础设施，[Hoard Cloud](/pricing) 提供由我们托管的同样同步功能，并有免费档可供起步。无论哪种方式，应用和你的存档都保持可迁移——以后可以随时切换。

<!-- faq -->

## 常见问题

### 自托管的 Hoard 会回连你们吗？

不会。桌面应用只与你给它的服务器地址通信。你的存档、你的用户和你的日志都留在你的机器上，其中没有任何内容会到达我们这里。

### 自托管服务器和 Hoard Cloud 是同一份代码吗？

是的，同一个 `hoard-server` 二进制，采用 AGPL-3.0。没有功能删减的社区版，也没有只留给托管版的功能。

### 存档实际保存在哪里？

默认在你分配给容器的 Docker 卷里，也就是你自己的磁盘上。如果你已经在跑对象存储，服务器同样支持 S3，MinIO、Garage 或 Backblaze B2 都可以作为后端。无论哪种方式，你的设备始终只与你的服务器通信。

### 可以跑在 NAS 上吗？

可以，任何能运行 Docker 的 NAS 都行。仓库里附带了 Unraid 模板，镜像会降权到你指定的 `PUID`/`PGID`，这样绑定挂载的文件夹归属正确的用户，而不是 root。

### 需要域名和 HTTPS 吗？

在自家局域网里不需要。一旦服务器可以从外部访问，就在前面放一个反向代理并在那里终止 TLS——Caddy、nginx 或 Traefik 都可以。

### 如果我玩完时服务器正好没开呢？

快照是在本地生成的，不会丢失任何东西。等服务器重新响应，它会自行上传。

### 可以先用 Hoard Cloud，以后再迁移吗？

可以，双向都行。你能在账号页面导出全部数据，应用也可以指向另一台服务器，无需重装。
