<template>
  <section class="auth-scene" aria-label="GenMod MOD 创作工作台">
    <div class="auth-brand">
      <img src="/gen-mod-icon.png" alt="" width="40" height="40" class="auth-brand-icon">
      <div>
        <p class="auth-brand-name">
          GenMod
        </p>
        <p class="auth-brand-caption">
          MOD 创作工作台
        </p>
      </div>
    </div>

    <div class="auth-intro">
      <p class="auth-eyebrow">
        A SPACE TO CREATE
      </p>
      <h1>把灵感，<br>写进<span>游戏。</span></h1>
      <p class="auth-intro-description">
        从一个好玩的想法开始，构建你的下一种玩法。
      </p>
    </div>

    <div class="auth-assembly" aria-hidden="true">
      <div class="assembly-grid" />
      <div class="assembly-plane assembly-plane-bottom" />
      <div class="assembly-plane assembly-plane-top">
        <span class="plane-cross plane-cross-start" />
        <span class="plane-cross plane-cross-end" />
      </div>
      <div class="assembly-tile assembly-tile-one" />
      <div class="assembly-tile assembly-tile-two" />
      <div class="assembly-tile assembly-tile-three" />
      <div class="assembly-tile assembly-tile-four" />
      <div class="assembly-core">
        <img src="/gen-mod-icon.png" alt="" width="208" height="208" draggable="false">
      </div>
      <span class="assembly-axis assembly-axis-x">X</span>
      <span class="assembly-axis assembly-axis-y">Y</span>
      <span class="assembly-coordinate">BUILD SOMETHING PLAYFUL</span>
    </div>

    <p class="auth-scene-footer">
      <span />
      小小改动，也能带来全新体验。
    </p>
  </section>
</template>

<style scoped>
.auth-scene {
  --scene-ink: oklch(0.3 0.028 175);
  --scene-accent: oklch(0.58 0.095 175);
  --scene-mint: oklch(0.88 0.075 168);
  position: relative;
  display: flex;
  min-width: 0;
  flex-direction: column;
  align-items: flex-start;
  padding-block: 12px;
}

.auth-brand {
  display: flex;
  align-items: center;
  gap: 12px;
}

.auth-brand-icon {
  flex-shrink: 0;
}
.auth-brand-name {
  color: var(--scene-ink);
  font-size: 23px;
  font-weight: 650;
  letter-spacing: -0.035em;
  line-height: 1.15;
}
.auth-brand-caption {
  margin-top: 4px;
  color: var(--muted-foreground);
  font-size: 11px;
  letter-spacing: 0.08em;
}
.auth-intro {
  margin-top: clamp(36px, 6vh, 64px);
}
.auth-eyebrow {
  color: var(--scene-accent);
  font-family: Consolas, monospace;
  font-size: 10px;
  letter-spacing: 0.18em;
}
.auth-intro h1 {
  margin-top: 16px;
  color: var(--scene-ink);
  font-size: clamp(34px, 3.8vw, 50px);
  font-weight: 650;
  line-height: 1.42;
  letter-spacing: -0.025em;
  text-wrap: balance;
}
.auth-intro h1 span {
  color: var(--scene-accent);
}
.auth-intro-description {
  max-width: 28em;
  margin-top: 18px;
  color: var(--muted-foreground);
  font-size: 13px;
  line-height: 1.9;
}

