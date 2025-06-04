import init from "../pkg/portfolio.js";
import { Terminal3D } from "./terminal3d.js";
import { soundManager } from "./sound.js";

let isWebsiteStarted = false;
let isMuted = false;
let typingInterval;

function typeText() {
  const text = "objz@portfolio";
  const typedElement = document.getElementById("typed-text");
  let i = 0;

  typingInterval = setInterval(() => {
    if (i < text.length) {
      typedElement.textContent += text.charAt(i);
      i++;
    } else {
      clearInterval(typingInterval);
    }
  }, 100);
}

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
  const minimalOverlay = document.getElementById("minimal-overlay");

  loading.classList.add("hidden");

  setTimeout(() => {
    minimalOverlay.classList.add("visible");
    typeText();
  }, 1000);

  if (window.terminal3d) {
    window.terminal3d.startIntroSequence();
  }

  setTimeout(() => {
    loading.style.display = "none";
  }, 500);
}

function toggleMute() {
  const muteButton = document.getElementById("mute-btn");
  const audioIcon = document.getElementById("audio-icon");

  isMuted = !isMuted;

  if (isMuted) {
    soundManager.sounds.intro.volume = 0;
    soundManager.sounds.loop.volume = 0;

    muteButton.classList.add("muted");
    audioIcon.textContent = "♪̸";
    muteButton.title = "Unmute Ambient Music";
  } else {
    soundManager.sounds.intro.volume = soundManager.ambientVolume;
    soundManager.sounds.loop.volume = soundManager.ambientVolume;

    muteButton.classList.remove("muted");
    audioIcon.textContent = "♪";
    muteButton.title = "Mute Ambient Music";
  }
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
  const muteBtn = document.getElementById("mute-btn");

  simulateLoading();

  startButton.addEventListener("click", startWebsite);
  muteBtn.addEventListener("click", toggleMute);

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

    if (isWebsiteStarted && event.key.toLowerCase() === "m") {
      event.preventDefault();
      toggleMute();
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
  if (typingInterval) {
    clearInterval(typingInterval);
  }
});

export { Terminal3D };
