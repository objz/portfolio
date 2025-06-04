import init from "../pkg/portfolio.js";
import { Terminal3D } from "./terminal3d.js";

let isWebsiteStarted = false;

function simulateLoading() {
  const progressBar = document.getElementById("loading-progress");
  const startButton = document.getElementById("start-button");
  let progress = 0;

  const interval = setInterval(() => {
    progress += Math.random() * 15;
    if (progress >= 100) {
      progress = 100;
      clearInterval(interval);

      document
        .querySelector(".loading-bar")
        .setAttribute("data-text", "READY!");

      setTimeout(() => {
        startButton.classList.remove("hidden");
        startButton.classList.add("visible");
      }, 500);
    }
    progressBar.style.width = progress + "%";
  }, 200);
}

function startWebsite() {
  if (isWebsiteStarted) return;
  isWebsiteStarted = true;

  const loading = document.getElementById("loading");

  loading.classList.add("hidden");

  if (window.terminal3d) {
    window.terminal3d.startIntroSequence();
  }

  setTimeout(() => {
    loading.style.display = "none";
  }, 500);
}

init().then(() => {
  console.log("Portfolio loaded successfully!");

  setTimeout(() => {
    window.terminal3d = new Terminal3D({
      bulge: 0.9,
      scanlineIntensity: 0.02,
      scanlineCount: 640,
      vignetteIntensity: 0.3,
      vignetteRadius: 0.26,
      glowIntensity: 0.005,
      glowColor: {
        x: 0,
        y: 0.01,
        z: 0.01,
      },
      brightness: 0.85,
      contrast: 1.05,
      offsetX: 0.54,
      offsetY: 0.7,
      sceneX: 0,
      sceneY: 0,
      sceneZ: 0,
      skipIntro: true,
    });

    console.log("3D Terminal loaded in background!");
  }, 1000);
});

document.addEventListener("DOMContentLoaded", () => {
  const startButton = document.getElementById("start-button");

  simulateLoading();

  startButton.addEventListener("click", startWebsite);

  document.addEventListener("keydown", (event) => {
    if (
      !isWebsiteStarted &&
      (event.code === "Enter" || event.code === "Space")
    ) {
      event.preventDefault();
      startWebsite();
    }

    if (
      isWebsiteStarted &&
      (event.ctrlKey || event.metaKey) &&
      event.key === "d"
    ) {
      event.preventDefault();
      window.toggleDebug();
    }
  });

  window.toggleDebug = () => {
    if (window.terminal3d) {
      window.terminal3d.toggleDebugPanel();
    }
  };
});

document.addEventListener("terminalReady", (event) => {
  console.log("Terminal is ready", event.detail.terminal3d);
});

window.addEventListener("beforeunload", () => {
  if (window.terminal3d) {
    window.terminal3d.dispose();
  }
});

export { Terminal3D };
