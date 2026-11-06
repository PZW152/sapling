import { Component } from "react";
import type { ReactNode } from "react";

type Props = {
  /** 出錯時顯示的名稱。沒有名稱的小區塊出錯時直接隱藏，不佔版面。 */
  name?: string;
  children: ReactNode;
};

type State = { message: string | null };

/** 把錯誤關在單一模塊裡：一個模塊崩潰時，其他頁面和導航照常運作。 */
export class FeatureBoundary extends Component<Props, State> {
  state: State = { message: null };

  static getDerivedStateFromError(error: unknown): State {
    return { message: error instanceof Error ? error.message : String(error) };
  }

  render() {
    const { message } = this.state;
    if (message === null) {
      return this.props.children;
    }
    if (!this.props.name) {
      return null;
    }
    return (
      <section className="panel feature-error" role="alert">
        <h2>「{this.props.name}」出了問題</h2>
        <p>其他功能不受影響。可以再試一次，或重開啟動器。</p>
        <p className="error">{message}</p>
        <button className="secondary" onClick={() => this.setState({ message: null })}>
          再試一次
        </button>
      </section>
    );
  }
}
