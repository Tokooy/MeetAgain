import type {
  Config,
  Encounter,
  EncounterPage,
  Graph,
  Simulation,
} from "./types";
export class ApiError extends Error {
  constructor(
    public status: number,
    message: string,
  ) {
    super(message);
  }
}
async function request<T>(path: string, options?: RequestInit): Promise<T> {
  const response = await fetch(path, options);
  if (!response.ok) {
    const messages: Record<number, string> = {
      404: "这次模拟已过期或服务已重启。请重新运行；当前轨迹仍可导出。",
      422: "参数不符合要求，请检查人数、观察天数和旅程编号。",
      429: "引擎正在处理另一项模拟，请稍后重试。",
    };
    throw new ApiError(
      response.status,
      messages[response.status] ??
        `请求失败（${response.status}），请稍后重试。`,
    );
  }
  return response.json() as Promise<T>;
}
export const getGraph = (signal?: AbortSignal) =>
  request<Graph>("/api/graph", { signal });
export const runSimulation = (config: Config) =>
  request<Simulation>("/api/simulations", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(config),
  });
export const getEncounters = (
  id: string,
  cursor?: string | null,
  signal?: AbortSignal,
) => {
  const query = new URLSearchParams({ limit: "1000" });
  if (cursor) query.set("cursor", cursor);
  return request<EncounterPage>(
    `/api/simulations/${encodeURIComponent(id)}/encounters?${query}`,
    { signal },
  );
};
export async function getAllEncounters(
  id: string,
  signal?: AbortSignal,
): Promise<Encounter[]> {
  const events: Encounter[] = [];
  let cursor: string | null = null;
  do {
    const page = await getEncounters(id, cursor, signal);
    events.push(...page.events);
    cursor = page.next_cursor;
  } while (cursor !== null);
  return events;
}
export function downloadJson(filename: string, data: unknown) {
  const url = URL.createObjectURL(
    new Blob([JSON.stringify(data, null, 2)], { type: "application/json" }),
  );
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
