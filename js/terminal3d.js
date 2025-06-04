import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";
import { ModelManager } from "./model.js";
import { DebugManager } from "./debug.js";
import { EffectsManager } from "./effects.js";
import { CRTShader } from "./crt-shader.js";
import { AnimationManager } from "./animation.js";
import { Mouse } from "./mouse.js";
import { soundManager } from "./sound.js";

export class Terminal3D {
  constructor(options = {}) {
    this.debugEnabled = options.debug ?? false;
    this.skipIntro = options.skipIntro ?? false;

    this.performanceMode = this.detectPerformanceMode();
    this.frameRateTarget = this.performanceMode ? 30 : 60;
    this.lastFrameTime = 0;
    this.frameInterval = 1000 / this.frameRateTarget;

    const defaultValues = this.debugEnabled
      ? this.getDebugDefaults()
      : this.performanceMode
        ? this.getPerformanceDefaults()
        : this.getNormalDefaults();

    this.crtSettings = {
      bulge: options.bulge ?? defaultValues.bulge,
      scanlineIntensity:
        options.scanlineIntensity ?? defaultValues.scanlineIntensity,
      scanlineCount: options.scanlineCount ?? defaultValues.scanlineCount,
      vignetteIntensity:
        options.vignetteIntensity ?? defaultValues.vignetteIntensity,
      vignetteRadius: options.vignetteRadius ?? defaultValues.vignetteRadius,
      glowIntensity: options.glowIntensity ?? defaultValues.glowIntensity,
      glowColor: options.glowColor ?? new THREE.Vector3(0.0, 0.02, 0.02),
      brightness: options.brightness ?? defaultValues.brightness,
      contrast: options.contrast ?? defaultValues.contrast,
      offsetX: options.offsetX ?? defaultValues.offsetX,
      offsetY: options.offsetY ?? defaultValues.offsetY,
    };

    this.scenePosition = {
      x: options.sceneX ?? 0,
      y: options.sceneY ?? 0,
      z: options.sceneZ ?? 0,
    };

    this.scene = null;
    this.camera = null;
    this.renderer = null;
    this.controls = null;
    this.terminalTexture = null;
    this.terminalCanvas = null;
    this.hiddenInput = null;
    this.isTerminalFocused = false;
    this.animationId = null;
    this.isHoveringScreen = false;
    this.scrollHistory = [];
    this.maxScrollHistory = this.performanceMode ? 25 : 50;
    this.currentScrollPosition = 0;

    this.modelManager = null;
    this.debugManager = null;
    this.effectsManager = null;
    this.crtShader = null;
    this.animationManager = null;
    this.mouseDynamics = null;

    this.raycaster = null;
    this.mouse = null;

    this.defaultCameraPosition = new THREE.Vector3(4, 5, 10);
    this.defaultCameraTarget = new THREE.Vector3(0, 1.5, 0);

    this.performanceStats = {
      frameCount: 0,
      lastFPSUpdate: Date.now(),
      currentFPS: 60,
      adaptiveQuality: 1.0,
    };

    this.introSequenceComplete = false;
    this.hasEverBeenUnfocused = false;

    this.init();
  }

  detectPerformanceMode() {
    const canvas = document.createElement("canvas");
    const gl =
      canvas.getContext("webgl") || canvas.getContext("experimental-webgl");

    if (!gl) return true;

    const renderer = gl.getParameter(gl.RENDERER);

    const isMobile =
      /Android|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(
        navigator.userAgent,
      );
    const isLowEndGPU = /Intel|Mali|Adreno 3|Adreno 4|PowerVR SGX/i.test(
      renderer,
    );
    const memory = navigator.deviceMemory || 4;
    const cores = navigator.hardwareConcurrency || 4;
    const isLowMemory = memory < 4;
    const isLowCPU = cores < 4;

    const isLowRes = window.screen.width < 1920 || window.screen.height < 1080;

    canvas.remove();

    return isMobile || isLowEndGPU || isLowMemory || isLowCPU || isLowRes;
  }

