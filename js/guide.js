function compilePattern(pattern) {
  if (typeof pattern !== "string" || !pattern.trim()) return null;
  try {
    return new RegExp(pattern, "i");
  } catch {
    return null;
  }
}

function normalizeChapter(ch) {
  if (!ch || typeof ch !== "object") return null;
  if (!Array.isArray(ch.tasks) || ch.tasks.length === 0) return null;

  const tasks = ch.tasks
    .map((t) => {
      if (!t || typeof t !== "object") return null;
      const matcher = compilePattern(t.match);
      if (!matcher) return null;
      return {
        label: typeof t.label === "string" ? t.label.trim() : "task",
        matcher,
      };
    })
    .filter(Boolean);

  if (tasks.length === 0) return null;

  const hints = Array.isArray(ch.hints)
    ? ch.hints.filter((h) => typeof h === "string" && h.trim()).map((h) => h.trim())
    : [];

  return {
    title: typeof ch.title === "string" ? ch.title.trim() : "Chapter",
    tasks,
    hints,
    funFact: typeof ch.funFact === "string" ? ch.funFact.trim() : "",
  };
}

export class GuideManager {
  constructor(configUrl = "./guide.json") {
    this.configUrl = configUrl;

    this.chapters = [];
    this.currentChapter = 0;
    this.taskState = [];
    this.initialized = false;
    this.visible = false;

    this.hintTimers = [];
    this.revealedHints = 0;
    this.lastProgressTime = 0;

    this.funFactTimer = null;
    this.typingInterval = null;
    this.chapterTransitionTimer = null;
    this.allDone = false;

    this.elChecklist = null;
    this.elFunFact = null;

    this.boundCommand = this.onTerminalCommand.bind(this);
  }

  async init() {
    this.elChecklist = document.getElementById("guide-checklist");
    this.elFunFact = document.getElementById("guide-funfact");

    if (!this.elChecklist || !this.elFunFact) return;

    const config = await this.loadConfig();
    if (!config) return;

    this.chapters = Array.isArray(config.chapters)
      ? config.chapters.map(normalizeChapter).filter(Boolean)
      : [];

    if (this.chapters.length === 0) return;

    this.taskState = this.chapters.map((ch) => new Array(ch.tasks.length).fill(false));

    window.addEventListener("terminalCommand", this.boundCommand);
    this.initialized = true;
  }

  async loadConfig() {
    try {
      const res = await fetch(this.configUrl, { cache: "no-store" });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      return await res.json();
    } catch (e) {
      console.warn("Guide config failed:", e);
      return null;
    }
  }

  show() {
    if (!this.initialized) return;
    this.visible = true;
    this.lastProgressTime = Date.now();
    this.revealedHints = 0;
    this.render();
    this.startHintTimers();
  }

  hide() {
    this.visible = false;
    this.clearHintTimers();
    this.clearFunFact();
    if (this.elChecklist) this.elChecklist.classList.add("hidden");
  }

  // --- rendering ---

  render() {
    if (!this.visible || !this.elChecklist) return;

    const ch = this.chapters[this.currentChapter];
    if (!ch) return;

    const state = this.taskState[this.currentChapter];
    const total = this.chapters.length;
    const current = this.currentChapter + 1;

    let html = "";

    // chapter title + progress (on top)
    html += `<div class="guide-chapter-bar"><span class="guide-chapter-title">${ch.title}</span><span class="guide-progress">${current}/${total}</span></div>`;

    // tasks
    for (let i = 0; i < ch.tasks.length; i++) {
      const done = state[i];
      const check = done ? "[x]" : "[ ]";
      const cls = done ? "guide-task done" : "guide-task";
      html += `<div class="${cls}"><span class="guide-check-mark">${check}</span> ${ch.tasks[i].label}</div>`;
    }

    // hints (revealed by timer)
    for (let i = 0; i < this.revealedHints && i < ch.hints.length; i++) {
      html += `<div class="guide-hint">hint: ${ch.hints[i]}</div>`;
    }

    this.elChecklist.innerHTML = html;
    this.elChecklist.classList.remove("hidden");
  }

  // --- command detection ---

  onTerminalCommand(event) {
    if (!this.initialized || !this.visible || this.allDone) return;

    const cmd = typeof event.detail === "string" ? event.detail.trim() : "";
    if (!cmd) return;

    const ch = this.chapters[this.currentChapter];
    if (!ch) return;

    const state = this.taskState[this.currentChapter];
    let changed = false;

    for (let i = 0; i < ch.tasks.length; i++) {
      if (state[i]) continue;
      if (ch.tasks[i].matcher.test(cmd)) {
        state[i] = true;
        changed = true;
      }
    }

    if (!changed) return;

    this.lastProgressTime = Date.now();
    this.render();

    // chapter complete?
    if (state.every(Boolean)) {
      this.onChapterComplete();
    } else {
      // reset hint timers on progress
      this.revealedHints = 0;
      this.clearHintTimers();
      this.startHintTimers();
    }
  }

