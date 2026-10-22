const { chromium } = require(
  process.env.HOMEBOARD_PLAYWRIGHT_MODULE || "playwright",
);
const assert = require("node:assert/strict");
const fs = require("fs");
const os = require("node:os"),
  path = require("node:path"),
  net = require("node:net");
const { spawn } = require("node:child_process");
let testServer, testBrowser, tempDir;
async function cleanup() {
  if (testBrowser) await testBrowser.close();
  if (testServer && testServer.exitCode === null) {
    const done = new Promise((resolve) => testServer.once("exit", resolve));
    testServer.kill();
    await done;
  }
  if (tempDir) fs.rmSync(tempDir, { recursive: true, force: true });
}
(async () => {
  const executable = path.resolve(
    process.env.HOMEBOARD_TEST_EXE ||
      path.join(
        "target",
        "debug",
        "homeboard" + (process.platform === "win32" ? ".exe" : ""),
      ),
  );
  if (!fs.existsSync(executable))
    throw Error("Build Homeboard first, or set HOMEBOARD_TEST_EXE.");
  tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "homeboard-browser-"));
  const port = await new Promise((resolve) => {
    const server = net.createServer();
    server.listen(0, "127.0.0.1", () => {
      const port = server.address().port;
      server.close(() => resolve(port));
    });
  });
  fs.writeFileSync(
    path.join(tempDir, "server.json"),
    JSON.stringify({ bind: "127.0.0.1", port }),
  );
  testServer = spawn(executable, ["--data-dir", tempDir], { stdio: "ignore", cwd: tempDir });
  const base = "http://127.0.0.1:" + port;
  let ready = false;
  for (let i = 0; i < 40; i++) {
    try {
      if ((await fetch(base + "/api/tasks")).ok) {
        ready = true;
        break;
      }
    } catch {}
    await new Promise((r) => setTimeout(r, 200));
  }
  if (!ready) throw Error("Isolated test backend did not start.");
  testBrowser = await chromium.launch({
    executablePath: process.env.HOMEBOARD_CHROMIUM || undefined,
    headless: true,
    args: ["--no-sandbox"],
  });
  const browser = testBrowser;
  const ctx = await browser.newContext({
    viewport: { width: 1512, height: 1100 },
    locale: "de-CH",
    timezoneId: "Europe/Zurich",
    reducedMotion: "reduce",
  });
  const page = await ctx.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.stack));
  const screenshotDir =
    process.env.HOMEBOARD_SCREENSHOTS || "work/browser-screenshots";
  await page.goto(base + "/edit");
  await page.locator("#login-panel:not(.hidden)").waitFor();
  await page.locator("#password").fill("Task-Browser-123456!");
  if (await page.locator("#confirm-field").isVisible())
    await page.locator("#confirm-password").fill("Task-Browser-123456!");
  await page.locator("#login-submit").click();
  await page.locator("#editor:not(.hidden)").waitFor();
  await page.evaluate(async () => {
    const d = await (await fetch("/api/tasks")).json(),
      s = await (await fetch("/api/tasks/session")).json();
    d.tasks = [];
    d.views = [];
    await fetch("/api/tasks", {
      method: "PUT",
      headers: {
        "Content-Type": "application/json",
        "X-CSRF-Token": s.csrf,
        "X-Homeboard-Operation": "restore",
      },
      body: JSON.stringify(d),
    });
  });
  await page.goto(base + "/tasks");
  await page
    .getByText("Dein Kopf darf leichter werden.", { exact: true })
    .waitFor();
  await page.locator("#add-task").click();
  await page.locator("#detail-title").fill("Release vorbereiten");
  await page.locator("#detail-project").selectOption("work");
  await page.locator("#detail-priority").selectOption("3");
  await page.locator("#detail-due").fill("2026-10-22");
  await page
    .locator("#detail-description")
    .fill("Ein klarer Plan für den nächsten Schritt.");
  await page.locator("#detail-tags").fill("Homeboard, Planung");
  await page
    .getByRole("button", { name: "＋ Unteraufgabe", exact: true })
    .click();
  await page
    .getByLabel("Unteraufgabe", { exact: true })
    .fill("Testplan prüfen");
  await page.locator("#task-form button[type=submit]").click();
  await page.locator("#task-dialog").waitFor({ state: "hidden" });
  await page
    .getByRole("button", { name: "Release vorbereiten", exact: true })
    .waitFor();
  // Populate additional realistic test records via the same authenticated production API.
  await page.evaluate(async () => {
    const [data, s] = await Promise.all([
      fetch("/api/tasks").then((r) => r.json()),
      fetch("/api/tasks/session").then((r) => r.json()),
    ]);
    const { newTask } = await import("/assets/tasks-core.mjs");
    const items = [
      ["Wochenplanung abschliessen", "personal", "doing", 2, "2026-10-23"],
      ["Ideen für das Wochenende", "personal", "todo", 1, "2026-10-24"],
      ["Rückmeldung zum Entwurf", "work", "waiting", 2, "2026-10-26"],
      ["Dokumentation aktualisieren", "work", "done", 1, "2026-10-20"],
      ["Abonnement prüfen", "personal", "todo", 3, "2026-10-20"],
      ["Bewegung einplanen", "personal", "todo", 1, "2026-10-22"],
    ];
    for (const [title, project, status, priority, due] of items) {
      const t = {
        ...newTask(title),
        project,
        status,
        priority,
        due,
        start: "2026-10-19",
        tags: [project === "work" ? "Arbeit" : "Alltag"],
      };
      if (status === "done") t.completed_at = new Date().toISOString();
      data.tasks.push(t);
    }
    const r = await fetch("/api/tasks", {
      method: "PUT",
      headers: { "Content-Type": "application/json", "X-CSRF-Token": s.csrf },
      body: JSON.stringify(data),
    });
    if (!r.ok) throw Error(await r.text());
  });
  await page.reload();
  await page.locator(".task-table tbody tr").nth(5).waitFor();
  fs.mkdirSync(screenshotDir, { recursive: true });
  await page.screenshot({
    path: path.join(screenshotDir, "tasks-table.png"),
    fullPage: true,
  });
  // Search / group / columns / direct editing.
  await page.locator("#task-search").fill("Release");
  await page.waitForTimeout(250);
  assert.equal(await page.locator(".task-table tbody tr").count(), 1);
  await page
    .getByLabel("Priorität: Release vorbereiten", { exact: true })
    .selectOption("2");
  await page.waitForTimeout(200);
  await page.locator("#task-search").fill("");
  await page.waitForTimeout(250);
  await page.getByLabel("Gruppierung", { exact: true }).selectOption("project");
  assert.ok((await page.locator(".group-heading").count()) >= 2);
  await page.getByLabel("Gruppierung", { exact: true }).selectOption("none");
  await page.locator("#column-tools").click();
  await page.getByLabel("Zeit (Min)", { exact: true }).check();
  await page.locator("#utility-dialog .close-dialog").click();
  assert.ok(
    await page
      .locator(".task-table th")
      .filter({ hasText: "Zeit (Min)" })
      .count(),
  );
  // Save a custom view and reopen it.
  await page.locator("#save-view").click();
  await page
    .getByLabel("Name der Ansicht", { exact: true })
    .fill("Mein Überblick");
  await page
    .locator("#utility-dialog")
    .getByRole("button", { name: "Ansicht speichern", exact: true })
    .click();
  await page.locator("#utility-dialog").waitFor({ state: "hidden" });
  await page
    .locator("#saved-nav")
    .getByRole("button", { name: "◇ Mein Überblick" })
    .click();
  const views = [
    "list",
    "kanban",
    "calendar",
    "week",
    "agenda",
    "timeline",
    "matrix",
    "projects",
    "stats",
    "archive",
    "table",
  ];
  for (const id of views) {
    const label = {
      list: "Liste",
      kanban: "Kanban",
      calendar: "Kalender",
      week: "Wochenplan",
      agenda: "Agenda",
      timeline: "Zeitleiste",
      matrix: "Prioritätenmatrix",
      projects: "Projekte",
      stats: "Statistiken",
      archive: "Archiv",
      table: "Tabelle",
    }[id];
    await page
      .locator("#view-nav")
      .getByRole("button", { name: new RegExp(label + "$") })
      .click();
    await page.waitForTimeout(80);
    assert.equal(await page.locator("#crumb").innerText(), label);
    if (["kanban", "calendar", "stats"].includes(id))
      await page.screenshot({
        path: path.join(screenshotDir, `tasks-${id}.png`),
        fullPage: true,
      });
  }
  // Edit subtasks / note / manual time and duplicate.
  await page
    .getByRole("button", { name: "Release vorbereiten", exact: true })
    .click();
  await page
    .getByLabel("Unteraufgabe erledigt: Testplan prüfen", { exact: true })
    .check();
  await page
    .getByLabel("Neue Notiz", { exact: true })
    .fill("Browserprüfung abgeschlossen.");
  await page
    .getByRole("button", { name: "＋ Notiz hinzufügen", exact: true })
    .click();
  await page.locator("#detail-spent").fill("25");
  await page
    .getByRole("button", { name: "▶ Timer starten", exact: true })
    .click();
  await page.waitForTimeout(1100);
  await page
    .getByRole("button", { name: "■ Timer stoppen", exact: true })
    .click();
  assert.equal(await page.locator("#detail-spent").inputValue(), "26");
  await page.screenshot({
    path: path.join(screenshotDir, "tasks-detail.png"),
    fullPage: true,
  });
  await page.locator("#task-form button[type=submit]").click();
  await page.locator("#task-dialog").waitFor({ state: "hidden" });
  await page
    .getByRole("button", { name: "Release vorbereiten", exact: true })
    .click();
  await page.getByRole("button", { name: "Duplizieren", exact: true }).click();
  await page.locator("#task-dialog").waitFor({ state: "hidden" });
  await page
    .getByRole("button", { name: "Release vorbereiten (Kopie)", exact: true })
    .waitFor();
  await page.locator("#undo-task").click();
  await page.waitForTimeout(200);
  assert.equal(
    await page
      .getByRole("button", { name: "Release vorbereiten (Kopie)", exact: true })
      .count(),
    0,
  );
  // Kanban drag and status persistence.
  await page
    .locator("#view-nav")
    .getByRole("button", { name: /Kanban$/ })
    .click();
  const card = page
    .locator(".task-card")
    .filter({
      has: page.getByRole("button", {
        name: "Release vorbereiten",
        exact: true,
      }),
    });
  await card.dragTo(page.locator(".kanban-column[data-status=doing]"), {
    targetPosition: { x: 30, y: 30 },
  });
  await page.waitForTimeout(250);
  assert.equal(
    await page
      .locator(".kanban-column[data-status=doing]")
      .getByRole("button", { name: "Release vorbereiten", exact: true })
      .count(),
    1,
  );
  // Export real JSON and CSV, then merge import without duplicating IDs.
  await page.locator("#data-tools").click();
  let event = page.waitForEvent("download");
  await page
    .getByRole("button", { name: "↓ Alle Aufgaben als JSON", exact: true })
    .click();
  let dl = await event;
  await dl.saveAs("work/tasks-export.json");
  event = page.waitForEvent("download");
  await page
    .getByRole("button", { name: "↓ Aktuelle Ansicht als CSV", exact: true })
    .click();
  dl = await event;
  await dl.saveAs("work/tasks-export.csv");
  await page.locator("#utility-dialog .close-dialog").click();
  await page.locator("#task-import").setInputFiles("work/tasks-export.json");
  await page
    .getByRole("button", { name: "Zusammenführen", exact: true })
    .click();
  await page.locator("#utility-dialog").waitFor({ state: "hidden" });
  await page.waitForTimeout(200);
  assert.equal(await page.locator(".task-card").count(), 7);
  // Archive and restore via details.
  await page
    .getByRole("button", { name: "Release vorbereiten", exact: true })
    .click();
  await page.getByRole("button", { name: "Archivieren", exact: true }).click();
  await page.locator("#task-dialog").waitFor({ state: "hidden" });
  await page
    .locator("#view-nav")
    .getByRole("button", { name: /Archiv$/ })
    .click();
  await page
    .getByRole("button", { name: "Release vorbereiten", exact: true })
    .click();
  await page.getByRole("button", { name: "Zurückholen", exact: true }).click();
  await page.locator("#task-dialog").waitFor({ state: "hidden" });
  // Mobile should keep the page bounded; the table scrolls within its container.
  await page.setViewportSize({ width: 390, height: 844 });
  await page
    .locator("#view-nav")
    .getByRole("button", { name: /Liste$/ })
    .click();
  assert.ok(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  );
  await page.screenshot({
    path: path.join(screenshotDir, "tasks-mobile.png"),
    fullPage: true,
  });
  await page.setViewportSize({ width: 1512, height: 1100 });
  await page.locator("#task-login").click();
  await page.getByText("Leseansicht", { exact: true }).waitFor();
  assert.equal(
    await page.locator(".complete-toggle:not([disabled])").count(),
    0,
  );
  await page.locator("#task-login").click();
  await page.locator("#tasks-password").fill("Task-Browser-123456!");
  await page.getByRole("button", { name: "Anmelden", exact: true }).click();
  await page.locator("#utility-dialog").waitFor({ state: "hidden" });
  assert.deepEqual(errors, []);
  console.log(
    JSON.stringify({
      views: 11,
      passed: [
        "create",
        "inline priority",
        "search",
        "group",
        "columns",
        "saved view",
        "subtask",
        "note",
        "time",
        "duplicate",
        "undo",
        "kanban drag",
        "JSON export",
        "CSV export",
        "import merge",
        "archive",
        "restore",
        "mobile",
        "read only",
        "login",
      ],
      pageErrors: errors,
    }),
  );
  await cleanup();
})().catch(async (e) => {
  console.error(e);
  await cleanup();
  process.exitCode = 1;
});
