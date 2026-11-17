import { PanelSection } from "./PanelSection";
import { useState, type FormEvent } from "react";
import { ArrowUpRight, RotateCcw, SlidersHorizontal, Info } from "lucide-react";
import type { Config } from "../types";
interface Props {
  busy: boolean;
  ready: boolean;
  onStart: (config: Config) => void;
  onReset: () => void;
}
const MAX_DAYS = 5300;
const MAX_PAIR_CHECKS = 500_000;
const defaults = {
  agents: "20",
  ticks: "365",
  sameHometown: false,
  startDate: "2026-11-16",
  home: 0.1,
  away: 0.25,
  back: 0.6,
};
export function Controls({ busy, ready, onStart, onReset }: Props) {
  const [values, setValues] = useState(defaults);
  const [initial, setInitial] = useState(false);
  const [advanced, setAdvanced] = useState(false);
  const agents = Number(values.agents);
  const pairsPerDay = (agents * (agents - 1)) / 2;
  const budget = pairsPerDay * (Number(values.ticks) + Number(initial));
  const maxDaysForAgents =
    Math.floor(MAX_PAIR_CHECKS / pairsPerDay) - Number(initial);
  const start = Date.parse(values.startDate);
  const calendarValid =
    start >= Date.parse("1901-01-01") &&
    start + Number(values.ticks) * 86_400_000 <= Date.parse("2099-12-31");
  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (budget > MAX_PAIR_CHECKS || !calendarValid || busy || !ready) return;
    onStart({
      agents: Number(values.agents),
      ticks: Number(values.ticks),
      same_hometown: values.sameHometown,
      start_date: values.startDate,
      movement: {
        home_move_probability: values.home,
        away_move_probability: values.away,
        home_return_probability: values.back,
      },
      encounters: { include_tick_zero: initial, cross_tick_tolerance: 0 },
    });
  }
  return (
    <aside className="controls panel" aria-label="模拟控制">
      <PanelSection title="实验设置" icon={<SlidersHorizontal size={15} />}>
        <form onSubmit={submit}>
          <div className="section-heading">
            <span className="section-index">01</span>
            <h2>设定这段旅程</h2>
          </div>
          <p className="muted description">
            不约定目的地，也不追随彼此。
            <br />
            只是偶尔出发，时常想家。
          </p>
          <label className="field">
            <span>
              同行人数 <small>INDIVIDUALS</small>
            </span>
            <div className="number-wrap">
              <input
                aria-label="同行人数"
                type="number"
                min="2"
                max="100"
                required
                value={values.agents}
                onChange={(e) =>
                  setValues({ ...values, agents: e.target.value })
                }
              />
              <span>人</span>
            </div>
          </label>
          <label className="checkbox-field shared-hometown">
            <input
              type="checkbox"
              checked={values.sameHometown}
              onChange={(e) =>
                setValues({ ...values, sameHometown: e.target.checked })
              }
              aria-describedby="shared-hometown-hint"
            />
            所有人从同一座家乡城市出发
          </label>
          <p className="field-hint shared-hometown" id="shared-hometown-hint">
            勾选后，系统随机选一座城市作为大家的家乡，每个人仍独立决定去哪儿。
          </p>
          <label className="field">
            <span>
              观察时长 <small>DAYS</small>
            </span>
            <div className="number-wrap">
              <input
                aria-label="观察时长"
                type="number"
                min="1"
                max={MAX_DAYS}
                required
                value={values.ticks}
                onChange={(e) =>
                  setValues({ ...values, ticks: e.target.value })
                }
              />
              <span>天</span>
            </div>
          </label>
          <label className="field">
            <span>分开日期（旅程编号）</span>
            <input
              aria-label="分开日期（旅程编号）"
              aria-describedby="departure-date-hint"
              className="departure-date"
              type="date"
              min="1901-01-01"
              max="2099-12-31"
              required
              value={values.startDate}
              onChange={(e) =>
                setValues({ ...values, startDate: e.target.value })
              }
            />
          </label>
          <p className="field-hint" id="departure-date-hint">
            这是模拟的第 0 天。选同一天、用相同的设置，就能重跑同一段旅程。
          </p>
          <button
            className="advanced-toggle"
            type="button"
            aria-expanded={advanced}
            onClick={() => setAdvanced(!advanced)}
          >
            <SlidersHorizontal size={14} /> 行为参数{" "}
            <span>{advanced ? "−" : "+"}</span>
          </button>
          {advanced && (
            <div className="advanced-fields">
              {(
                [
                  {
                    key: "home",
                    label: "在家乡时，今天出发的概率",
                    hint: "10% 表示今天有 10% 的机会离开家乡，去一座相邻城市。",
                  },
                  {
                    key: "away",
                    label: "在外地时，今天换城市的概率",
                    hint: "25% 表示今天有 25% 的机会去另一座城市，其余时候留在原地。",
                  },
                  {
                    key: "back",
                    label: "在外地决定出发后，往家走的概率",
                    hint: "平日为 60% 时，每 100 次外地出发约有 60 次沿最短路线往家乡走一站。其余会随机选邻近城市，也可能碰巧走向家乡。",
                  },
                ] as const
              ).map(({ key, label, hint }) => (
                <label className="slider-field" key={key}>
                  <span>
                    {label}
                    <b>{Math.round(values[key] * 100)}%</b>
                  </span>
                  <input
                    aria-label={label}
                    aria-describedby={`behavior-${key}-hint`}
                    type="range"
                    min="0"
                    max="1"
                    step="0.01"
                    value={values[key]}
                    onChange={(e) =>
                      setValues({ ...values, [key]: Number(e.target.value) })
                    }
                  />
                  <small className="field-hint" id={`behavior-${key}-hint`}>
                    {hint}
                  </small>
                </label>
              ))}
              <p className="field-hint">
                春节前 21
                天，出发后往家走的概率逐步提高，初一到初七保持最高。初七之后，7
                天内降回平日水平。
              </p>
              <label className="checkbox-field">
                <input
                  type="checkbox"
                  checked={initial}
                  onChange={(e) => setInitial(e.target.checked)}
                />
                计入初始时刻的同城相遇
              </label>
            </div>
          )}
          {budget > MAX_PAIR_CHECKS && (
            <p className="error" role="alert">
              当前人数和观察天数的组合超过模拟上限。请减少人数，或将观察时长缩短至{" "}
              {maxDaysForAgents.toLocaleString("zh-CN")} 天以内。
            </p>
          )}
          {!calendarValid && (
            <p className="error" role="alert">
              请调整分开日期或观察天数，确保模拟日期都在 1901 至 2099 年内。
            </p>
          )}
          <button
            className="primary-button"
            type="submit"
            disabled={
              busy || !ready || budget > MAX_PAIR_CHECKS || !calendarValid
            }
          >
            {busy ? "正在推演…" : "开始模拟"}
            <ArrowUpRight size={18} />
          </button>
          <button
            className="reset-button"
            type="button"
            disabled={busy}
            onClick={() => {
              setValues(defaults);
              setInitial(false);
              onReset();
            }}
          >
            <RotateCcw size={13} />
            重置实验
          </button>
        </form>
        <div className="model-note">
          <Info size={15} />
          <div>
            <strong>怎样算一次相遇？</strong>
            <p>
              同一天在同一座城市，就记录一次相遇。只比较每天的位置，途中交叉不计。
            </p>
          </div>
        </div>
      </PanelSection>
    </aside>
  );
}
