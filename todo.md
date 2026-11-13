# TODO — wirecheck 未完成项

更新时间：2026-11-14 · 基线 `5530c1f`（`main` 与远端一致，工作区干净）
标记：`[ ]` 未做 · `[~]` 等外部结果 · `[!]` 需先决策（不可逆或改范围）

> `SPEC.md` §15 已把「真实修复动作 / 提权 / 发布者签名」正式列为**已接受的范围外**，
> 第 4 组不是待修 bug；要做必须先在这里留一条带日期的新决策。

## 1. 验证（打 v1.0 tag 前阻塞）

- [~] **macOS CI 出结论**。GitHub Actions 最新可见的是 run #3（`f0065ce`），整体 `queued`：
  ubuntu ✅、windows ✅、`dependency licenses` ✅，只有 `test (macos-latest)` 排队。
  `5530c1f` 的 run 在查询时尚未出现在 API，需复查；CNB 同步流水线
  `cnb-ngs-1k7b6q0bi` 已 `success`。看 https://github.com/h1s97x/wirecheck/actions
- [ ] **MSRV 实跑**：`rust-version` 已从 1.75 修正为 1.85（依赖 `clap`/`indexmap`/`hashbrown`
  声明 1.85），但本机只有 1.100，没在 1.85 上跑过。执行 `cargo +1.85.0 test --workspace`
- [ ] `cargo doc --no-deps` 未跑
- [ ] **其余 3 个 crate 的打包/发布预演**：目前只跑过
  `cargo package --list -p wirecheck-core` 和 `cargo publish --dry-run -p wirecheck-core`；
  `wirecheck-builtins`、`wirecheck-report`、`wirecheck-cli` 还没跑（含内部 path 依赖，
  要按发布顺序验证）
- [ ] **精确确认 crate 名未被占用**：只做过 `q=wirecheck` 模糊搜索（无同名结果），
  尚未逐个查 `wirecheck-core` / `-builtins` / `-report` / `-cli` 的 crates.io 端点
- [ ] 发布前按 `CONTRIBUTING.md` 同步核对 `LICENSE`、`NOTICE`、`CHANGELOG.md`
- [ ] **§8 验收「usable as a library dependency」**：现在只有 workspace 内的
  `crates/wirecheck-core/examples/embed.rs`；还需一个 workspace 外的独立项目依赖
  已发布的 `wirecheck-core` 验证一次
- [ ] `README.md` 状态仍写「preparing v1.0」，发版后同步更新

## 2. 发版当天的维护者动作

- [ ] crates.io 账号 / token
- [ ] 按依赖顺序 `cargo publish`：`core → builtins → report → cli`
- [ ] 打新版本 tag `v0.1.0` / `v0.2.0` / `v0.3.0` / `v1.0.0`。
  目前仓库里只有 `1.0.0`–`1.0.5`，全是**旧 C# 项目**的 tag；推 tag 会触发
  `.cnb.yml` 的 `push_tags: true` 同步到 GitHub
- [ ] 写 Release Notes / 建 GitHub Release
- [!] **处理旧 tags `1.0.0`–`1.0.5`**：删除或重命名，避免和 wirecheck 版本号混淆。
  删除远端 tag 不可逆，先决策再动手

## 3. 前端（按你决定暂缓）

- [ ] 先定形态：独立 Web App（读 `report.json`）还是桌面 GUI；两者都要先改
  `SPEC.md` §1 与 §14 的范围定义
- [ ] 若做，另立前端工程、技术栈与部署方式；现在的 HTML 只是单文件报告工件

## 4. 已接受的范围外（§15，需新决策）

- [ ] 平台动作适配器：让 `--apply --yes` 真正执行并回滚。现在默认
  `DisabledExecutor` 把每个动作记为 `blocked`，真实 apply/rollback 只在
  `RecordingExecutor` 单测里验证
- [ ] 提权：目前是集中拒绝，不做任何特权提升
- [ ] 发布者签名 / 密钥管理：`--trust` 只是本地 SHA-256 指纹白名单，不验证发布者身份
- [ ] async `Probe`（§9 的原始决策）：现在是同步 trait + scoped 线程并发

## 5. 已知功能缺口（§10–§13）

- [ ] HTTPS / TLS：http 探针遇到 `https` 直接报错
- [ ] ICMP native raw socket：降级链从系统 `ping`、再退到 TCP 近似
- [ ] per-probe 类型化 `expect`：现在是固定键集合，未知键报 `warn`
- [ ] 显式敏感度标签：脱敏仍靠 `classify()` 按字段名猜测
- [ ] 探针深度：`proxy` 只读环境变量（不读系统代理配置）；`interface` 只返回一个
  本地 IP（不列全部接口/网关）；DNS 只手写 A 记录，`with.record` 实际被忽略；
  `traceroute` 依赖系统命令；`captive_portal` 是重定向/登录页启发式
- [ ] M3 验收「新用户 10 分钟配好」还没做真人验收

## 6. 杂项 / 可选

- [ ] 清理剩余历史里程碑注释（`model.rs`、`observation.rs`、`runner.rs`、
  `plan.rs`、`actions.rs` 的模块头仍写着 M0/M1/M2）
- [ ] 远端 `restore/m0-snapshot-v2` 按你的决定**有意保留**（含
  `SESSION-TRANSCRIPT.md`）；若要删除需明确授权
- 不用处理：`reports/`、`target/`、`.wirecheck/` 都是本地状态，已 gitignore
