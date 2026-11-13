# 云原生开发会话完整记录（2026-11-13）

环境 `cnb-cvn-1k7a6brg8`，仓库 slug `h1s97x/SduNetCheckTool`，时间 2026-11-13 18:13–19:24 (+08:00)。
来源：平台 AI 请求审计（`cnb build get-build-ai-request*`），共 83 次请求、同一会话 `f05046ab6c884ddc963d5e76a5ba69ec`。
工具调用的入参/出参已折叠说明，只保留人与 AI 的自然语言往来。

---

## 1. 我（h1s97x）

1.调研了解一下这个仓库。
2、我有两个方向：1.接手这个项目的开发，维护。但这样对于我自身而言觉得无趣。2.基于这个项目出一些新的idea。我从零做，这样所有的归属权都在我，我比较喜欢MIT协议

> `[工具结果] {"status":"success","success":true,"result":{"type":"todo_write_result","merge":false,"todos":[{"id":"1","status":"in_pr…`
> `[工具结果] /workspace/   - SduNetCheckTool.Core/     [21 files in subtree: 20 *.cs, 1 *.csproj]   - SduNetCheckTool.GUI/     [30 fi…`
> `[工具结果] Found 1 files LICENSE.txt (34.33 KB)…`
> `[工具结果] Execution Summary: 102 tool uses, cost: 486.32s <persisted-output> # /workspace 只读调研结论  **一句话**：这是一个面向山东大学青岛校区的 **Window…`
> `[工具结果]      1:# /workspace 只读调研结论      2:      3:**一句话**：这是一个面向山东大学青岛校区的 **Windows 桌面端「一键校园网体检/修复」工具**（C# / WPF / .NET Framewor…`
> `[工具结果]      1:# SduNetCheckTool      2:      3:为山东大学青岛校区开发的一键式校园网检测工具，欢迎Pull Request && Issue。      4:      5:## 实现的功能 && TODO …`
> `[工具结果]      1:                    GNU GENERAL PUBLIC LICENSE      2:                       Version 3, 29 June 2007      3:     …`
> `[工具结果] {"status":"success","success":true,"result":{"type":"execute_command_result","stdout":"=== commit count ===\r\n\r\n45\r\…`
> `[工具结果]      1:# Copyright (c) SduNetCheckTool      2:# Copyright (c) Microsoft Corporation.      3:# Licensed under the MIT lic…`
> `[工具结果]      1:using System;      2:using SduNetCheckTool.Core.Repairs;      3:      4:namespace SduNetCheckTool.Core.Tests     …`
> `[工具结果]      1:using System.Collections.Generic;      2:using System.Collections.ObjectModel;      3:using System.Threading.Task…`
> `[工具结果] {"status":"success","success":true,"result":{"type":"execute_command_result","stdout":"=== h1s97x 提交 ===\r\n\r\n0a5c823 …`
> `[工具结果] {   "instruction": "These tools are NOT directly callable (they are not in your tool list). To run one, call DeferExecut…`
> `[工具结果] {"status":"success","success":true,"result":{"type":"web_fetch_tool_result","data":"{\"id\":664604109,\"node_id\":\"R_kg…`
> `[工具结果]      1:// internal class ProxySetting code from https://github.com/2dust/v2rayN/blob/master/v2rayN/v2rayN/Handler/ProxyS…`
> `[工具结果]      1:using System;      2:using System.Text;      3:using SduNetCheckTool.Core.Repairs;      4:using SduNetCheckTool.C…`
> `[工具结果]      1:using CommunityToolkit.Mvvm.ComponentModel;      2:using SduNetCheckTool.Core.Repairs;      3:using SduNetCheckTo…`
> `[工具结果]      1:<UserControl      2:    x:Class="SduNetCheckTool.GUI.Views.AboutView"      3:    xmlns="http://schemas.microsoft.…`
> `[工具结果] {"status":"success","success":true,"result":{"type":"todo_write_result","merge":true,"todos":[{"id":"1","status":"comple…`
### AI

## 一、仓库调研结论

**它是什么**：面向山东大学青岛校区的 Windows 桌面「一键校园网体检/修复」工具，WPF + .NET Framework 4.8，单文件 exe 交付。三层结构 `Core`（系统能力）← `Mvvm`（ViewModel）← `GUI`（XAML），DI + `WeakReferenceMessenger` 解耦，规模约 2129 行 C# + 480 行 XAML。

**关键事实**（本地 git + 上游 GitHub API 核实）：

| 维度 | 结论 |
|---|---|
| 生命周期 | 45 次提交，2023-07-10 建库 → **2024-03-11 最后一次推送，至今约 2 年 8 个月无提交**；6 个 tag，最新 `1.0.5` |
| 贡献者 | 6 人，但 Forgot-Dream 一人占 34 次；另有 6 次来自 刘振伟 |
| 社区热度 | 上游 5 star / 6 fork / **0 open issue、0 PR**，未归档但事实停更 |
| 协议 | **GPLv3**（`LICENSE.txt` 为完整未改动全文，README 与关于页均确认） |
| 测试/CI | 无单元测试项目、CI 无测试步骤；CI 仍装已 EOL 的 dotnet 3.1.x，`Costura.Fody` 靠 CI 临时 `dotnet add` 注入，构建不可复现 |
| 主要技术债 | 校园网 IP/站点/DNS/报告路径全硬编码；README 8 项功能中有孤儿代码；大量 `catch { // ignored }`、`.Result` 同步阻塞、`Task.Run` 里改 `ObservableCollection` |

**真正值得带走的设计思想**（不是代码）：

1. 「检测 → 结果+说明 → 修复器」三元组契约，诊断与自愈解耦、天然可扩展：

```12:12:SduNetCheckTool.Core/Tests/ITest.cs
Tuple<TestResult,string,IRepair> Test();
```

2. 状态机驱动 UI：4 态枚举 + `IValueConverter` 映射图标/进度，XAML 零后台逻辑。
3. 「可分享的诊断证据」：一键复制结果、导出桌面报告 —— 面向小白的闭环。
4. 关于页里那个 **「校园网炸了么 2 群」链接**，是被原项目完全没解决、却最真实的用户痛点：*到底是我断了，还是全校断了*。

