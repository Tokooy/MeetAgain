import { lazy, Suspense, useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowUpRight, Check, Compass, Moon, Sun, X, Info } from "lucide-react";
import { getAllEncounters, getGraph, runSimulation } from "./api";
import { useView } from "./store";
import { Controls } from "./components/Controls";
import { Playback } from "./components/Playback";
import { Results } from "./components/Results";
import type { Config } from "./types";
const CityMap = lazy(() => import("./components/CityMap"));

export default function App() {
  const { theme, toggleTheme, resetView } = useView();
  const [showAbout, setShowAbout] = useState(false);
  const dialog = useRef<HTMLDialogElement>(null);
  const queryClient = useQueryClient();
  const graph = useQuery({
    queryKey: ["graph"],
    queryFn: ({ signal }) => getGraph(signal),
    staleTime: Infinity,
    retry: 1,
  });
  const simulation = useMutation({
    mutationFn: runSimulation,
    onSuccess: () => resetView(),
  });
  const result = simulation.data;
  const events = useQuery({
    queryKey: ["encounters", result?.id],
    queryFn: ({ signal }) => getAllEncounters(result!.id, signal),
    enabled: !!result,
    staleTime: Infinity,
    retry: false,
  });
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);
  useEffect(() => {
    if (showAbout) dialog.current?.showModal();
    else dialog.current?.close();
  }, [showAbout]);
  function reset() {
    simulation.reset();
    queryClient.removeQueries({ queryKey: ["encounters"] });
    resetView();
  }
  function start(config: Config) {
    resetView();
    simulation.mutate(config);
  }
  return (
    <div className="app-shell">
      <header className="app-header">
        <div className="brand">
          <span className="brand-symbol" aria-hidden="true">
            <svg viewBox="0 0 32 32">
              <path d="M5 24V8l11 13L27 8v16" />
              <circle cx="16" cy="21" r="3" />
            </svg>
          </span>
          <span>
            MeetAgain<small>再 · 相逢</small>
          </span>
        </div>
        <div className="header-center">
          <span>SIMULATION LAB</span>
          <i />
          城市漫游实验
        </div>
        <div className="header-actions">
          <button className="about-button" onClick={() => setShowAbout(true)}>
            <Info size={14} />
            <span>关于模型</span>
          </button>
          <span className="header-divider" />
          <button
            className="icon-button"
            aria-label={theme === "dark" ? "切换浅色主题" : "切换深色主题"}
            onClick={toggleTheme}
          >
            {theme === "dark" ? <Sun size={17} /> : <Moon size={17} />}
          </button>
        </div>
      </header>
      <div className="intro-bar">
        <div>
          <h1>
            各自生活，还会再相逢吗<span>？</span>
          </h1>
          <p className="intro-subtitle">
            让独立的轨迹，回答一个关于重逢的问题。
          </p>
        </div>
        <div className="model-metrics">
          <div>
            <b>{graph.data?.nodes.length ?? "—"}</b>
            <span>座城市</span>
          </div>
          <i />
          <div>
            <b>
              {graph.data?.nodes
                .reduce((total, city) => total + city.street_segments, 0)
                .toLocaleString("zh-CN") ?? "—"}
            </b>
            <span>条道路分段</span>
          </div>
          <span className="model-tag">
            <Compass size={13} />
            独立个体 · 离散时间
          </span>
        </div>
      </div>
      {simulation.error && (
        <div className="global-error" role="alert">
          {simulation.error.message}
          <button
            aria-label="关闭错误提示"
            className="icon-button"
            onClick={() => simulation.reset()}
          >
            <X size={14} />
          </button>
        </div>
      )}
      <main className="workspace">
        <Controls
          busy={simulation.isPending}
          cities={graph.data?.nodes ?? []}
          onStart={start}
          onReset={reset}
        />
        <section className="map-panel" aria-label="城市地图与回放">
          <div className="map-stage">
            {graph.data ? (
              <Suspense
                fallback={<div className="map-loading">正在展开道路地图…</div>}
              >
                <CityMap graph={graph.data} result={result} />
              </Suspense>
            ) : (
              <div className="map-loading">
                {graph.error ? (
                  <>
                    <p>地图数据读取失败，请检查服务是否正常。</p>
                    <button
                      className="text-button"
                      onClick={() => void graph.refetch()}
                    >
                      重试
                    </button>
                  </>
                ) : (
                  "正在读取城市网络…"
                )}
              </div>
            )}
            {simulation.isPending && (
              <div className="simulation-progress" role="status">
                正在推演各自的旅程…
              </div>
            )}
          </div>
          <Playback ticks={result?.config.ticks ?? 0} enabled={!!result} />
        </section>
        <Results
          result={result}
          graph={graph.data}
          events={events.data ?? []}
          loading={!!result && events.isPending}
          error={events.error}
          onRetry={() => void events.refetch()}
        />
      </main>
      <footer className="app-footer">
        <span>
          <span className={`status-dot ${graph.data ? "online" : ""}`} />
          {graph.data ? "城市网络已就绪" : "正在连接引擎"}
        </span>
        <span>不联络 · 不追随 · 偶然相遇</span>
        <span>MEETAGAIN / MODEL 01</span>
      </footer>
      <dialog
        ref={dialog}
        className="about-dialog"
        onCancel={() => setShowAbout(false)}
        onClick={(e) => {
          if (e.target === dialog.current) setShowAbout(false);
        }}
      >
        <button
          className="icon-button dialog-close"
          aria-label="关闭模型说明"
          onClick={() => setShowAbout(false)}
        >
          <X size={18} />
        </button>
        <p className="eyebrow">ABOUT THE MODEL</p>
        <h2>一次关于偶然的实验</h2>
        <p>
          每个人第 0 天在自己的家乡。如果选了毕业后的去向，第 1
          天到达该城市；此后每天按出行概率决定是否换城市。决定从外地出发后，可以沿最短路线往家乡走一站，也可以去其他相邻城市。
        </p>
        <ul>
          <li>
            <Check size={14} />
            同一天在同一条街且直线距离不超过 50 米，才记录一次相遇。
          </li>
          <li>
            <Check size={14} />
            模拟用城市中心之间的近似距离决定出行方向。
          </li>
          <li>
            <Check size={14} />
            临近农历新年时更容易往家乡走，初七之后逐渐降回平日水平。
          </li>
          <li>
            <Check size={14} />
            每天在所在城市的道路上采样一次位置。只有采样时的位置用于判断相遇。
          </li>
        </ul>
        <p>
          城市边界和街道来自
          OpenStreetMap。地图上的虚线表示模拟中的城市连接，个体点位表示当天采样的道路位置。
        </p>
        <p>
          有限模拟中的结果不等于现实预测。“至少一对相遇”和“全员同时相聚”会分别报告。事件缓存保留
          15 分钟、最多 4 次模拟，建议及时导出。
        </p>
        <a
          href={
            graph.data?.provenance.license_url ??
            "https://www.openstreetmap.org/copyright"
          }
          target="_blank"
          rel="noreferrer"
        >
          查看地图数据来源
          <ArrowUpRight size={14} />
        </a>
      </dialog>
    </div>
  );
}
