import * as THREE from "three";

export class DebugManager {
  constructor(terminal3d) {
    this.terminal3d = terminal3d;
    this.debugPanel = null;
    this.debugCenter = null;
    this.debugEnabled = terminal3d.debugEnabled;
    this.isVisible = false;
  }

  showDebug() {
    if (!this.debugPanel) {
      this.createDebugGUI();
    }
    this.debugPanel.style.display = "block";
    this.isVisible = true;
    console.log("Debug panel shown");
  }

  hideDebug() {
    if (this.debugPanel) {
      this.debugPanel.style.display = "none";
      this.isVisible = false;
      console.log("Debug panel hidden");
    }
  }

  toggleDebug() {
    if (this.isVisible) {
      this.hideDebug();
    } else {
      this.showDebug();
    }
  }

  createDebugGUI() {
    this.debugPanel = document.createElement("div");
    this.debugPanel.id = "debug-panel";
    this.debugPanel.style.cssText = `
      position: fixed;
      top: 20px;
      right: 20px;
      width: 340px;
      background: rgba(0, 0, 0, 0.95);
      border: 1px solid #333;
      border-radius: 8px;
      padding: 15px;
      font-family: 'Courier New', monospace;
      font-size: 12px;
      color: #50fa7b;
      z-index: 1000;
      box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5);
      backdrop-filter: blur(10px);
      max-height: 85vh;
      overflow-y: auto;
    `;

    const titleContainer = document.createElement("div");
    titleContainer.style.cssText = `
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 15px;
      padding-bottom: 8px;
      border-bottom: 1px solid #333;
    `;

    const title = document.createElement("div");
    title.textContent = "CRT Debug Controls";
    title.style.cssText = `
      font-size: 14px;
      font-weight: bold;
      color: #8be9fd;
    `;

    const closeButton = document.createElement("button");
    closeButton.textContent = "×";
    closeButton.style.cssText = `
      background: rgba(255, 85, 85, 0.2);
      border: 1px solid #ff5555;
      border-radius: 4px;
      color: #ff5555;
      width: 25px;
      height: 25px;
      cursor: pointer;
      font-size: 16px;
      font-weight: bold;
      display: flex;
      align-items: center;
      justify-content: center;
      transition: all 0.2s ease;
    `;

    closeButton.addEventListener("mouseenter", () => {
      closeButton.style.background = "rgba(255, 85, 85, 0.3)";
    });

    closeButton.addEventListener("mouseleave", () => {
      closeButton.style.background = "rgba(255, 85, 85, 0.2)";
    });

    closeButton.addEventListener("click", () => this.hideDebug());

    titleContainer.appendChild(title);
    titleContainer.appendChild(closeButton);
    this.debugPanel.appendChild(titleContainer);

    this.createSection("Scene Position");
    this.createSlider("sceneX", "Scene X", -10, 10, 0.1);
    this.createSlider("sceneY", "Scene Y", -10, 10, 0.1);
    this.createSlider("sceneZ", "Scene Z", -10, 10, 0.1);

    this.createSection("CRT Effects");
    this.createSlider("bulge", "Bulge", 0, 3, 0.01);
    this.createSlider("offsetX", "Offset X", -2, 2, 0.01);
    this.createSlider("offsetY", "Offset Y", -2, 2, 0.01);
    this.createSlider("scanlineIntensity", "Scanlines", 0, 0.3, 0.001);
    this.createSlider("scanlineCount", "Scanline Count", 100, 2000, 10);
    this.createSlider("vignetteIntensity", "Vignette", 0, 1, 0.01);
    this.createSlider("vignetteRadius", "Vignette Radius", 0, 1, 0.01);
    this.createSlider("glowIntensity", "Glow", 0, 0.1, 0.001);
    this.createSlider("brightness", "Brightness", 0.1, 3, 0.01);
    this.createSlider("contrast", "Contrast", 0.1, 3, 0.01);

    this.createSection("Lighting");
    this.createSlider("ambientIntensity", "Ambient Light", 0, 2, 0.01);
    this.createSlider("mainLightIntensity", "Main Light", 0, 5, 0.01);
    this.createSlider("fillLightIntensity", "Fill Light", 0, 3, 0.01);
    this.createSlider("monitorLightIntensity", "Monitor Light", 0, 5, 0.01);
    this.createSlider("rimLightIntensity", "Rim Light", 0, 2, 0.01);
    this.createSlider("accentLight1Intensity", "Accent Light 1", 0, 3, 0.01);
    this.createSlider("accentLight2Intensity", "Accent Light 2", 0, 3, 0.01);
    this.createSlider("hemiLightIntensity", "Hemisphere Light", 0, 2, 0.01);
    this.createSlider("frontLightIntensity", "Front Light", 0, 3, 0.01);

    this.createSection("Camera & Effects");
    this.createSlider("fov", "Field of View", 20, 120, 1);
    this.createSlider("toneMappingExposure", "Exposure", 0.1, 3, 0.01);
    this.createSlider("shadowMapSize", "Shadow Quality", 512, 4096, 512);

    this.createSection("Debug Tools");
    this.createButton(
      "Show Center Point",
      () => this.toggleDebugCenter(),
      "Shows/hides a red dot at the screen center for bulge calibration",
    );
    this.createButton("Reset Effects", () => this.resetEffectsToZero());
    this.createButton("Reset Lighting", () => this.resetLightingToDefaults());
    this.createButton("Reset All", () => this.resetToDefaults());
    this.createButton("Copy Values", () => this.copyCurrentValues());

    document.body.appendChild(this.debugPanel);
    this.isVisible = true;
    console.log("Debug GUI created.");
  }

