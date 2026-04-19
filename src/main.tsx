import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { QuickCapture } from "./windows/QuickCapture";

const params = new URLSearchParams(window.location.search);
const root = ReactDOM.createRoot(
  document.getElementById("root") as HTMLElement
);

if (params.get("window") === "quick-capture") {
  root.render(
    <React.StrictMode>
      <QuickCapture />
    </React.StrictMode>
  );
} else {
  root.render(
    <React.StrictMode>
      <App />
    </React.StrictMode>
  );
}
