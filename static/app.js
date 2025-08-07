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
});
