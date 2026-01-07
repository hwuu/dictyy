import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { Bubble } from "./Bubble";
import "./index.css";

// 根据 URL 路径渲染不同组件
function Router() {
  const path = window.location.pathname;

  if (path === "/bubble" || path.startsWith("/bubble?")) {
    return <Bubble />;
  }

  return <App />;
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Router />
  </React.StrictMode>,
);