  getDebugDefaults() {
    return {
      bulge: 0,
      scanlineIntensity: 0,
      scanlineCount: 0,
      vignetteIntensity: 0,
      vignetteRadius: 0,
      glowIntensity: 0,
      brightness: 1.0,
      contrast: 1.0,
      offsetX: 0,
      offsetY: 0,
    };
  }

  getPerformanceDefaults() {
    return {
      bulge: 0.6,
      scanlineIntensity: 0.05,
      scanlineCount: 400,
      vignetteIntensity: 0.2,
      vignetteRadius: 0.2,
      glowIntensity: 0.01,
      brightness: 1.0,
      contrast: 1.05,
      offsetX: 1.0,
      offsetY: 0.0,
    };
  }

  getNormalDefaults() {
    return {
      bulge: 0.9,
      scanlineIntensity: 0.1,
      scanlineCount: 800,
      vignetteIntensity: 0.3,
      vignetteRadius: 0.3,
      glowIntensity: 0.02,
      brightness: 1.0,
      contrast: 1.1,
      offsetX: 1.2,
      offsetY: 0.0,
    };
  }

  async init() {
    try {
      this.updateLoadingProgress(5);

      await soundManager.init();
      this.updateLoadingProgress(10);

      await this.setupScene();
      this.updateLoadingProgress(30);

      this.modelManager = new ModelManager(this.scene, (progress) =>
        this.updateLoadingProgress(progress),
      );
      this.effectsManager = new EffectsManager(
        this.renderer,
        this.scene,
        this.camera,
      );
      this.debugManager = new DebugManager(this);

      await this.modelManager.loadPCModel();
      this.updateLoadingProgress(60);

      this.mouseDynamics = new Mouse(
        this.camera,
        this.controls,
        this.modelManager,
        this.performanceMode,
      );

      await this.setupTerminalTexture();
      this.updateLoadingProgress(80);

      this.setupEventListeners();

      if (this.performanceMode) {
        console.log("performance mode enabled");
      } else {
        await this.effectsManager.trySetupPostProcessing();
      }

      this.animationManager = new AnimationManager(this.camera, this.controls);

      if (this.debugEnabled) {
        this.debugManager.createDebugGUI();
      }

      this.updateLoadingProgress(100);

      if (!this.skipIntro) {
        this.hideLoading();
        this.showTerminal();
        this.startIntroSequence();
      } else {
        const sceneContainer = document.getElementById("scene-container");
        if (sceneContainer) {
          sceneContainer.style.animationPlayState = "running";
          sceneContainer.style.visibility = "visible";
        }
        this.showTerminal();
        this.introSequenceComplete = true;

        if (this.mouseDynamics) {
          this.mouseDynamics.onIntroComplete();
        }
      }

      this.animate();
    } catch (e) {
      console.error("3D init failed:", e);
      this.showError();
    }
  }

  startIntroSequence() {
    window.dispatchEvent(
      new CustomEvent("terminalReady", {
        detail: { terminal3d: this },
      }),
    );

    setTimeout(() => {
      const focusEvent = new CustomEvent("terminalFocus");
      window.dispatchEvent(focusEvent);

      this.introSequenceComplete = true;

      if (this.mouseDynamics) {
        this.mouseDynamics.onIntroComplete();
      }
    }, 15000);

    soundManager.play("boot");
    soundManager.play("fan");

    if (this.animationManager) {
      this.animationManager.startupAnimation();
    }

    console.log("Intro sequence started!");
  }

