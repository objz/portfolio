function getBasePath() {
  let path = window.location.pathname;
  if (!path.endsWith("/")) path = path.substring(0, path.lastIndexOf("/") + 1);
  return path;
}

const BASE_PATH = getBasePath();
const KEY_SOUND_COUNT = 17;
const MAIN_PRELOAD_TIMEOUT_MS = 2500;
const KEY_PRELOAD_TIMEOUT_MS = 1200;

export class AudioManager {
  constructor() {
    this.sounds = {
      boot: new Audio(`${BASE_PATH}audio/boot.wav`),
      intro: new Audio(`${BASE_PATH}audio/intro.wav`),
      loop: new Audio(`${BASE_PATH}audio/loop.wav`),
      fan: new Audio(`${BASE_PATH}audio/fan.wav`),
      click: new Audio(`${BASE_PATH}audio/click.wav`),
    };

    this.keySoundPool = [];
    this.poolSize = 5;
    this.mouseDownTime = null;

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

    try {
      await this.preloadSounds();
    } catch (error) {
      console.warn("Audio preload completed with issues:", error);
    }

    this.setVolume(this.mainVolume);
    this.setupClickListener();
    this.setupKeyListener();
    this.initialized = true;
  }

  async preloadSounds() {
    const mainPromises = Object.values(this.sounds).map((audio) =>
      this.waitForAudioReady(audio, MAIN_PRELOAD_TIMEOUT_MS),
    );

    const mainResults = await Promise.allSettled(mainPromises);
    const loadedMainCount = mainResults.filter(
      (result) => result.status === "fulfilled" && result.value,
    ).length;

    this.buildKeySoundPool();
    this.soundsLoaded = true;

    void this.preloadKeySoundsInBackground();

    console.log(
      `Main sounds ready (${loadedMainCount}/${Object.keys(this.sounds).length}). Key sounds warming in background.`,
    );
  }

  buildKeySoundPool() {
    this.keySoundPool = [];

    for (let keyNumber = 1; keyNumber <= KEY_SOUND_COUNT; keyNumber++) {
      const keyInstances = [];

      for (let i = 0; i < this.poolSize; i++) {
        const audio = new Audio(`${BASE_PATH}audio/keys/key${keyNumber}.wav`);
        audio.preload = "auto";
        keyInstances.push(audio);
      }

      this.keySoundPool.push({
        keyNumber,
        instances: keyInstances,
        currentIndex: 0,
      });
    }
  }

  async preloadKeySoundsInBackground() {
    const keyPromises = this.keySoundPool.flatMap((pool) =>
      pool.instances.map((audio) =>
        this.waitForAudioReady(audio, KEY_PRELOAD_TIMEOUT_MS),
      ),
    );

    const keyResults = await Promise.allSettled(keyPromises);
    const loadedKeyCount = keyResults.filter(
      (result) => result.status === "fulfilled" && result.value,
    ).length;
    const totalKeyCount = keyResults.length;

    console.log(
      `Key sound warmup finished (${loadedKeyCount}/${totalKeyCount}).`,
    );
  }

  waitForAudioReady(audio, timeoutMs) {
    return new Promise((resolve) => {
      if (audio.readyState >= 2) {
        resolve(true);
        return;
      }

      let settled = false;
      let timeoutId = null;

      const cleanup = () => {
        audio.removeEventListener("canplaythrough", onReady);
        audio.removeEventListener("loadeddata", onReady);
        audio.removeEventListener("error", onError);
        if (timeoutId !== null) {
          clearTimeout(timeoutId);
          timeoutId = null;
        }
      };

      const finish = (ok) => {
        if (settled) return;
        settled = true;
        cleanup();
        resolve(ok);
      };

      const onReady = () => finish(true);

      const onError = () => {
        console.warn(`Failed to load audio: ${audio.src}`);
        finish(false);
      };

      timeoutId = window.setTimeout(() => {
        console.warn(`Audio preload timeout: ${audio.src}`);
        finish(false);
      }, timeoutMs);

      audio.addEventListener("canplaythrough", onReady);
      audio.addEventListener("loadeddata", onReady);
      audio.addEventListener("error", onError);

      try {
        audio.load();
      } catch (error) {
        console.warn(`Audio load threw for ${audio.src}:`, error);
        finish(false);
      }
    });
  }

  safePlay(audio, name) {
    if (!audio) return;

    const playPromise = audio.play();
    if (playPromise && typeof playPromise.catch === "function") {
      playPromise.catch((error) => {
        console.warn(`Failed to play '${name}':`, error);
      });
    }
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
    this.safePlay(this.sounds.intro, "intro");

    const onIntroEnd = () => {
      this.sounds.loop.currentTime = 0;
      this.safePlay(this.sounds.loop, "loop");
    };

    this.sounds.intro.addEventListener("ended", onIntroEnd, { once: true });
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
      this.safePlay(snd, name);

      const onBootEnd = () => {
        setTimeout(() => {
          this.playAmbient();
        }, 1200);
      };

      snd.addEventListener("ended", onBootEnd, { once: true });
    } else {
      const snd = this.sounds[name];
      snd.currentTime = 0;
      this.safePlay(snd, name);
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
