# 本地游戏资源

本目录随应用打包，运行时不请求 Steam 或 GenMod 业务接口。清单优先中小体量独立游戏与 MOD 生态，不是实时热度排行榜；鸭科夫、星露谷固定排前两位。

## 管理规则

- `catalog.json`：`schemaVersion: 1`，`games` 保存全部游戏。稳定 `id` 不随显示名称变化。
- 每个游戏独立文件夹：`images/<id>/cover.jpg` 保存横版封面，`images/<id>/icon.jpg` 保存独立方形图标。
- 封面格式：460 × 215 JPEG；图标采用 Steam 原始方形 JPEG，不拉伸放大。每张不超过 2 MiB；不使用热链接或 Base64。
- `cover`、`icon` 均为相对于本目录的本地路径，`steamAppId` 为 Steam 商店 ID，`sortOrder` 唯一并升序展示。
- `supportStatus` 为 `supported` 或 `planned`。目前鸭科夫与星露谷开放创建；实际还需对应 Skill 有效并启用。
- 新增或调整游戏时，同步清单、对应图片文件夹与下方来源；不要遗留无引用图片。
- 图片缺失、损坏或路径越界应显示资源错误，不回退到网络。
- 未来接口沿用清单中的字段；封面与图标字段仍表示资源引用，本轮没有远程同步。

## 图片来源

2026-11-06 通过 Steam 商店页面或 appdetails 核对 AppID，从 Steam 官方 CDN 下载，以下商店名称用于核对 AppID。图片属于各自权利人。