  async setupScene() {
    this.scene = new THREE.Scene();
    this.scene.background = new THREE.Color(0x0a0a0a);

    this.camera = new THREE.PerspectiveCamera(
      45,
      window.innerWidth / window.innerHeight,
      0.1,
      100,
    );
    this.camera.position.copy(this.defaultCameraPosition);

    this.renderer = new THREE.WebGLRenderer({
      antialias: !this.performanceMode,
      powerPreference: this.performanceMode ? "default" : "high-performance",
      alpha: true,
      precision: this.performanceMode ? "mediump" : "highp",
    });

    this.renderer.setSize(window.innerWidth, window.innerHeight);

    const maxPixelRatio = this.performanceMode ? 1.5 : 2;
    this.renderer.setPixelRatio(
      Math.min(window.devicePixelRatio, maxPixelRatio),
    );

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

    this.controls = new OrbitControls(this.camera, this.renderer.domElement);
    this.controls.enableDamping = true;
    this.controls.dampingFactor = this.performanceMode ? 0.1 : 0.05;
    this.controls.enableZoom = false;
    this.controls.enablePan = true;
    this.controls.enableRotate = true;
    this.controls.target.copy(this.defaultCameraTarget);
    this.controls.minDistance = 3;
    this.controls.maxDistance = 20;
    this.controls.minPolarAngle = Math.PI * 0.05;
    this.controls.maxPolarAngle = Math.PI * 0.75;
    this.controls.minAzimuthAngle = -Math.PI * 0.75;
    this.controls.maxAzimuthAngle = Math.PI * 0.75;

    this.setupLighting();
  }

  setupLighting() {
    const ambientLight = new THREE.AmbientLight(
      0x404040,
      this.performanceMode ? 0.8 : 0.6,
    );
    this.scene.add(ambientLight);

    const mainLight = new THREE.DirectionalLight(
      0xffffff,
      this.performanceMode ? 1.0 : 1.2,
    );
    mainLight.position.set(8, 10, 6);

    if (!this.performanceMode) {
      mainLight.castShadow = true;
      mainLight.shadow.mapSize.setScalar(2048);
      mainLight.shadow.bias = -0.0001;
      mainLight.shadow.camera.near = 0.1;
      mainLight.shadow.camera.far = 50;
      mainLight.shadow.camera.left = mainLight.shadow.camera.bottom = -15;
      mainLight.shadow.camera.right = mainLight.shadow.camera.top = 15;
    }
    this.scene.add(mainLight);

    if (!this.performanceMode) {
      const fillLight = new THREE.DirectionalLight(0x87ceeb, 0.4);
      fillLight.position.set(-5, 6, -4);
      this.scene.add(fillLight);

      const rimLight = new THREE.DirectionalLight(0x50fa7b, 0.2);
      rimLight.position.set(0, 4, -8);
      this.scene.add(rimLight);

      const accentLight1 = new THREE.PointLight(0x87ceeb, 0.6, 12);
      accentLight1.position.set(4, 4, 4);
      this.scene.add(accentLight1);

      const accentLight2 = new THREE.PointLight(0x50fa7b, 0.4, 10);
      accentLight2.position.set(-4, 3, 2);
      this.scene.add(accentLight2);

      const frontLight = new THREE.DirectionalLight(0xf8f8ff, 0.6);
      frontLight.position.set(0, 8, 12);
      this.scene.add(frontLight);
    }

    const monitorLight = new THREE.PointLight(0x50fa7b, 0.8, 8);
    monitorLight.position.set(0, 3, 2);
    this.scene.add(monitorLight);

    const hemiLight = new THREE.HemisphereLight(0x87ceeb, 0x2c2c54, 0.5);
    hemiLight.position.set(0, 25, 0);
    this.scene.add(hemiLight);
  }

  async setupTerminalTexture() {
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

    this.crtShader = new CRTShader(this.terminalTexture, this.crtSettings);

    if (this.modelManager.screenMesh) {
      this.modelManager.screenMesh.material =
        this.crtShader.createCRTMaterial();
      this.crtShader.applyRealBulgeToScreen(this.modelManager.screenMesh);
    }

    this.terminalTexture.offset.y = 0.0;
    this.terminalTexture.repeat.y = 1.0;

    const updateInterval = this.performanceMode ? 33 : 16;

    this.updateTerminalTexture = () => {
      try {
        if (this.terminalTexture) {
          this.terminalTexture.needsUpdate = true;
        }
      } catch (e) {
        console.warn("Texture update failed:", e);
      }
    };

    setInterval(() => {
      this.updateTerminalTexture();
    }, updateInterval);
  }

