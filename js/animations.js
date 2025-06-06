import * as THREE from "three";

export class AnimationManager {
  constructor(camera, controls, performanceMode) {
    this.camera = camera;
    this.controls = controls;
    this.performanceMode = performanceMode;

    this.startupPhase = "waiting";
    this.cameraStates = {
      default: {
        position: new THREE.Vector3(4, 5, 10),
        target: new THREE.Vector3(0, 1.5, 0),
      },
      focused: {
        position: new THREE.Vector3(0, 3.0, 5.5),
        target: new THREE.Vector3(0, 2.3, 0),
      },
      overview: {
        position: new THREE.Vector3(6, 6, 12),
        target: new THREE.Vector3(0, 1.5, 0),
      },
      idle: {
        position: new THREE.Vector3(3, 4.5, 9),
        target: new THREE.Vector3(0, 1.5, 0),
      },
    };

    this.currentState = "default";
    this.animatingCamera = false;
    this.idleRotationActive = false;
    this.lastInteractionTime = Date.now();
    this.lastFocusedTime = Date.now();
    this.inactivityDelay = 10000;
    this.focusedInactivityDelay = 180000;
    this.idleRotationSpeed = 0.0002;
    this.idleRadius = 7;
    this.idleHeight = 4.2;
    this.idleAngle = 0;
    this.maxIdleRotation = Math.PI * 1.2;
    this.idleRotationCount = 0;

    this.cameraBounds = {
      position: { x: [-15, 15], y: [2, 12], z: [3, 20] },
      target: { x: [-5, 5], y: [0, 5], z: [-3, 3] },
    };
    this.lastValidPosition = new THREE.Vector3();
    this.lastValidTarget = new THREE.Vector3();

    this.animationFrameSkip = this.performanceMode ? 2 : 1;
    this.frameCounter = 0;
  }

  startupAnimation() {
    this.startupPhase = "waiting";

    const delay = this.performanceMode ? 500 : 1000;
    setTimeout(() => {
      this.startupPhase = "focusing";
      const duration = this.performanceMode ? 1500 : 2000;
      this.animateToState("focused", duration, () => {
        this.startupPhase = "complete";
        this.lastInteractionTime = Date.now();
        this.lastFocusedTime = Date.now();
      });
    }, delay);
  }

  animateToState(stateName, duration = 1500, onComplete = null) {
    if (!this.cameraStates[stateName] || this.animatingCamera) return;

    this.animatingCamera = true;
    this.idleRotationActive = false;
    this.currentState = stateName;

    if (this.performanceMode) {
      duration = Math.max(duration * 0.7, 800);
    }

    const targetState = this.cameraStates[stateName];
    const startPosition = this.camera.position.clone();
    const startTarget = this.controls.target.clone();
    const startTime = Date.now();

    const animateStep = () => {
      this.frameCounter++;
      if (
        this.performanceMode &&
        this.frameCounter % this.animationFrameSkip !== 0
      ) {
        if (Date.now() - startTime < duration) {
          requestAnimationFrame(animateStep);
        }
        return;
      }

      const elapsed = Date.now() - startTime;
      const progress = Math.min(elapsed / duration, 1);
      const easeProgress = this.easeInOutCubic(progress);

      this.camera.position.lerpVectors(
        startPosition,
        targetState.position,
        easeProgress,
      );
      this.controls.target.lerpVectors(
        startTarget,
        targetState.target,
        easeProgress,
      );

      if (progress < 1) {
        requestAnimationFrame(animateStep);
      } else {
        this.animatingCamera = false;
        if (onComplete) onComplete();
      }
    };

    animateStep();
  }