  createSection(title) {
    const section = document.createElement("div");
    section.style.cssText = `
      margin: 20px 0 10px 0;
      padding-top: 15px;
      border-top: 1px solid #333;
      font-weight: bold;
      color: #8be9fd;
      font-size: 13px;
    `;
    section.textContent = title;
    this.debugPanel.appendChild(section);
  }

  createSlider(property, label, min, max, step) {
    const container = document.createElement("div");
    container.style.cssText = `
      margin-bottom: 12px;
      padding: 8px;
      background: rgba(255, 255, 255, 0.05);
      border-radius: 4px;
    `;

    const labelDiv = document.createElement("div");
    labelDiv.style.cssText = `
      display: flex;
      justify-content: space-between;
      margin-bottom: 5px;
      font-size: 11px;
    `;

    const labelText = document.createElement("span");
    labelText.textContent = label;
    labelText.style.color = "#f8f8f2";

    const valueDisplay = document.createElement("span");
    const currentValue = this.getCurrentValue(property);
    valueDisplay.textContent = currentValue.toFixed(3);
    valueDisplay.style.color = "#50fa7b";
    valueDisplay.style.fontWeight = "bold";

    labelDiv.appendChild(labelText);
    labelDiv.appendChild(valueDisplay);

    const slider = document.createElement("input");
    slider.type = "range";
    slider.min = min;
    slider.max = max;
    slider.step = step;
    slider.value = currentValue;
    slider.style.cssText = `
      width: 100%;
      height: 4px;
      border-radius: 2px;
      background: #44475a;
      outline: none;
      -webkit-appearance: none;
    `;

    if (!document.getElementById("slider-styles")) {
      const style = document.createElement("style");
      style.id = "slider-styles";
      style.textContent = `
        #debug-panel input[type="range"]::-webkit-slider-thumb {
          -webkit-appearance: none;
          appearance: none;
          width: 12px;
          height: 12px;
          border-radius: 50%;
          background: #50fa7b;
          cursor: pointer;
          box-shadow: 0 0 10px rgba(80, 250, 123, 0.5);
        }
        #debug-panel input[type="range"]::-moz-range-thumb {
          width: 12px;
          height: 12px;
          border-radius: 50%;
          background: #50fa7b;
          cursor: pointer;
          border: none;
          box-shadow: 0 0 10px rgba(80, 250, 123, 0.5);
        }
      `;
      document.head.appendChild(style);
    }

    slider.addEventListener("input", (e) => {
      const value = parseFloat(e.target.value);
      valueDisplay.textContent = value.toFixed(3);
      this.updateProperty(property, value);
    });

    container.appendChild(labelDiv);
    container.appendChild(slider);
    this.debugPanel.appendChild(container);
  }