  updateCRTSettings(newSettings) {
    if (this.crtShader) {
      this.crtShader.updateSettings(newSettings, this.modelManager.screenMesh);
    }

    if (this.debugManager) {
      this.debugManager.updateDebugCenter();
    }
  }

  updateScenePosition(axis, value) {
    this.scenePosition[axis] = value;

    if (this.modelManager && this.modelManager.pcModel) {
      if (axis === "x") {
        this.modelManager.pcModel.position.x = this.scenePosition.x;
      } else if (axis === "y") {
        this.modelManager.pcModel.position.y = this.scenePosition.y;
      } else if (axis === "z") {
        this.modelManager.pcModel.position.z = this.scenePosition.z;
      }
    }
  }

  toggleDebugPanel() {
    if (this.debugManager) {
      this.debugManager.toggleDebugPanel();
    }
  }

  setupEventListeners() {
    window.addEventListener("resize", () => {
      this.camera.aspect = window.innerWidth / window.innerHeight;
      this.camera.updateProjectionMatrix();
      this.renderer.setSize(window.innerWidth, window.innerHeight);
      this.effectsManager.onResize();
    });

    this.raycaster = new THREE.Raycaster();
    this.mouse = new THREE.Vector2();

    let mouseEventThrottle = false;
    const mouseThrottleDelay = this.performanceMode ? 50 : 16;

    this.renderer.domElement.addEventListener("mousemove", (event) => {
      if (mouseEventThrottle) return;
      mouseEventThrottle = true;

      setTimeout(() => {
        mouseEventThrottle = false;
      }, mouseThrottleDelay);

      this.animationManager.stopIdleRotation();

      this.mouse.x = (event.clientX / window.innerWidth) * 2 - 1;
      this.mouse.y = -(event.clientY / window.innerHeight) * 2 + 1;

      if (this.mouseDynamics) {
        this.mouseDynamics.updateMousePosition(
          this.mouse.x,
          this.mouse.y,
          this.isTerminalFocused,
        );
      }

      this.raycaster.setFromCamera(this.mouse, this.camera);

      const intersectables = [];
      if (this.modelManager.pcModel) {
        this.modelManager.pcModel.traverse((child) => {
          if (child.isMesh) {
            intersectables.push(child);
          }
        });
      }

      const intersects = this.raycaster.intersectObjects(intersectables);

      if (intersects.length > 0) {
        const hoveredObject = intersects[0].object;
        const isScreen = this.modelManager.isScreenObject(hoveredObject);

        if (isScreen) {
          this.renderer.domElement.style.cursor = "pointer";
          this.isHoveringScreen = true;
        } else {
          this.renderer.domElement.style.cursor = "grab";
          this.isHoveringScreen = false;
        }
      } else {
        this.renderer.domElement.style.cursor = "default";
        this.isHoveringScreen = false;
      }
    });

    this.renderer.domElement.addEventListener("click", (event) => {
      this.animationManager.stopIdleRotation();

      this.mouse.x = (event.clientX / window.innerWidth) * 2 - 1;
      this.mouse.y = -(event.clientY / window.innerHeight) * 2 + 1;

      this.raycaster.setFromCamera(this.mouse, this.camera);

      const intersectables = [];
      if (this.modelManager.pcModel) {
        this.modelManager.pcModel.traverse((child) => {
          if (child.isMesh) {
            intersectables.push(child);
          }
        });
      }

      const intersects = this.raycaster.intersectObjects(intersectables);

      if (intersects.length > 0) {
        const clickedObject = intersects[0].object;
        const isScreen = this.modelManager.isScreenObject(clickedObject);

        if (isScreen) {
          this.isTerminalFocused = true;
          this.animationManager.animateToState("focused");

          if (this.hiddenInput) {
            this.hiddenInput.focus();
          }

          if (this.mouseDynamics) {
            this.mouseDynamics.onTerminalFocused();
          }

          const focusEvent = new CustomEvent("terminalFocus");
          window.dispatchEvent(focusEvent);
        } else {
          const wasTerminalFocused = this.isTerminalFocused;
          this.isTerminalFocused = false;
          this.animationManager.animateToState("default");

          if (this.hiddenInput) {
            this.hiddenInput.blur();
          }

          if (wasTerminalFocused && this.introSequenceComplete) {
            this.hasEverBeenUnfocused = true;
          }

          if (this.mouseDynamics) {
            this.mouseDynamics.onTerminalBlurred(this.hasEverBeenUnfocused);
          }

          const blurEvent = new CustomEvent("terminalBlur");
          window.dispatchEvent(blurEvent);
        }
      }
    });

    this.renderer.domElement.addEventListener("wheel", (event) => {
      if (this.isHoveringScreen) {
        event.preventDefault();
        this.animationManager.stopIdleRotation();

        const currentSnapshot = this.captureTerminalSnapshot();
        if (
          this.scrollHistory.length === 0 ||
          this.scrollHistory[this.scrollHistory.length - 1] !== currentSnapshot
        ) {
          this.scrollHistory.push(currentSnapshot);
          if (this.scrollHistory.length > this.maxScrollHistory) {
            this.scrollHistory.shift();
          }
        }

        if (event.deltaY < 0) {
          this.scrollUp();
        } else {
          this.scrollDown();
        }
      }
    });

    window.addEventListener("terminalFocus", () => {
      this.isTerminalFocused = true;
      if (this.hiddenInput) {
        this.hiddenInput.focus();
      }
      if (this.mouseDynamics) {
        this.mouseDynamics.onTerminalFocused();
      }
    });

    window.addEventListener("terminalBlur", () => {
      const wasTerminalFocused = this.isTerminalFocused;
      this.isTerminalFocused = false;
      if (this.hiddenInput) {
        this.hiddenInput.blur();
      }

      if (wasTerminalFocused && this.introSequenceComplete) {
        this.hasEverBeenUnfocused = true;
      }

      if (this.mouseDynamics) {
        this.mouseDynamics.onTerminalBlurred(this.hasEverBeenUnfocused);
      }
    });

    if (this.performanceMode) {
      this.setupPerformanceMonitoring();
    }
  }

