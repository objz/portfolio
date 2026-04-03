import init from "../pkg/portfolio.js";
import { SceneManager } from "./scene.js";
import { GuideManager } from "./guide.js";

export class AppManager {
  constructor() {
    this.sceneManager = null;
    this.guideManager = null;
    this.isStarted = false;
    this.isMuted = false;
    this.sceneReady = false;
    this.typingInterval = null;
    this.startupFallbackTimer = null;

    this.startButton = null;
    this.audioButton = null;
    this.audioIcon = null;
    this.typedElement = null;
    this.loading = null;
    this.overlay = null;

    this.init_self();
  }

  async init_self() {
    try {
      await this.initWasm();
      this.initElements();
      await this.initGuide();
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
    this.startButton = document.getElementById("start-btn");
    this.audioButton = document.getElementById("audio-btn");
    this.audioIcon = document.getElementById("audio-icon");
    this.typedElement = document.getElementById("typed-text");
    this.loading = document.getElementById("loading");
    this.overlay = document.getElementById("overlay");
  }

  async initGuide() {
    this.guideManager = new GuideManager("./guide.json");
    await this.guideManager.init();
    
    // Expose guide to window for dev access: window.guide.skipTo(16)
    window.guide = this.guideManager;
  }

  initEventListeners() {
    if (this.startButton) {
      this.startButton.addEventListener("click", () => this.startWebsite());
    }

    if (this.audioButton) {
      this.audioButton.addEventListener("click", () => this.toggleMute());
    }

    window.addEventListener("beforeunload", () => this.dispose());

    window.addEventListener("sceneReady", (event) => {
      this.handleSceneReady(event);
    });
  }

  async initScene() {
    this.sceneManager = new SceneManager();
    this.startStartupFallbackTimer();
    console.log("3D Scene initialized!");
  }

  startStartupFallbackTimer() {
    this.clearStartupFallbackTimer();

    this.startupFallbackTimer = window.setTimeout(() => {
      if (this.sceneReady || this.isStarted) {
        return;
      }

      const notice = document.querySelector(".desktop-notice");
      if (notice) {
        notice.textContent = "Loading is taking longer than expected...";
        notice.style.color = "#ffb86c";
      }
    }, 15000);
  }

  clearStartupFallbackTimer() {
    if (!this.startupFallbackTimer) {
      return;
    }

    clearTimeout(this.startupFallbackTimer);
    this.startupFallbackTimer = null;
  }

  handleSceneReady(event) {
    this.sceneReady = true;
    this.clearStartupFallbackTimer();

    const details = event && event.detail ? event.detail : null;

    if (details && !details.modelLoaded && !this.isStarted) {
      this.showFatalError("Failed to load. Please try refreshing the page.");
      return;
    }

    if (!this.isStarted) {
      this.enableManualStart();
    }

    console.log("Scene is ready!");
  }

  enableManualStart(
    noticeMessage,
    loadingStateText = "Ready",
    loadingStateColor = "#ffffff",
  ) {
    if (this.startButton) {
      this.startButton.classList.remove("hidden");
      this.startButton.classList.add("visible");
    }

    const progress = document.getElementById("loading-progress");
    if (progress) {
      const currentWidth = Number.parseFloat(progress.style.width || "0");
      if (currentWidth < 95) {
        progress.style.width = "95%";
      }
    }

    const loadingText = document.querySelector(".loading-text");
    if (loadingText && !this.isStarted) {
      loadingText.textContent = loadingStateText;
      loadingText.style.color = loadingStateColor;
    }

    const notice = document.querySelector(".desktop-notice");
    if (notice && noticeMessage) {
      notice.textContent = noticeMessage;
      notice.style.color = "#ffb86c";
    }
  }

  startWebsite() {
    if (this.isStarted) return;
    this.isStarted = true;
    this.clearStartupFallbackTimer();

    console.log("Starting website...");

    this.hideLoading();

    this.showTerminalOverlay();

    this.showScene();

    this.startIntroSequence();

    if (this.guideManager) {
      this.guideManager.show();
    }
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
      if (this.overlay) {
        this.overlay.classList.add("visible");
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

    this.typedElement.textContent = "";

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
    if (!this.audioButton || !this.audioIcon) return;

    if (this.isMuted) {
      this.audioButton.classList.add("muted");
      this.audioIcon.textContent = "♪̸";
      this.audioButton.title = "Unmute Ambient Music";
    } else {
      this.audioButton.classList.remove("muted");
      this.audioIcon.textContent = "♪";
      this.audioButton.title = "Mute Ambient Music";
    }
  }

  showFatalError(message) {
    const txt = document.querySelector(".loading-text");
    if (txt) {
      txt.textContent = message;
      txt.style.color = "#ff5555";
    }

    if (this.startButton) {
      this.startButton.classList.remove("visible");
      this.startButton.classList.add("hidden");
    }

    const notice = document.querySelector(".desktop-notice");
    if (notice) {
      notice.textContent = "";
    }
  }

  showError() {
    this.showFatalError("Failed to load application. Please refresh the page.");
  }

  dispose() {
    this.clearStartupFallbackTimer();

    if (this.typingInterval) {
      clearInterval(this.typingInterval);
      this.typingInterval = null;
    }

    if (this.sceneManager) {
      this.sceneManager.dispose();
      this.sceneManager = null;
    }

    if (this.guideManager) {
      this.guideManager.dispose();
      this.guideManager = null;
    }
  }
}

let appManager = null;

document.addEventListener("DOMContentLoaded", () => {
  appManager = new AppManager();
});