---

## 二、方向一：接手开发维护

坦率说，**这条路和你的两个诉求都是冲突的**：

- **归属权天花板**：6 位贡献者共同持有版权，GPLv3 是 copyleft。即使你成为唯一 maintainer，也**无法单方面把项目改成 MIT** —— 需要全体版权人书面同意。你的新代码写进 fork，整个衍生作品仍须 GPLv3，以后想商用/双授权都会被锁死。
- **接手的"资产"其实很薄**：上游 5 star、0 issue、0 PR、停更 2 年多。没有社区势能、没有用户反馈压力，你接到的主要是技术债 —— 一套你大概率自己都不用的 .NET Framework 4.8 / WPF / Windows-only 技术栈。
- **维护型工作的动力问题**：你觉得无趣，这是最可靠的信号。无外部用户 + 无内在兴趣，通常撑不过一两个月。
- 唯一的正收益：简历上「维护了一个真实开源项目」、以及帮到本校同学 —— 但这些用少量 PR 也能拿到，不必整个接盘。

顺带提醒一个坑：`.github/workflows/build.yml` 头部写着 `Licensed under the MIT license.`：

```1:3:.github/workflows/build.yml
# Copyright (c) SduNetCheckTool
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT license.
```

这是复制微软模板残留的**错误元数据，不构成对主项目的再授权**，不能据此认为可以当 MIT 用。

---

## 三、方向二：从零做、MIT、100% 自有 —— 更契合，但有边界

法律上（非法律意见，重大决策请咨询律师）：**著作权保护表达，不保护思想、功能、架构、算法**。所以借鉴「检测→修复契约」「可分享报告」「众包故障判断」这些 idea 完全安全；复制、逐行翻译、改写源码则构成衍生作品，必须 GPLv3。

从零做的洁净做法：

- 先用自己的话写 `SPEC.md`（能力清单/接口设计），**实现阶段不对照源码**；保留设计决策记录，作为独立创作的证据链。
- 绝不复制 `/workspace/SduNetCheckTool.Core/Utils/ProxySetting.cs` —— 它是从 v2rayN vendored 的 GPL-3.0 代码，355 行、占全仓 16%：

```1:2:SduNetCheckTool.Core/Utils/ProxySetting.cs
// internal class ProxySetting code from https://github.com/2dust/v2rayN/blob/master/v2rayN/v2rayN/Handler/ProxySetting.cs
// which licensed under GPL-3.0 license
```

- 不搬目录命名、UI 文案（那些带颜文字的中文提示）、注释、图标；避开 `Sdu` / 「山东大学网管会」等名称与标识（这是名称/商标问题，与著作权无关）。认证接口地址属于事实，但要自己查证并做成配置项而非硬编码。
- 两个仓库代码**不要互相复制**：别把新项目代码提给上游 PR，否则会引出"污染"争议。

**关于"归属权都在我"与 MIT 的一个澄清**：从零独立创作，著作权自动且完整归你，MIT 只是你对外授予的许可，不转移所有权。但 MIT 允许任何人闭源商用、再授权你的代码 —— 它保的是「署名 + 你能自由决定自己怎么授权」，不是「别人不能拿去用」。因为你 100% 持有版权，你随时可以**双授权**（社区 MIT + 商业许可）或对后续版本换协议；已发出的 MIT 版本不可撤销。若将来要接受外部贡献，记得加 **CLA/DCO**，否则又会退化成共同版权，这正是你在方向一里想避开的局面。

---

## 四、三个可落地的新 idea（按推荐度）

**A. 声明式网络体检引擎（工程性最强、最容易做干净）**
Rust 或 Go，MIT。用 YAML/TOML 描述 probe（DNS、HTTP/captive portal、ICMP/网关、系统代理、traceroute）与 remediation（dry-run、需确认、可回滚），输出 JSON / SARIF / 单文件 HTML 报告；CLI + 可嵌入库双形态。与原项目的差别正好是它的债：配置驱动而非硬编码 IP、跨平台、测试先行、能进 CI。换语言 + 换抽象层次，归属最干净，受众从"一所学校"扩到 SRE / 企业 IT / Homelab。

**B. 「是不是只有我断了？」—— 众包故障感知（最有产品感）**
轻量 agent 可选上报 + 自托管服务端，聚合多人探测，回答「我单机故障 vs 楼栋/全校/运营商故障」，输出故障时间线、影响范围、公告页。这直接把关于页那个 QQ 群产品化，是原项目**没做过的空白**，因此几乎没有衍生争议。可先从「单机探测 + 手动分享报告链接」起步，验证需求后再上服务端。

**C. 诊断证据的可分享报告工具链（最小验证，1–2 周出 v0.1）**
把「检测结果 → 脱敏、可分享的单文件 HTML/Markdown 报告」做成独立小库，能接任意脚本/诊断器。用来快速验证"是否真有人需要"，再决定往 A 还是 B 投入。

---

## 五、我的建议

**走方向二，方向一不接（保留非代码层面的善意即可）**。理由很直接：你的首要诉求是归属权与自由度，而 GPLv3 + 6 位版权人恰好是最硬的反向约束；停更两年、零 issue、5 star 的上游也不提供任何值得用自由去换的资源。

建议路径：

1. 新建独立仓库，先落 `LICENSE`(MIT, Copyright © 2026 你的名字)、`SPEC.md`、`NOTICE`、`CONTRIBUTING`(含 DCO/CLA)。
2. 先做 **C** 验证需求，同时把它长成 **A** 的 core（probe / result / remediation 三元组，但用你自己的接口设计与命名）。
3. 有真实用户后再考虑 **B** 的服务端，或预留 MIT + 商业双授权。
4. 上游想关照，就提一两个纯 bugfix PR 或在新项目 README 致谢（思想借鉴不强制署名，但体面），仅此而已。

如果你愿意，我可以接着帮你把 A/C 的 `SPEC.md` 和仓库骨架（MIT + DCO + 规则集 schema + CLI 原型）写出来。

## 2. 我（h1s97x）

那就A，给我方案吧

### AI