  onChapterComplete() {
    this.clearHintTimers();
    this.revealedHints = 0;
    this.render(); // re-render to clear hints from display

    const ch = this.chapters[this.currentChapter];

    // show fun fact with typing effect at bottom center
    if (ch.funFact) {
      this.showFunFact(ch.funFact);
    }

    // advance to next chapter after a delay
    const isLast = this.currentChapter >= this.chapters.length - 1;

    if (isLast) {
      // final chapter done - fade out checklist after fun fact
      this.chapterTransitionTimer = setTimeout(() => {
        this.allDone = true;
        if (this.elChecklist) {
          this.elChecklist.classList.add("guide-fade-out");
          setTimeout(() => {
            this.elChecklist.classList.add("hidden");
            this.elChecklist.classList.remove("guide-fade-out");
          }, 600);
        }
      }, 2000);
    } else {
      this.chapterTransitionTimer = setTimeout(() => {
        this.currentChapter++;
        this.revealedHints = 0;
        this.lastProgressTime = Date.now();
        this.render();
        this.startHintTimers();
      }, 1500);
    }
  }

  // --- hints ---

  startHintTimers() {
    this.clearHintTimers();

    const ch = this.chapters[this.currentChapter];
    if (!ch || ch.hints.length === 0) return;

    // first hint after 30s
    const t1 = setTimeout(() => {
      if (this.revealedHints < 1 && !this.isChapterDone()) {
        this.revealedHints = 1;
        this.render();
      }
    }, 30000);

    this.hintTimers.push(t1);

    // second hint after 60s total (30+30)
    if (ch.hints.length > 1) {
      const t2 = setTimeout(() => {
        if (this.revealedHints < 2 && !this.isChapterDone()) {
          this.revealedHints = 2;
          this.render();
        }
      }, 60000);
      this.hintTimers.push(t2);
    }
  }

  clearHintTimers() {
    for (const t of this.hintTimers) clearTimeout(t);
    this.hintTimers = [];
  }

  isChapterDone() {
    const state = this.taskState[this.currentChapter];
    return state && state.every(Boolean);
  }

  // --- fun fact typing effect ---

  showFunFact(text) {
    this.clearFunFact();
    if (!this.elFunFact) return;

    this.elFunFact.textContent = "";
    this.elFunFact.classList.remove("hidden", "guide-fade-out");
    this.elFunFact.classList.add("visible");

    let i = 0;
    this.typingInterval = setInterval(() => {
      if (i < text.length) {
        this.elFunFact.textContent += text.charAt(i);
        i++;
      } else {
        clearInterval(this.typingInterval);
        this.typingInterval = null;

        // fade out after 15 seconds
        this.funFactTimer = setTimeout(() => {
          if (this.elFunFact) {
            this.elFunFact.classList.add("guide-fade-out");
            setTimeout(() => {
              if (this.elFunFact) {
                this.elFunFact.classList.add("hidden");
                this.elFunFact.classList.remove("visible", "guide-fade-out");
              }
            }, 600);
          }
        }, 15000);
      }
    }, 30);
  }

  clearFunFact() {
    if (this.typingInterval) {
      clearInterval(this.typingInterval);
      this.typingInterval = null;
    }
    if (this.funFactTimer) {
      clearTimeout(this.funFactTimer);
      this.funFactTimer = null;
    }
    if (this.chapterTransitionTimer) {
      clearTimeout(this.chapterTransitionTimer);
      this.chapterTransitionTimer = null;
    }
  }

  dispose() {
    window.removeEventListener("terminalCommand", this.boundCommand);
    this.clearHintTimers();
    this.clearFunFact();
  }

  // Dev function: skip to a specific chapter (1-indexed)
  // Usage in browser console: window.guide.skipTo(16)
  skipTo(chapterNum) {
    if (!this.initialized) {
      console.warn("Guide not initialized");
      return;
    }
    const idx = chapterNum - 1;
    if (idx < 0 || idx >= this.chapters.length) {
      console.warn(`Invalid chapter: ${chapterNum}. Valid range: 1-${this.chapters.length}`);
      return;
    }
    
    // Mark all previous chapters as complete
    for (let i = 0; i < idx; i++) {
      this.taskState[i] = this.taskState[i].map(() => true);
    }
    
    // Reset current chapter
    this.taskState[idx] = this.taskState[idx].map(() => false);
    
    this.currentChapter = idx;
    this.revealedHints = 0;
    this.allDone = false;
    this.clearHintTimers();
    this.clearFunFact();
    this.lastProgressTime = Date.now();
    this.render();
    this.startHintTimers();
    
    console.log(`Skipped to chapter ${chapterNum}: ${this.chapters[idx].title}`);
  }
}
