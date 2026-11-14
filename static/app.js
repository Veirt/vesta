function formatDate(date) {
  return date.toLocaleDateString(undefined, {
    weekday: "long",
    month: "long",
    day: "2-digit",
  });
}

function pad(value) {
  return String(value).padStart(2, "0");
}

function formatTime(date) {
  return `${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
}

function initClock(root) {
  if (root.dataset.clockInitialized === "true") return;
  root.dataset.clockInitialized = "true";

  const time = root.querySelector("[data-clock-time]");
  const date = root.querySelector("[data-clock-date]");
  const year = root.querySelector("[data-clock-year]");
  const zone = root.querySelector("[data-clock-tz]");
  const utc = root.querySelector("[data-clock-utc]");

  function update() {
    const now = new Date();
    if (time) time.textContent = formatTime(now);
    if (date) date.textContent = formatDate(now);
    if (year) year.textContent = String(now.getFullYear());
    if (zone) zone.textContent = Intl.DateTimeFormat().resolvedOptions().timeZone || "";
    if (utc)
      utc.textContent = `UTC ${pad(now.getUTCHours())}:${pad(now.getUTCMinutes())}:${pad(now.getUTCSeconds())}`;
  }

  update();
  window.setInterval(update, 1000);
}

function initAllClocks(root = document) {
  root.querySelectorAll("[data-clock]").forEach(initClock);
}

document.addEventListener("DOMContentLoaded", () => {
  initAllClocks();
  window.addEventListener("htmx:afterSettle", () => initAllClocks());
});