# 方案 A：声明式网络体检引擎

**决断**：Rust、MIT、库 + CLI 双形态、规则集用 TOML 为主、v0.1 只做只读探测不做修复。项目代号建议 `netprobe`（发布前查 crates.io 重名，备选 `linkscope`）。

---

## 1. 定位与边界

一句话：**把「网络体检」变成一份可版本化、可分享、可进 CI 的配置，而不是一个写死在学校里的 exe。**

| 做 | 不做（明确划掉，防范围蔓延） |
|---|---|
| 声明式规则集描述「探测什么 / 什么算失败 / 建议怎么修」 | 任何 GUI、托盘、常驻守护进程 |
| 并发探测 + 结构化 Finding + 单文件可分享报告 | 内置任何特定学校的 IP、域名、品牌、文案 |
| 库可被其他 Rust 程序嵌入；CLI 给人用、给 CI 用 | 动态插件 / wasm（v1 全内建） |
| 修复动作封闭白名单，默认 dry-run | 规则集里执行任意 shell 命令（= RCE） |
| 预留上报接口给未来的方向 B | 默认任何遥测 / 数据上传 |

---

## 2. 技术选型：Rust

对标原项目「单文件 exe」这个优点，但摆脱 .NET Framework 4.8 / Windows-only 的锁死。Rust 的 `trait` + `serde` 组合最适合「可嵌入库 + 声明式配置」双形态。

**关键依赖**（全部 MIT / Apache-2.0 双授权，用 `cargo-deny` 设白名单卡住 GPL 传递）：`tokio`、`clap`、`serde` + `toml`、`reqwest`、`hickory-resolver`、`thiserror`/`anyhow`、`minijinja`（HTML）。SARIF 自己按 2.1.0 schema 用 serde 生成，不依赖冷门 crate。

**三个必须提前知道的坑**：

1. **别用 `serde_yaml`**（已归档停维护）。规则集以 TOML 为一等公民，YAML 若要支持用 `serde_yml`/`serde_norway` 做可选前端。
2. **跨平台 ICMP 是最大工程风险**：raw socket 在 Linux 要 `CAP_NET_RAW`/`ping_group_range`，Windows 常需管理员。方案：probe 分级降级 `surge-ping` → 系统 `ping` 子进程 → TCP connect 到网关作为可达性近似，并在 Finding 里标注用了哪种方式，别静默降级。
3. **系统代理 / DNS 读取修改三平台各异**，单独放 `platform/` 层（`windows` / `macos` / `linux`），core 不碰 `cfg`。

> 如果你评估后更在意「两周内出东西 + 以后做 B 的服务端」，换 Go 能省 30% 工期（`net`/`x/net/icmp`/`yaml.v3` 齐备）。但 Rust 的类型系统对「库 + 稳定 schema」更划算，且更符合你想从零做点有意思东西的动机。按你上轮的取向，我按 Rust 展开。

---

## 3. 洁净室与归属权落地（最优先，先于写代码）

这是你这个选择的核心诉求，要在**第一天**就固化成仓库资产：

1. **先写 `SPEC.md`**，用你自己的话描述能力、接口、字段语义、验收标准；写完就不再打开 `/workspace`。保留这份文档的 git 历史作为独立创作证据链。
2. **抽象层刻意不同于上游**，这既是工程改进也是清晰的界限。上游是「同步调用 + `Tuple<结果, 字符串, 修复器>` + 检测项直接吐出修复器」；本方案是「async + 结构化 `Finding` + 独立 `Remediation` 注册表 + `plan/apply/rollback` 三段」。不复制它的接口形状、类型名、目录命名、中文提示文案和颜文字。
3. **全仓统一 SPDX 头**：每个源文件 `// SPDX-License-Identifier: MIT` + `// Copyright (c) 2026 <你>`（这一条正好补上原项目缺的元数据一致性）。
4. `LICENSE`（MIT）、`NOTICE`（依赖清单 + 致谢）、`CONTRIBUTING.md`、`cargo-deny.toml` 白名单。
5. **CLA ≠ DCO，别搞混**：DCO 只证明贡献者「有权贡献」，版权仍分散；要维持 100% 单一版权，需 **CTA / 独家许可 CLA**。务实做法：v0.x 阶段**不接受外部代码 PR**（只收 issue 和规则集建议），示例规则集自己写；真有贡献者时再上 CLA。
6. 因为你是唯一版权人，**预留双授权空间**：社区 MIT + 你自己的商业许可；将来加企业功能时不需要回头求任何人。
7. 规则集里的示例 endpoint 用 `example.com` / RFC 5737 文档地址，配置文件不含真实校园网接口。

---

## 4. 架构与目录

```
netprobe/
├── LICENSE  NOTICE  SPEC.md  CONTRIBUTING.md  cargo-deny.toml
├── Cargo.toml                # workspace
├── crates/
│   ├── netprobe-core/        # 模型 + trait + Runner + 规则 schema（零平台依赖，可独立嵌入）
│   ├── netprobe-builtins/    # probes + 封闭 remediation 动作 + platform/
│   ├── netprobe-report/      # json / sarif / html + 脱敏
│   └── netprobe-cli/         # clap + 退出码 + 审计日志
├── rules/                   # 通用示例规则集
└── .github/workflows/ci.yml  # 三平台 matrix + 测试 + 许可检查
```

`core` 与 `builtins` 分开的意义：第三方可以把 core 嵌进自己的程序、只实现自己的 probe，不必拉进 `reqwest`/平台代码。

---

## 5. 核心抽象

```rust
#[async_trait]
pub trait Probe: Send + Sync {
    fn kind(&self) -> &'static str;
    async fn run(&self, ctx: &Context) -> Result<Observation>;
}

pub struct Finding {
    pub probe_id: String,
    pub outcome: Outcome,        // Pass | Fail | Warn | Skipped | Error
    pub message: String,
    pub evidence: Evidence,      // 结构化、可脱敏；不是自由文本 blob
    pub remediation: Option<ActionId>,  // 只引用注册表里的动作 ID
    pub elapsed: Duration,
    pub method: ProbeMethod,     // 记录 ICMP 降级到了哪一级
}

#[async_trait]
pub trait Remediation: Send + Sync {
    async fn plan(&self, ctx: &Context) -> Result<Plan>;  // 描述将改变什么，不执行
    async fn apply(&self, ctx: &Context) -> Result<()>;
    async fn rollback(&self, ctx: &Context) -> Result<()>;
}

pub trait Sink: Send + Sync {   // 留给方向 B 的口子
    async fn emit(&self, report: &Report) -> Result<()>;
}
```

