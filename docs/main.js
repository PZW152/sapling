const RELEASES_API = "https://api.github.com/repos/PZW152/sapling/releases";
const BYTES_PER_MB = 1024 * 1024;
const calm = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

/** 標題一個字一個字跳上來，和啟動器的版本號一樣。 */
function splitTitle() {
  for (const title of document.querySelectorAll("[data-split]")) {
    title.setAttribute("aria-label", title.textContent);
    title.textContent = "";
    let index = 0;
    for (const line of title.dataset.split.split("|")) {
      const row = document.createElement("span");
      row.className = "line";
      row.setAttribute("aria-hidden", "true");
      for (const character of line) {
        const part = document.createElement("span");
        part.className = "char";
        part.style.setProperty("--i", index);
        part.textContent = character;
        row.append(part);
        index += 1;
      }
      title.append(row);
    }
  }
}

function revealOnScroll() {
  const targets = document.querySelectorAll("[data-reveal]");
  if (calm || !("IntersectionObserver" in window)) {
    targets.forEach((target) => target.classList.add("in"));
    return;
  }
  document.documentElement.classList.add("can-reveal");
  const observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (entry.isIntersecting) {
          entry.target.classList.add("in");
          observer.unobserve(entry.target);
        }
      }
    },
    { threshold: 0.12 },
  );
  targets.forEach((target) => observer.observe(target));
}

/** 下載按鈕直接指向最新版。查不到時保留原本的 Releases 連結。 */
async function loadRelease() {
  const response = await fetch(RELEASES_API, { headers: { Accept: "application/vnd.github+json" } });
  if (!response.ok) {
    return;
  }
  const releases = await response.json();
  const latest = releases.find((release) => !release.draft && !release.prerelease);
  const asset = latest?.assets.find((item) => /^SPL-.*\.exe$/.test(item.name));
  if (!asset) {
    return;
  }
  const version = latest.tag_name.replace(/^v/, "");
  document.querySelectorAll("[data-version]").forEach((item) => {
    item.textContent = version;
  });
  document.querySelectorAll("[data-download]").forEach((link) => {
    link.href = asset.browser_download_url;
  });
  document.querySelector("[data-size]").textContent = `${(asset.size / BYTES_PER_MB).toFixed(1)} MB`;

  const total = releases
    .flatMap((release) => release.assets)
    .reduce((sum, item) => sum + item.download_count, 0);
  if (total > 0) {
    document.querySelector("[data-downloads]").textContent = total.toLocaleString("zh-Hant");
    document.querySelector("[data-downloads-row]").hidden = false;
  }
  if (asset.digest) {
    document.querySelector("[data-hash]").textContent = asset.digest.replace("sha256:", "");
    document.querySelector("[data-hash-row]").hidden = false;
  }
}

function tour() {
  const tabs = [...document.querySelectorAll(".tour-tabs [role=tab]")];
  const image = document.querySelector(".tour-view img");

  function choose(tab) {
    for (const item of tabs) {
      const selected = item === tab;
      item.setAttribute("aria-selected", String(selected));
      item.tabIndex = selected ? 0 : -1;
    }
    image.src = `assets/${tab.dataset.shot}.png`;
    image.alt = tab.dataset.alt;
    image.getAnimations().forEach((animation) => animation.cancel());
    if (!calm) {
      image.animate(
        [
          { opacity: 0.2, transform: "translateY(10px) scale(0.985)" },
          { opacity: 1, transform: "none" },
        ],
        { duration: 460, easing: getComputedStyle(document.documentElement).getPropertyValue("--spring") },
      );
    }
  }

  tabs.forEach((tab, index) => {
    tab.addEventListener("click", () => choose(tab));
    tab.addEventListener("keydown", (event) => {
      const step = { ArrowDown: 1, ArrowRight: 1, ArrowUp: -1, ArrowLeft: -1 }[event.key];
      if (step) {
        event.preventDefault();
        const next = tabs[(index + step + tabs.length) % tabs.length];
        next.focus();
        choose(next);
      }
    });
  });
}

function compare() {
  const box = document.querySelector("[data-compare]");
  box.querySelector("input").addEventListener("input", (event) => {
    box.style.setProperty("--split", `${event.target.value}%`);
  });
}

/** 影片進到畫面才載入和播放。 */
function video() {
  const element = document.querySelector("[data-video]");
  if (calm) {
    element.controls = true;
    return;
  }
  new IntersectionObserver(([entry]) => {
    if (entry.isIntersecting) {
      element.play().catch(() => {
        element.controls = true;
      });
    } else {
      element.pause();
    }
  }).observe(element);
}

/** 首頁截圖跟著滑鼠微微傾斜。 */
function tilt() {
  const area = document.querySelector("[data-tilt]");
  if (calm || !window.matchMedia("(hover: hover)").matches) {
    return;
  }
  window.addEventListener("pointermove", (event) => {
    const x = event.clientX / window.innerWidth - 0.5;
    const y = event.clientY / window.innerHeight - 0.5;
    area.style.setProperty("--tilt-x", `${(-y * 4).toFixed(2)}deg`);
    area.style.setProperty("--tilt-y", `${(x * 6).toFixed(2)}deg`);
  });
}

splitTitle();
revealOnScroll();
tour();
compare();
video();
tilt();
loadRelease().catch(() => {});
