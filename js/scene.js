import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";
import { ModelManager } from "./model.js";
import { EffectsManager } from "./effects.js";
import { ShaderManager } from "./shader.js";
import { AnimationManager } from "./animations.js";
import { MouseManager } from "./mouse.js";
import { AudioManager } from "./audio.js";
import { EventManager } from "./events.js";
import { PerformanceManager } from "./performance.js";

export class SceneManager {
  constructor() {
    this.performanceMode = this.detectDevice();
    this.frameTarget = this.performanceMode ? 30 : 60;
    this.frameInterval = 1000 / this.frameTarget;
    this.lastFrame = 0;
    this.animationId = null;
    this.textureUpdateRateMs = this.performanceMode ? 33 : 16;
    this.lastTextureUpdate = 0;

    this.crtSettings = this.getCrtSettings();
    this.startupState = "booting";
    this.startupIssues = [];
    this.isDegraded = false;
    this.modelLoaded = false;

    this.scene = null;
    this.camera = null;
    this.renderer = null;
    this.controls = null;
    this.terminalTexture = null;
    this.terminalCanvas = null;
    this.hiddenInput = null;

    this.terminalFocused = false;
    this.hoveringScreen = false;
    this.introComplete = false;
    this.everUnfocused = false;

    this.modelManager = null;
    this.effectsManager = null;
    this.shaderManager = null;
    this.animationManager = null;
    this.mouseManager = null;
    this.audioManager = null;
    this.eventManager = null;
    this.performanceManager = null;

    this.init();
  }

  detectDevice() {
    const canvas = document.createElement("canvas");
    const gl =
      canvas.getContext("webgl") || canvas.getContext("experimental-webgl");

    if (!gl) return true;

    const renderer = gl.getParameter(gl.RENDERER);
    const isMobile =
      /Android|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(
        navigator.userAgent,
      );
    const lowGpu = /Intel|Mali|Adreno 3|Adreno 4|PowerVR SGX/i.test(renderer);
    const memory = navigator.deviceMemory || 4;
    const cores = navigator.hardwareConcurrency || 4;
    const lowRes = window.screen.width < 1920 || window.screen.height < 1080;

    canvas.remove();
    return isMobile || lowGpu || memory < 4 || cores < 4 || lowRes;
  }

  getCrtSettings() {
    const lowPower = this.performanceMode;

    return {
      bulge: lowPower ? 0.6 : 0.75,
      scanlineIntensity: lowPower ? 0.013 : 0.018,
      scanlineCount: lowPower ? 520 : 620,
      vignetteIntensity: lowPower ? 0.2 : 0.26,
      vignetteRadius: 0.28,
      glowIntensity: lowPower ? 0.003 : 0.0045,
      glowColor: new THREE.Vector3(0, 0.01, 0.01),
      brightness: lowPower ? 0.9 : 0.87,
      contrast: lowPower ? 1.02 : 1.05,
      offsetX: 0.0,
      offsetY: 0.0,
    };
  }

  async init() {
    try {
      this.updateProgress(5);

      this.audioManager = new AudioManager();
      await this.executeStep(
        "Audio initialization",
        () => this.audioManager.init(),
        { timeoutMs: 3500, optional: true },
      );
      this.updateProgress(12);

      this.createScene();
      this.createCamera();
      this.createRenderer();
      this.createControls();
      this.createLighting();
      this.updateProgress(30);

      this.modelManager = new ModelManager(this.scene, (progress) =>
        this.updateProgress(progress),
      );
      this.effectsManager = new EffectsManager(
        this.renderer,
        this.scene,
        this.camera,
        this.performanceMode,
      );

      this.modelLoaded = await this.loadModelWithRetry();
      this.updateProgress(60);

      this.mouseManager = new MouseManager(
        this.camera,
        this.controls,
        this.modelManager,
        this.performanceMode,
      );

      await this.executeStep("Terminal texture setup", () => this.setupTexture(), {
        timeoutMs: 4000,
        optional: true,
      });
      this.updateProgress(80);

      this.eventManager = new EventManager(this);
      this.eventManager.setup();

      if (!this.performanceMode) {
        await this.executeStep(
          "Post-processing setup",
          () => this.effectsManager.setupPostProcessing(),
          { timeoutMs: 4500, optional: true },
        );
      }

      this.animationManager = new AnimationManager(
        this.camera,
        this.controls,
        this.performanceMode,
      );

      if (this.performanceMode) {
        this.performanceManager = new PerformanceManager(this);
        this.performanceManager.start();
      }
    } catch (error) {
      this.recordStartupIssue("Scene bootstrap", error);
      this.startupState = "error";
      this.showError("Failed to initialize 3D scene. Please refresh the page.");
    } finally {
      this.finalizeStartup();
    }
  }