  setupPerformanceMonitoring() {
    setInterval(() => {
      this.monitorPerformance();
    }, 2000);
  }

  monitorPerformance() {
    const now = Date.now();
    const deltaTime = now - this.performanceStats.lastFPSUpdate;

    if (deltaTime >= 1000) {
      this.performanceStats.currentFPS =
        (this.performanceStats.frameCount * 1000) / deltaTime;
      this.performanceStats.frameCount = 0;
      this.performanceStats.lastFPSUpdate = now;

      if (this.performanceStats.currentFPS < 25) {
        this.performanceStats.adaptiveQuality = Math.max(
          0.5,
          this.performanceStats.adaptiveQuality - 0.1,
        );
        this.applyAdaptiveQuality();
      } else if (
        this.performanceStats.currentFPS > 40 &&
        this.performanceStats.adaptiveQuality < 1.0
      ) {
        this.performanceStats.adaptiveQuality = Math.min(
          1.0,
          this.performanceStats.adaptiveQuality + 0.05,
        );
        this.applyAdaptiveQuality();
      }
    }
  }

  applyAdaptiveQuality() {
    const quality = this.performanceStats.adaptiveQuality;

    const targetPixelRatio = Math.min(window.devicePixelRatio * quality, 1.5);
    this.renderer.setPixelRatio(targetPixelRatio);

    if (this.crtShader) {
      const adaptedSettings = {
        ...this.crtSettings,
        scanlineCount: Math.floor(this.crtSettings.scanlineCount * quality),
        scanlineIntensity: this.crtSettings.scanlineIntensity * quality,
      };
      this.crtShader.updateSettings(
        adaptedSettings,
        this.modelManager.screenMesh,
      );
    }
  }

