import init from "../pkg/portfolio.js";
import { SceneManager } from "./scene.js";

export class AppManager {
  constructor() {
    this.sceneManager = null;
    this.isStarted = false;
    this.isMuted = false;
    this.typingInterval = null;

    this.startButton = null;
    this.muteButton = null;
    this.audioIcon = null;
    this.typedElement = null;
    this.loading = null;
    this.minimalOverlay = null;

    this.init_self();
  }

  async init_self() {
    try {
      await this.initWasm();
      this.initElements();
      this.initEventListeners();
      this.initScene();
    } catch (error) {
      console.error("App initialization failed:", error);
      this.showError();
    }
  }

  async initWasm() {
    await init();
    console.log("WASM loaded successfully!");
  }

  initElements() {
    this.startButton = document.getElementById("start-button");
    this.muteButton = document.getElementById("mute-btn");
    this.audioIcon = document.getElementById("audio-icon");
    this.typedElement = document.getElementById("typed-text");
    this.loading = document.getElementById("loading");
    this.minimalOverlay = document.getElementById("minimal-overlay");
  }

  initEventListeners() {
    if (this.startButton) {
      this.startButton.addEventListener("click", () => this.startWebsite());
    }

    if (this.muteButton) {
      this.muteButton.addEventListener("click", () => this.toggleMute());
    }

    window.addEventListener("beforeunload", () => this.dispose());

    window.addEventListener("sceneReady", () => {
      console.log("Scene is ready!");
    });
  }

  async initScene() {
    this.sceneManager = new SceneManager();
    console.log("3D Scene initialized!");
  }

  startWebsite() {
    if (this.isStarted) return;
    this.isStarted = true;

    console.log("Starting website...");

    this.hideLoading();

    this.showTerminalOverlay();

    this.showScene();

    this.startIntroSequence();
  }

  hideLoading() {
    if (this.loading) {
      this.loading.classList.add("hidden");
      setTimeout(() => {
        this.loading.style.display = "none";
      }, 500);
    }
  }

  showTerminalOverlay() {
    setTimeout(() => {
      if (this.minimalOverlay) {
        this.minimalOverlay.classList.add("visible");
        this.typeText();
      }
    }, 1000);
  }

  showScene() {
    if (this.sceneManager) {
      this.sceneManager.showScene();
    }
  }

  startIntroSequence() {
    if (this.sceneManager) {
      this.sceneManager.startIntro();
    }
  }

  typeText() {
    const text = "objz@portfolio";
    if (!this.typedElement) return;

    let i = 0;
    this.typingInterval = setInterval(() => {
      if (i < text.length) {
        this.typedElement.textContent += text.charAt(i);
        i++;
      } else {
        clearInterval(this.typingInterval);
        this.typingInterval = null;
      }
    }, 100);
  }

  toggleMute() {
    this.isMuted = !this.isMuted;

    if (this.sceneManager && this.sceneManager.audioManager) {
      this.updateAudioState();
      this.updateMuteButton();
    }
  }

  updateAudioState() {
    const audioManager = this.sceneManager.audioManager;

    if (this.isMuted) {
      if (audioManager.sounds.intro) {
        audioManager.sounds.intro.volume = 0;
      }
      if (audioManager.sounds.loop) {
        audioManager.sounds.loop.volume = 0;
      }
    } else {
      if (audioManager.sounds.intro) {
        audioManager.sounds.intro.volume = audioManager.ambientVolume;
      }
      if (audioManager.sounds.loop) {
        audioManager.sounds.loop.volume = audioManager.ambientVolume;
      }
    }
  }

  updateMuteButton() {
    if (!this.muteButton || !this.audioIcon) return;

    if (this.isMuted) {
      this.muteButton.classList.add("muted");
      this.audioIcon.textContent = "♪̸";
      this.muteButton.title = "Unmute Ambient Music";
    } else {
      this.muteButton.classList.remove("muted");
      this.audioIcon.textContent = "♪";
      this.muteButton.title = "Mute Ambient Music";
    }
  }

  showError() {
    const txt = document.querySelector(".loading-text");
    if (txt) {
      txt.textContent = "Failed to load application";
      txt.style.color = "#ff5555";
    }
  }

  dispose() {
    if (this.typingInterval) {
      clearInterval(this.typingInterval);
      this.typingInterval = null;
    }

    if (this.sceneManager) {
      this.sceneManager.dispose();
      this.sceneManager = null;
    }
  }
}

let appManager = null;

document.addEventListener("DOMContentLoaded", () => {
  appManager = new AppManager();
});
