function formatDate(d) {
	return d.toLocaleDateString(undefined, {
		weekday: "long",
		month: "long",
		day: "2-digit",
	});
}

function pad(n) {
	return String(n).padStart(2, "0");
}

function formatTime(d) {
	return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}

function formatUTCTime(d) {
	return `${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())}:${pad(d.getUTCSeconds())}`;
}

function initClock(root) {
	const timeEl = root.querySelector("[data-clock-time]");
	const dateEl = root.querySelector("[data-clock-date]");
	const yearEl = root.querySelector("[data-clock-year]");
	const tzEl = root.querySelector("[data-clock-tz]");
	const utcEl = root.querySelector("[data-clock-utc]");

	function update() {
		const now = new Date();
		if (timeEl) timeEl.textContent = formatTime(now);
		if (dateEl) dateEl.textContent = formatDate(now);
		if (yearEl) yearEl.textContent = String(now.getFullYear());
		if (tzEl)
			tzEl.textContent = Intl.DateTimeFormat().resolvedOptions().timeZone || "";
		if (utcEl) {
			const label = "UTC: ";
			utcEl.textContent = label + formatUTCTime(now);
		}
	}

	update();
	const ms = 1000 - (Date.now() % 1000);
	setTimeout(() => {
		update();
		setInterval(update, 1000);
	}, ms);
}

function initAllClocks() {
	document.querySelectorAll("[data-clock]").forEach((root) => initClock(root));
}

document.addEventListener("DOMContentLoaded", () => {
	const sidebar = document.getElementById("sidebar");
	const menuToggle = document.getElementById("mobile-menu-toggle");
	const menuClose = document.getElementById("mobile-menu-close");
	const overlay = document.querySelector(".mobile-menu-overlay");

	function toggleMobileMenu() {
		sidebar.classList.toggle("-translate-x-full");
		document.body.classList.toggle("mobile-menu-open");
		menuToggle.classList.toggle("hidden");
	}

	function closeMobileMenu() {
		document.body.classList.remove("mobile-menu-open");
		sidebar.classList.add("-translate-x-full");
		menuToggle.classList.remove("hidden");
	}

	if (menuToggle) {
		menuToggle.addEventListener("click", toggleMobileMenu);
	}

	if (menuClose) {
		menuClose.addEventListener("click", closeMobileMenu);
	}

	if (overlay) {
		overlay.addEventListener("click", closeMobileMenu);
	}

	const sidebarLinks = document.querySelectorAll("#sidebar a");
	sidebarLinks.forEach((link) => {
		link.addEventListener("click", closeMobileMenu);
	});

	document.addEventListener("keydown", (event) => {
		if (event.key === "Escape") {
			closeMobileMenu();
		}
	});

	initAllClocks();

	if (window.htmx) {
		window.addEventListener("htmx:afterSettle", () => {
			initAllClocks();
		});
	}
});
