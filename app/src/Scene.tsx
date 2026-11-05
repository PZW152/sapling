import { useEffect, useRef } from "react";

/** 場景裡的花粉和落葉數量。 */
const POLLEN_COUNT = 42;
const LEAF_COUNT = 7;
const BURST_LEAVES = 16;
/** 滑鼠視差跟隨的速度，越小越慢。 */
const POINTER_FOLLOW = 0.05;
const GRAVITY = 0.09;

type Mote = { x: number; y: number; size: number; speed: number; phase: number };
type Leaf = Mote & { angle: number; spin: number };
type BurstLeaf = {
  x: number;
  y: number;
  vx: number;
  vy: number;
  angle: number;
  spin: number;
  life: number;
};

let bursts: BurstLeaf[] = [];

/** 從畫面某一點噴出一小撮葉子。 */
export function burstLeaves(x: number, y: number) {
  for (let index = 0; index < BURST_LEAVES; index += 1) {
    const direction = -Math.PI / 2 + (Math.random() - 0.5) * 2.2;
    const speed = 2.5 + Math.random() * 4;
    bursts.push({
      x,
      y,
      vx: Math.cos(direction) * speed,
      vy: Math.sin(direction) * speed,
      angle: Math.random() * Math.PI * 2,
      spin: (Math.random() - 0.5) * 0.3,
      life: 1,
    });
  }
}

/** 全窗口的活動背景：太陽、山丘、花粉和隨風飄的落葉。 */
export function Scene() {
  const canvas = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const element = canvas.current;
    const context = element?.getContext("2d");
    if (!element || !context) {
      return;
    }
    const root = document.documentElement;
    const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const random = (max: number) => Math.random() * max;
    const pollen: Mote[] = Array.from({ length: POLLEN_COUNT }, () => ({
      x: random(1),
      y: random(1),
      size: 0.8 + random(1.8),
      speed: 0.3 + random(0.7),
      phase: random(Math.PI * 2),
    }));
    const leaves: Leaf[] = Array.from({ length: LEAF_COUNT }, () => ({
      x: random(1),
      y: random(1),
      size: 5 + random(5),
      speed: 0.35 + random(0.5),
      phase: random(Math.PI * 2),
      angle: random(Math.PI * 2),
      spin: (Math.random() - 0.5) * 0.03,
    }));

    let width = 0;
    let height = 0;
    let frame = 0;
    let pointerX = 0;
    let pointerY = 0;
    let smoothX = 0;
    let smoothY = 0;

    function resize() {
      const scale = window.devicePixelRatio || 1;
      width = window.innerWidth;
      height = window.innerHeight;
      element!.width = width * scale;
      element!.height = height * scale;
      context!.setTransform(scale, 0, 0, scale, 0, 0);
    }

    function color(name: string) {
      return getComputedStyle(root).getPropertyValue(name).trim();
    }

    function drawLeaf(x: number, y: number, size: number, angle: number) {
      context!.save();
      context!.translate(x, y);
      context!.rotate(angle);
      context!.beginPath();
      context!.moveTo(-size, 0);
      context!.quadraticCurveTo(0, -size * 0.8, size, 0);
      context!.quadraticCurveTo(0, size * 0.8, -size, 0);
      context!.fill();
      context!.restore();
    }

    function draw(time: number) {
      const wind = Math.sin(time / 3200) * 0.5 + Math.sin(time / 1300) * 0.2;
      context!.clearRect(0, 0, width, height);

      context!.fillStyle = color("--color-pollen");
      for (const mote of pollen) {
        mote.x += (0.00016 + wind * 0.00022) * mote.speed;
        mote.y -= 0.00012 * mote.speed;
        if (mote.y < -0.02) mote.y = 1.02;
        if (mote.x > 1.02) mote.x = -0.02;
        if (mote.x < -0.02) mote.x = 1.02;
        const glow = 0.35 + Math.sin(time / 900 + mote.phase) * 0.3;
        context!.globalAlpha = Math.max(glow, 0.05);
        context!.beginPath();
        context!.arc(
          mote.x * width + smoothX * 10 * mote.size,
          mote.y * height + smoothY * 6 * mote.size,
          mote.size,
          0,
          Math.PI * 2,
        );
        context!.fill();
      }

      context!.fillStyle = color("--color-accent-leaf");
      context!.globalAlpha = 0.55;
      for (const leaf of leaves) {
        leaf.x += (0.00035 + wind * 0.0006) * leaf.speed;
        leaf.y += 0.0007 * leaf.speed;
        leaf.angle += leaf.spin + wind * 0.01;
        if (leaf.y > 1.05) {
          leaf.y = -0.05;
          leaf.x = Math.random();
        }
        if (leaf.x > 1.05) leaf.x = -0.05;
        if (leaf.x < -0.05) leaf.x = 1.05;
        const sway = Math.sin(time / 800 + leaf.phase) * 14;
        drawLeaf(leaf.x * width + sway, leaf.y * height, leaf.size, leaf.angle);
      }

      context!.fillStyle = color("--color-accent");
      for (const leaf of bursts) {
        leaf.vy += GRAVITY;
        leaf.vx *= 0.985;
        leaf.x += leaf.vx;
        leaf.y += leaf.vy;
        leaf.angle += leaf.spin;
        leaf.life -= 0.012;
        context!.globalAlpha = Math.max(leaf.life, 0);
        drawLeaf(leaf.x, leaf.y, 7, leaf.angle);
      }
      bursts = bursts.filter((leaf) => leaf.life > 0);
      context!.globalAlpha = 1;
    }

    function tick(time: number) {
      smoothX += (pointerX - smoothX) * POINTER_FOLLOW;
      smoothY += (pointerY - smoothY) * POINTER_FOLLOW;
      root.style.setProperty("--px", smoothX.toFixed(3));
      root.style.setProperty("--py", smoothY.toFixed(3));
      draw(time);
      // 窗口不在前面時停止繪製，遊戲運行時不佔用顯示卡。
      frame = document.hasFocus() || bursts.length > 0 ? requestAnimationFrame(tick) : 0;
    }

    function start() {
      if (!frame && !reducedMotion) {
        frame = requestAnimationFrame(tick);
      }
    }

    function trackPointer(event: PointerEvent) {
      pointerX = (event.clientX / width) * 2 - 1;
      pointerY = (event.clientY / height) * 2 - 1;
      start();
    }

    resize();
    draw(0);
    start();
    window.addEventListener("resize", resize);
    window.addEventListener("pointermove", trackPointer);
    window.addEventListener("focus", start);
    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener("resize", resize);
      window.removeEventListener("pointermove", trackPointer);
      window.removeEventListener("focus", start);
    };
  }, []);

  return (
    <div className="scene" aria-hidden="true">
      <div className="sun" />
      <div className="sun-rays" />
      <svg className="hills hills-far" viewBox="0 0 800 220" preserveAspectRatio="none">
        <path d="M0 120C150 70 300 150 460 96S700 60 800 110V220H0Z" />
      </svg>
      <svg className="hills hills-back" viewBox="0 0 800 220" preserveAspectRatio="none">
        <path d="M0 170C190 130 430 190 800 40V220H0Z" />
      </svg>
      <canvas ref={canvas} className="scene-canvas" />
      <svg className="hills hills-front" viewBox="0 0 800 220" preserveAspectRatio="none">
        <path d="M0 196C260 180 520 200 800 110V220H0Z" />
      </svg>
    </div>
  );
}
