import { useEffect } from "react";
import { Pause, Play, SkipBack } from "lucide-react";
import { useView } from "../store";
interface Props {
  ticks: number;
  enabled: boolean;
}
export function Playback({ ticks, enabled }: Props) {
  const { tick, playing, setTick, setPlaying } = useView();
  useEffect(() => {
    if (!playing || !enabled) return;
    const timer = window.setInterval(() => {
      const view = useView.getState();
      if (view.tick >= ticks) view.setPlaying(false);
      else view.setTick(view.tick + 1);
    }, 300);
    return () => clearInterval(timer);
  }, [playing, enabled, ticks]);
  return (
    <div className="playback" aria-label="轨迹播放">
      <button
        className="icon-button"
        disabled={!enabled}
        aria-label="回到初始时刻"
        onClick={() => {
          setTick(0);
          setPlaying(false);
        }}
      >
        <SkipBack size={16} />
      </button>
      <button
        className="play-button"
        disabled={!enabled}
        aria-label={playing ? "暂停播放" : "播放轨迹"}
        onClick={() => {
          if (tick >= ticks) setTick(0);
          setPlaying(!playing);
        }}
      >
        {playing ? <Pause size={16} /> : <Play size={16} />}
      </button>
      <div className="scrubber">
        <label htmlFor="tick">
          旅程回放{" "}
          <span>
            <b>{String(tick).padStart(3, "0")}</b> / {enabled ? ticks : "—"}
          </span>
        </label>
        <input
          id="tick"
          aria-label="当前步数"
          type="range"
          min="0"
          max={Math.max(1, ticks)}
          value={tick}
          disabled={!enabled}
          onChange={(e) => {
            setPlaying(false);
            setTick(Number(e.target.value));
          }}
        />
      </div>
      <span className="tick-label">TICK</span>
    </div>
  );
}