`Evidence` 结构化是关键：上游把结果拼成一个大字符串，导致报告无法机读、无法脱敏、无法进 CI。这里从第一天就分开「人看的消息」和「机器读的证据」。

---

## 6. 规则集 schema（TOML）

```toml
version = 1
name = "generic-network-health"
description = "通用网络连通性体检"

[defaults]
timeout = "3s"
retries = 1
concurrency = 4

[redact]
ip = "mask"          # none | mask | hash
user = "hash"
mac = "mask"

[[probes]]
id = "dns_resolve"
type = "dns"
server = "223.5.5.5"
query = "example.com"
record = "A"
expect = { status = "ok", min_answers = 1 }

[[probes]]
id = "portal_reachable"
type = "http"
url = "https://example.com/health"
method = "GET"
expect = { status = 200, body_not_contains = "not_online_error" }

[[probes]]
id = "gateway_ping"
type = "icmp"
target = "{{ vars.gateway }}"
depends_on = ["dns_resolve"]
expect = { packet_loss_below = 0.5 }

[[probes]]
id = "system_proxy"
type = "proxy"
expect = { enabled = false }

[[remediations]]
for = "system_proxy"
id = "disable_system_proxy"
action = "proxy.set"          # 必须是封闭 enum 里的 ID
params = { enabled = false }
risk = "low"
needs_confirm = false
rollback = { action = "proxy.set", params = { enabled = true } }
```

**字段分层**：`type` 决定 probe 实现，`expect` 是断言（把「什么算失败」从代码搬进配置），`depends_on` 构成 DAG，`{{ vars.* }}` 做受限变量插值（只查白名单变量表，不求值表达式、不执行命令）。

---

## 7. 执行模型与安全

- **Runner**：解析 → 校验（`netprobe validate` 静态检查所有动作 ID 存在、参数类型正确、DAG 无环）→ 按 DAG 拓扑分层并发（受 `concurrency` 限制）→ 每层超时/重试 → 聚合 `Report`。
- **失败语义**：单个 probe 超时/异常记为 `Error`，不中断整体；依赖它的节点记 `Skipped`。退出码：`0` 全通过、`1` 有 Fail、`2` 配置/运行错误 —— CI 友好。
- **修复的安全红线**：规则集是**数据**，可能来自网上，因此
  - remediation 只能是**编译期封闭的动作枚举**（`proxy.set` / `dns.set` / `route.flush` …），实现全在 `builtins`，规则文件无法引入新行为；
  - `--dry-run` 是默认，`--apply` 才执行；`needs_confirm = true` 强制交互确认；无 TTY 时拒绝执行；
  - 执行前打印 `plan`，写审计日志（时间、动作、参数、结果、回滚结果）；
  - 需要特权的动作显式标注 `requires_elevation`，由 CLI 统一处理提权，不让规则集自己拉起进程；
  - v0.1 **完全不实现 `apply`**，只出 `plan`（即「修复建议」），把风险最高的部分推到 v0.2。
- **降级链**：ICMP → 系统 ping → TCP connect，每级在 `Finding.method` 留痕。

---

## 8. 报告与脱敏（承接上游最有价值的洞察）

| 格式 | 用途 |
|---|---|
| JSON | 稳定、带 `schema_version`，给程序/CI/未来的 B 消费 |
| SARIF 2.1.0 | 进 GitHub Code Scanning、PR 里直接标结果，这是原项目完全没有的开发者场景 |
| HTML 单文件 | 内联 CSS/JS、零外部依赖，双击即看、可直接发给 IT 支持群 —— 对应上游「导出报告 / 复制结果」，但做成可分享、脱敏、机读的版本 |

脱敏在 **report 层统一执行**（按 `[redact]` 对 `Evidence` 里的 IP/MAC/用户名做 mask 或 hash），core 保留原始值供本地调试。默认 `hash` 兜底，避免报告一转发就泄露内网拓扑。

---

## 9. CLI 与库 API

```bash
netprobe validate --rules rules/basic.toml          # 静态校验，CI 里跑
netprobe run --rules rules/basic.toml \
             --format json,sarif,html --out ./reports
netprobe run --rules ... --plan                     # 只列修复计划（默认）
netprobe run --rules ... --apply --yes              # 显式执行（v0.2）
netprobe list-probes / netprobe explain proxy.set   # 自解释，降低配置门槛
```

库侧：`netprobe_core::Runner::new(RuleSet::from_toml(..)?).with_sink(..).run().await?`，返回 `Report`，调用方自己决定怎么展示 —— 这就是「可嵌入」的承诺。

---

## 10. 测试与 CI（原项目 0 覆盖，这里当一等公民）

- 单元：`expect` 断言求值、DAG 拓扑/成环检测、超时与重试、脱敏、退出码映射。
- 快照：`insta` 锁死 JSON / SARIF / HTML 输出，防止 schema 悄悄漂移。
- 集成：本地起 mock HTTP/DNS 服务跑完整规则集；probe 通过 trait mock，测试不依赖真实外网。
- CI：`ubuntu / macos / windows` 三平台 matrix + `cargo test` + `cargo clippy -D warnings` + `cargo fmt --check` + `cargo-deny check licenses` + `cargo audit` + 构建产物冒烟测试。
- 每个 release 附 `LICENSE` 与 `NOTICE`（沿用上游做对的那一点）。

---

## 11. 里程碑