  updateIdleRotation(terminalFocused) {
    if (this.startupPhase !== "complete") return;
    if (this.animatingCamera) return;

    const currentTime = Date.now();
    const timeSinceInteraction = currentTime - this.lastInteractionTime;
    const timeSinceFocused = currentTime - this.lastFocusedTime;

    const inactivityDelay = this.performanceMode
      ? this.inactivityDelay * 1.5
      : this.inactivityDelay;
    const focusedDelay = this.performanceMode
      ? this.focusedInactivityDelay * 1.2
      : this.focusedInactivityDelay;

    if (this.currentState === "focused" && terminalFocused) {
      if (timeSinceFocused > focusedDelay && !this.idleRotationActive) {
        this.startIdleRotation();
      }
    } else if (this.currentState !== "focused") {
      if (timeSinceInteraction > inactivityDelay && !this.idleRotationActive) {
        this.startIdleRotation();
      }
    }

    if (this.idleRotationActive) {
      const rotationSpeed = this.performanceMode
        ? this.idleRotationSpeed * 0.7
        : this.idleRotationSpeed;

      this.idleAngle += rotationSpeed;
      this.idleRotationCount += rotationSpeed;

      if (this.idleRotationCount >= this.maxIdleRotation) {
        this.resetIdleRotation();
        return;
      }

      const x = Math.cos(this.idleAngle) * this.idleRadius;
      const z = Math.sin(this.idleAngle) * this.idleRadius;

      this.camera.position.set(x, this.idleHeight, z);
      this.controls.target.set(0, 1.5, 0);
    }
  }

  stopIdleRotation() {
    this.idleRotationActive = false;
    this.lastInteractionTime = Date.now();
    this.lastFocusedTime = Date.now();

    if (this.currentState === "idle") {
      const duration = this.performanceMode ? 800 : 1000;
      this.animateToState("default", duration);
    }
  }

  startIdleRotation() {
    this.idleRotationActive = false;
    this.animatingCamera = true;
    this.idleRotationCount = 0;

    const startAngle = Math.atan2(
      this.camera.position.z - 0,
      this.camera.position.x - 0,
    );
    this.idleAngle = startAngle;

    const idealStartPosition = new THREE.Vector3(
      Math.cos(this.idleAngle) * this.idleRadius,
      this.idleHeight,
      Math.sin(this.idleAngle) * this.idleRadius,
    );

    const startPosition = this.camera.position.clone();
    const startTarget = this.controls.target.clone();
    const targetTarget = new THREE.Vector3(0, 1.5, 0);
    const startTime = Date.now();
    const duration = this.performanceMode ? 1200 : 1500;

    const animateToIdleStart = () => {
      const elapsed = Date.now() - startTime;
      const progress = Math.min(elapsed / duration, 1);
      const easeProgress = this.easeInOutCubic(progress);

      this.camera.position.lerpVectors(
        startPosition,
        idealStartPosition,
        easeProgress,
      );
      this.controls.target.lerpVectors(startTarget, targetTarget, easeProgress);

      if (progress < 1) {
        requestAnimationFrame(animateToIdleStart);
      } else {
        this.animatingCamera = false;
        this.idleRotationActive = true;
        this.currentState = "idle";
      }
    };

    animateToIdleStart();
  }

  resetIdleRotation() {
    this.idleRotationActive = false;
    this.idleRotationCount = 0;

    const duration = this.performanceMode ? 1200 : 1500;
    this.animateToState("default", duration, () => {
      const timeout = this.performanceMode ? 1500 : 2000;
      setTimeout(() => {
        if (
          this.currentState === "default" &&
          this.startupPhase === "complete"
        ) {
          this.startIdleRotation();
        }
      }, timeout);
    });
  }

  checkCameraBounds() {
    const pos = this.camera.position;
    const target = this.controls.target;
    let needsCorrection = false;

    if (
      pos.x < this.cameraBounds.position.x[0] ||
      pos.x > this.cameraBounds.position.x[1] ||
      pos.y < this.cameraBounds.position.y[0] ||
      pos.y > this.cameraBounds.position.y[1] ||
      pos.z < this.cameraBounds.position.z[0] ||
      pos.z > this.cameraBounds.position.z[1]
    ) {
      needsCorrection = true;
    }

    if (
      target.x < this.cameraBounds.target.x[0] ||
      target.x > this.cameraBounds.target.x[1] ||
      target.y < this.cameraBounds.target.y[0] ||
      target.y > this.cameraBounds.target.y[1] ||
      target.z < this.cameraBounds.target.z[0] ||
      target.z > this.cameraBounds.target.z[1]
    ) {
      needsCorrection = true;
    }

    if (needsCorrection && !this.animatingCamera) {
      this.correctPosition();
    } else if (!needsCorrection) {
      this.lastValidPosition.copy(pos);
      this.lastValidTarget.copy(target);
    }
  }

  correctPosition() {
    const duration = this.performanceMode ? 800 : 1000;
    this.animateToState("default", duration);
  }

  easeInOutCubic(t) {
    return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
  }
}
