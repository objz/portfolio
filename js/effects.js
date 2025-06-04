import * as THREE from "three";

export class EffectsManager {
  constructor(renderer, scene, camera) {
    this.renderer = renderer;
    this.scene = scene;
    this.camera = camera;
    this.composer = null;
    this.supportsPostProcessing = false;
    this.bloomPulse = 0;
    this.bloomDirection = 1;
  }

  async trySetupPostProcessing() {
    try {
      const [
        { EffectComposer },
        { RenderPass },
        { UnrealBloomPass },
        { OutputPass },
      ] = await Promise.all([
        import(
          "https://unpkg.com/three@0.160.0/examples/jsm/postprocessing/EffectComposer.js"
        ),
        import(
          "https://unpkg.com/three@0.160.0/examples/jsm/postprocessing/RenderPass.js"
        ),
        import(
          "https://unpkg.com/three@0.160.0/examples/jsm/postprocessing/UnrealBloomPass.js"
        ),
        import(
          "https://unpkg.com/three@0.160.0/examples/jsm/postprocessing/OutputPass.js"
        ),
      ]);

      this.setupPostProcessing(
        EffectComposer,
        RenderPass,
        UnrealBloomPass,
        OutputPass,
      );
      this.supportsPostProcessing = true;
    } catch (error) {
      console.warn("Post-processing not available:", error);
      this.supportsPostProcessing = false;
      this.setupManualBloom();
    }
  }

  setupPostProcessing(EffectComposer, RenderPass, UnrealBloomPass, OutputPass) {
    this.composer = new EffectComposer(this.renderer);

    const renderPass = new RenderPass(this.scene, this.camera);
    this.composer.addPass(renderPass);

    const bloomPass = new UnrealBloomPass(
      new THREE.Vector2(window.innerWidth, window.innerHeight),
      0.6,
      0.8,
      0.7,
    );
    this.composer.addPass(bloomPass);

    const outputPass = new OutputPass();
    this.composer.addPass(outputPass);
  }

  setupManualBloom() {
    this.bloomPulse = 0;
    this.bloomDirection = 1;
  }

  updateManualBloom(screenMesh) {
    if (!this.supportsPostProcessing && screenMesh && screenMesh.material) {
      this.bloomPulse += 0.02 * this.bloomDirection;

      if (this.bloomPulse > 1) {
        this.bloomPulse = 1;
        this.bloomDirection = -1;
      } else if (this.bloomPulse < 0.5) {
        this.bloomPulse = 0.5;
        this.bloomDirection = 1;
      }

      screenMesh.material.emissiveIntensity = 0.7 + this.bloomPulse * 0.5;
    }
  }

  render() {
    if (this.composer && this.supportsPostProcessing) {
      this.composer.render();
    } else {
      this.renderer.render(this.scene, this.camera);
    }
  }

  dispose() {
    if (this.composer) this.composer.dispose();
  }

  onResize() {
    if (this.composer) {
      this.composer.setSize(window.innerWidth, window.innerHeight);
    }
  }
}
