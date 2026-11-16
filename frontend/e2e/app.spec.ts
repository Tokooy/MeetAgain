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
  await expect(page.locator(".city-name")).toHaveCount(13);
  await expect(page.locator(".map-error")).toHaveCount(0);
  await expect(page.getByTestId("city-map")).toHaveAttribute(
    "data-rendered",
    "true",
  );
  expect(
    (await page.getByTestId("city-map").boundingBox())!.height,
  ).toBeGreaterThan(350);
  await expect(page.locator(".city-name").first()).toBeInViewport();
  await page.screenshot({
    path: "test-results/desktop-dark.png",
    fullPage: true,
  });
  await page.getByRole("button", { name: "开始模拟" }).click();
  await expect(page.getByTestId("pair-count")).toHaveText("152对");
  await expect(page.getByText("5,207 次事件")).toBeVisible();
  await expect(
    page.getByRole("button", { name: "完整结果", exact: true }),
  ).toBeEnabled();
  await page.getByRole("button", { name: "播放轨迹", exact: true }).click();
  await expect
    .poll(() => page.getByLabel("当前步数", { exact: true }).inputValue())
    .not.toBe("0");
  await page.getByRole("button", { name: "暂停播放", exact: true }).click();
  await page.getByLabel("跟随一位同行者", { exact: true }).selectOption("0");
  await expect(page.getByText("个体 01 的旅程")).toBeVisible();
  await page.getByLabel("当前步数", { exact: true }).fill("100");
  await expect(page.getByLabel("当前步数", { exact: true })).toHaveValue("100");
  const downloadEvent = page.waitForEvent("download");
  await page.getByRole("button", { name: "完整结果", exact: true }).click();
  const download = await downloadEvent;
  const stream = await download.createReadStream();
  const chunks: Buffer[] = [];
  for await (const chunk of stream!) chunks.push(Buffer.from(chunk));
  const result = JSON.parse(Buffer.concat(chunks).toString());
  expect(result.encounters.length).toBe(5207);
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
  await page.getByRole("button", { name: "切换浅色主题" }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
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
  expect(errors).toEqual([]);
});

test("mobile layout collapses, respects reduced motion and has no horizontal overflow", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/");
  await expect(page.getByTestId("city-map")).toHaveAttribute(
    "data-ready",
    "true",
  );
  await expect(
    page.locator(".controls details.mobile-section"),
  ).not.toHaveAttribute("open");
  await page.getByText("实验设置", { exact: true }).click();
  await page.getByLabel("同行人数", { exact: true }).fill("4");
  await page.getByLabel("观察时长", { exact: true }).fill("20");
  await page.getByRole("button", { name: "开始模拟" }).click();
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
