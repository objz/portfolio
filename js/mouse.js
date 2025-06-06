export class MouseManager {
  constructor(camera, controls, modelManager, performanceMode) {
    this.camera = camera;
    this.controls = controls;
    this.modelManager = modelManager;
    this.performanceMode = performanceMode;

    this.mousePosition = { x: 0, y: 0 };
    this.lastPosition = { x: 0, y: 0 };
    this.mouseVelocity = { x: 0, y: 0 };

    this.reactionIntensity = this.performanceMode ? 0.15 : 0.25;
    this.dampening = this.performanceMode ? 0.88 : 0.92;
    this.maxMovement = this.performanceMode ? 0.1 : 0.15;
    this.maxFloat = this.performanceMode ? 0.05 : 0.08;

    this.active = false;
    this.ready = false;
    this.currentPosition = { x: 0, y: 0, z: 0 };
    this.targetPosition = { x: 0, y: 0, z: 0 };

    this.originalPosition = null;

    this.lastUpdateTime = 0;
    this.updateInterval = this.performanceMode ? 50 : 16;

    this.introComplete = false;
    this.canActivate = false;

    this.init();
  }

  init() {
    setTimeout(() => {
      this.checkReadiness();
    }, 100);
  }

  checkReadiness() {
    if (this.modelManager.pcModel && !this.originalPosition) {
      this.originalPosition = {
        x: this.modelManager.pcModel.position.x,
        y: this.modelManager.pcModel.position.y,
        z: this.modelManager.pcModel.position.z,
      };

      this.currentPosition = { ...this.originalPosition };
      this.targetPosition = { ...this.originalPosition };
      this.ready = true;
    } else if (!this.originalPosition) {
      setTimeout(() => {
        this.checkReadiness();
      }, 500);
    }
  }

  updatePosition(normalizedX, normalizedY, terminalFocused) {
    if (!this.ready || !this.introComplete || !this.canActivate) {
      return;
    }

    const now = Date.now();

    if (now - this.lastUpdateTime < this.updateInterval) {
      return;
    }
    this.lastUpdateTime = now;

    this.lastPosition.x = this.mousePosition.x;
    this.lastPosition.y = this.mousePosition.y;

    this.mousePosition.x = normalizedX;
    this.mousePosition.y = normalizedY;

    this.mouseVelocity.x = (this.mousePosition.x - this.lastPosition.x) * 5;
    this.mouseVelocity.y = (this.mousePosition.y - this.lastPosition.y) * 5;

    this.active = !terminalFocused;

    if (this.active) {
      this.calculateMovement();
    } else {
      this.returnToRest();
    }
  }

  calculateMovement() {
    if (!this.modelManager.pcModel || !this.originalPosition) return;

    const mouseInfluenceX = this.mousePosition.x * this.reactionIntensity;
    const mouseInfluenceY = this.mousePosition.y * this.reactionIntensity;

    const velocityInfluenceX = Math.max(
      -0.05,
      Math.min(0.05, this.mouseVelocity.x * 0.2),
    );
    const velocityInfluenceY = Math.max(
      -0.05,
      Math.min(0.05, this.mouseVelocity.y * 0.2),
    );

    this.targetPosition.x =
      this.originalPosition.x +
      Math.max(
        -this.maxMovement,
        Math.min(this.maxMovement, mouseInfluenceX + velocityInfluenceX),
      );

    this.targetPosition.y =
      this.originalPosition.y +
      Math.max(
        -this.maxFloat,
        Math.min(this.maxFloat, mouseInfluenceY + velocityInfluenceY),
      );

    const depthInfluence = this.mousePosition.x * this.mousePosition.y * 0.1;
    this.targetPosition.z =
      this.originalPosition.z +
      Math.max(
        -this.maxFloat * 0.5,
        Math.min(this.maxFloat * 0.5, depthInfluence),
      );
  }

  returnToRest() {
    if (!this.originalPosition) return;

    this.targetPosition.x = this.originalPosition.x;
    this.targetPosition.y = this.originalPosition.y;
    this.targetPosition.z = this.originalPosition.z;
  }

  update() {
    if (!this.modelManager.pcModel || !this.ready) return;

    this.currentPosition.x +=
      (this.targetPosition.x - this.currentPosition.x) * (1 - this.dampening);
    this.currentPosition.y +=
      (this.targetPosition.y - this.currentPosition.y) * (1 - this.dampening);
    this.currentPosition.z +=
      (this.targetPosition.z - this.currentPosition.z) * (1 - this.dampening);

    this.modelManager.pcModel.position.x = this.currentPosition.x;
    this.modelManager.pcModel.position.y = this.currentPosition.y;
    this.modelManager.pcModel.position.z = this.currentPosition.z;
  }

  setEnabled(enabled) {
    this.active = enabled;
    if (!enabled) {
      this.returnToRest();
    }
  }

  setIntensity(intensity) {
    this.reactionIntensity = Math.max(0, Math.min(1, intensity));
  }

  onIntroComplete() {
    this.introComplete = true;
  }

  onTerminalFocused() {
    this.active = false;
    this.returnToRest();
  }

  onTerminalBlurred(hasEverBeenUnfocused = false) {
    if (this.ready && this.introComplete && hasEverBeenUnfocused) {
      this.canActivate = true;
      this.active = true;
    }
  }

  dispose() {
    if (this.modelManager.pcModel && this.originalPosition) {
      this.modelManager.pcModel.position.x = this.originalPosition.x;
      this.modelManager.pcModel.position.y = this.originalPosition.y;
      this.modelManager.pcModel.position.z = this.originalPosition.z;
    }
  }
}
