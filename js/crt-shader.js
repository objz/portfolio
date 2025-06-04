import * as THREE from "three";

export class CRTShader {
  constructor(terminalTexture, crtSettings) {
    this.terminalTexture = terminalTexture;
    this.crtSettings = crtSettings;
    this.originalScreenGeometry = null;
  }

  createCRTMaterial() {
    const vertexShader = `
      varying vec2 vUv;
      varying vec3 vNormal;
      varying vec3 vPosition;
      
      void main() {
        vUv = uv;
        vNormal = normalize(normalMatrix * normal);
        vPosition = (modelViewMatrix * vec4(position, 1.0)).xyz;
        gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
      }
    `;

    const fragmentShader = `
      uniform sampler2D map;
      uniform float bulge;
      uniform float scanlineIntensity;
      uniform float scanlineCount;
      uniform float vignetteIntensity;
      uniform float vignetteRadius;
      uniform float glowIntensity;
      uniform vec3 glowColor;
      uniform float brightness;
      uniform float contrast;
      uniform float time;
      
      varying vec2 vUv;
      varying vec3 vNormal;
      varying vec3 vPosition;

      vec2 crtDistort(vec2 uv) {
        uv = uv * 2.0 - 1.0;
        float r2 = dot(uv, uv);
        float distortion = 1.0 + bulge * r2 * 0.3;
        uv *= distortion;
        uv = uv * 0.5 + 0.5;
        return uv;
      }

      void main() {
        vec2 distortedUV = crtDistort(vUv);
        
        vec4 color = texture2D(map, distortedUV);
        
        color.rgb = color.rgb * brightness;
        color.rgb = ((color.rgb - 0.5) * contrast) + 0.5;
        
        float scanlinePattern = sin(distortedUV.y * scanlineCount) * scanlineIntensity;
        color.rgb += scanlinePattern;
        
        float depth = length(vPosition);
        float depthGlow = 1.0 / (1.0 + depth * 0.1);
        color.rgb += glowColor * glowIntensity * depthGlow;
        
        float vignette = smoothstep(vignetteRadius, 1.0, distance(vUv, vec2(0.5)));
        color.rgb *= (1.0 - vignette * vignetteIntensity);
        
        float flicker = 1.0 + sin(time * 60.0) * 0.005;
        color.rgb *= flicker;
        
        gl_FragColor = color;
      }
    `;

    return new THREE.ShaderMaterial({
      uniforms: {
        map: { value: this.terminalTexture },
        bulge: { value: this.crtSettings.bulge * 0.2 },
        scanlineIntensity: { value: this.crtSettings.scanlineIntensity },
        scanlineCount: { value: this.crtSettings.scanlineCount },
        vignetteIntensity: { value: this.crtSettings.vignetteIntensity },
        vignetteRadius: { value: this.crtSettings.vignetteRadius },
        glowIntensity: { value: this.crtSettings.glowIntensity },
        glowColor: { value: this.crtSettings.glowColor },
        brightness: { value: this.crtSettings.brightness },
        contrast: { value: this.crtSettings.contrast },
        time: { value: 0.0 },
      },
      vertexShader: vertexShader,
      fragmentShader: fragmentShader,
      transparent: true,
    });
  }

  createBulgedScreenGeometry(originalGeometry, bulgeAmount) {
    const geometry = originalGeometry.clone();
    const positionAttribute = geometry.getAttribute("position");
    const positions = positionAttribute.array;

    geometry.computeBoundingBox();
    const bbox = geometry.boundingBox;
    const width = bbox.max.x - bbox.min.x;
    const height = bbox.max.y - bbox.min.y;
    const centerX = (bbox.max.x + bbox.min.x) / 2;
    const centerY = (bbox.max.y + bbox.min.y) / 2;

    const scale = Math.max(width, height) / 2;

    for (let i = 0; i < positions.length; i += 3) {
      const x = positions[i];
      const y = positions[i + 1];
      const z = positions[i + 2];

      let normalizedX = (x - centerX) / scale;
      let normalizedY = (y - centerY) / scale;

      normalizedX += this.crtSettings.offsetX || 0;
      normalizedY += this.crtSettings.offsetY || 0;

      const distanceFromCenter = Math.sqrt(
        normalizedX * normalizedX + normalizedY * normalizedY,
      );
      const r = Math.min(distanceFromCenter, 1.0);

      const bulgeOffset = bulgeAmount * (1 - Math.exp(-r * r * 2)) * 0.15;
      positions[i + 2] = z - bulgeOffset;
    }

    positionAttribute.needsUpdate = true;
    geometry.computeVertexNormals();

    return geometry;
  }

  applyRealBulgeToScreen(screenMesh) {
    if (!screenMesh || !screenMesh.geometry) return;

    if (!this.originalScreenGeometry) {
      this.originalScreenGeometry = screenMesh.geometry.clone();
    }

    const bulgedGeometry = this.createBulgedScreenGeometry(
      this.originalScreenGeometry,
      this.crtSettings.bulge,
    );

    screenMesh.geometry.dispose();
    screenMesh.geometry = bulgedGeometry;
  }

  updateSettings(newSettings, screenMesh) {
    Object.assign(this.crtSettings, newSettings);

    if (
      newSettings.bulge !== undefined ||
      newSettings.offsetX !== undefined ||
      newSettings.offsetY !== undefined
    ) {
      this.applyRealBulgeToScreen(screenMesh);
    }

    if (screenMesh?.material?.uniforms) {
      const uniforms = screenMesh.material.uniforms;
      if (newSettings.bulge !== undefined)
        uniforms.bulge.value = this.crtSettings.bulge * 0.2;
      if (newSettings.scanlineIntensity !== undefined)
        uniforms.scanlineIntensity.value = this.crtSettings.scanlineIntensity;
      if (newSettings.scanlineCount !== undefined)
        uniforms.scanlineCount.value = this.crtSettings.scanlineCount;
      if (newSettings.vignetteIntensity !== undefined)
        uniforms.vignetteIntensity.value = this.crtSettings.vignetteIntensity;
      if (newSettings.vignetteRadius !== undefined)
        uniforms.vignetteRadius.value = this.crtSettings.vignetteRadius;
      if (newSettings.glowIntensity !== undefined)
        uniforms.glowIntensity.value = this.crtSettings.glowIntensity;
      if (newSettings.glowColor !== undefined)
        uniforms.glowColor.value = this.crtSettings.glowColor;
      if (newSettings.brightness !== undefined)
        uniforms.brightness.value = this.crtSettings.brightness;
      if (newSettings.contrast !== undefined)
        uniforms.contrast.value = this.crtSettings.contrast;
    }
  }

  updateTime(time, screenMesh) {
    if (screenMesh?.material?.uniforms) {
      screenMesh.material.uniforms.time.value = time;
    }
  }

  dispose() {
    if (this.originalScreenGeometry) {
      this.originalScreenGeometry.dispose();
    }
  }
}
