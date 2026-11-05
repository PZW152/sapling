/** 全窗口背景：清晨的天色和淡出的遠山。 */
export function Scene() {
  return (
    <div className="scene" aria-hidden="true">
      <div className="glow" />
      <svg className="hills hills-back" viewBox="0 0 800 220" preserveAspectRatio="none">
        <defs>
          <linearGradient id="hill-fade" x1="0" y1="0" x2="0" y2="1">
            <stop className="hill-stop-top" offset="0" />
            <stop className="hill-stop-bottom" offset="1" />
          </linearGradient>
        </defs>
        <path fill="url(#hill-fade)" d="M0 170C190 130 430 190 800 40V220H0Z" />
      </svg>
      <svg className="hills hills-front" viewBox="0 0 800 220" preserveAspectRatio="none">
        <path fill="url(#hill-fade)" d="M0 196C260 180 520 200 800 110V220H0Z" />
      </svg>
    </div>
  );
}
