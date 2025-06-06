function getBasePath() {
  let path = window.location.pathname;
  if (!path.endsWith("/")) path = path.substring(0, path.lastIndexOf("/") + 1);
  return path;
}

const BASE_PATH = getBasePath();

export class AudioManager {
  constructor() {
    this.sounds = {
      boot: new Audio(`${BASE_PATH}audio/boot.wav`),
      intro: new Audio(`${BASE_PATH}audio/intro.wav`),
      loop: new Audio(`${BASE_PATH}audio/loop.wav`),
      fan: new Audio(`${BASE_PATH}audio/fan.wav`),
      click: new Audio(`${BASE_PATH}audio/click.wav`),
    };

    this.keySounds = [];
    this.keySoundPool = [];
    this.poolSize = 5;

    this.sounds.fan.loop = true;
    this.sounds.loop.loop = true;
    this.ambientStarted = false;
    this.clickEnabled = true;
    this.keyEnabled = true;
    this.soundsLoaded = false;
    this.initialized = false;

    this.ambientVolume = 0.25;
    this.mainVolume = 0.4;
  }

  async init() {
    if (this.initialized) return;

    await this.preloadSounds();
    this.setVolume(this.mainVolume);
    this.setupClickListener();
    this.setupKeyListener();
    this.initialized = true;
  }

  async preloadSounds() {
    const mainPromises = Object.values(this.sounds).map((audio) => {
      return new Promise((resolve) => {
        if (audio.readyState >= 2) {
          resolve();
        } else {
          audio.addEventListener("canplaythrough", resolve, { once: true });
          audio.addEventListener(
            "error",
            () => {
              console.warn(`Failed to load: ${audio.src}`);
              resolve();
            },
            { once: true },
          );
          audio.load();
        }
      });
    });

    const existingKeys = await this.checkKeyFiles();

    for (const keyNumber of existingKeys) {
      const keyInstances = [];

      for (let i = 0; i < this.poolSize; i++) {
        const audio = new Audio(`${BASE_PATH}audio/keys/key${keyNumber}.wav`);
        keyInstances.push(audio);
      }

      this.keySoundPool.push({
        keyNumber,
        instances: keyInstances,
        currentIndex: 0,
      });
    }

    const keyPromises = this.keySoundPool.flatMap((pool) =>
      pool.instances.map((audio) => {
        return new Promise((resolve) => {
          if (audio.readyState >= 2) {
            resolve();
          } else {
            audio.addEventListener("canplaythrough", resolve, { once: true });
            audio.addEventListener(
              "error",
              () => {
                console.warn(`Failed to load: ${audio.src}`);
                resolve();
              },
              { once: true },
            );
            audio.load();
          }
        });
      }),
    );

    await Promise.all([...mainPromises, ...keyPromises]);

    this.soundsLoaded = true;
    console.log(
      `Sounds loaded! (${this.keySoundPool.length} key types with ${this.poolSize} instances each)`,
    );
  }

  async checkKeyFiles() {
    const existingFiles = [];

    for (let i = 1; i <= 17; i++) {
      try {
        const response = await fetch(`${BASE_PATH}audio/keys/key${i}.wav`, {
          method: "HEAD",
        });
        if (response.ok) {
          existingFiles.push(i);
        }
      } catch (error) {}
    }

    return existingFiles;
  }

  setVolume(volume) {
    this.mainVolume = volume;

    Object.entries(this.sounds).forEach(([name, audio]) => {
      if (name !== "intro" && name !== "loop") {
        audio.volume = volume;
      }
    });

    this.sounds.intro.volume = this.ambientVolume;
    this.sounds.loop.volume = this.ambientVolume;

    this.keySoundPool.forEach((pool) => {
      pool.instances.forEach((audio) => {
        audio.volume = volume;
      });
    });
  }

  setupClickListener() {
    document.addEventListener("mousedown", (event) => {
      if (event.button === 0 || event.button === 2) {
        this.mouseDownTime = Date.now();
      }
    });

    document.addEventListener("mouseup", (event) => {
      if (
        this.clickEnabled &&
        this.mouseDownTime &&
        (event.button === 0 || event.button === 2)
      ) {
        const holdDuration = Date.now() - this.mouseDownTime;

        if (holdDuration < 200) {
          this.play("click");
        }

        this.mouseDownTime = null;
      }
    });

    document.addEventListener("mouseleave", () => {
      this.mouseDownTime = null;
    });
  }

  setupKeyListener() {
    document.addEventListener("keydown", (_event) => {
      if (this.keyEnabled && this.soundsLoaded) {
        this.playRandomKey();
      }
    });
  }

  playRandomKey() {
    if (this.keySoundPool.length === 0) return;

    const randomPoolIndex = Math.floor(
      Math.random() * this.keySoundPool.length,
    );
    const selectedPool = this.keySoundPool[randomPoolIndex];

    const audio = selectedPool.instances[selectedPool.currentIndex];
    selectedPool.currentIndex =
      (selectedPool.currentIndex + 1) % selectedPool.instances.length;

    audio.currentTime = 0;
    audio.play().catch((e) => {
      console.warn("Failed to play key sound:", e);
    });
  }

  playAmbient() {
    if (this.ambientStarted) return;

    this.ambientStarted = true;

    this.sounds.intro.volume = this.ambientVolume;
    this.sounds.loop.volume = this.ambientVolume;

    this.sounds.intro.currentTime = 0;
    this.sounds.intro.play();

    const onIntroEnd = () => {
      this.sounds.intro.removeEventListener("ended", onIntroEnd);
      this.sounds.loop.currentTime = 0;
      this.sounds.loop.play();
    };

    this.sounds.intro.addEventListener("ended", onIntroEnd);
  }

  play(name) {
    if (!this.initialized) {
      console.warn(`Audio not initialized, cannot play: ${name}`);
      return;
    }

    if (!this.sounds[name]) return;

    if (name === "ambient") {
      this.playAmbient();
    } else if (name === "boot") {
      const snd = this.sounds[name];
      snd.currentTime = 0;
      snd.play();

      const onBootEnd = () => {
        snd.removeEventListener("ended", onBootEnd);
        setTimeout(() => {
          this.playAmbient();
        }, 2000);
      };

      snd.addEventListener("ended", onBootEnd);
    } else {
      const snd = this.sounds[name];
      snd.currentTime = 0;
      snd.play();
    }
  }

  stop(name) {
    if (this.sounds[name]) {
      this.sounds[name].pause();
      this.sounds[name].currentTime = 0;
    }
  }

  dispose() {
    Object.values(this.sounds).forEach((audio) => {
      audio.pause();
      audio.currentTime = 0;
    });

    this.keySoundPool.forEach((pool) => {
      pool.instances.forEach((audio) => {
        audio.pause();
        audio.currentTime = 0;
      });
    });

    this.keySoundPool = [];
    this.soundsLoaded = false;
    this.initialized = false;
  }
}