| 阶段 | 交付 | 验收 |
|---|---|---|
| **M0**（1–2 天） | 仓库治理四件套 + `SPEC.md` + schema 定义 + CI 骨架 + `validate` 命令 | `cargo deny check` 通过，能校验一份 TOML 并报出所有非法动作 ID |
| **M1 / v0.1**（约 1 周） | 3 个 probe（dns / http / icmp 含降级）+ 并发 Runner + JSON/HTML 报告 + 脱敏 + 退出码 | 三平台 CI 绿，跑出一份可分享的单文件 HTML，**零修复执行** |
| **M2 / v0.2**（+1–2 周） | `depends_on` + 变量插值 + SARIF + `plan` + 2–3 个封闭动作（Windows/Linux 的 proxy/dns） | `--apply --yes` 能执行并回滚，审计日志完整 |
| **M3 / v0.3**（+2–3 周） | 提权处理、规则集签名/`--trust`、完整 probe 集（tcp / traceroute / interface / captive-portal）、文档与示例规则集 | 外部人员照文档 10 分钟内配出自己环境的体检 |
| **v1.0** | schema 冻结 + 语义化版本承诺 + crates.io 发布 + CHANGELOG | 可作为依赖被第三方嵌入 |

> 估算按「业余时间每周约 10 小时」给，M1 是最小可用点：先把它发出去看有没有人用，再决定往 M2/M3 还是转向方向 B。

---

## 12. 风险与预留

- **最大风险不是技术，是「通用化后没人需要」**：所以 M1 必须自带 2–3 份开箱即用的通用规则集（家庭网络 / 办公网 / CI 外网依赖），别让用户面对空配置。
- **跨平台特权**容易吞掉工期：靠 M1「只读」把这部分整体后移。
- **留给方向 B 的接口**：`Report` schema 稳定 + `Sink` trait + `Finding` 里带可选匿名化的环境标签（楼栋/运营商/ASN）。将来做「是不是只有我断了」时，加一个 `HttpSink` 和服务端即可，core 不用重构。
- **与原项目的关系**：不 fork、不复制、不提代码 PR；README 里一句「设计思路受某校园网体检工具启发」是体面，不是法律义务。

---

**下一步**：要不要我直接把 **M0** 落地？我会新建独立目录（不改动 `/workspace` 里的任何文件），产出 `LICENSE`/`NOTICE`/`SPEC.md`/`CONTRIBUTING.md`/`Cargo.toml` workspace 骨架、TOML 规则的 serde 类型定义，以及一个能跑的 `netprobe validate`。确认后我就开工；如果你更想先看 `SPEC.md` 的正文内容再定，也可以只出文档。

## 3. 我（h1s97x）

OK就按这个做，先给项目起个名字

> `[工具结果] [web_fetch] Error during web fetch for https://crates.io/api/v1/crates/linkscope: Request failed with status code 404 No…`
> `[工具结果] [web_fetch] Error during web fetch for https://crates.io/api/v1/crates/hopscope: Request failed with status code 404 Not…`
> `[工具结果] [web_fetch] Error during web fetch for https://crates.io/api/v1/crates/wirecheck: Request failed with status code 404 No…`
> `[工具结果] {"status":"success","success":true,"result":{"type":"web_fetch_tool_result","data":"{\"crate\":{\"id\":\"netprobe\",\"na…`
> `[工具结果] {"status":"success","success":true,"result":{"type":"web_fetch_tool_result","data":"{\"total_count\":72,\"incomplete_res…`
> `[工具结果] {"status":"success","success":true,"result":{"type":"web_fetch_tool_result","data":"**来源与元信息**\n\n- **来源**：GitHub REST A…`
> `[工具结果] {"status":"success","success":true,"result":{"type":"web_fetch_tool_result","data":"{\"total_count\":15,\"incomplete_res…`
> `[工具结果] {"status":"success","success":true,"result":{"type":"web_fetch_tool_result","data":"{\"crates\":[{\"id\":\"oxipoint\",\"…`
### AI

发现一个必须先纠正的问题：我上一轮推荐的 `netprobe` **不能用了**。查了 crates.io——`shellrow/netprobe` 已注册（MIT，"Cross-Platform Network Probe Library"，v0.4.0，约 1.1 万下载），名字和语义都撞。另外三个候选在 GitHub 上也都有同名项目。让我再验一批更独特的名字。

> `[工具结果] [web_fetch] Error during web fetch for https://crates.io/api/v1/crates/netassay: Request failed with status code 404 Not…`
> `[工具结果] [web_fetch] Error during web fetch for https://crates.io/api/v1/crates/vitalink: Request failed with status code 404 Not…`
> `[工具结果] [web_fetch] Error during web fetch for https://crates.io/api/v1/crates/linkverdict: Request failed with status code 404 …`
> `[工具结果] {"status":"success","success":true,"result":{"type":"web_fetch_tool_result","data":"{\"total_count\":4,\"incomplete_resu…`
> `[工具结果] {"status":"success","success":true,"result":{"type":"web_fetch_tool_result","data":"{\"total_count\":300,\"incomplete_re…`
> `[工具结果] {"status":"success","success":true,"result":{"type":"web_fetch_tool_result","data":"total_count：0\n\n| full_name | descr…`
### AI

## 先纠正上一轮的错误

`netprobe` **不能用**了。查 crates.io 发现已被 `shellrow/netprobe` 占用（MIT，"Cross-Platform Network Probe Library"，v0.4.0，约 1.1 万次下载，GitHub 同名仓库）。名字和语义双重撞车，正好是「归属权独立」最该避开的情形。我上一轮说"发布前查重"是偷懒了，现在补上。

## 查重结果

| 候选 | crates.io | GitHub 同名 | 判断 |
|---|---|---|---|
| `netprobe` | 已注册 | 有活跃项目 | 弃用 |
| `linkscope` | 未注册 | 72 个，含 499 star 的 `AccentuSoft/LinkScope_Client`(AGPL)、60 star 的 `Skythinker616/LinkScope`(GPL) | 品牌不干净，弃用 |
| `hopscope` | 未注册 | 7 个，其中 `dasosis/hopscope` 描述是「hop-by-hop 网络诊断 in Go」 | 同语义撞车，弃用 |
| `wirecheck` | 未注册 | 15 个，语义杂乱 | 平淡，备选 |
| `vitalink` | 未注册 | 300 个，全是健康医疗项目 | 强歧义，弃用 |
| `netassay` | **未注册** | 4 个，全是 2014–2020 年 0 star 的学术残留（SIGCOMM poster / Tofino P4），无同名活跃仓库 | **推荐** |
| `linkverdict` | **未注册** | **0 个** | 最干净，但偏长 |

