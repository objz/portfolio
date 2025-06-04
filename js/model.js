import * as THREE from "three";
import { GLTFLoader } from "three/addons/loaders/GLTFLoader.js";

export class ModelManager {
  constructor(scene, updateProgress) {
    this.scene = scene;
    this.updateProgress = updateProgress;
    this.pcModel = null;
    this.screenMesh = null;
    this.sceneMeshes = [];
    this.fadeStartTime = null;
    this.fadeInDuration = 4000;

    this.performanceMode = this.detectPerformanceMode();
    this.textureSize = this.performanceMode ? 512 : 1024;
    this.shadowMapSize = this.performanceMode ? 1024 : 2048;
    this.enableMipmaps = !this.performanceMode;

    this.geometrySimplification = this.performanceMode ? 0.5 : 1.0;
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

    return isMobile || isLowEndGPU || memory < 4;
  }

  async loadPCModel() {
    return new Promise((resolve) => {
      const loader = new GLTFLoader();

      if (this.performanceMode) {
        loader.setDRACOLoader(null);
      }

      loader.load(
        "./pc.glb",
        (gltf) => {
          this.pcModel = gltf.scene;
          this.pcModel.scale.setScalar(1);
          this.pcModel.position.set(0, 0, 0);

          this.optimizeModel();
          this.scene.add(this.pcModel);
          this.startSceneFadeIn();
          resolve();
        },
        (prog) => {
          const pct = 30 + (prog.loaded / prog.total) * 30;
          this.updateProgress(pct);
        },
        (err) => {
          console.warn("Model load failed:", err);
          resolve();
        },
      );
    });
  }

  optimizeModel() {
    let meshCount = 0;
    const maxMeshes = this.performanceMode ? 50 : 200;

    this.pcModel.traverse((c) => {
      if (c.isMesh && meshCount < maxMeshes) {
        meshCount++;
        c.castShadow = !this.performanceMode;
        c.receiveShadow = !this.performanceMode;
        this.sceneMeshes.push(c);

        if (this.performanceMode && c.geometry) {
          this.simplifyGeometry(c);
        }

        if (c.material) {
          this.optimizeMaterial(c.material);
          this.identifyScreenMesh(c);
        }
      } else if (c.isMesh) {
        c.parent.remove(c);
      }
    });

    if (!this.screenMesh) {
      this.findScreenMesh();
    }
  }

  simplifyGeometry(mesh) {
    if (!mesh.geometry.attributes.position) return;

    const geometry = mesh.geometry;
    const positionArray = geometry.attributes.position.array;
    const indexArray = geometry.index ? geometry.index.array : null;

    if (positionArray.length < 300) return;

    if (indexArray && indexArray.length > 1000) {
      const decimationFactor = 0.7;
      const newIndexArray = new Uint16Array(
        Math.floor(indexArray.length * decimationFactor),
      );

      for (let i = 0; i < newIndexArray.length; i += 3) {
        const sourceIndex = Math.floor(i / decimationFactor / 3) * 3;
        if (sourceIndex + 2 < indexArray.length) {
          newIndexArray[i] = indexArray[sourceIndex];
          newIndexArray[i + 1] = indexArray[sourceIndex + 1];
          newIndexArray[i + 2] = indexArray[sourceIndex + 2];
        }
      }

      geometry.setIndex(new THREE.BufferAttribute(newIndexArray, 1));
    }
  }

  optimizeMaterial(material) {
    if (material.map) {
      this.optimizeTexture(material.map);
    }

    [
      "normalMap",
      "roughnessMap",
      "metalnessMap",
      "emissiveMap",
      "aoMap",
    ].forEach((mapType) => {
      if (material[mapType]) {
        this.optimizeTexture(material[mapType]);
      }
    });

    if (material.isMeshStandardMaterial) {
      material.envMapIntensity = this.performanceMode ? 0.1 : 0.3;

      material.roughness = Math.max(
        this.performanceMode ? 0.5 : 0.4,
        material.roughness * (this.performanceMode ? 1.3 : 1.2),
      );
      material.metalness = Math.min(
        this.performanceMode ? 0.5 : 0.6,
        material.metalness * (this.performanceMode ? 0.7 : 0.8),
      );

      if (material.emissive) {
        material.emissive.multiplyScalar(this.performanceMode ? 0.3 : 0.5);
      }
    } else if (material.isMeshBasicMaterial) {
      const newMaterial = new THREE.MeshStandardMaterial({
        color: material.color,
        map: material.map,
        transparent: material.transparent,
        opacity: material.opacity,
        roughness: this.performanceMode ? 0.8 : 0.7,
        metalness: this.performanceMode ? 0.05 : 0.1,
      });

      if (this.performanceMode) {
        newMaterial.normalMap = null;
        newMaterial.roughnessMap = null;
        newMaterial.metalnessMap = null;
      }

      return newMaterial;
    }
  }