  async executeStep(label, fn, options = {}) {
    const { timeoutMs = 5000, optional = false } = options;

    try {
      return await this.withTimeout(
        Promise.resolve().then(() => fn()),
        timeoutMs,
        `${label} timed out after ${timeoutMs}ms`,
      );
    } catch (error) {
      this.recordStartupIssue(label, error);

      if (!optional) {
        throw error;
      }

      this.isDegraded = true;
      if (this.startupState !== "error") {
        this.startupState = "degraded";
      }
      return null;
    }
  }

  async loadModelWithRetry() {
    const attempts = [
      { timeoutMs: 20000, label: "Model load" },
      { timeoutMs: 35000, label: "Model load (retry 1)" },
      { timeoutMs: 50000, label: "Model load (retry 2)" },
    ];

    for (let i = 0; i < attempts.length; i++) {
      const { timeoutMs, label } = attempts[i];

      try {
        await this.withTimeout(
          Promise.resolve().then(() => this.modelManager.loadModel()),
          timeoutMs,
          `${label} timed out after ${timeoutMs}ms`,
        );
        return true;
      } catch (error) {
        this.recordStartupIssue(label, error);

        if (i < attempts.length - 1) {
          console.warn(`${label} failed, retrying...`);
          this.setLoadingNotice(
            `Loading took too long, retrying (${i + 2}/${attempts.length})...`,
            true,
          );
        }
      }
    }

    return false;
  }

  withTimeout(promise, timeoutMs, timeoutMessage) {
    return new Promise((resolve, reject) => {
      let settled = false;

      const finish = (handler, value) => {
        if (settled) return;
        settled = true;
        clearTimeout(timerId);
        handler(value);
      };

      const timerId = window.setTimeout(() => {
        finish(reject, new Error(timeoutMessage));
      }, timeoutMs);

      Promise.resolve(promise)
        .then((value) => finish(resolve, value))
        .catch((error) => finish(reject, error));
    });
  }

  recordStartupIssue(label, error) {
    const message =
      error instanceof Error ? error.message : String(error || "Unknown error");
    const entry = `${label}: ${message}`;
    this.startupIssues.push(entry);
    console.warn(`[Startup] ${entry}`);
  }

  setLoadingNotice(text, isWarning = false) {
    const notice = document.querySelector(".desktop-notice");
    if (!notice) return;

    notice.textContent = text;
    notice.style.color = isWarning ? "#ffb86c" : "#8be9fd";
  }

  finalizeStartup() {
    this.updateProgress(100);
    this.showTerminal();

    if (this.renderer && !this.animationId) {
      this.animate();
    }

    if (!this.modelLoaded) {
      this.startupState = "error";
      this.isDegraded = true;
      this.showError("Failed to load 3D model. Please refresh the page.");
    } else if (this.startupIssues.length > 0) {
      this.isDegraded = true;
      if (this.startupState !== "error") {
        this.startupState = "degraded";
      }
      this.showStartButton();
    } else {
      if (this.startupState !== "error") {
        this.startupState = "ready";
      }
      this.showStartButton();
    }

    window.dispatchEvent(
      new CustomEvent("sceneReady", {
        detail: {
          state: this.startupState,
          degraded: this.isDegraded,
          modelLoaded: this.modelLoaded,
          issues: this.startupIssues.slice(),
          performanceMode: this.performanceMode,
        },
      }),
    );
  }

  createScene() {
    this.scene = new THREE.Scene();
    this.scene.background = null;
    this.scene.fog = new THREE.FogExp2(0x14212c, this.performanceMode ? 0.028 : 0.02);
  }

  createCamera() {
    this.camera = new THREE.PerspectiveCamera(
      45,
      window.innerWidth / window.innerHeight,
      0.1,
      100,
    );
    this.camera.position.set(4, 5, 10);
  }