| 游戏目录 | Steam AppID / 商店名称 | 图片来源 |
| --- | --- | --- |
| `duckov` | [3167020 · Escape From Duckov](https://store.steampowered.com/app/3167020/) | [cover.jpg](https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/3167020/header.jpg?t=1788419722) |
| `stardew-valley` | [413150 · Stardew Valley](https://store.steampowered.com/app/413150/) | [cover.jpg](https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/413150/header.jpg?t=1786554168) |
| `rimworld` | [294100 · RimWorld](https://store.steampowered.com/app/294100/) | [cover.jpg](https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/294100/76e0613b8197327299d5dbabb0a7fd7828a689a2/header.jpg?t=1782756789) |
| `terraria` | [105600 · Terraria](https://store.steampowered.com/app/105600/) | [cover.jpg](https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/105600/header.jpg?t=1792530636) |
| `project-zomboid` | [108600 · Project Zomboid](https://store.steampowered.com/app/108600/) | [cover.jpg](https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/108600/header.jpg?t=1793224893) |
| `dont-starve-together` | [322330 · Don't Starve Together](https://store.steampowered.com/app/322330/) | [cover.jpg](https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/322330/050ea9c51a4e96f707309d33b7cdd38e0a6cbfff/header_alt_assets_51.jpg?t=1793044716) |
| `factorio` | [427520 · Factorio](https://store.steampowered.com/app/427520/) | [cover.jpg](https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/427520/ab9a2df0a77caebde20dc45adc021de8cdcf7cc5/header.jpg?t=1792752343) |
| `slay-the-spire` | [646570 · Slay the Spire](https://store.steampowered.com/app/646570/) | [cover.jpg](https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/646570/header.jpg) |
| `balatro` | [2379780 · Balatro](https://store.steampowered.com/app/2379780/) | [cover.jpg](https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/2379780/header.jpg) |
| `oxygen-not-included` | [457140 · Oxygen Not Included](https://store.steampowered.com/app/457140/) | [cover.jpg](https://cdn.cloudflare.steamstatic.com/steam/apps/457140/header.jpg) |
| `noita` | [881100 · Noita](https://store.steampowered.com/app/881100/) | [cover.jpg](https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/881100/header.jpg?t=1675987871) |
| `brotato` | [1942280 · Brotato](https://store.steampowered.com/app/1942280/) | [cover.jpg](https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/1942280/bf970261a3a42f346624ae9c48b5b317fc724857/header_alt_assets_4.jpg?t=1793641955) |
| `core-keeper` | [1621690 · Core Keeper](https://store.steampowered.com/app/1621690/) | [cover.jpg](https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/1621690/header.jpg) |
| `rain-world` | [312520 · Rain World](https://store.steampowered.com/app/312520/) | [cover.jpg](https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/312520/header.jpg) |
| `barotrauma` | [602960 · Barotrauma](https://store.steampowered.com/app/602960/) | [cover.jpg](https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/602960/header.jpg?t=1793897290) |
| `mindustry` | [1127400 · Mindustry](https://store.steampowered.com/app/1127400/) | [cover.jpg](https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/1127400/header.jpg) |
| `starbound` | [211820 · Starbound](https://store.steampowered.com/app/211820/) | [cover.jpg](https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/211820/header.jpg) |
| `prison-architect` | [233450 · Prison Architect](https://store.steampowered.com/app/233450/) | [cover.jpg](https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/233450/header.jpg?t=1759221867) |
| `battle-brothers` | [365360 · Battle Brothers](https://store.steampowered.com/app/365360/) | [cover.jpg](https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/365360/c6ad528c1ec98716cccc6b251d570bd1c0f2d283/header.jpg?t=1793270141) |
| `lethal-company` | [1966720 · Lethal Company](https://store.steampowered.com/app/1966720/) | [cover.jpg](https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/1966720/header.jpg) |

## 图标来源

2026-11-06 下载 Steam 原始方形游戏图标。《监狱建筑师》取自官方游戏组 XML 的 avatarIcon，其余取自各游戏社区首页的应用图标。图标原样保留，不由横版封面裁剪。

| 游戏目录 | 图标来源 |
| --- | --- |
| `duckov` | [icon.jpg](https://shared.fastly.steamstatic.com/community_assets/images/apps/3167020/5bb8d31708442a8f3ddbc100d7be7c3fde530d18.jpg) |
| `stardew-valley` | [icon.jpg](https://shared.akamai.steamstatic.com/community_assets/images/apps/413150/35d1377200084a4034238c05b0c8930451e2fb40.jpg) |
| `rimworld` | [icon.jpg](https://shared.akamai.steamstatic.com/community_assets/images/apps/294100/addbe3b704b267060b4d5d7649cfb292de61bd70.jpg) |
| `terraria` | [icon.jpg](https://shared.fastly.steamstatic.com/community_assets/images/apps/105600/858961e95fbf869f136e1770d586e0caefd4cfac.jpg) |
| `project-zomboid` | [icon.jpg](https://shared.fastly.steamstatic.com/community_assets/images/apps/108600/2bd4642ae337e378e7b04a19d19683425c5f81a4.jpg) |
| `dont-starve-together` | [icon.jpg](https://shared.fastly.steamstatic.com/community_assets/images/apps/322330/a80aa6cff8eebc1cbc18c367d9ab063e1553b0ee.jpg) |
| `factorio` | [icon.jpg](https://shared.fastly.steamstatic.com/community_assets/images/apps/427520/267f5a89f36ab287e600a4e7d4e73d3d11f0fd7d.jpg) |
| `slay-the-spire` | [icon.jpg](https://shared.fastly.steamstatic.com/community_assets/images/apps/646570/33ea124ea8c03a9ce7012d34c3b348a351612fca.jpg) |
| `balatro` | [icon.jpg](https://shared.akamai.steamstatic.com/community_assets/images/apps/2379780/b6018068070ab0e23561694c11f7950dd6f4c752.jpg) |
| `oxygen-not-included` | [icon.jpg](https://shared.fastly.steamstatic.com/community_assets/images/apps/457140/d37534163ebac8351e0bda0bf30f8f2174c921b6.jpg) |
| `noita` | [icon.jpg](https://shared.fastly.steamstatic.com/community_assets/images/apps/881100/a21dbd1d3bd01f0a688f2d7029978d5cdbe14e74.jpg) |
| `brotato` | [icon.jpg](https://shared.fastly.steamstatic.com/community_assets/images/apps/1942280/d9dfcab8f300a9548439f70bef7b5f4e53d0fc28.jpg) |
| `core-keeper` | [icon.jpg](https://shared.fastly.steamstatic.com/community_assets/images/apps/1621690/53517a1b2eb22e22ac4a1f71c814209f1b84c063.jpg) |
| `rain-world` | [icon.jpg](https://shared.fastly.steamstatic.com/community_assets/images/apps/312520/5854494b840a18a660a495d6259d562b51f21240.jpg) |
| `barotrauma` | [icon.jpg](https://shared.akamai.steamstatic.com/community_assets/images/apps/602960/a0adb0f29ce819c6dffa32446624fb7648defc5a.jpg) |
| `mindustry` | [icon.jpg](https://shared.fastly.steamstatic.com/community_assets/images/apps/1127400/f519c34b9f3bd1983d224d3a2b6c2a60cf962211.jpg) |
| `starbound` | [icon.jpg](https://shared.fastly.steamstatic.com/community_assets/images/apps/211820/4dbf4041c33eb2edfc41f552320a2e584519134c.jpg) |
| `prison-architect` | [icon.jpg](https://avatars.fastly.steamstatic.com/c166c7911beec4d63a74cdddf25f26b73c84556b.jpg) |
| `battle-brothers` | [icon.jpg](https://shared.fastly.steamstatic.com/community_assets/images/apps/365360/b99db2c64ea125ff01c2d31583ff560a834e999c.jpg) |
| `lethal-company` | [icon.jpg](https://shared.fastly.steamstatic.com/community_assets/images/apps/1966720/80d2453285274e4723c884a671cdd3f8fe2f766f.jpg) |