  getCurrentValue(property) {
    if (property.startsWith("scene")) {
      const axis = property.replace("scene", "").toLowerCase();
      return this.terminal3d.scenePosition[axis] || 0;
    } else if (
      property.endsWith("Intensity") ||
      property === "fov" ||
      property === "toneMappingExposure" ||
      property === "shadowMapSize"
    ) {
      return this.getLightingValue(property);
    } else {
      return this.terminal3d.crtSettings[property] || 0;
    }
  }
  getLightingValue(property) {
    const scene = this.terminal3d.scene;
    if (!scene) return 1.0;

    const lightMap = {
      ambientIntensity: () =>
        scene.children.find((l) => l.type === "AmbientLight")?.intensity || 0.6,
      mainLightIntensity: () =>
        scene.children.find(
          (l) => l.type === "DirectionalLight" && l.position.x > 7,
        )?.intensity || 1.2,
      fillLightIntensity: () =>
        scene.children.find(
          (l) => l.type === "DirectionalLight" && l.position.x < 0,
        )?.intensity || 0.4,
      monitorLightIntensity: () =>
        scene.children.find(
          (l) => l.type === "PointLight" && l.position.z === 2,
        )?.intensity || 0.8,
      rimLightIntensity: () =>
        scene.children.find(
          (l) => l.type === "DirectionalLight" && l.position.z < 0,
        )?.intensity || 0.2,
      accentLight1Intensity: () =>
        scene.children.find(
          (l) => l.type === "PointLight" && l.position.x === 4,
        )?.intensity || 0.6,
      accentLight2Intensity: () =>
        scene.children.find(
          (l) => l.type === "PointLight" && l.position.x === -4,
        )?.intensity || 0.4,
      hemiLightIntensity: () =>
        scene.children.find((l) => l.type === "HemisphereLight")?.intensity ||
        0.5,
      frontLightIntensity: () =>
        scene.children.find(
          (l) => l.type === "DirectionalLight" && l.position.z > 10,
        )?.intensity || 0.6,
      fov: () => this.terminal3d.camera?.fov || 45,
      toneMappingExposure: () =>
        this.terminal3d.renderer?.toneMappingExposure || 1.0,
      shadowMapSize: () => 2048,
    };

    return lightMap[property] ? lightMap[property]() : 1.0;
  }

  updateProperty(property, value) {
    if (property.startsWith("scene")) {
      const axis = property.replace("scene", "").toLowerCase();
      this.terminal3d.updateScenePosition(axis, value);
    } else if (property.endsWith("Intensity")) {
      this.updateLightIntensity(property, value);
    } else if (property === "fov") {
      if (this.terminal3d.camera) {
        this.terminal3d.camera.fov = value;
        this.terminal3d.camera.updateProjectionMatrix();
      }
    } else if (property === "toneMappingExposure") {
      if (this.terminal3d.renderer) {
        this.terminal3d.renderer.toneMappingExposure = value;
      }
    } else if (property === "shadowMapSize") {
      this.updateShadowMapSize(value);
    } else {
      const newSettings = {};
      newSettings[property] = value;
      this.terminal3d.updateCRTSettings(newSettings);
    }
  }

  updateLightIntensity(property, intensity) {
    const scene = this.terminal3d.scene;
    if (!scene) return;

    const lightMap = {
      ambientIntensity: () =>
        scene.children.find((l) => l.type === "AmbientLight"),
      mainLightIntensity: () =>
        scene.children.find(
          (l) => l.type === "DirectionalLight" && l.position.x > 7,
        ),
      fillLightIntensity: () =>
        scene.children.find(
          (l) => l.type === "DirectionalLight" && l.position.x < 0,
        ),
      monitorLightIntensity: () =>
        scene.children.find(
          (l) => l.type === "PointLight" && l.position.z === 2,
        ),
      rimLightIntensity: () =>
        scene.children.find(
          (l) => l.type === "DirectionalLight" && l.position.z < 0,
        ),
      accentLight1Intensity: () =>
        scene.children.find(
          (l) => l.type === "PointLight" && l.position.x === 4,
        ),
      accentLight2Intensity: () =>
        scene.children.find(
          (l) => l.type === "PointLight" && l.position.x === -4,
        ),
      hemiLightIntensity: () =>
        scene.children.find((l) => l.type === "HemisphereLight"),
      frontLightIntensity: () =>
        scene.children.find(
          (l) => l.type === "DirectionalLight" && l.position.z > 10,
        ),
    };

    const light = lightMap[property]?.();
    if (light) {
      light.intensity = intensity;
    }
  }