## 定名：`netassay`

**net + assay**（assay = 化验、检定）。理由是它准确对应这个项目真正的内核：不是"探测一下"，而是**按规则集取样、给出结论、留下可分享的证据**——这正是方案里 `Finding` / `Outcome` / HTML 报告要做的事，也是原项目「一键体检」这个概念的英文正解。

- 8 个字符，CLI 好打，可发音，好记
- 不含 `Sdu`、`Check`、`Tool`、`Probe`，与上游零形似，也避开 `netprobe` 那一族
- crates.io 名字可注册；GitHub 无同名活跃项目
- 能承载方向 B：assay 本身就是"采样测定"，将来做众包故障判断不违和，不用改名

**配套命名**（一起定下来，避免后面各处不一致）：

```
仓库 / CLI          netassay
workspace crates    netassay-core / netassay-builtins / netassay-report / netassay-cli
规则集文件          *.assay.toml        （如 home.assay.toml、office.assay.toml）
报告产物            netassay-report.html / netassay-report.json / *.sarif
tagline             Declarative network health checks, with evidence you can share.
```

`LICENSE` 会写 `Copyright (c) 2026 <你的名字>`，`SPEC.md` 里用 `netassay` 作为项目代号，全仓 SPDX 统一 MIT。

## 两个备选，供你一句话切换

- **`linkverdict`**：如果你把「绝对零占用」看得比语义更重要 —— 它是唯一在 crates.io 和 GitHub 都 0 命中的。代价是 11 个字符、`verdict` 略带裁决味。
- **`wirecheck`**：如果你觉得 `assay` 太生僻。但我认为它辨识度最弱。

> 查重范围是 crates.io + GitHub 仓库名，**不含商标库、域名、npm/PyPI**。正式发布前建议再查一次 `netassay` 的域名与商标；crates.io 名称是先发布先得，v0.1 时顺手把 crate 名占住。方向 B 若将来独立成产品，另起产品名（比如 `isitjustme`），不占用 A 的命名空间。

---

**我的默认**：就用 `netassay`。你没异议的话，我下一步按 M0 落地 —— 在 `/workspace/netassay/` 新建独立仓库（不动 `/workspace` 里现有任何文件），产出 `LICENSE` / `NOTICE` / `SPEC.md` / `CONTRIBUTING.md` / `Cargo.toml` workspace 骨架、`.assay.toml` 的 serde 类型定义，以及能跑的 `netassay validate`。要换名字现在说一声就行。

## 4. 我（h1s97x）

wirecheck。就在本仓库改即可，把之前的历史全部清空。

> `[工具结果] {"status":"success","success":true,"result":{"type":"todo_write_result","merge":false,"todos":[{"id":"1","status":"in_pr…`
> `[工具结果] {"status":"success","success":true,"result":{"type":"execute_command_result","stdout":"=== rust toolchain ===\r\n\r\n(ev…`
### AI

环境里**没有 Rust 工具链**（`cargo`/`rustc` 都未安装），git 全局身份也未设置。我先执行清理，同时探测能否安装工具链用于验证。

> `[工具结果] {"status":"success","success":true,"result":{"type":"execute_command_result","stdout":"已初始化空的 Git 仓库于 /workspace/.git/\r…`
> `[工具结果] {"status":"success","success":true,"result":{"type":"execute_command_result","stdout":"=== rustup? ===\r\n\r\nnone\r\nno…`
### AI

清理完成：工作区已空，`.git` 重建为**零提交**的新仓库（旧 45 个提交、6 个 tag、旧 remote 全部消失）。现在写入 `wirecheck` 的 M0 骨架。

> `[工具结果] {"status":"success","success":true,"result":{"type":"todo_write_result","merge":true,"todos":[{"id":"1","status":"comple…`
> `[工具结果] Wrote contents to /workspace/LICENSE.…`
> `[工具结果] Wrote contents to /workspace/NOTICE.…`
> `[工具结果] Wrote contents to /workspace/SPEC.md.…`
> `[工具结果] Wrote contents to /workspace/CONTRIBUTING.md.…`
> `[工具结果] Wrote contents to /workspace/README.md.…`
> `[工具结果] Wrote contents to /workspace/deny.toml.…`
> `[工具结果] Wrote contents to /workspace/.gitignore.…`
> `[工具结果] Wrote contents to /workspace/.gitattributes.…`
> `[工具结果] Wrote contents to /workspace/.github/workflows/ci.yml.…`
> `[工具结果] Wrote contents to /workspace/Cargo.toml.…`
> `[工具结果] Wrote contents to /workspace/crates/wirecheck-core/Cargo.toml.…`
> `[工具结果] Wrote contents to /workspace/crates/wirecheck-core/src/lib.rs.…`
> `[工具结果] Wrote contents to /workspace/crates/wirecheck-core/src/model.rs.…`
> `[工具结果] Wrote contents to /workspace/crates/wirecheck-core/src/rules.rs.…`
> `[工具结果] Wrote contents to /workspace/crates/wirecheck-core/src/validate.rs.…`
> `[工具结果] Wrote contents to /workspace/crates/wirecheck-core/src/tests.rs.…`
> `[工具结果] Wrote contents to /workspace/crates/wirecheck-builtins/Cargo.toml.…`
> `[工具结果] Wrote contents to /workspace/crates/wirecheck-builtins/src/lib.rs.…`
> `[工具结果] Wrote contents to /workspace/crates/wirecheck-report/Cargo.toml.…`
> `[工具结果] Wrote contents to /workspace/crates/wirecheck-report/src/lib.rs.…`
> `[工具结果] Wrote contents to /workspace/crates/wirecheck-cli/Cargo.toml.…`
> `[工具结果] Wrote contents to /workspace/crates/wirecheck-cli/src/main.rs.…`
> `[工具结果] Wrote contents to /workspace/rules/home.rules.toml.…`
> `[工具结果] Wrote contents to /workspace/rules/ci.rules.toml.…`
> `[工具结果] Replace file succeeded, the file /workspace/deny.toml has been updated.  7 lines removed.1 lines added.  If your next ed…`
> `[工具结果] {"status":"success","success":true,"result":{"type":"execute_command_result","stdout":"info: downloading installer\r\n\r…`
### AI

