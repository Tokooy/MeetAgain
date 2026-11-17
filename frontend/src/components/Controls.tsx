import { PanelSection } from "./PanelSection";
import { useState, type FormEvent } from "react";
import { ArrowUpRight, RotateCcw, SlidersHorizontal, Info } from "lucide-react";
import type { City, Config, ParticipantCities } from "../types";
import { ParticipantSetup } from "./ParticipantSetup";
interface Props {
  busy: boolean;
  cities: City[];
  onStart: (config: Config) => void;
  onReset: () => void;
}
const MAX_DAYS = 10_958;
const MAX_PAIR_CHECKS = 500_000;
const defaults = {
  agents: "2",
  ticks: "365",
  sameHometown: false,
  startDate: "2026-11-16",
  home: 0.1,
  away: 0.25,
  back: 0.6,
};
export function Controls({ busy, cities, onStart, onReset }: Props) {
  const [values, setValues] = useState(defaults);
  const [initial, setInitial] = useState(false);
  const [advanced, setAdvanced] = useState(false);
  const [choices, setChoices] = useState<ParticipantCities[]>([]);
  const [sharedCity, setSharedCity] = useState("");
  const agents = Number(values.agents);
  const validAgentCount =
    Number.isInteger(agents) && agents >= 2 && agents <= 100;
  const ready = cities.length > 0;
  const pairsPerDay = (agents * (agents - 1)) / 2;
  const budget = pairsPerDay * (Number(values.ticks) + Number(initial));
  const maxDaysForAgents =
    Math.floor(MAX_PAIR_CHECKS / pairsPerDay) - Number(initial);
  const start = Date.parse(values.startDate);
  const calendarValid =
    start >= Date.parse("1901-01-01") &&
    start + Number(values.ticks) * 86_400_000 <= Date.parse("2099-12-31");
  function chooseCity(
    index: number,
    field: keyof ParticipantCities,
    city: string,
  ) {
    setChoices((previous) => {
      const next = [...previous];
      next[index] = { ...next[index], [field]: city || undefined };
      return next;
    });
  }

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (
      budget > MAX_PAIR_CHECKS ||
      !calendarValid ||
      !validAgentCount ||
      busy ||
      !ready
    )
      return;
    const participants = Array.from({ length: agents }, (_, index) => ({
      ...choices[index],
      ...(values.sameHometown ? { home_city_id: sharedCity || undefined } : {}),
    }));
    onStart({
      agents,
      ...(participants.some(
        ({ home_city_id, graduation_city_id }) =>
          home_city_id || graduation_city_id,
      )
        ? { participants }
        : {}),
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
          <ParticipantSetup
            cities={cities}
            count={validAgentCount ? agents : 2}
            choices={choices}
            shared={values.sameHometown}
            sharedCity={sharedCity}
            onSharedChange={(sameHometown) =>
              setValues({ ...values, sameHometown })
            }
            onSharedCityChange={setSharedCity}
            onChoiceChange={chooseCity}
          />
          <label className="field">
            <span>
              观察时长 <small>最长约 30 年</small>
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
              <p className="field-hint">
                以下换城概率用于休息日。工作日的活动范围为定居点周围 15
                公里以内。
              </p>
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
                    hint: "25% 表示今天有 25% 的机会去另一座城市，其余时候留在同一座城市。",
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
                计入第 0 天的相遇
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
              busy ||
              !ready ||
              !validAgentCount ||
              budget > MAX_PAIR_CHECKS ||
              !calendarValid
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
              setChoices([]);
              setSharedCity("");
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
              每天记录一次位置。同一天在同一条街、相距不超过 50
              米，才算一次相遇。
            </p>
          </div>
        </div>
      </PanelSection>
    </aside>
  );
}
