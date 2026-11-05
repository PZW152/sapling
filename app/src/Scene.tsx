/** 全窗口背景：一層柔和的光和兩層山丘，切換分頁時山丘輕微移動。 */
export function Scene() {
  return (
    <div className="scene" aria-hidden="true">
      <div className="sun" />
      <svg className="hills hills-back" viewBox="0 0 800 220" preserveAspectRatio="none">
        <path d="M0 170C190 130 430 190 800 40V220H0Z" />
      </svg>
      <svg className="hills hills-front" viewBox="0 0 800 220" preserveAspectRatio="none">
        <path d="M0 196C260 180 520 200 800 110V220H0Z" />
      </svg>
    </div>
  );
}
