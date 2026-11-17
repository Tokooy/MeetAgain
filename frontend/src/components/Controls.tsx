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
  seed: "20261116",
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
  const seedValid =
    /^[0-9]{1,20}$/.test(values.seed) &&
    BigInt(values.seed) <= 18446744073709551615n;
  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (budget > MAX_PAIR_CHECKS || !seedValid || busy || !ready) return;
    onStart({
      agents: Number(values.agents),
      ticks: Number(values.ticks),
      seed: values.seed,
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
            <span>旅程编号</span>
            <input
              aria-label="旅程编号"
              aria-describedby="journey-code-hint"
              className="seed-input"
              type="text"
              inputMode="numeric"
              pattern="[0-9]{1,20}"
              maxLength={20}
              required
              value={values.seed}
              onChange={(e) => setValues({ ...values, seed: e.target.value })}
            />
          </label>
          <p className="field-hint" id="journey-code-hint">
            编号和设置相同，模拟结果就相同。想试试不同的旅程，可以换个编号。
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
                  { key: "home", label: "在家出行概率" },
                  { key: "away", label: "在外出行概率" },
                  { key: "back", label: "出行时返乡倾向" },
                ] as const
              ).map(({ key, label }) => (
                <label className="slider-field" key={key}>
                  <span>
                    {label}
                    <b>{Math.round(values[key] * 100)}%</b>
                  </span>
                  <input
                    aria-label={label}
                    type="range"
                    min="0"
                    max="1"
                    step="0.01"
                    value={values[key]}
                    onChange={(e) =>
                      setValues({ ...values, [key]: Number(e.target.value) })
                    }
                  />
                </label>
              ))}
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
          {!seedValid && (
            <p className="error" role="alert">
              旅程编号须为 0 至 18446744073709551615 的整数。
            </p>
          )}
          <button
            className="primary-button"
            type="submit"
            disabled={busy || !ready || budget > MAX_PAIR_CHECKS || !seedValid}
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
