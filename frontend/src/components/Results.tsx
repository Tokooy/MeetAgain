import { PanelSection } from "./PanelSection";
import { useEffect, useMemo, useState } from "react";
import { Area, AreaChart, ResponsiveContainer, Tooltip, XAxis } from "recharts";
import {
  ArrowDownToLine,
  ArrowUpRight,
  UsersRound,
  Route,
  Clock3,
  ChevronDown,
} from "lucide-react";
import type { Encounter, Graph, Position, Simulation } from "../types";
import { useView } from "../store";
import { downloadJson } from "../api";
interface Props {
  result?: Simulation;
  graph?: Graph;
  events: Encounter[];
  loading: boolean;
  error?: Error | null;
  onRetry: () => void;
}
const streetLabel = (location: Pick<Position, "street_id" | "street_name">) =>
  location.street_name ?? `道路 #${location.street_id}`;

export function Results({
  result,
  graph,
  events,
  loading,
  error,
  onRetry,
}: Props) {
  const { selectedAgent, selectAgent, setTick, setPlaying, focusMap } =
    useView();
  const [shown, setShown] = useState(30);
  useEffect(() => setShown(30), [result?.id]);
  const cityName = (id: string) =>
    graph?.nodes.find((c) => c.id === id)?.name.replace("市", "") ?? id;
  const agent = result?.agents.find((a) => a.id === selectedAgent);
  const chart = useMemo(() => {
    if (!result) return [];
    const counts = new Array<number>(result.config.ticks + 1).fill(0);
    for (const event of events) counts[event.tick]++;
    const bin = Math.max(1, Math.ceil(counts.length / 40));
    const data: { tick: number; count: number }[] = [];
    for (let i = 0; i < counts.length; i += bin)
      data.push({
        tick: i,
        count: counts.slice(i, i + bin).reduce((a, b) => a + b, 0),
      });
    return data;
  }, [result, events]);
  const filtered = useMemo(
    () =>
      selectedAgent === null
        ? events
        : events.filter((e) => e.agent_ids.includes(selectedAgent)),
    [events, selectedAgent],
  );
  function jump(event: Encounter) {
    setPlaying(false);
    setTick(event.tick);
    selectAgent(event.agent_ids[0]);
    const sample = result?.agents[event.agent_ids[0]]?.trajectory[event.tick];
    if (sample) focusMap(sample.coordinates);
  }
  return (
    <aside className="results panel" aria-label="模拟结果">
      <PanelSection title="观察结果" icon={<UsersRound size={15} />}>
        <div className="section-heading">
          <span className="section-index">03</span>
          <h2>相逢的可能</h2>
          {result && <span className="complete-badge">已完成</span>}
        </div>
        {!result ? (
          <div className="results-empty">
            <div className="empty-orbits" aria-hidden="true">
              <span />
              <span />
              <i />
              <b />
            </div>
            <h3>
              下一次相遇，
              <br />
              会在哪里？
            </h3>
            <p>
              开始一场模拟，让各自的生活
              <br />
              在这张地图上慢慢展开。
            </p>
            <div className="empty-stats">
              <span>
                —<small>相遇人数对</small>
              </span>
              <span>
                —<small>首次相遇</small>
              </span>
            </div>
          </div>
        ) : (
          <>
            <p className="result-conclusion" role="status">
              {result.summary.any_encounter
                ? "各自忙碌，依然有过交集。"
                : "这段旅程里，没有人在同一条街相距 50 米以内。"}
            </p>
            <div className="stat-grid">
              <div className="stat-card featured">
                <span>
                  相遇人数对 <UsersRound size={13} />
                </span>
                <strong data-testid="pair-count">
                  {result.summary.unique_encounter_pairs}
                  <small>对</small>
                </strong>
                <p>去重后的两人组合</p>
              </div>
              <div className="stat-card">
                <span>
                  首次相遇 <Clock3 size={13} />
                </span>
                <strong>
                  {result.summary.first_encounter_tick ?? "—"}
                  <small>
                    {result.summary.first_encounter_tick === null ? "" : "天"}
                  </small>
                </strong>
                <p>同一天、同街、50 米以内</p>
              </div>
            </div>
            <div className="reunion-note">
              <span>全员同时相聚</span>
              <b>
                {result.summary.all_agents_met
                  ? `第 ${result.summary.first_all_agents_tick} 天`
                  : "本次未发生"}
              </b>
            </div>
            <div className="result-subheading">
              <h3>相遇频率</h3>
              <span>
                {result.summary.encounter_event_count.toLocaleString()} 次事件
              </span>
            </div>
            {loading ? (
              <p className="muted small">正在读取完整事件…</p>
            ) : error ? (
              <div role="alert" className="error">
                <p>{error.message}</p>
                <button className="text-button" onClick={onRetry}>
                  重试读取
                </button>
              </div>
            ) : (
              <div
                className="chart"
                role="img"
                aria-label={`整个旅程共 ${events.length} 次相遇事件；图表按时间分组`}
              >
                <ResponsiveContainer width="100%" height="100%">
                  <AreaChart
                    data={chart}
                    margin={{ top: 8, right: 0, bottom: 0, left: 0 }}
                  >
                    <defs>
                      <linearGradient
                        id="chart-fill"
                        x1="0"
                        y1="0"
                        x2="0"
                        y2="1"
                      >
                        <stop
                          offset="0%"
                          stopColor="var(--accent)"
                          stopOpacity={0.25}
                        />
                        <stop
                          offset="100%"
                          stopColor="var(--accent)"
                          stopOpacity={0}
                        />
                      </linearGradient>
                    </defs>
                    <XAxis
                      dataKey="tick"
                      axisLine={false}
                      tickLine={false}
                      tick={{ fill: "var(--muted)", fontSize: 10 }}
                      minTickGap={35}
                    />
                    <Tooltip
                      content={({ active, payload }) =>
                        active && payload?.length ? (
                          <div className="chart-tooltip">
                            第 {String(payload[0].payload.tick)} 天起：
                            {String(payload[0].value)} 次
                          </div>
                        ) : null
                      }
                    />
                    <Area
                      dataKey="count"
                      stroke="var(--accent)"
                      strokeWidth={1.5}
                      fill="url(#chart-fill)"
                      isAnimationActive={false}
                    />
                  </AreaChart>
                </ResponsiveContainer>
              </div>
            )}
            <label className="agent-select">
              <span>
                <Route size={13} />
                跟随一位同行者
              </span>
              <select
                aria-label="跟随一位同行者"
                value={selectedAgent ?? ""}
                onChange={(e) =>
                  selectAgent(
                    e.target.value === "" ? null : Number(e.target.value),
                  )
                }
              >
                <option value="">全部个体</option>
                {result.agents.map((a) => (
                  <option key={a.id} value={a.id}>
                    个体 {String(a.id + 1).padStart(2, "0")} · 家在
                    {cityName(a.home_city_id)}
                  </option>
                ))}
              </select>
            </label>
            {agent && (
              <div className="agent-detail">
                <strong>
                  个体 {String(agent.id + 1).padStart(2, "0")} 的旅程
                </strong>
                <p>
                  家乡：{cityName(agent.home_city_id)}
                  {agent.graduation_city_id &&
                    ` · 毕业后到${cityName(agent.graduation_city_id)}`}
                  {" · 最终停留"}
                  {cityName(agent.final_city_id)}
                </p>
                <div>
                  <span>{agent.move_count} 次换城</span>
                  <span>{agent.visited_city_ids.length} 座城市</span>
                  <span>约 {Math.round(agent.distance_km)} km</span>
                </div>
                <details>
                  <summary>
                    查看完整位置序列
                    <ChevronDown size={13} />
                  </summary>
                  <ol className="trajectory-list">
                    {agent.trajectory.map((p) => (
                      <li key={p.tick}>
                        <button
                          onClick={() => {
                            setTick(p.tick);
                            setPlaying(false);
                            focusMap(p.coordinates);
                          }}
                        >
                          第 {p.tick} 天
                          <span>
                            {cityName(p.city_id)} · {streetLabel(p)}
                          </span>
                        </button>
                      </li>
                    ))}
                  </ol>
                </details>
              </div>
            )}
            <div className="result-subheading">
              <h3>相遇时刻</h3>
              <span>{selectedAgent === null ? "全部个体" : "仅所选个体"}</span>
            </div>
            {!loading && !error && (
              <div className="event-list" aria-label="相遇事件列表">
                {filtered.length === 0 ? (
                  <p className="muted small">没有符合条件的相遇事件。</p>
                ) : (
                  <>
                    {filtered.slice(0, shown).map((e, i) => (
                      <button
                        className="event"
                        key={`${e.tick}-${e.city_id}-${e.agent_ids.join("-")}`}
                        onClick={() => jump(e)}
                      >
                        <span className="event-tick">
                          {String(e.tick).padStart(3, "0")}
                          <small>DAY</small>
                        </span>
                        <span className="event-body">
                          <b>{streetLabel(e)}</b>
                          <span>
                            {cityName(e.city_id)} · 相距{" "}
                            {e.distance_m.toLocaleString("zh-CN", {
                              maximumFractionDigits: 1,
                            })}{" "}
                            米
                          </span>
                          <span>
                            个体 {String(e.agent_ids[0] + 1).padStart(2, "0")}{" "}
                            <i>与</i>{" "}
                            {String(e.agent_ids[1] + 1).padStart(2, "0")}
                          </span>
                        </span>
                        <ArrowUpRight size={14} aria-hidden="true" />
                        <span className="sr-only">
                          ，跳至第 {e.tick} 天相遇事件 {i + 1}
                        </span>
                      </button>
                    ))}
                    {shown < filtered.length && (
                      <button
                        className="load-more"
                        onClick={() => setShown(shown + 50)}
                      >
                        加载更多 · 还有 {filtered.length - shown} 次
                        <ChevronDown size={13} />
                      </button>
                    )}
                  </>
                )}
              </div>
            )}
            <div className="export-row">
              <button
                className="text-button"
                onClick={() =>
                  downloadJson(
                    `meetagain-${result.config.start_date ?? result.config.seed}-trajectories.json`,
                    result,
                  )
                }
              >
                <ArrowDownToLine size={13} />
                导出轨迹
              </button>
              <button
                className="text-button"
                disabled={loading || !!error}
                onClick={() =>
                  downloadJson(
                    `meetagain-${result.config.start_date ?? result.config.seed}-complete.json`,
                    { ...result, encounters: events },
                  )
                }
              >
                <ArrowDownToLine size={13} />
                完整结果
              </button>
            </div>
            <p className="result-footnote">
              完整模拟结果，不随回放进度改变。观察期内未相遇，不代表永远不会相逢。个体显示编号从
              01 开始；导出 ID 从 0 开始。
            </p>
          </>
        )}
      </PanelSection>
    </aside>
  );
}
