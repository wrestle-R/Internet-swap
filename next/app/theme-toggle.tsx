"use client";
export default function ThemeToggle() {
  return (
    <button
      className="theme-button"
      aria-label="Toggle light or dark mode"
      title="Change appearance"
      onClick={() => {
        const mode =
          document.documentElement.dataset.mode === "dark" ? "light" : "dark";
        document.documentElement.dataset.mode = mode;
        try {
          localStorage.setItem("internet-swap-mode", mode);
        } catch {
          /* Session appearance still works if storage is unavailable. */
        }
      }}
    >
      <span className="to-light" aria-hidden="true">
        ☀
      </span>
      <span className="to-dark" aria-hidden="true">
        ☾
      </span>
    </button>
  );
}
