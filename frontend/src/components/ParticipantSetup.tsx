import type { City, ParticipantCities } from "../types";

interface Props {
  cities: City[];
  count: number;
  choices: ParticipantCities[];
  shared: boolean;
  sharedCity: string;
  onSharedChange: (value: boolean) => void;
  onSharedCityChange: (value: string) => void;
  onChoiceChange: (
    index: number,
    field: keyof ParticipantCities,
    city: string,
  ) => void;
}

export function ParticipantSetup({
  cities,
  count,
  choices,
  shared,
  sharedCity,
  onSharedChange,
  onSharedCityChange,
  onChoiceChange,
}: Props) {
  const cityOptions = cities.map((city) => (
    <option key={city.id} value={city.id}>
      {city.name}
    </option>
  ));

  function participant(index: number) {
    const label = `个体 ${String(index + 1).padStart(2, "0")}`;
    const choice = choices[index];
    return (
      <fieldset key={index} className="participant-card">
        <legend>{label}</legend>
        <div className="participant-fields">
          {!shared && (
            <label>
              <span>家乡</span>
              <select
                aria-label={`${label} 的家乡`}
                value={choice?.home_city_id ?? ""}
                onChange={(event) =>
                  onChoiceChange(index, "home_city_id", event.target.value)
                }
              >
                <option value="">随机选择</option>
                {cityOptions}
              </select>
            </label>
          )}
          <label>
            <span>毕业后去向</span>
            <select
              aria-label={`${label} 的毕业后去向`}
              value={choice?.graduation_city_id ?? ""}
              onChange={(event) =>
                onChoiceChange(index, "graduation_city_id", event.target.value)
              }
            >
              <option value="">按出行规则</option>
              {cityOptions}
            </select>
          </label>
        </div>
      </fieldset>
    );
  }

  return (
    <section className="participant-setup" aria-labelledby="participant-title">
      <h3 id="participant-title">家乡与毕业去向</h3>
      <p className="field-hint">
        第 0 天在家乡。选定毕业去向后，第 1 天到达该城市。
      </p>
      <label className="checkbox-field">
        <input
          type="checkbox"
          checked={shared}
          onChange={(event) => onSharedChange(event.target.checked)}
        />
        所有人从同一座家乡城市出发
      </label>
      {shared && (
        <label className="shared-city-select">
          <span>共同家乡</span>
          <select
            aria-label="共同家乡"
            value={sharedCity}
            onChange={(event) => onSharedCityChange(event.target.value)}
          >
            <option value="">随机选择</option>
            {cityOptions}
          </select>
        </label>
      )}
      {participant(0)}
      {participant(1)}
      {count > 2 && (
        <details className="more-participants">
          <summary>设置其余 {count - 2} 人</summary>
          {Array.from({ length: count - 2 }, (_, index) =>
            participant(index + 2),
          )}
        </details>
      )}
    </section>
  );
}
