import { useApp } from "../../shell/AppContext";

export function LaunchAdvanced() {
  const { selected, updateInstance } = useApp();

  return (
    <section className="panel setting">
      <div className="setting-head">
        <label htmlFor="java-args">額外 Java 參數</label>
      </div>
      <input
        id="java-args"
        type="text"
        spellCheck={false}
        placeholder="不確定的話留空"
        disabled={!selected}
        value={selected?.javaArgs ?? ""}
        onChange={(event) => updateInstance({ javaArgs: event.target.value })}
      />
    </section>
  );
}
