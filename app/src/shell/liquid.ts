/**
 * 液態玻璃：像真的玻璃一樣，邊緣會把後面的畫面折彎。
 *
 * 瀏覽器沒有折射，所以這裡用光學公式算出每個像素該往哪裡偏，存成一張圖，
 * 再交給 SVG 位移濾鏡當 backdrop-filter。這個寫法只有 Chromium 支援，
 * 而 WebView2 就是 Chromium。
 */

const REFRACTIVE_INDEX = 1.5;
const SVG_NS = "http://www.w3.org/2000/svg";

type Lens = { width: number; height: number; radius: number; bezel: number };

/** 玻璃邊緣的高度曲線：圓潤的凸面，到中間變平。 */
function surfaceHeight(t: number) {
  return Math.pow(1 - Math.pow(1 - t, 4), 0.25);
}

/** 光線垂直射進斜面後，橫向偏移多少。t 是離邊緣的距離，0 到 1。 */
export function bendAt(t: number) {
  const step = 0.001;
  const clamped = Math.min(Math.max(t, step), 1 - step);
  const slope = (surfaceHeight(clamped + step) - surfaceHeight(clamped - step)) / (2 * step);
  const incoming = Math.atan(slope);
  const outgoing = Math.asin(Math.sin(incoming) / REFRACTIVE_INDEX);
  return Math.tan(incoming - outgoing) * (1 - surfaceHeight(clamped) * 0.5);
}

/** 圓角矩形內一點到邊緣的距離（內部為正）。 */
export function edgeDistance(x: number, y: number, lens: Lens) {
  const halfW = lens.width / 2;
  const halfH = lens.height / 2;
  const qx = Math.abs(x - halfW) - (halfW - lens.radius);
  const qy = Math.abs(y - halfH) - (halfH - lens.radius);
  const outside = Math.hypot(Math.max(qx, 0), Math.max(qy, 0));
  const inside = Math.min(Math.max(qx, qy), 0);
  return lens.radius - outside - inside;
}

/** 畫出位移圖。紅綠兩色是橫直偏移，正中的灰代表不動。 */
function displacementMap({ width, height, radius, bezel }: Lens) {
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext("2d");
  if (!context) {
    return null;
  }
  const lens = { width, height, radius, bezel };
  const image = context.createImageData(width, height);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const px = x + 0.5;
      const py = y + 0.5;
      const distance = edgeDistance(px, py, lens);
      let dx = 0;
      let dy = 0;
      if (distance > 0 && distance < bezel) {
        const nx = edgeDistance(px + 1, py, lens) - edgeDistance(px - 1, py, lens);
        const ny = edgeDistance(px, py + 1, lens) - edgeDistance(px, py - 1, lens);
        const length = Math.hypot(nx, ny) || 1;
        const bend = bendAt(distance / bezel);
        // 往外取樣：靜止時旁邊是空的，看起來乾淨；滑過內容時邊緣才折彎。
        dx = (-nx / length) * bend;
        dy = (-ny / length) * bend;
      }
      const index = (y * width + x) * 4;
      image.data[index] = 128 + dx * 127;
      image.data[index + 1] = 128 + dy * 127;
      image.data[index + 2] = 128;
      image.data[index + 3] = 255;
    }
  }
  context.putImageData(image, 0, 0);
  return canvas.toDataURL("image/png");
}

function filterHost() {
  let host = document.getElementById("liquid-filters");
  if (!host) {
    host = document.createElementNS(SVG_NS, "svg") as unknown as HTMLElement;
    host.id = "liquid-filters";
    host.setAttribute("aria-hidden", "true");
    host.setAttribute("width", "0");
    host.setAttribute("height", "0");
    host.style.position = "absolute";
    document.body.append(host);
  }
  return host;
}

const made = new Map<string, string>();

/** 取得這個尺寸的濾鏡。同樣大小只算一次。 */
function lensFilter(lens: Lens, strength: number) {
  const key = `${lens.width}x${lens.height}r${lens.radius}b${lens.bezel}s${strength}`;
  const existing = made.get(key);
  if (existing) {
    return existing;
  }
  const map = displacementMap(lens);
  if (!map) {
    return null;
  }
  const id = `liquid-${made.size}`;
  const filter = document.createElementNS(SVG_NS, "filter");
  filter.id = id;
  filter.setAttribute("filterUnits", "userSpaceOnUse");
  filter.setAttribute("primitiveUnits", "userSpaceOnUse");
  filter.setAttribute("color-interpolation-filters", "sRGB");
  filter.setAttribute("x", "0");
  filter.setAttribute("y", "0");
  filter.setAttribute("width", String(lens.width));
  filter.setAttribute("height", String(lens.height));

  const image = document.createElementNS(SVG_NS, "feImage");
  image.setAttribute("href", map);
  image.setAttribute("x", "0");
  image.setAttribute("y", "0");
  image.setAttribute("width", String(lens.width));
  image.setAttribute("height", String(lens.height));
  image.setAttribute("preserveAspectRatio", "none");
  image.setAttribute("result", "map");

  const displace = document.createElementNS(SVG_NS, "feDisplacementMap");
  displace.setAttribute("in", "SourceGraphic");
  displace.setAttribute("in2", "map");
  displace.setAttribute("scale", String(strength * 2));
  displace.setAttribute("xChannelSelector", "R");
  displace.setAttribute("yChannelSelector", "G");

  filter.append(image, displace);
  filterHost().append(filter);
  made.set(key, id);
  return id;
}

/** 把元件變成液態玻璃。尺寸改變時再呼叫一次。 */
export function applyLiquid(element: HTMLElement, width: number, height: number) {
  const w = Math.round(width);
  const h = Math.round(height);
  if (w < 8 || h < 8) {
    return;
  }
  const style = getComputedStyle(element);
  const radius = Math.min(parseFloat(style.borderTopLeftRadius) || 0, w / 2, h / 2);
  const bezel = Math.max(Math.min(radius, h / 2, 16), 6);
  const id = lensFilter({ width: w, height: h, radius, bezel }, Math.min(h * 0.3, 14));
  if (id) {
    element.style.setProperty("--liquid", `url(#${id})`);
  }
}