Rust 1.100.0 装好了，但环境缺 C linker（`cc`）。先做类型检查，同时确认链接器情况。

> `[工具结果] {"status":"error","success":false,"result":{"type":"execute_command_result","stdout":"=== linker tools ===\r\n\r\ncc: mi…`
### AI

需要系统链接器才能编译（build script 也要链接）。安装 `gcc`。

> `[工具结果] {"status":"error","success":false,"result":{"type":"execute_command_result","stdout":"(eval):1: read-only variable: stat…`
> `[工具结果] {"status":"success","success":true,"result":{"type":"execute_command_result","stdout":"正在读取软件包列表...\r\n正在分析软件包的依赖关系树...\…`
### AI

`cc` 现在可用了。跑全套验证：格式化、类型检查、clippy、测试、`validate` 冒烟。

> `[工具结果] {"status":"error","success":false,"result":{"type":"execute_command_result","stdout":"=== last 100 lines ===\r\n\r\n  = …`
### AI

缺 C 运行库（`Scrt1.o`、`libc.so`）。补装 `libc6-dev` 后再跑。

> `[工具结果] {"status":"success","success":true,"result":{"type":"execute_command_result","stdout":"正在解压 libcrypt-dev:amd64 (1:4.4.38…`
> `[工具结果] {"status":"error","success":false,"result":{"type":"execute_command_result","stdout":"=== last 120 lines ===\r\n\r\n   C…`
### AI

类型检查通过，clippy 报两处 `derivable_impls`。读一下格式化后的文件再修。

> `[工具结果]      1:// SPDX-License-Identifier: MIT      2:// Copyright (c) 2026 h1s97x      3:      4://! Runtime data contract. M0 …`
> `[工具结果] Replace file succeeded, the file /workspace/crates/wirecheck-core/src/model.rs has been updated.  14 lines removed.4 lin…`
> `[工具结果] {"status":"success","success":true,"result":{"type":"execute_command_result","stdout":"=== last 120 lines ===\r\n\r\n   …`
### AI

全部通过：`cargo check`、`fmt`、clippy `-D warnings`、15 个单元测试、两份示例规则集 `validate`。再验证 CLI 的错误路径与退出码，并确认仓库状态。

> `[工具结果] {"status":"success","success":true,"result":{"type":"execute_command_result","stdout":"=== invalid rule set (expect exit…`
> `[工具结果] {"status":"success","success":true,"result":{"type":"read_lints_result","diagnostics":[""],"totalCount":0,"hint":"Review…`
> `[工具结果] Replace file succeeded, the file /workspace/README.md has been updated.  0 lines removed.3 lines added.  If your next ed…`
> `[工具结果] {"status":"success","success":true,"result":{"type":"execute_command_result","stdout":"total crates: 54\r\n  (MIT OR Apa…`
### AI

`cargo metadata` 确认 54 个依赖中**没有 GPL/AGPL/LGPL 等 copyleft**。有个旧式许可字符串 `MIT/Apache-2.0` 可能让 `cargo deny` 的 SPDX 解析报错——实测一下 CI 配置。

> `[工具结果] {"status":"success","success":true,"result":{"type":"execute_command_result","stdout":"nonstandard: memchr 2.8.3 -> Unli…`
### AI

实测结果：`licenses ok`，但 `bans` 因内部 path 依赖被 `wildcards = "deny"` 误判。修正配置。

> `[工具结果] Replace file succeeded, the file /workspace/deny.toml has been updated.  0 lines removed.2 lines added.  If your next ed…`
> `[工具结果] {"status":"success","success":true,"result":{"type":"execute_command_result","stdout":"22 │ wirecheck-builtins = { path …`
### AI

`cargo-deny` 指出根因：准备发布到 crates.io 的内部 path 依赖缺版本号。这是真问题，补上版本比放宽检查更正确。

> `[工具结果] Replace file succeeded, the file /workspace/Cargo.toml has been updated.  3 lines removed.5 lines added.  If your next e…`
> `[工具结果] {"status":"success","success":true,"result":{"type":"execute_command_result","stdout":"=== build/test tail ===\r\n\r\n  …`
> `[工具结果] {"status":"success","success":true,"result":{"type":"todo_write_result","merge":true,"todos":[{"id":"2","status":"comple…`
### AI

按 `wirecheck` 落地了 M0，**在 `/workspace` 原地改造**。

## 已完成

**历史清空**：删除全部旧代码（3 个 C# 项目、`.sln`、GPLv3 `LICENSE.txt`、旧 README/CI）并删掉 `.git` 重建。现在是 **0 提交**、无 tag、无 remote 的全新仓库，旧 45 个提交和 6 个 tag 已彻底消失。工作区 25 个文件全是新写的，已 `git add` 暂存。

**治理层**：`LICENSE`(MIT)、`NOTICE`、`SPEC.md`（含 2026-11-13 的独立设计决策记录）、`CONTRIBUTING.md`（v0.x 不收外部代码 PR、明确 CLA≠DCO）、`README.md`、`deny.toml`、三平台 `ci.yml`、全仓 SPDX 头。

**代码层**：
- `wirecheck-core` — TOML 规则集 schema、时长解析、一次性报全的静态校验器（E001–E019：重复 ID、未知 probe kind、未知 action ID、依赖缺失/成环、未定义变量、报告格式…）
- `wirecheck-builtins` — 编译期封闭的 probe/action 注册表，规则集无法引入新行为
- `wirecheck-report` — mask / SHA-256 hash 脱敏
- `wirecheck-cli` — `validate`（含 `--json`）、`list-probes`、`explain`
- `rules/home.rules.toml`、`rules/ci.rules.toml` — 只用 `example.com` 和 RFC 5737 文档地址

## 验证结果（全部实跑通过）