  createRenderer() {
    this.renderer = new THREE.WebGLRenderer({
      antialias: !this.performanceMode,
      powerPreference: this.performanceMode ? "default" : "high-performance",
      alpha: true,
      precision: this.performanceMode ? "mediump" : "highp",
    });

    this.renderer.setSize(window.innerWidth, window.innerHeight);

    const maxRatio = this.performanceMode ? 1.5 : 2;
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, maxRatio));

    this.renderer.shadowMap.enabled = !this.performanceMode;
    if (this.renderer.shadowMap.enabled) {
      this.renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    }

    this.renderer.toneMapping = THREE.ACESFilmicToneMapping;
    this.renderer.toneMappingExposure = 1.0;
    this.renderer.outputColorSpace = THREE.SRGBColorSpace;

    if (!this.performanceMode) {
      this.renderer.physicallyCorrectLights = true;
      this.renderer.gammaFactor = 2.2;
    }

    document
      .getElementById("scene-container")
      .appendChild(this.renderer.domElement);
  }

  createControls() {
    this.controls = new OrbitControls(this.camera, this.renderer.domElement);
    this.controls.enableDamping = true;
    this.controls.dampingFactor = this.performanceMode ? 0.1 : 0.05;
    this.controls.enableZoom = false;
    this.controls.enablePan = true;
    this.controls.enableRotate = true;
    this.controls.target.set(0, 1.5, 0);
    this.controls.minDistance = 3;
    this.controls.maxDistance = 20;
    this.controls.minPolarAngle = Math.PI * 0.05;
    this.controls.maxPolarAngle = Math.PI * 0.75;
    this.controls.minAzimuthAngle = -Math.PI * 0.75;
    this.controls.maxAzimuthAngle = Math.PI * 0.75;
  }

  createLighting() {
    const ambient = new THREE.AmbientLight(
      0x404040,
      this.performanceMode ? 0.8 : 0.6,
    );
    this.scene.add(ambient);

    const main = new THREE.DirectionalLight(
      0xffffff,
      this.performanceMode ? 1.0 : 1.2,
    );
    main.position.set(8, 10, 6);

    if (!this.performanceMode) {
      main.castShadow = true;
      main.shadow.mapSize.setScalar(2048);
      main.shadow.bias = -0.0001;
      main.shadow.camera.near = 0.1;
      main.shadow.camera.far = 50;
      main.shadow.camera.left = main.shadow.camera.bottom = -15;
      main.shadow.camera.right = main.shadow.camera.top = 15;
    }
    this.scene.add(main);

    if (!this.performanceMode) {
      const fill = new THREE.DirectionalLight(0x87ceeb, 0.4);
      fill.position.set(-5, 6, -4);
      this.scene.add(fill);

      const rim = new THREE.DirectionalLight(0x50fa7b, 0.2);
      rim.position.set(0, 4, -8);
      this.scene.add(rim);

      const accent1 = new THREE.PointLight(0x87ceeb, 0.6, 12);
      accent1.position.set(4, 4, 4);
      this.scene.add(accent1);

      const accent2 = new THREE.PointLight(0x50fa7b, 0.4, 10);
      accent2.position.set(-4, 3, 2);
      this.scene.add(accent2);

      const front = new THREE.DirectionalLight(0xf8f8ff, 0.6);
      front.position.set(0, 8, 12);
      this.scene.add(front);
    }

    const monitor = new THREE.PointLight(0x50fa7b, 0.8, 8);
    monitor.position.set(0, 3, 2);
    this.scene.add(monitor);

    const hemi = new THREE.HemisphereLight(0x87ceeb, 0x2c2c54, 0.5);
    hemi.position.set(0, 25, 0);
    this.scene.add(hemi);
  }

  async setupTexture() {
    this.terminalCanvas = document.getElementById("terminal");
    this.hiddenInput = document.getElementById("hidden-input");

    if (!this.terminalCanvas || !this.hiddenInput) {
      console.error("Terminal elements not found!");
      return;
    }

    await new Promise((r) => requestAnimationFrame(r));

    this.terminalTexture = new THREE.CanvasTexture(this.terminalCanvas);
    this.terminalTexture.minFilter = THREE.LinearFilter;
    this.terminalTexture.magFilter = THREE.LinearFilter;
    this.terminalTexture.flipY = true;

    this.shaderManager = new ShaderManager(
      this.terminalTexture,
      this.crtSettings,
    );

    if (this.modelManager.screenMesh) {
      this.modelManager.screenMesh.material =
        this.shaderManager.createMaterial();
      this.shaderManager.applyBulgeToScreen(this.modelManager.screenMesh);
    }

    this.terminalTexture.offset.y = 0.0;
    this.terminalTexture.repeat.y = 1.0;
    this.lastTextureUpdate = performance.now();
  }

  startIntro() {
    if (this.introComplete) return;
    window.dispatchEvent(
      new CustomEvent("terminalReady", { detail: { sceneManager: this } }),
    );
    window.dispatchEvent(new CustomEvent("terminalFocus"));

    setTimeout(() => {
      window.dispatchEvent(new CustomEvent("terminalFocus"));
      this.introComplete = true;
      if (this.mouseManager) {
        this.mouseManager.onIntroComplete();
      }
    }, 15000);

    if (this.audioManager) {
      this.audioManager.play("boot");
      this.audioManager.play("fan");
    }

    if (this.animationManager) {
      this.animationManager.startupAnimation();
    }
  }

  updateProgress(p) {
    const bar = document.getElementById("loading-progress");
    if (!bar) return;

    const clampedProgress = Math.max(0, Math.min(100, Math.round(p)));
    bar.style.width = `${clampedProgress}%`;
  }

  showStartButton() {
    const startButton = document.getElementById("start-btn");
    if (!startButton) return;

    startButton.classList.remove("hidden");
    startButton.classList.add("visible");
  }

  showError(message = "Failed to load") {
    const txt = document.querySelector(".loading-text");
    if (txt) {
      txt.textContent = message;
      txt.style.color = "#ff5555";
    }
  }

  showTerminal() {
    const terminal = document.getElementById("terminal");
    if (terminal) {
      terminal.style.visibility = "visible";
    }
  }

  showScene() {
    const container = document.getElementById("scene-container");
    if (container) {
      container.style.animationPlayState = "running";
      container.style.visibility = "visible";
    }
  }

  animate() {
    const now = performance.now();
    const screenMesh = this.modelManager ? this.modelManager.screenMesh : null;

    if (now - this.lastFrame < this.frameInterval) {
      this.animationId = requestAnimationFrame(() => this.animate());
      return;
    }

    const deltaMs = this.lastFrame > 0 ? now - this.lastFrame : this.frameInterval;
    const deltaSeconds = Math.min(0.05, deltaMs / 1000);

    this.lastFrame = now;
    this.animationId = requestAnimationFrame(() => this.animate());

    if (this.controls) this.controls.update();
    if (this.shaderManager)
      this.shaderManager.updateTime(now * 0.001, screenMesh);

    if (
      this.terminalTexture &&
      now - this.lastTextureUpdate >= this.textureUpdateRateMs
    ) {
      this.terminalTexture.needsUpdate = true;
      this.lastTextureUpdate = now;
    }

    if (this.animationManager) {
      this.animationManager.updateIdleRotation(this.terminalFocused, deltaSeconds);
      this.animationManager.checkCameraBounds();
    }
    if (this.mouseManager) this.mouseManager.update();
    if (this.modelManager) this.modelManager.updateFadeIn();
    if (this.effectsManager) {
      this.effectsManager.updateBloom(screenMesh);
      this.effectsManager.render();
    }
    if (this.performanceManager) this.performanceManager.updateFrame();
  }

  dispose() {
    if (this.animationId) cancelAnimationFrame(this.animationId);
    if (this.renderer) this.renderer.dispose();
    if (this.eventManager) this.eventManager.dispose();
    if (this.audioManager) this.audioManager.dispose();
    if (this.effectsManager) this.effectsManager.dispose();
    if (this.terminalTexture) this.terminalTexture.dispose();
    if (this.shaderManager) this.shaderManager.dispose();
    if (this.modelManager) this.modelManager.dispose();
    if (this.mouseManager) this.mouseManager.dispose();
    if (this.performanceManager) this.performanceManager.dispose();
  }
}
