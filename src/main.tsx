import React from "react";
import { createRoot } from "react-dom/client";
import { App } from "./app/App";
import "./styles/app.css";
class ErrorBoundary extends React.Component<
  React.PropsWithChildren,
  { error: string }
> {
  state = { error: "" };
  static getDerivedStateFromError(error: Error) {
    return { error: error.message };
  }
  render() {
    if (this.state.error)
      return (
        <main className="fatal">
          <img src="/nexus.svg" width="48" />
          <h1>NEXUS needs to reload</h1>
          <p>
            Your engagement remains on disk. Reload to recover the workspace.
          </p>
          <pre>{this.state.error}</pre>
          <button onClick={() => location.reload()}>Reload workspace</button>
        </main>
      );
    return this.props.children;
  }
}
createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </React.StrictMode>,
);