`cargo check` / `cargo fmt --check` / `clippy -D warnings` / **15 个单元测试** / 两份示例 `validate` 冒烟 → `exit 0`；故意写错的规则集一次报出 2 个 `E014` + 2 个 `W003`，退出码 2；`cargo deny check licenses bans sources` → `bans ok, licenses ok, sources ok`。54 个依赖中无 GPL/AGPL/LGPL。

为验证我在环境里装了 Rust 1.100、`gcc` 和 `libc6-dev`（工作区外），并实测修掉了两个真问题：内部依赖缺版本号导致 `cargo deny` 的 wildcard 报错、`clippy::derivable_impls`。

## 需要你做的三件事

1. **首次提交我没代做** —— 环境没配 git 身份，而这个提交的作者署名直接关系「归属权都在我」，应该由你本人写：
   ```bash
   git config user.name "你的名字"
   git config user.email "你的邮箱"
   git commit -m "chore: initialize wirecheck (MIT)"
   ```
2. **版权人暂填 `h1s97x`**（取自你的 CNB 用户名）。要换真名，`grep -rln h1s97x --exclude-dir=.git --exclude-dir=target .` 列出位置后统一替换（`LICENSE`/`NOTICE`/`SPEC.md`/`Cargo.toml` 的 authors / 所有 SPDX 头）。
3. **远端还是旧的** —— 我**没有 push，也没有 force push**。CNB 上的 `h1s97x/SduNetCheckTool` 仍是 GPLv3 旧仓库（45 提交、6 tag）。建议新建空的 `wirecheck` 仓库再正常推送，而不是覆盖 `master`：
   ```bash
   git remote add origin <新建的 wirecheck 空仓库地址>
   git push -u origin master
   ```
   若一定要在原仓库覆盖，需你自行 `git push --force --all` 并清理 tag —— 那会重写远端默认分支、影响所有协作者和 fork，我不会代做。

## 两个提醒

- **清空历史 ≠ 洁净室**。归属权来自 `SPEC.md` 的设计决策记录 + 零代码/文案/结构复制，不是删 git log。已公开的 GPLv3 版本及其 fork 仍然存在，删除本地历史不会追溯改变它们的许可；但 `wirecheck` 作为独立作品用 MIT 没有问题。
- **M0 边界**：probe 全是 `planned`，没有任何网络调用、没有 `--apply`、没有报告生成。`wirecheck validate` 已可用于 CI，M1 才是 dns/http/icmp + runner + JSON/HTML 报告。

下一步想让我直接做 M1，还是你先提交并建好远端仓库？另外 `cargo deny` 那份窄白名单（只放 MIT / Apache-2.0 / Unicode-3.0）是我按「新增依赖先审查再放行」设的，如果你希望更宽松也可以调。

## 5. 我（h1s97x）

在main分支提交，

### AI

先确认当前分支、提交身份和仓库里对 `master` 的引用，再切到 `main` 并提交。

> `[工具结果] {"status":"success","success":true,"result":{"type":"execute_command_result","stdout":"=== current branch ===\r\n\r\nmas…`
### AI

身份已配置好（`h1s97x`）。只需把 CI 的触发分支同步改成 `main`，然后切换分支并提交。

> `[工具结果] Replace file succeeded, the file /workspace/.github/workflows/ci.yml has been updated.  1 lines removed.1 lines added.  …`
> `[工具结果] {"status":"success","success":true,"result":{"type":"execute_command_result","stdout":"[main（根提交） 6073dc4] chore(repo): …`
### AI

提交完成。做一次提交后的轻量复核。

> `[工具结果] {"status":"success","success":true,"result":{"type":"execute_command_result","stdout":"=== any master left ===\r\n\r\nno…`
> `[工具结果] {"status":"success","success":true,"result":{"type":"execute_command_result","stdout":"test result: ok. 3 passed; 0 fail…`
### AI

已在 **`main`** 分支提交完成。

```
6073dc4 (HEAD -> main) chore(repo): initialize wirecheck (MIT)
```

- **根提交**：25 个文件，2481 行，作者为已配置的 `h1s97x`。历史里只有这一个提交，无旧提交、无 tag、`master` 引用为 0。
- 同步把 `.github/workflows/ci.yml` 的 push 触发分支从 `master` 改成了 `main`，否则推送后 CI 不会跑。
- 提交后复核：`cargo test --workspace` → **15 个测试全通过**（3 + 7 + 5 + doc-tests），工作区干净，`## main` 无未提交改动。
- **没有配置 remote、没有 push**。远端 `h1s97x/SduNetCheckTool` 仍是旧的 GPLv3 仓库；建议在 CNB 新建空的 `wirecheck` 仓库后 `git remote add origin <地址> && git push -u origin main`，避免 force-push 覆盖默认分支。

下一步做 M1（dns/http/icmp 三个 probe + 并发 Runner + JSON/HTML 报告），还是你先把远端仓库建好？

## 6. 我（h1s97x）

h1s97x，yang1297656998@outlook.com

> `[工具结果] {"status":"success","success":true,"result":{"type":"execute_command_result","stdout":"=== git identity (config) ===\r\n…`
> `[工具结果]      1:wirecheck      2:Copyright (c) 2026 h1s97x      3:SPDX-License-Identifier: MIT      4:      5:This project is lic…`
> `[工具结果]      1:# SPDX-License-Identifier: MIT      2:# Copyright (c) 2026 h1s97x      3:      4:[workspace]      5:resolver = "2…`
### AI

收到。核对结果：仓库里 23 个文件用的都是 `h1s97x`，但**没有任何邮箱记录**；git 配置和根提交作者是 `h1s97x <Yang1297656998@outlook.com>`（首字母大写）。我把你给的值作为正式维护者信息固化进去。

> `[工具结果] Replace file succeeded, the file /workspace/NOTICE has been updated.  0 lines removed.1 lines added.  If your next edit …`
> `[工具结果] Replace file succeeded, the file /workspace/Cargo.toml has been updated.  1 lines removed.1 lines added.  If your next e…`
> `[工具结果] {"status":"error","success":false,"result":{"type":"execute_command_result","stdout":"check_exit=0\r\n\r\n错误：gpg 无法为数据签名…`
