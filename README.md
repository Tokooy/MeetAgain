# MeetAgain · 再相逢

**各自生活，还会再相逢吗？** 在南京、镇江、扬州之间模拟独立出行，观察同行者何时同城相遇。地图显示三座城市的实际道路和行政边界。

Rust 模拟引擎与 React 可视化界面分目录开发，最终交付 **一个 Docker 镜像、一个后端进程、一个 HTTP 端口**。后端提供 JSON API，并通过 `tower-http::ServeDir` 托管静态前端。无数据库、地图密钥或运行时外部瓦片依赖。

![深色地图与模拟结果](docs/screenshots/desktop-dark.png)

[浅色界面](docs/screenshots/desktop-light.png) · [手机界面](docs/screenshots/mobile.png) · [API 契约](docs/API.md) · [逐模块交付记录](docs/DELIVERY.md)

## Docker 运行

支持 Linux amd64。三个构建阶段分别使用固定 digest 的 Node slim、Rust slim 和 Debian bookworm-slim；Cargo/npm 锁文件已提交。运行用户为 `65532:65532`，镜像包含后端二进制、前端 `dist/` 和默认地图数据。

```sh
docker build --platform=linux/amd64 -t meetagain:latest .
docker run --rm --name meetagain -p 8080:8080 meetagain:latest
```

浏览器访问本机的 8080 端口。默认命令为 `serve --static-dir /app/static`。没有第二个 Web 服务或容器。

在本云环境的 HTTPS 代理下，使用可信 CA 的 BuildKit secret，保持 TLS 校验；CA 不写入镜像：

```sh
docker build --platform=linux/amd64 \
  --secret id=proxy_ca,src=/etc/ssl/certs/ca-certificates.crt \
  -t meetagain:latest .
```

Dockerfile 固定的是依赖和基础镜像输入，不承诺不同构建时间产生相同镜像字节。更新依赖时需重新验证固定种子结果。

## 本地开发

工具版本：Rust **1.90.0**（`rust-toolchain.toml` 自动选择）、Node **24.22.1**、npm 11。使用已有 checkout 即可，不需要额外 worktree。

```sh
npm --prefix frontend ci
npm --prefix frontend run build
cargo run --locked -p meetagain -- serve \
  --address 127.0.0.1:8080 --static-dir frontend/dist
```

后端编译无需前端构建产物。前端修改后重新执行 `npm --prefix frontend run build` 并刷新页面；后端修改后重启服务。两侧仅以 HTTP/JSON 交互，前端不导入后端代码或地理数据文件。

```text
backend/
  data/map/         城市清单、模拟连线、行政边界、街道 GeoJSON 和来源
  src/graph.rs      图加载、校验、距离、最短返乡路径
  src/simulation.rs agent 移动、轨迹、相遇与汇总
  src/api.rs        API、缓存、分页、静态资源
frontend/
  src/components/   地图、控制面板、回放、结果与折叠布局
  src/api.ts        HTTP 客户端与下载
  src/store.ts      主题、播放和选择状态
  e2e/              浏览器流程、无障碍与 JSON Schema 契约测试
scripts/import-osm-map.py  从 OSM PBF 导入城市数据
docs/               API Schema、截图、交付证据
```

前端使用 React 19、TypeScript strict、Vite、Tailwind CSS v4、MapLibre GL JS v6、Recharts v3、Zustand 和 TanStack Query。深浅主题可切换；窄屏控制与结果默认折叠；位置过渡 200ms，遵循 `prefers-reduced-motion`。地图 worker 由 Vite 独立打包，不从 CDN 加载。

## 道路数据在哪里