  optimizeTexture(texture) {
    texture.generateMipmaps = this.enableMipmaps;
    texture.minFilter = this.enableMipmaps
      ? THREE.LinearMipmapLinearFilter
      : THREE.LinearFilter;
    texture.magFilter = THREE.LinearFilter;

    if (this.performanceMode && texture.image) {
      const canvas = document.createElement("canvas");
      const ctx = canvas.getContext("2d");

      canvas.width = Math.min(texture.image.width, this.textureSize);
      canvas.height = Math.min(texture.image.height, this.textureSize);

      ctx.drawImage(texture.image, 0, 0, canvas.width, canvas.height);
      texture.image = canvas;
      texture.needsUpdate = true;
    }
  }

  identifyScreenMesh(mesh) {
    const n = mesh.name.toLowerCase();
    const m = mesh.material?.name?.toLowerCase() || "";

    if (
      n.includes("screen") ||
      n.includes("monitor") ||
      m.includes("screen") ||
      m.includes("monitor") ||
      mesh.name === "Plane008_Material002_0"
    ) {
      this.screenMesh = mesh;
    }
  }

  findScreenMesh() {
    const candidates = [];
    this.pcModel.traverse((c) => {
      if (c.isMesh && c.geometry) {
        const box = new THREE.Box3().setFromObject(c);
        const s = box.getSize(new THREE.Vector3());
        const flat = Math.min(s.x, s.y, s.z) < Math.max(s.x, s.y, s.z) * 0.1;
        const big = Math.max(s.x, s.y, s.z) > 0.5;
        if (flat && big) candidates.push({ mesh: c, area: s.x * s.y * s.z });
      }
    });

    if (candidates.length) {
      candidates.sort((a, b) => b.area - a.area);
      this.screenMesh = candidates[0].mesh;
    } else {
      this.pcModel.traverse((c) => {
        if (!this.screenMesh && c.isMesh) this.screenMesh = c;
      });
    }
  }

  startSceneFadeIn() {
    this.fadeStartTime = Date.now();

    if (this.performanceMode) {
      this.fadeInDuration = 2500;
    }

    this.sceneMeshes.forEach((mesh) => {
      if (mesh !== this.screenMesh && mesh.material) {
        mesh.material.transparent = true;
        mesh.material.opacity = 0;
      }
    });
  }

  updateSceneFadeIn() {
    if (!this.fadeStartTime) return;

    const elapsed = Date.now() - this.fadeStartTime;
    const progress = Math.min(elapsed / this.fadeInDuration, 1);
    const opacity = this.easeInOutCubic(progress);

    const materialsToUpdate = [];

    this.sceneMeshes.forEach((mesh) => {
      if (
        mesh !== this.screenMesh &&
        mesh.material &&
        mesh.material.transparent
      ) {
        if (!materialsToUpdate.includes(mesh.material)) {
          materialsToUpdate.push(mesh.material);
          mesh.material.opacity = opacity;
        }

        if (progress >= 1) {
          mesh.material.transparent = false;
          mesh.material.opacity = 1;
        }
      }
    });

    if (progress >= 1) {
      this.fadeStartTime = null;
    }
  }

  easeInOutCubic(t) {
    return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
  }

  isScreenObject(object) {
    return (
      object.name === "Plane008_Material002_0" ||
      object === this.screenMesh ||
      (object.name && object.name.toLowerCase().includes("screen"))
    );
  }

  dispose() {
    this.sceneMeshes.forEach((mesh) => {
      if (mesh.geometry) mesh.geometry.dispose();
      if (mesh.material) {
        if (mesh.material.map) mesh.material.map.dispose();
        if (mesh.material.normalMap) mesh.material.normalMap.dispose();
        if (mesh.material.roughnessMap) mesh.material.roughnessMap.dispose();
        if (mesh.material.metalnessMap) mesh.material.metalnessMap.dispose();
        mesh.material.dispose();
      }
    });

    if (this.pcModel) {
      this.scene.remove(this.pcModel);
    }
  }
}