  updateShadowMapSize(size) {
    const scene = this.terminal3d.scene;
    if (!scene) return;

    scene.children.forEach((child) => {
      if (child.type === "DirectionalLight" && child.castShadow) {
        child.shadow.mapSize.setScalar(size);
        child.shadow.map?.dispose();
        child.shadow.map = null;
      }
    });
  }

  createButton(text, onClick, tooltip = "") {
    const button = document.createElement("button");
    button.textContent = text;
    button.title = tooltip;
    button.style.cssText = `
      width: 100%;
      padding: 8px;
      margin: 5px 0;
      background: rgba(80, 250, 123, 0.2);
      border: 1px solid #50fa7b;
      border-radius: 4px;
      color: #50fa7b;
      font-family: 'Courier New', monospace;
      font-size: 11px;
      cursor: pointer;
      transition: all 0.2s ease;
    `;

    button.addEventListener("mouseenter", () => {
      button.style.background = "rgba(80, 250, 123, 0.3)";
      button.style.boxShadow = "0 0 10px rgba(80, 250, 123, 0.3)";
    });

    button.addEventListener("mouseleave", () => {
      button.style.background = "rgba(80, 250, 123, 0.2)";
      button.style.boxShadow = "none";
    });

    button.addEventListener("click", onClick);
    this.debugPanel.appendChild(button);
  }

  toggleDebugPanel() {
    this.toggleDebug();
  }