道路和行政边界来自 **OpenStreetMap**。默认地图使用 [Geofabrik 江苏下载页](https://download.geofabrik.de/asia/china/jiangsu.html) 上的 [2026-11-16 江苏 PBF 快照](https://download.geofabrik.de/asia/china/jiangsu-261116.osm.pbf)。可从下载页选择更新的 PBF；其他省份或国家可从 [Geofabrik 下载目录](https://download.geofabrik.de/) 找到对应区域。选择包含目标城市完整边界及道路的区域文件。

| 城市 | 城市 ID | OSM 行政边界 | 导入的道路分段 |
| --- | --- | --- | ---: |
| 南京市 | `320100` | [relation 2131524](https://www.openstreetmap.org/relation/2131524) | 46,963 |
| 镇江市 | `321100` | [relation 3429680](https://www.openstreetmap.org/relation/3429680) | 19,997 |
| 扬州市 | `321000` | [relation 4430826](https://www.openstreetmap.org/relation/4430826) | 24,589 |

导入器把高速、主干道、支路、居民区道路、服务道路等裁剪至每座城市的行政区边界。地图缩放后显示更多街道，点击道路可查看名称或道路编号（源数据有记录时）。数据保留 OSM way ID、道路类型和可用的桥梁、隧道及单向属性。数据覆盖随 OpenStreetMap 更新。见[数据快照、SHA-256 与许可](backend/data/map/SOURCES.md)。地图数据 © [OpenStreetMap contributors](https://www.openstreetmap.org/copyright)，采用 ODbL 1.0。

内置数据在 `backend/data/map/cities/<城市 ID>/`；每座城市有 `boundary.geojson` 和 `streets.geojson.gz`。下载单城道路，可请求 `GET /api/map/roads/{city_id}`，例如：

```sh
curl -fsS --compressed http://127.0.0.1:8080/api/map/roads/320100 -o nanjing-streets.geojson
```

## 添加城市

新城市通过 **导入命令 + `map.json` 清单**添加，无需修改 Rust 或 React 源码。下面以常州为示例，默认地图仍只加载南京、镇江和扬州。

1. 下载覆盖该市的 OSM PBF。在 [OpenStreetMap](https://www.openstreetmap.org/) 搜索城市，打开行政区边界的 `relation` 页面并记下 ID。常州示例为 [relation 3429666](https://www.openstreetmap.org/relation/3429666)。
2. 复制 `backend/data/map` 到自定义目录。使用 Python 3.10 或更新版本，安装依赖并导入。以下命令在项目根目录运行，PBF 已下载至当前目录。

   ```sh
   cp -R backend/data/map ../meetagain-map
   python -m pip install -r scripts/map-requirements.txt
   python scripts/import-osm-map.py jiangsu-261116.osm.pbf --map-dir ../meetagain-map \
     --city 320400=3429666 --source-url https://download.geofabrik.de/asia/china/jiangsu-261116.osm.pbf
   ```

   PowerShell 可用 `Copy-Item -Recurse backend/data/map ../meetagain-map` 复制，再将导入命令写在一行。可以重复传入 `--city 城市ID=关系ID` 批量导入。导入从本地 PBF 读取，不调用在线服务。
3. 打开 `../meetagain-map/map.json`，在 `cities` 中追加 `"320400"`，在 `links` 中追加 `["321100", "320400"]`，表示镇江和常州可以相互出行。ID 可用行政区划代码，也可用 1 至 32 位的小写字母、数字、短横线和下划线，首字符为字母或数字。城市连接必须连通。
4. 校验清单并用它启动应用：

   ```sh
   cargo run --locked -p meetagain -- graph --map-dir ../meetagain-map
   cargo run --locked -p meetagain -- serve --map-dir ../meetagain-map --static-dir frontend/dist
   ```

   Docker 用户可将自定义目录以只读方式挂载到 `/custom-map`，在 `serve --static-dir /app/static` 后加 `--map-dir /custom-map`。如果已有容器使用默认路径，也可以用 `docker cp ../meetagain-map/. 容器名:/app/map` 更新数据，再执行 `docker restart 容器名`。每座城市的道路文件解压后最多为 64 MiB。

   服务启动后，`GET /api/graph` 会列出新城市和道路分段数量，`GET /api/map/roads/320400` 会返回常州的道路 GeoJSON。

## 行为模型

1. 每个 agent 独立、均匀地随机选择一个初始城市，同时把它作为 home。
2. 每天，在 home 时以 `0.10` 概率出行，在外时以 `0.25` 概率出行，否则停留。
3. 决定出行且不在 home 时，以 `0.60` 概率沿加权最短路径向 home 移动到相邻城市；否则在邻居中按 `1 / distance_km` 加权选择。
4. 全部 agent 同步更新；不共享位置、不追逐、不协调目的地。每个模拟日最多沿一条边移动。
5. 同一天、同城市的每一对 agent 产生一个事件。三人同城产生三对；同一对连续同城，每天各记一次。

每个 tick 对应一个模拟日，tick 0 表示初始位置，默认不计入相遇。只匹配同一天的位置，途中交叉不计为相遇。模拟日用于安排出行决策，距离不影响旅程所用的模拟天数。

勾选「所有人从同一座家乡城市出发」后，随机抽取一座城市作为所有人的家乡和初始位置。每个人仍独立出行，同城仍按现有规则记录相遇。

网页用「分开日期」作为旅程编号，默认为 2026-11-16，可选 1901–2099 年内的模拟区间。正月初一由农历换算得到。节前 21 天逐日提高外地出发后沿最短路线返乡的概率，初一到初七保持最高值（例如平日 60% 升至 90%），节后 7 天内逐日降回平日值。CLI 可用 `--start-date YYYY-MM-DD` 设置分开日期，用 `--same-hometown` 选择共同家乡。

默认模拟在南京、镇江、扬州之间使用三条加权无向连接。地图上的实线来自 OSM 道路，虚线表示模拟中的城市连接。模拟移动和相遇以城市为单位；距离权重近似为城市中心之间的球面距离。

## 离线 CLI 与可复现性

```sh
cargo run --locked -p meetagain -- graph
cargo run --locked -p meetagain -- simulate \
  --agents 20 --ticks 365 --seed 20261116 > result.json

# 同一镜像也支持无网络的离线模拟，覆盖默认 serve 命令：
docker run --rm --network none meetagain:latest \
  simulate --agents 20 --ticks 365 --seed 20261116 > result.json
```

CLI 可用 `--home-move`、`--away-move`、`--home-return` 调整概率，`--include-tick-zero` 计入初始同城。

ChaCha8、固定 seed、稳定的城市/邻居遍历顺序与锁定依赖实现同代码、同地图版本和同配置复现。seed 为十进制 u64 字符串，避免 JavaScript 整数精度损失。Agent ID 从 0 开始，界面显示编号从 01 开始。

输出包含每人的完整 tick 轨迹、家与起终城市、移动次数、访问城市和近似距离，以及全部 `(tick, city_id, agent_ids)` 事件。事件按 tick、城市 ID、agent ID 排序。汇总分别报告至少一对相遇、去重相遇人数对、首次 tick，以及全员同时相聚。API 的运行 ID 为独立随机标识，不属于可复现模拟输出。

有限模拟可以证明某次相遇发生过，不能从零事件推出永不相遇。默认正概率、可停留、独立移动的有限连通 Markov 模型在无限时域以概率 1 到达相聚状态；这不等于现实承诺，也不适用于把某些移动概率设为零后失去遍历性的配置。

## API

| 方法 | 路径 | 用途 |
|---|---|---|
| GET | `/api/health` | 图加载成功后的健康状态 |
| GET | `/api/graph` | 城市、边界、模拟连线、地图版本与来源 |
| GET | `/api/map/roads/{city_id}` | 活动城市的道路 GeoJSON，支持 gzip |
| POST | `/api/simulations` | 同步执行模拟，返回 201、汇总和轨迹 |
| GET | `/api/simulations/{id}/encounters` | 分页事件，`limit` 1–1000，`cursor` 不透明 |

```sh
curl -fsS -X POST http://127.0.0.1:8080/api/simulations \
  -H 'Content-Type: application/json' \
  -d '{"agents":20,"ticks":365,"seed":"20261116","movement":{"home_move_probability":0.1,"away_move_probability":0.25,"home_return_probability":0.6},"encounters":{"include_tick_zero":false,"cross_tick_tolerance":0}}'
```

拒绝未知字段；`same_hometown` 可省略。`start_date` 和旧版 `seed` 至少提供一项。N 为 2–100，ticks 为 1–5300（模拟天数），且最坏成对检查数
`N * (N - 1) / 2 * (ticks + include_tick_zero)` 不超过 500,000。观察 5,300 天时，最多可选择 14 人。界面中的「分开日期」对应 API 的 `start_date`。相同的日期和设置可以重现家乡、每日轨迹及相遇结果。超限返回错误，不截断结果。

单次模拟请求体最多 16 KiB；一次只执行一个模拟任务，并发请求返回 JSON 429。结果缓存在内存中，15 分钟过期，最多保留 4 次，先进先出淘汰；重启丢失。前端提供轨迹和完整结果下载，并提示缓存过期。完整形状及错误码见 [API 契约](docs/API.md) 和 [JSON Schema](docs/api.schema.json)。

## 检查与调试

```sh
bash scripts/check.sh
npm --prefix frontend run format:check

# 先启动上面的服务；也可直接测试已运行的 Docker 容器：
npm --prefix frontend run test:e2e
```

Playwright 优先使用系统 `/usr/bin/chromium`；可用 `CHROMIUM_PATH` 指定浏览器。没有系统浏览器时执行 `cd frontend && npx playwright install chromium`。可用 `BASE_URL` 指向另外的测试地址。

Rust 测试覆盖图验证、连通性、返乡、同步移动、独立重建事件、seed 复现、CLI、API 错误、分页、并发及缓存。浏览器测试覆盖实际渲染的地图、完整模拟/下载、主题、播放、筛选、窄屏、减少动画与自动化 WCAG AA 检查。Ajv 校验真实 HTTP 响应与已确认 Schema。

`RUST_LOG=meetagain=debug,tower_http=debug` 开启服务日志，写入 stderr，不污染 CLI JSON。`cargo run -- --help` 查看全部命令。服务响应 SIGINT/SIGTERM，可正常停止。
