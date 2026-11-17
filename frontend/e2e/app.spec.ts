import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

test("map, simulation, playback, filtering, download and reset work through real HTTP", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto("/");
  await expect(page.getByTestId("city-map")).toHaveAttribute(
    "data-ready",
    "true",
  );
  await expect(page.locator(".city-name")).toHaveText([
    "南京市",
    "扬州市",
    "镇江市",
  ]);
  await expect(page.locator(".map-error")).toHaveCount(0);
  await expect(page.getByTestId("city-map")).toHaveAttribute(
    "data-rendered",
    "true",
  );
  await expect(page.getByTestId("city-map")).toHaveAttribute(
    "data-streets-rendered",
    "true",
  );
  await expect(
    page.getByRole("link", { name: "© OpenStreetMap contributors" }),
  ).toBeVisible();
  const cityFocus = page.getByLabel("缩放到城市");
  await cityFocus.selectOption("321100");
  await expect(cityFocus).toHaveValue("321100");
  await page.getByRole("button", { name: "显示全部城市" }).click();
  await expect(cityFocus).toHaveValue("");
  expect(
    (await page.getByTestId("city-map").boundingBox())!.height,
  ).toBeGreaterThan(350);
  await expect(page.locator(".city-name").first()).toBeInViewport();
  expect((await page.locator(".workspace").boundingBox())!.y).toBeLessThan(140);
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await page.getByRole("button", { name: /行为参数/ }).click();
  await expect(page.getByLabel("在家乡时，今天出发的概率")).toHaveValue("0.1");
  await expect(page.getByLabel("在外地时，今天换城市的概率")).toHaveValue(
    "0.25",
  );
  await expect(page.getByLabel("在外地决定出发后，往家走的概率")).toHaveValue(
    "0.6",
  );
  await expect(page.getByText(/初一到初七保持最高/)).toBeVisible();
  await expect(page.getByLabel("分开日期（旅程编号）")).toHaveValue(
    "2026-11-16",
  );
  await page.getByRole("button", { name: /行为参数/ }).click();
  const run = page.waitForResponse("**/api/simulations");
  await page.getByRole("button", { name: "开始模拟" }).click();
  const { summary } = await (await run).json();
  await expect(page.getByTestId("pair-count")).toHaveText(
    `${summary.unique_encounter_pairs}对`,
  );
  await expect(
    page.getByText(
      `${summary.encounter_event_count.toLocaleString("zh-CN")} 次事件`,
    ),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "完整结果", exact: true }),
  ).toBeEnabled();
  await page.getByRole("button", { name: "播放轨迹", exact: true }).click();
  await expect
    .poll(() => page.getByLabel("当前天数", { exact: true }).inputValue())
    .not.toBe("0");
  await page.getByRole("button", { name: "暂停播放", exact: true }).click();
  await page.getByLabel("跟随一位同行者", { exact: true }).selectOption("0");
  await expect(page.getByText("个体 01 的旅程")).toBeVisible();
  await page.getByLabel("当前天数", { exact: true }).fill("100");
  await expect(page.getByLabel("当前天数", { exact: true })).toHaveValue("100");
  const downloadEvent = page.waitForEvent("download");
  await page.getByRole("button", { name: "完整结果", exact: true }).click();
  const download = await downloadEvent;
  const stream = await download.createReadStream();
  const chunks: Buffer[] = [];
  for await (const chunk of stream!) chunks.push(Buffer.from(chunk));
  const result = JSON.parse(Buffer.concat(chunks).toString());
  expect(result.encounters.length).toBe(summary.encounter_event_count);
  expect(result.agents.length).toBe(20);
  expect(result.agents[0].trajectory.length).toBe(366);
  await page.locator(".results").evaluate((e) => {
    e.scrollTop = 0;
  });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollHeight <= innerHeight,
    ),
  ).toBe(true);
  await page.screenshot({
    path: "test-results/desktop-result.png",
    fullPage: true,
  });
  await page.screenshot({
    path: "test-results/desktop-light.png",
    fullPage: true,
  });
  expect(
    (
      await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
        .analyze()
    ).violations,
  ).toEqual([]);
  await page.getByRole("button", { name: "重置实验" }).click();
  await expect(page.getByTestId("pair-count")).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "播放轨迹", exact: true }),
  ).toBeDisabled();
  await page.getByRole("button", { name: "切换深色主题" }).click();
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await expect(page.getByTestId("city-map")).toHaveAttribute(
    "data-streets-rendered",
    "true",
  );
  await page.screenshot({
    path: "test-results/desktop-dark.png",
    fullPage: true,
  });
  await page.getByRole("button", { name: "切换浅色主题" }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  expect(errors).toEqual([]);
});

test("mobile layout collapses, respects reduced motion and has no horizontal overflow", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/");
  await expect(page.getByTestId("city-map")).toHaveAttribute(
    "data-streets-rendered",
    "true",
  );
  await expect(
    page.locator(".controls details.mobile-section"),
  ).not.toHaveAttribute("open");
  await page.getByText("实验设置", { exact: true }).click();
  await page.getByLabel("同行人数", { exact: true }).fill("4");
  await page.getByLabel("观察时长", { exact: true }).fill("5300");

  await page.getByLabel("所有人从同一座家乡城市出发").check();
  await page.getByLabel("分开日期（旅程编号）").fill("2027-01-01");
  const simulation = page.waitForResponse("**/api/simulations");
  await page.getByRole("button", { name: "开始模拟" }).click();
  const response = await simulation;
  expect(response.status()).toBe(201);
  const result = await response.json();
  expect(result.config).not.toHaveProperty("seed");
  expect(result.config).toMatchObject({
    ticks: 5300,
    same_hometown: true,
    start_date: "2027-01-01",
  });
  expect(
    new Set(
      result.agents.map(
        (agent: { home_city_id: string }) => agent.home_city_id,
      ),
    ).size,
  ).toBe(1);
  await expect(page.getByLabel("当前天数", { exact: true })).toHaveAttribute(
    "max",
    "5300",
  );
  await page.getByText("观察结果", { exact: true }).click();
  await expect(page.getByTestId("pair-count")).toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({ path: "test-results/mobile.png", fullPage: true });
  expect(
    (
      await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
        .analyze()
    ).violations,
  ).toEqual([]);
});

test("invalid workload is explained and unavailable API is recoverable", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByLabel("同行人数", { exact: true }).fill("100");
  await page.getByLabel("观察时长", { exact: true }).fill("1000");
  await expect(page.getByText(/组合超过/)).toBeVisible();
  await expect(page.getByRole("button", { name: "开始模拟" })).toBeDisabled();
  await page.getByRole("button", { name: "重置实验" }).click();
  await page.route("**/api/simulations", (route) =>
    route.fulfill({
      status: 429,
      contentType: "application/json",
      body: JSON.stringify({
        error: { code: "busy", message: "busy", field: null },
      }),
    }),
  );
  await page.getByRole("button", { name: "开始模拟" }).click();
  await expect(page.locator(".global-error")).toContainText(
    "引擎正在处理另一项模拟",
  );
});