  resetEffectsToZero() {
    const zeroEffects = {
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

    this.terminal3d.updateCRTSettings(zeroEffects);
    this.updateSliders(zeroEffects);
    console.log("All effects reset to zero (except brightness/contrast)");
  }

  resetLightingToDefaults() {
    const defaultLighting = {
      ambientIntensity: 0.6,
      mainLightIntensity: 1.2,
      fillLightIntensity: 0.4,
      monitorLightIntensity: 0.8,
      rimLightIntensity: 0.2,
      accentLight1Intensity: 0.6,
      accentLight2Intensity: 0.4,
      hemiLightIntensity: 0.5,
      frontLightIntensity: 0.6,
    };

    Object.entries(defaultLighting).forEach(([property, value]) => {
      this.updateProperty(property, value);
    });

    this.updateSliders(defaultLighting);
    console.log("Lighting reset to realistic defaults");
  }

  resetToDefaults() {
    const defaults = {
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

    this.terminal3d.updateCRTSettings(defaults);
    this.terminal3d.updateScenePosition("x", 0);
    this.terminal3d.updateScenePosition("y", 0);
    this.terminal3d.updateScenePosition("z", 0);
    this.resetLightingToDefaults();

    this.updateSliders(defaults);
    this.updateSceneSliders();
    console.log("Reset to defaults");
  }

  updateSliders(settings) {
    if (!this.debugPanel) return;

    const sliders = this.debugPanel.querySelectorAll('input[type="range"]');

    Object.entries(settings).forEach(([key, value]) => {
      const slider = Array.from(sliders).find((s) => {
        const container = s.closest("div");
        const label = container.querySelector("span");
        return label && this.getPropertyFromLabel(label.textContent) === key;
      });

      if (slider) {
        slider.value = value;
        const valueDisplay = slider
          .closest("div")
          .querySelector('span[style*="color: #50fa7b"]');
        if (valueDisplay) {
          valueDisplay.textContent = value.toFixed(3);
        }
      }
    });
  }

  updateSceneSliders() {
    if (!this.debugPanel) return;

    const sceneX = this.terminal3d.scenePosition.x || 0;
    const sceneY = this.terminal3d.scenePosition.y || 0;
    const sceneZ = this.terminal3d.scenePosition.z || 0;

    this.updateSliders({ sceneX, sceneY, sceneZ });
  }

  getPropertyFromLabel(labelText) {
    const labelMap = {
      Bulge: "bulge",
      "Offset X": "offsetX",
      "Offset Y": "offsetY",
      Scanlines: "scanlineIntensity",
      "Scanline Count": "scanlineCount",
      Vignette: "vignetteIntensity",
      "Vignette Radius": "vignetteRadius",
      Glow: "glowIntensity",
      Brightness: "brightness",
      Contrast: "contrast",
      "Scene X": "sceneX",
      "Scene Y": "sceneY",
      "Scene Z": "sceneZ",
      "Ambient Light": "ambientIntensity",
      "Main Light": "mainLightIntensity",
      "Fill Light": "fillLightIntensity",
      "Monitor Light": "monitorLightIntensity",
      "Rim Light": "rimLightIntensity",
      "Accent Light 1": "accentLight1Intensity",
      "Accent Light 2": "accentLight2Intensity",
      "Hemisphere Light": "hemiLightIntensity",
      "Front Light": "frontLightIntensity",
      "Field of View": "fov",
      Exposure: "toneMappingExposure",
      "Shadow Quality": "shadowMapSize",
    };
    return labelMap[labelText];
  }

  copyCurrentValues() {
    const values = { ...this.terminal3d.crtSettings };
    values.sceneX = this.terminal3d.scenePosition.x || 0;
    values.sceneY = this.terminal3d.scenePosition.y || 0;
    values.sceneZ = this.terminal3d.scenePosition.z || 0;

    const code = `new Terminal3D(${JSON.stringify(values, null, 2)});`;

    navigator.clipboard.writeText(code).then(() => {
      console.log("Current values copied to clipboard:");
      console.log(code);

      this.showNotification("Values copied to clipboard!");
    });
  }

  showNotification(message) {
    const notification = document.createElement("div");
    notification.textContent = message;
    notification.style.cssText = `
      position: fixed;
      top: 50%;
      left: 50%;
      transform: translate(-50%, -50%);
      background: rgba(80, 250, 123, 0.9);
      color: #000;
      padding: 15px 25px;
      border-radius: 8px;
      font-family: 'Courier New', monospace;
      font-weight: bold;
      z-index: 10000;
      box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5);
    `;

    document.body.appendChild(notification);
    setTimeout(() => {
      if (document.body.contains(notification)) {
        document.body.removeChild(notification);
      }
    }, 2000);
  }

  toggleDebugCenter() {
    if (this.debugCenter) {
      this.terminal3d.scene.remove(this.debugCenter);
      this.debugCenter = null;
      console.log("Debug center hidden");
    } else {
      this.addDebugCenter();
      console.log("Debug center shown (red dot)");
    }
  }

  addDebugCenter() {
    if (this.debugCenter) {
      this.terminal3d.scene.remove(this.debugCenter);
    }

    if (!this.terminal3d.modelManager.screenMesh) return;

    const screenMesh = this.terminal3d.modelManager.screenMesh;
    screenMesh.geometry.computeBoundingBox();
    const bbox = screenMesh.geometry.boundingBox;

    const centerX =
      (bbox.max.x + bbox.min.x) / 2 + this.terminal3d.crtSettings.offsetX;
    const centerY =
      (bbox.max.y + bbox.min.y) / 2 + this.terminal3d.crtSettings.offsetY;
    const centerZ = bbox.max.z + 0.01;

    const geometry = new THREE.SphereGeometry(0.02, 8, 8);
    const material = new THREE.MeshBasicMaterial({ color: 0xff0000 });
    this.debugCenter = new THREE.Mesh(geometry, material);

    const worldPos = new THREE.Vector3(centerX, centerY, centerZ);
    screenMesh.localToWorld(worldPos);
    this.debugCenter.position.copy(worldPos);

    this.terminal3d.scene.add(this.debugCenter);
  }

  updateDebugCenter() {
    if (this.debugCenter) {
      this.addDebugCenter();
    }
  }

  dispose() {
    if (this.debugCenter) {
      this.terminal3d.scene.remove(this.debugCenter);
    }
    if (this.debugPanel && document.body.contains(this.debugPanel)) {
      document.body.removeChild(this.debugPanel);
    }
  }
}