.auth-assembly {
  position: relative;
  width: min(100%, 430px);
  height: 270px;
  margin-top: 10px;
  perspective: 900px;
}
.assembly-grid {
  position: absolute;
  inset: 12px -24px -12px -48px;
  background-image: radial-gradient(color-mix(in oklch, var(--scene-ink) 13%, transparent) 1px, transparent 1px);
  background-size: 20px 20px;
  mask-image: radial-gradient(ellipse at center, black 15%, transparent 70%);
}
.assembly-plane {
  position: absolute;
  left: 80px;
  top: 70px;
  width: 258px;
  height: 220px;
  border: 1px solid color-mix(in oklch, var(--scene-accent) 22%, transparent);
  border-radius: 18px;
  transform: rotateX(58deg) rotateZ(-35deg);
}
.assembly-plane-bottom {
  top: 95px;
  background: color-mix(in oklch, var(--scene-mint) 13%, transparent);
}
.assembly-plane-top {
  background: linear-gradient(
    135deg,
    color-mix(in oklch, var(--scene-mint) 24%, transparent),
    color-mix(in oklch, var(--background) 55%, transparent)
  );
  box-shadow: 10px 14px 32px color-mix(in oklch, var(--scene-ink) 5%, transparent);
}
.plane-cross {
  position: absolute;
  width: 10px;
  height: 10px;
}
.plane-cross::before,
.plane-cross::after {
  position: absolute;
  content: '';
  background: color-mix(in oklch, var(--scene-accent) 55%, transparent);
}
.plane-cross::before {
  top: 4px;
  width: 10px;
  height: 1px;
}
.plane-cross::after {
  left: 4px;
  height: 10px;
  width: 1px;
}
.plane-cross-start {
  top: 18px;
  left: 18px;
}
.plane-cross-end {
  right: 18px;
  bottom: 18px;
}
.assembly-core {
  position: absolute;
  left: 120px;
  top: 25px;
  width: 208px;
  transform: rotate(-9deg);
  animation: core-hover 9s ease-in-out infinite;
}
.assembly-core img {
  display: block;
  width: 100%;
  height: auto;
  filter: drop-shadow(5px 20px 13px color-mix(in oklch, var(--scene-ink) 18%, transparent));
}
.assembly-tile {
  position: absolute;
  width: 40px;
  height: 40px;
  border: 1px solid color-mix(in oklch, var(--scene-accent) 28%, transparent);
  border-radius: 8px;
  background: color-mix(in oklch, var(--scene-mint) 65%, var(--background));
  box-shadow: 5px 6px 0 color-mix(in oklch, var(--scene-accent) 14%, var(--background));
  transform: rotate(-35deg) skewY(12deg);
}
.assembly-tile-one {
  top: 60px;
  left: 50px;
  width: 32px;
  height: 32px;
}
.assembly-tile-two {
  top: 212px;
  left: 88px;
  width: 26px;
  height: 26px;
  background: var(--background);
}
.assembly-tile-three {
  top: 118px;
  right: 22px;
}
.assembly-tile-four {
  top: 34px;
  right: 46px;
  width: 16px;
  height: 16px;
  background: var(--background);
  box-shadow: 3px 4px 0 color-mix(in oklch, var(--scene-accent) 9%, var(--background));
}
.assembly-axis {
  position: absolute;
  color: color-mix(in oklch, var(--scene-accent) 70%, var(--foreground));
  font-family: Consolas, monospace;
  font-size: 9px;
}
.assembly-axis-x {
  left: 30px;
  top: 170px;
}
.assembly-axis-y {
  right: 4px;
  top: 200px;
}
.assembly-coordinate {
  position: absolute;
  left: 128px;
  bottom: 0;
  color: var(--muted-foreground);
  font-family: Consolas, monospace;
  font-size: 8px;
  letter-spacing: 0.13em;
}
.auth-scene-footer {
  display: flex;
  align-items: center;
  gap: 9px;
  margin-top: 18px;
  color: var(--muted-foreground);
  font-size: 11px;
}
.auth-scene-footer span {
  width: 5px;
  height: 5px;
  background: var(--scene-accent);
  border-radius: 1px;
  transform: rotate(45deg);
}

@keyframes core-hover {
  0%,
  100% {
    transform: translateY(0) rotate(-9deg);
  }
  50% {
    transform: translateY(-8px) rotate(-7deg);
  }
}
@media (max-width: 900px) {
  .auth-scene {
    align-items: center;
    padding: 0;
  }
  .auth-intro,
  .auth-assembly,
  .auth-scene-footer {
    display: none;
  }
}
@media (prefers-reduced-motion: reduce) {
  .assembly-core {
    animation: none;
  }
}
</style>
