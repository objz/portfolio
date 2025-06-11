import * as THREE from "three";

export class EventManager {
  constructor(sceneManager) {
    this.sceneManager = sceneManager;
    this.raycaster = new THREE.Raycaster();
    this.mouse = new THREE.Vector2();
    this.boundHandlers = {};
  }

  setup() {
    this.bindHandlers();
    this.setupEventListeners();
  }

  bindHandlers() {
    this.boundHandlers = {
      resize: this.onResize.bind(this),
      mousemove: this.onMouseMove.bind(this),
      click: this.onClick.bind(this),
      keydown: this.onKeyDown.bind(this),
      wheel: this.onWheel.bind(this),
      contextmenu: this.onContextMenu.bind(this),
      terminalFocus: this.onTerminalFocus.bind(this),
      terminalBlur: this.onTerminalBlur.bind(this),
    };
  }

  setupEventListeners() {
    window.addEventListener("resize", this.boundHandlers.resize);
    window.addEventListener("mousemove", this.boundHandlers.mousemove);
    window.addEventListener("click", this.boundHandlers.click);
    window.addEventListener("keydown", this.boundHandlers.keydown);
    window.addEventListener("wheel", this.boundHandlers.wheel, {
      passive: false,
    });
    window.addEventListener("contextmenu", this.boundHandlers.contextmenu);

    window.addEventListener("terminalFocus", this.boundHandlers.terminalFocus);
    window.addEventListener("terminalBlur", this.boundHandlers.terminalBlur);
  }

  onResize() {
    if (!this.sceneManager.camera || !this.sceneManager.renderer) return;

    this.sceneManager.camera.aspect = window.innerWidth / window.innerHeight;
    this.sceneManager.camera.updateProjectionMatrix();

    this.sceneManager.renderer.setSize(window.innerWidth, window.innerHeight);

    if (this.sceneManager.effectsManager) {
      this.sceneManager.effectsManager.onResize();
    }
  }

  onMouseMove(event) {
    if (!this.sceneManager.camera || !this.sceneManager.modelManager) return;

    this.mouse.x = (event.clientX / window.innerWidth) * 2 - 1;
    this.mouse.y = -(event.clientY / window.innerHeight) * 2 + 1;

    this.raycaster.setFromCamera(this.mouse, this.sceneManager.camera);

    if (this.sceneManager.modelManager.pcModel) {
      const intersects = this.raycaster.intersectObject(
        this.sceneManager.modelManager.pcModel,
        true,
      );

      const wasHovering = this.sceneManager.hoveringScreen;
      this.sceneManager.hoveringScreen = intersects.some((intersect) =>
        this.sceneManager.modelManager.isScreenObject(intersect.object),
      );

      if (this.sceneManager.hoveringScreen !== wasHovering) {
        document.body.style.cursor = this.sceneManager.hoveringScreen
          ? "pointer"
          : "default";
      }
    }

    if (this.sceneManager.mouseManager) {
      const normalizedX = this.mouse.x;
      const normalizedY = this.mouse.y;
      this.sceneManager.mouseManager.updatePosition(
        normalizedX,
        normalizedY,
        this.sceneManager.terminalFocused,
      );
    }

    if (this.sceneManager.animationManager) {
      this.sceneManager.animationManager.stopIdleRotation();
    }
  }

  onClick(event) {
    if (!this.sceneManager.camera || !this.sceneManager.modelManager) return;

    this.mouse.x = (event.clientX / window.innerWidth) * 2 - 1;
    this.mouse.y = -(event.clientY / window.innerHeight) * 2 + 1;

    this.raycaster.setFromCamera(this.mouse, this.sceneManager.camera);

    if (this.sceneManager.modelManager.pcModel) {
      const intersects = this.raycaster.intersectObject(
        this.sceneManager.modelManager.pcModel,
        true,
      );
      const screenClicked = intersects.some((intersect) =>
        this.sceneManager.modelManager.isScreenObject(intersect.object),
      );

      if (screenClicked) {
        window.dispatchEvent(new CustomEvent("terminalFocus"));

        if (
          this.sceneManager.animationManager &&
          this.sceneManager.animationManager.currentState !== "focused"
        ) {
          this.sceneManager.animationManager.animateToState("focused", 1000);
        }
      } else if (intersects.length > 0 && this.sceneManager.terminalFocused) {
        window.dispatchEvent(new CustomEvent("terminalBlur"));
      }
    }
  }

  onKeyDown(event) {
    if (event.code === "Escape" && this.sceneManager.terminalFocused) {
      window.dispatchEvent(new CustomEvent("terminalBlur"));
    }

    if (this.sceneManager.animationManager) {
      this.sceneManager.animationManager.stopIdleRotation();
    }
  }

  onWheel(event) {
    if (this.sceneManager.terminalFocused && this.sceneManager.hoveringScreen) {
      event.preventDefault();

      if (event.deltaY > 0) {
        window.dispatchEvent(new CustomEvent("terminalScrollDown"));
      } else {
        window.dispatchEvent(new CustomEvent("terminalScrollUp"));
      }
    }

    if (this.sceneManager.animationManager) {
      this.sceneManager.animationManager.stopIdleRotation();
    }
  }

  onContextMenu(event) {
    event.preventDefault();
  }

  onTerminalFocus() {
    this.sceneManager.terminalFocused = true;

    if (this.sceneManager.mouseManager) {
      this.sceneManager.mouseManager.onTerminalFocused();
    }

    if (this.sceneManager.hiddenInput) {
      this.sceneManager.hiddenInput.focus();
    }

    document.body.style.cursor = "text";
  }

  onTerminalBlur() {
    this.sceneManager.terminalFocused = false;
    this.sceneManager.everUnfocused = true;

    if (this.sceneManager.mouseManager) {
      this.sceneManager.mouseManager.onTerminalBlurred(
        this.sceneManager.everUnfocused,
      );
    }

    if (this.sceneManager.hiddenInput) {
      this.sceneManager.hiddenInput.blur();
    }

    document.body.style.cursor = "default";

    if (
      this.sceneManager.animationManager &&
      this.sceneManager.animationManager.currentState === "focused"
    ) {
      this.sceneManager.animationManager.animateToState("default", 1000);
    }
  }

  dispose() {
    window.removeEventListener("resize", this.boundHandlers.resize);
    window.removeEventListener("mousemove", this.boundHandlers.mousemove);
    window.removeEventListener("click", this.boundHandlers.click);
    window.removeEventListener("keydown", this.boundHandlers.keydown);
    window.removeEventListener("wheel", this.boundHandlers.wheel);
    window.removeEventListener("contextmenu", this.boundHandlers.contextmenu);

    window.removeEventListener(
      "terminalFocus",
      this.boundHandlers.terminalFocus,
    );
    window.removeEventListener("terminalBlur", this.boundHandlers.terminalBlur);
    window.removeEventListener(
      "terminalReady",
      this.boundHandlers.terminalReady,
    );
  }
}
