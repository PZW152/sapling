/** 全窗口背景：一整片主色和一道山丘。 */
export function Scene() {
  return (
    <div className="scene" aria-hidden="true">
      <p className="motto">天下為公</p>
      <svg className="hill" viewBox="0 0 860 170" preserveAspectRatio="none">
        <path d="M0 92C160 44 340 118 520 72S760 30 860 66V170H0Z" />
      </svg>
    </div>
  );
}