  captureTerminalSnapshot() {
    if (!this.terminalCanvas) return null;

    try {
      return {
        timestamp: Date.now(),
      };
    } catch (e) {
      console.warn("Failed to capture terminal snapshot:", e);
      return null;
    }
  }

  scrollUp() {
    if (this.scrollHistory.length === 0) return;

    if (this.currentScrollPosition < this.scrollHistory.length - 1) {
      this.currentScrollPosition++;

      const scrollEvent = new CustomEvent("terminalScrollUp", {
        detail: { position: this.currentScrollPosition },
      });
      window.dispatchEvent(scrollEvent);
    }
  }

  scrollDown() {
    if (this.currentScrollPosition > 0) {
      this.currentScrollPosition--;

      const scrollEvent = new CustomEvent("terminalScrollDown", {
        detail: { position: this.currentScrollPosition },
      });
      window.dispatchEvent(scrollEvent);
    } else {
      this.currentScrollPosition = 0;
      const scrollEvent = new CustomEvent("terminalScrollToBottom");
      window.dispatchEvent(scrollEvent);
    }
  }

  updateLoadingProgress(p) {
    const bar = document.getElementById("loading-progress");
    if (bar) bar.style.width = p + "%";
  }

  hideLoading() {
    setTimeout(() => {
      const L = document.getElementById("loading");
      if (L) L.classList.add("hidden");
    }, 500);
  }

  showError() {
    const txt = document.querySelector(".loading-text");
    if (txt) {
      txt.textContent = "Failed to load 3D. Showing terminal.";
      txt.style.color = "#ff5555";
    }
    setTimeout(() => {
      const L = document.getElementById("loading");
      if (L) L.classList.add("hidden");
      const term = document.getElementById("terminal");
      if (term) term.style.visibility = "visible";
    }, 2000);
  }

  showTerminal() {
    const terminal = document.getElementById("terminal");
    if (terminal) {
      terminal.style.visibility = "visible";
    }
  }

  animate() {
    const now = Date.now();

    if (now - this.lastFrameTime < this.frameInterval) {
      this.animationId = requestAnimationFrame(() => this.animate());
      return;
    }

    this.lastFrameTime = now;
    this.performanceStats.frameCount++;
    this.animationId = requestAnimationFrame(() => this.animate());

    if (this.controls) {
      this.controls.update();
    }

    if (this.crtShader) {
      this.crtShader.updateTime(now * 0.001, this.modelManager.screenMesh);
    }

    if (this.animationManager) {
      this.animationManager.updateIdleRotation(this.isTerminalFocused);
      this.animationManager.checkAndCorrectCameraBounds();
    }

    if (this.mouseDynamics) {
      this.mouseDynamics.update();
    }

    if (this.modelManager) {
      this.modelManager.updateSceneFadeIn();
    }

    if (this.effectsManager) {
      this.effectsManager.updateManualBloom(this.modelManager.screenMesh);
      this.effectsManager.render();
    }
  }

  dispose() {
    if (this.animationId) cancelAnimationFrame(this.animationId);
    if (this.renderer) this.renderer.dispose();
    if (this.effectsManager) this.effectsManager.dispose();
    if (this.terminalTexture) this.terminalTexture.dispose();
    if (this.crtShader) this.crtShader.dispose();
    if (this.debugManager) this.debugManager.dispose();
    if (this.modelManager) this.modelManager.dispose();
    if (this.mouseDynamics) this.mouseDynamics.dispose();
  }
}
