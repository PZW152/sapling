const RELEASES_API = "https://api.github.com/repos/PZW152/sapling/releases";
const BYTES_PER_MB = 1024 * 1024;
const calm = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

/** 標題一個字一個字跳上來，和啟動器的版本號一樣。 */
function splitTitle() {
  for (const title of document.querySelectorAll("[data-split]")) {
    title.setAttribute("aria-label", title.textContent);
    title.textContent = "";
    [...title.dataset.split].forEach((character, index) => {
      const part = document.createElement("span");
      part.className = "char";
      part.setAttribute("aria-hidden", "true");
      part.style.setProperty("--i", index);
      part.textContent = character;
      title.append(part);
    });
  }
}

function revealOnScroll() {
  const targets = document.querySelectorAll("[data-reveal]");
  if (calm || !("IntersectionObserver" in window)) {
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
    { threshold: 0.1 },
  );
  targets.forEach((target) => observer.observe(target));
}

/** 影片進到畫面才載入和播放，離開就暫停。 */
function videos() {
  const elements = document.querySelectorAll("[data-video]");
  if (calm) {
    elements.forEach((element) => {
      element.controls = true;
    });
    return;
  }
  const observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (entry.isIntersecting) {
          entry.target.play().catch(() => {
            entry.target.controls = true;
          });
        } else {
          entry.target.pause();
        }
      }
    },
    { threshold: 0.3 },
  );
  elements.forEach((element) => observer.observe(element));
}

function fill(selector, text) {
  document.querySelectorAll(selector).forEach((item) => {
    item.textContent = text;
  });
}

/** 下載資訊都是即時的。查不到時保留頁面上的預設內容。 */
async function loadRelease() {
  const response = await fetch(RELEASES_API);
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
  fill("[data-version]", version);
  fill("[data-version-cell]", version);
  fill("[data-file]", asset.name);
  fill("[data-size]", `${(asset.size / BYTES_PER_MB).toFixed(1)} MB`);
  fill("[data-date]", new Date(latest.published_at).toLocaleDateString("zh-Hant"));
  document.querySelectorAll("[data-download]").forEach((link) => {
    link.href = asset.browser_download_url;
  });
  const total = releases
    .flatMap((release) => release.assets)
    .reduce((sum, item) => sum + item.download_count, 0);
  if (total > 0) {
    fill("[data-downloads]", total.toLocaleString("zh-Hant"));
    document.querySelector("[data-downloads-row]").hidden = false;
  }
  if (asset.digest) {
    fill("[data-hash]", asset.digest.replace("sha256:", ""));
    document.querySelector("[data-hash-row]").hidden = false;
  }
}

splitTitle();
revealOnScroll();
videos();
loadRelease().catch(() => {});
