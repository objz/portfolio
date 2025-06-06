export class PerformanceManager {
  constructor(sceneManager) {
    this.sceneManager = sceneManager;
    this.stats = {
      frameCount: 0,
      lastFpsUpdate: Date.now(),
      currentFps: 60,
      adaptiveQuality: 1.0,
      frameTime: 0,
      lastFrameTime: 0,
    };

    this.targetFps = 30;
    this.qualityLevels = [1.0, 0.8, 0.6, 0.4];
    this.currentQualityIndex = 0;
    this.fpsHistory = [];
    this.maxHistory = 60;
    this.checkInterval = 2000;
    this.lastCheck = Date.now();
  }

  start() {
    this.monitor();
  }

  monitor() {
    setInterval(() => {
      this.checkPerformance();
    }, this.checkInterval);
  }

  updateFrame() {
    const now = Date.now();
    this.stats.frameCount++;
    this.stats.frameTime = now - this.stats.lastFrameTime;
    this.stats.lastFrameTime = now;

    if (now - this.stats.lastFpsUpdate >= 1000) {
      this.stats.currentFps = this.stats.frameCount;
      this.stats.frameCount = 0;
      this.stats.lastFpsUpdate = now;

      this.fpsHistory.push(this.stats.currentFps);
      if (this.fpsHistory.length > this.maxHistory) {
        this.fpsHistory.shift();
      }
    }
  }

  checkPerformance() {
    if (this.fpsHistory.length < 10) return;

    const avgFps =
      this.fpsHistory.reduce((a, b) => a + b, 0) / this.fpsHistory.length;
    const minFps = Math.min(...this.fpsHistory.slice(-10));

    if (avgFps < this.targetFps * 0.8 || minFps < this.targetFps * 0.6) {
      this.degradeQuality();
    } else if (avgFps > this.targetFps * 1.1 && minFps > this.targetFps * 0.9) {
      this.improveQuality();
    }
  }

  degradeQuality() {
    if (this.currentQualityIndex < this.qualityLevels.length - 1) {
      this.currentQualityIndex++;
      this.stats.adaptiveQuality = this.qualityLevels[this.currentQualityIndex];
      this.applyQualitySettings();
      console.log(
        `Performance: Quality reduced to ${this.stats.adaptiveQuality}`,
      );
    }
  }

  improveQuality() {
    if (this.currentQualityIndex > 0) {
      this.currentQualityIndex--;
      this.stats.adaptiveQuality = this.qualityLevels[this.currentQualityIndex];
      this.applyQualitySettings();
      console.log(
        `Performance: Quality improved to ${this.stats.adaptiveQuality}`,
      );
    }
  }

  applyQualitySettings() {
    const quality = this.stats.adaptiveQuality;

    if (this.sceneManager.renderer) {
      const maxRatio = quality === 1.0 ? 2 : quality > 0.6 ? 1.5 : 1;
      this.sceneManager.renderer.setPixelRatio(
        Math.min(window.devicePixelRatio, maxRatio),
      );
    }

    if (
      this.sceneManager.shaderManager &&
      this.sceneManager.modelManager.screenMesh
    ) {
      const settings = {
        scanlineCount: Math.floor(800 * quality),
        scanlineIntensity: 0.1 * quality,
        glowIntensity: 0.02 * quality,
      };
      this.sceneManager.shaderManager.updateSettings(
        settings,
        this.sceneManager.modelManager.screenMesh,
      );
    }

    this.sceneManager.frameTarget = quality > 0.6 ? 60 : 30;
    this.sceneManager.frameInterval = 1000 / this.sceneManager.frameTarget;
  }

  dispose() {
    this.fpsHistory = [];
    this.stats = null;
  }
}
